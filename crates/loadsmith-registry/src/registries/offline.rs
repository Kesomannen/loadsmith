use std::{collections::HashMap, pin::Pin};

use loadsmith_core::{Checksum, Dependency, FileUrl, PackageId, PackageRef, Version};
use serde::{Deserialize, Serialize};

use crate::{Error, Registry, ResolvedVersion, Result, VersionInfo};

/// A package definition used with [`OfflineRegistry`].
///
/// Holds a package identifier and a list of available versions.
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::Package;
/// use loadsmith_core::PackageId;
///
/// let pkg = Package::new(PackageId::new("author-name"), Vec::new());
/// assert_eq!(pkg.id.as_str(), "author-name");
/// assert!(pkg.versions.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: PackageId,
    pub versions: Vec<PackageVersion>,
}

impl Package {
    /// Create a new package with the given identifier and version list.
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

/// A single version entry inside an [`OfflineRegistry`] package.
///
/// Includes the download URL, optional size and checksum, and the dependency
/// list. Builder methods ([`with_size`](PackageVersion::with_size),
/// [`with_checksum`](PackageVersion::with_checksum),
/// [`with_deps`](PackageVersion::with_deps)) are provided for ergonomic
/// construction.
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::PackageVersion;
/// use loadsmith_core::{Version, FileUrl, Dependency, VersionReq};
///
/// let dep = Dependency::new("other-mod", VersionReq::STAR, "thunderstore");
/// let pv = PackageVersion::new(
///     Version::new(1, 0, 0),
///     FileUrl::try_from_url("https://example.com/pkg.zip").unwrap(),
/// )
/// .with_size(4096)
/// .with_deps(vec![dep]);
/// assert_eq!(pv.version.to_string(), "1.0.0");
/// assert_eq!(pv.size, Some(4096));
/// assert_eq!(pv.deps.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub version: Version,
    pub url: FileUrl,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub checksum: Option<Checksum>,
    pub deps: Vec<Dependency>,
}

impl PackageVersion {
    /// Create a new version entry with a version number and download URL.
    ///
    /// Optional fields (`size`, `checksum`, `deps`) are initialised to `None`
    /// or an empty vector. Use the builder methods to populate them.
    pub fn new(version: impl Into<Version>, download_url: impl Into<FileUrl>) -> Self {
        Self {
            version: version.into(),
            url: download_url.into(),
            size: None,
            checksum: None,
            deps: Vec::new(),
        }
    }

    /// Set the download size in bytes.
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the cryptographic checksum.
    pub fn with_checksum(mut self, checksum: impl Into<Checksum>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Replace the dependency list.
    pub fn with_deps(mut self, deps: Vec<Dependency>) -> Self {
        self.deps = deps;
        self
    }
}

/// An in-memory registry pre-populated with static package data.
///
/// Useful for testing, offline scenarios, or mirroring the public Thunderstore
/// index. Every package and its versions are loaded up-front and stored in a
/// [`HashMap`] keyed by [`PackageId`].
///
/// # Examples
///
/// ```rust
/// use std::collections::HashMap;
/// use loadsmith_registry::{OfflineRegistry, Package, PackageVersion, Registry};
/// use loadsmith_core::{PackageId, Version, FileUrl, PackageRef};
///
/// # #[tokio::main]
/// # async fn main() {
/// let pkg = Package::new(
///     PackageId::new("author-name"),
///     vec![PackageVersion::new(
///         Version::new(1, 0, 0),
///         FileUrl::try_from_url("https://example.com/pkg.zip").unwrap(),
///     )],
/// );
/// let mut packages = HashMap::new();
/// packages.insert(PackageId::new("author-name"), pkg);
/// let registry = OfflineRegistry::new(packages);
///
/// // List versions.
/// let versions = registry
///     .version_info(&PackageId::new("author-name"), None)
///     .await
///     .unwrap();
/// assert_eq!(versions.len(), 1);
///
/// // Resolve a specific version.
/// let ref_ = PackageRef::new("author-name", Version::new(1, 0, 0));
/// let resolved = registry.resolve(&ref_, None).await.unwrap();
/// assert!(resolved.url.to_string().contains("example.com"));
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OfflineRegistry {
    packages: HashMap<PackageId, Package>,
}

impl OfflineRegistry {
    /// Create a new offline registry from a map of packages.
    ///
    /// The map is keyed by [`PackageId`] — each entry must match the `id`
    /// stored inside its [`Package`] value.
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
                .map(|v| VersionInfo {
                    version: v.version.clone(),
                })
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
                    PackageVersion::new(
                        Version::new(1, 0, 0),
                        FileUrl::try_from_url("https://example.com/package-1.0.0.zip").unwrap(),
                    ),
                    PackageVersion::new(
                        Version::new(1, 1, 0),
                        FileUrl::try_from_url("https://example.com/package-1.1.0.zip").unwrap(),
                    ),
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
