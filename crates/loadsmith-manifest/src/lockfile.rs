use std::collections::HashMap;

use loadsmith_core::{Dependency, PackageId, PackageRef};
use serde::{Deserialize, Serialize};

use crate::{Diff, Diffable};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Lockfile {
    packages: Vec<LockedPackage>,
}

impl Lockfile {
    pub fn new(packages: Vec<LockedPackage>) -> Self {
        Self { packages }
    }

    pub fn packages(&self) -> &[LockedPackage] {
        &self.packages
    }

    pub fn package_by_id(&self, id: &PackageId) -> Option<&LockedPackage> {
        self.packages.iter().find(|locked| locked.ref_.id() == id)
    }

    pub(crate) fn id_to_package_map(&self) -> HashMap<&PackageId, &LockedPackage> {
        self.packages.iter().map(|p| (p.ref_.id(), p)).collect()
    }

    pub fn diff<'a>(&'a self, new: &'a Lockfile) -> Diff<'a, LockedPackage, LockedPackage> {
        Diff::compute(self.id_to_package_map(), new.id_to_package_map())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedPackage {
    #[serde(rename = "package")]
    pub ref_: PackageRef,
    pub source: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub transitive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<Dependency>,
}

impl LockedPackage {
    pub fn new(
        package: impl Into<PackageRef>,
        source: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            ref_: package.into(),
            source: source.into(),
            url: url.into(),
            transitive: false,
            size: None,
            checksum: None,
            deps: Vec::new(),
        }
    }

    pub fn with_transitive(mut self, transitive: bool) -> Self {
        self.transitive = transitive;
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }

    pub fn with_deps(mut self, deps: Vec<Dependency>) -> Self {
        self.deps = deps;
        self
    }
}

impl Diffable for LockedPackage {
    fn version(&self) -> &loadsmith_core::Version {
        self.ref_.version()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff() {
        let a = Lockfile::new(vec![
            LockedPackage::new(
                PackageRef::new("A".to_string(), (1, 0, 0)),
                "local",
                "https://example.com",
            ),
            LockedPackage::new(
                PackageRef::new("B".to_string(), (1, 0, 0)),
                "local",
                "https://example.com",
            ),
        ]);

        let b = Lockfile::new(vec![
            LockedPackage::new(
                PackageRef::new("B".to_string(), (2, 0, 0)),
                "local",
                "https://example.com",
            ),
            LockedPackage::new(
                PackageRef::new("C".to_string(), (1, 0, 0)),
                "local",
                "https://example.com",
            ),
        ]);

        let diff = a.diff(&b);

        assert_eq!(
            diff.added,
            vec![&LockedPackage::new(
                PackageRef::new("C".to_string(), (1, 0, 0)),
                "local",
                "https://example.com",
            )]
        );
        assert_eq!(
            diff.removed,
            vec![&LockedPackage::new(
                PackageRef::new("A".to_string(), (1, 0, 0)),
                "local",
                "https://example.com",
            )]
        );
        assert_eq!(
            diff.changed,
            vec![(
                &LockedPackage::new(
                    PackageRef::new("B".to_string(), (1, 0, 0)),
                    "local",
                    "https://example.com",
                ),
                &LockedPackage::new(
                    PackageRef::new("B".to_string(), (2, 0, 0)),
                    "local",
                    "https://example.com",
                )
            )]
        );
    }
}
