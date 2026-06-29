use std::{collections::HashMap, pin::Pin};

use loadsmith_core::{PackageId, PackageRef, Version};
use serde::{Deserialize, Serialize};

use crate::{Registry, RegistryId, Result};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub version: Version,
    pub download_url: String,
    #[serde(default)]
    pub checksum: Option<String>,
    pub deps: Vec<PackageRef>,
}

impl PackageVersion {
    pub fn new(version: impl Into<Version>, download_url: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            download_url: download_url.into(),
            checksum: None,
            deps: Vec::new(),
        }
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    pub fn with_deps(mut self, deps: Vec<PackageRef>) -> Self {
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
}

impl Registry for OfflineRegistry {
    fn id(&self) -> RegistryId {
        RegistryId("offline")
    }

    fn get_package<'a>(
        &'a self,
        id: &'a PackageId,
        _metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<crate::Package>>> + 'a>> {
        Box::pin(async move {
            let Some(package) = self.packages.get(id) else {
                return Ok(None);
            };

            let versions = package.versions.iter().map(|v| crate::PackageVersion {
                version: v.version,
                download_url: v.download_url.clone(),
                checksum: v.checksum.clone(),
                deps: v.deps.iter().map(|dep| dep.id.clone()).collect(),
            });

            let package = crate::Package {
                versions: versions.collect(),
            };

            Ok(Some(package))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let package = registry
            .get_package(&PackageId::new("author-name"), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(package.versions.len(), 2);

        let version_1_0_0 = package
            .versions
            .iter()
            .find(|v| v.version == Version::new(1, 0, 0))
            .unwrap();

        assert_eq!(
            version_1_0_0.download_url,
            "https://example.com/package-1.0.0.zip"
        );

        let version_1_1_0 = package
            .versions
            .iter()
            .find(|v| v.version == Version::new(1, 1, 0))
            .unwrap();

        assert_eq!(
            version_1_1_0.download_url,
            "https://example.com/package-1.1.0.zip"
        );

        let non_existent_package = registry
            .get_package(&PackageId::new("non-existent"), None)
            .await
            .unwrap();

        assert!(non_existent_package.is_none());
    }
}
