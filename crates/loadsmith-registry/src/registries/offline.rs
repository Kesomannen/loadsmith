use std::{collections::HashMap, pin::Pin};

use loadsmith_core::{Checksum, Dependency, PackageId, PackageRef, Version};
use serde::{Deserialize, Serialize};

use crate::{Error, Registry, ResolvedVersion, Result, VersionInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: PackageId,
    pub versions: Vec<PackageVersion>,
}

impl Package {
    pub fn new(id: impl Into<PackageId>, versions: Vec<PackageVersion>) -> Self {
        Self {
            id: id.into(),
            versions,
        }
    }

    fn version_by_version<'a>(&'a self, version: &Version) -> Result<&'a PackageVersion> {
        self.versions
            .iter()
            .find(|v| v.version == *version)
            .ok_or(Error::VersionNotFound)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub version: Version,
    pub url: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub checksum: Option<Checksum>,
    pub deps: Vec<Dependency>,
}

impl PackageVersion {
    pub fn new(version: impl Into<Version>, download_url: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            url: download_url.into(),
            size: None,
            checksum: None,
            deps: Vec::new(),
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_checksum(mut self, checksum: impl Into<Checksum>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    pub fn with_deps(mut self, deps: Vec<Dependency>) -> Self {
        self.deps = deps;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OfflineRegistry {
    packages: HashMap<PackageId, Package>,
}

impl OfflineRegistry {
    pub fn new(packages: HashMap<PackageId, Package>) -> Self {
        Self { packages }
    }

    fn package_by_id<'a>(&'a self, id: &PackageId) -> Result<&'a Package> {
        self.packages.get(id).ok_or(Error::PackageNotFound)
    }
}

impl Registry for OfflineRegistry {
    fn version_info<'a>(
        &'a self,
        id: &'a PackageId,
        _metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VersionInfo>>> + 'a>> {
        Box::pin(async move {
            let package = self.package_by_id(id)?;

            let versions = package
                .versions
                .iter()
                .map(|v| VersionInfo { version: v.version })
                .collect();

            Ok(versions)
        })
    }

    fn resolve<'a>(
        &'a self,
        ref_: &'a PackageRef,
        _metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion>> + 'a>> {
        Box::pin(async move {
            let version = self
                .package_by_id(ref_.id())?
                .version_by_version(ref_.version())?;

            Ok(ResolvedVersion {
                url: version.url.clone(),
                size: version.size,
                checksum: version.checksum.clone(),
                deps: version.deps.clone(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[tokio::test]
    async fn it_works() {
        let registry = OfflineRegistry::new(HashMap::from_iter([(
            PackageId::new("author-name"),
            Package::new(
                PackageId::new("author-name"),
                vec![
                    PackageVersion::new((1, 0, 0), "https://example.com/package-1.0.0.zip"),
                    PackageVersion::new((1, 1, 0), "https://example.com/package-1.1.0.zip"),
                ],
            ),
        )]));

        let versions = registry
            .version_info(&PackageId::new("author-name"), None)
            .await
            .unwrap();

        assert_eq!(versions.len(), 2);

        let non_existent_package = registry
            .version_info(&PackageId::new("non-existent"), None)
            .await;

        assert_matches!(non_existent_package, Err(Error::PackageNotFound));
    }
}
