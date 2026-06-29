use loadsmith_core::{LockedPackage, PackageId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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

    pub fn package_by_id_and_source(&self, id: &PackageId, source: &str) -> Option<&LockedPackage> {
        self.packages
            .iter()
            .find(|locked| locked.package == *id && locked.source == *source)
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::Version;

    use super::*;

    #[test]
    fn package_by_id_and_source() {
        let package_id = PackageId::new("author-name");
        let locked_package = LockedPackage {
            package: package_id.clone(),
            version: Version::new(1, 0, 0),
            source: "registry".to_string(),
            url: "https://example.com/package.tar.gz".to_string(),
            checksum: None,
            deps: vec![],
        };

        let lockfile = Lockfile::new(vec![locked_package.clone()]);

        let found_package = lockfile.package_by_id_and_source(&package_id, "registry");
        assert_eq!(found_package.unwrap(), &locked_package);

        let not_found_package = lockfile.package_by_id_and_source(&package_id, "other-registry");
        assert!(not_found_package.is_none());
    }
}
