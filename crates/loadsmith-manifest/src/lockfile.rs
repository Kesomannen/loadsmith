use std::collections::HashMap;

use loadsmith_core::{Checksum, FileUrl, PackageId, PackageRef};
use loadsmith_registry::Dependency;
use serde::{Deserialize, Serialize};

use crate::{Diff, Diffable, PackageStoreEntry};

/// A complete snapshot of every package that should be installed for a profile.
///
/// Lockfiles are produced by [`resolve`](crate::resolve) and can be diffed
/// against each other to determine what needs to be added, removed, or updated.
///
/// # Examples
///
/// ```rust
/// use loadsmith_core::{FileUrl, PackageRef, Version};
/// use loadsmith_manifest::{LockedPackage, Lockfile};
///
/// let url = FileUrl::try_from_url("https://example.com/pkg.zip").unwrap();
/// let lockfile = Lockfile::new(vec![
///     LockedPackage::new(PackageRef::new("Author-Pkg", Version::new(1, 0, 0)), "thunderstore", url),
/// ]);
///
/// assert_eq!(lockfile.packages().len(), 1);
/// assert!(lockfile.package_by_id(&PackageRef::new("Author-Pkg", Version::new(1, 0, 0)).id()).is_some());
/// ```
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Lockfile {
    packages: Vec<LockedPackage>,
}

impl Lockfile {
    /// Create a new lockfile from a list of locked packages.
    pub fn new(packages: Vec<LockedPackage>) -> Self {
        Self { packages }
    }

    /// Borrow the list of all locked packages in this lockfile.
    pub fn packages(&self) -> &[LockedPackage] {
        &self.packages
    }

    /// Look up a locked package by its [`PackageId`].
    ///
    /// Returns `None` if the package is not present.
    pub fn package_by_id(&self, id: &PackageId) -> Option<&LockedPackage> {
        self.packages.iter().find(|locked| locked.ref_.id() == id)
    }

    pub(crate) fn id_to_package_map(&self) -> HashMap<&PackageId, &LockedPackage> {
        self.packages.iter().map(|p| (p.ref_.id(), p)).collect()
    }

    /// Compute the difference between this lockfile and a newer one.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loadsmith_core::{FileUrl, PackageRef, Version};
    /// use loadsmith_manifest::{LockedPackage, Lockfile};
    ///
    /// let url = FileUrl::try_from_url("https://example.com/pkg.zip").unwrap();
    ///
    /// let old = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Author-A", Version::new(1, 0, 0)), "ts", url.clone()),
    /// ]);
    ///
    /// let new = Lockfile::new(vec![
    ///     LockedPackage::new(PackageRef::new("Author-B", Version::new(1, 0, 0)), "ts", url),
    /// ]);
    ///
    /// let diff = old.diff(&new);
    /// assert_eq!(diff.added.len(), 1);
    /// assert_eq!(diff.removed.len(), 1);
    /// ```
    pub fn diff<'a>(&'a self, new: &'a Lockfile) -> Diff<'a, LockedPackage, LockedPackage> {
        Diff::compute(self.id_to_package_map(), new.id_to_package_map())
    }
}

/// A single resolved package entry inside a [`Lockfile`].
///
/// Each entry records the exact version, download URL, checksum, dependencies,
/// and metadata needed to install and verify the package.
///
/// # Examples
///
/// ```rust
/// use loadsmith_core::{FileUrl, PackageRef, Version};
/// use loadsmith_manifest::LockedPackage;
///
/// let url = FileUrl::try_from_url("https://example.com/pkg.zip").unwrap();
/// let pkg = LockedPackage::new(PackageRef::new("Author-Mod", Version::new(2, 1, 0)), "thunderstore", url)
///     .with_transitive(false)
///     .with_size(42_000);
///
/// assert_eq!(pkg.ref_.version().to_string(), "2.1.0");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedPackage {
    #[serde(rename = "package")]
    pub ref_: PackageRef,
    pub source: String,
    pub url: FileUrl,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub transitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_metadata: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<Dependency>,
}

impl LockedPackage {
    /// Create a new locked package from a [`PackageRef`], source name, and
    /// download URL.
    ///
    /// All optional fields (`checksum`, `size`, `deps`, etc.) default to `None`
    /// or empty. Use the builder methods (`with_*`) to populate them.
    pub fn new(
        package: impl Into<PackageRef>,
        source: impl Into<String>,
        url: impl Into<FileUrl>,
    ) -> Self {
        Self {
            ref_: package.into(),
            source: source.into(),
            url: url.into(),
            transitive: false,
            registry_metadata: None,
            size: None,
            checksum: None,
            deps: Vec::new(),
        }
    }

    /// Set whether this package was pulled in transitively (as a dependency
    /// of another package) rather than requested directly.
    pub fn with_transitive(mut self, transitive: bool) -> Self {
        self.transitive = transitive;
        self
    }

    /// Attach a known download size (in bytes) to this entry.
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    /// Attach a checksum to this entry for integrity verification.
    pub fn with_checksum(mut self, checksum: impl Into<Checksum>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    /// Attach the list of dependencies this package declared.
    pub fn with_deps(mut self, deps: Vec<Dependency>) -> Self {
        self.deps = deps;
        self
    }

    /// Attach arbitrary registry-specific metadata (e.g. Thunderstore community
    /// info) to this entry.
    pub fn with_registry_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.registry_metadata = Some(metadata);
        self
    }

    /// Create a [`PackageStoreEntry`](crate::PackageStoreEntry) referencing
    /// this locked package (borrows the ref and checksum).
    pub fn store_entry(&self) -> PackageStoreEntry {
        PackageStoreEntry::new(self.ref_.clone(), self.checksum.clone())
    }

    /// Convert this locked package into a [`PackageStoreEntry`](crate::PackageStoreEntry),
    /// consuming it in the process.
    pub fn into_store_entry(self) -> PackageStoreEntry {
        PackageStoreEntry::new(self.ref_, self.checksum)
    }
}

impl Diffable for LockedPackage {
    /// Returns the checksum stored on this locked package, if any.
    fn checksum(&self) -> Option<&Checksum> {
        self.checksum.as_ref()
    }

    /// Returns the version of the package reference.
    fn version(&self) -> &loadsmith_core::Version {
        self.ref_.version()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use loadsmith_core::{PackageRef, Version};

    #[test]
    fn diff() {
        let example_com = FileUrl::try_from_url("https://example.com").unwrap();

        let a = Lockfile::new(vec![
            LockedPackage::new(
                PackageRef::new("A".to_string(), Version::new(1, 0, 0)),
                "local",
                example_com.clone(),
            ),
            LockedPackage::new(
                PackageRef::new("B".to_string(), Version::new(1, 0, 0)),
                "local",
                example_com.clone(),
            ),
        ]);

        let b = Lockfile::new(vec![
            LockedPackage::new(
                PackageRef::new("B".to_string(), Version::new(2, 0, 0)),
                "local",
                example_com.clone(),
            ),
            LockedPackage::new(
                PackageRef::new("C".to_string(), Version::new(1, 0, 0)),
                "local",
                example_com.clone(),
            ),
        ]);

        let diff = a.diff(&b);

        assert_eq!(
            diff.added,
            vec![&LockedPackage::new(
                PackageRef::new("C".to_string(), Version::new(1, 0, 0)),
                "local",
                example_com.clone(),
            )]
        );
        assert_eq!(
            diff.removed,
            vec![&LockedPackage::new(
                PackageRef::new("A".to_string(), Version::new(1, 0, 0)),
                "local",
                example_com.clone(),
            )]
        );
        assert_eq!(
            diff.changed,
            vec![(
                &LockedPackage::new(
                    PackageRef::new("B".to_string(), Version::new(1, 0, 0)),
                    "local",
                    example_com.clone(),
                ),
                &LockedPackage::new(
                    PackageRef::new("B".to_string(), Version::new(2, 0, 0)),
                    "local",
                    example_com.clone(),
                )
            )]
        );
    }
}
