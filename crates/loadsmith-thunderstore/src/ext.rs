use loadsmith_core::PackageId;
use thunderstore::{PackageIdent, VersionIdent};

use crate::{Error, Result};

pub trait PackageIdExt {
    fn into_ts_ident(self) -> Result<PackageIdent>;
    fn from_ts_ident(ident: PackageIdent) -> Self;
}

impl PackageIdExt for PackageId {
    fn into_ts_ident(self) -> Result<PackageIdent> {
        PackageIdent::try_from(self.0).map_err(Error::InvalidIdent)
    }

    fn from_ts_ident(ident: PackageIdent) -> Self {
        PackageId(ident.into_string())
    }
}

pub trait PackageRefExt {
    fn into_ts_ident(self) -> Result<VersionIdent>;
    fn from_ts_ident(ident: VersionIdent) -> Self;
}

impl PackageRefExt for loadsmith_core::PackageRef {
    fn into_ts_ident(self) -> Result<VersionIdent> {
        self.id
            .into_ts_ident()
            .map(|package| package.with_version(self.version.to_string()))
    }

    fn from_ts_ident(ident: VersionIdent) -> Self {
        Self {
            id: PackageId::from_ts_ident(ident.package_id()),
            version: ident.parsed_version().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_id_into_ts_ident() {
        let package_id = PackageId("Author-Name".to_string());
        let ts_ident = package_id.into_ts_ident().unwrap();
        assert_eq!(ts_ident.namespace(), "Author");
        assert_eq!(ts_ident.name(), "Name");
        assert_eq!(ts_ident.as_str(), "Author-Name");
    }

    #[test]
    fn package_id_into_ts_ident_hyphen_in_namespace() {
        let package_id = PackageId("Author-Name-Hyphen".to_string());
        let ts_ident = package_id.into_ts_ident().unwrap();
        assert_eq!(ts_ident.namespace(), "Author-Name");
        assert_eq!(ts_ident.name(), "Hyphen");
        assert_eq!(ts_ident.as_str(), "Author-Name-Hyphen");
    }

    #[test]
    fn package_id_from_ts_ident() {
        let ts_ident = PackageIdent::new("Author", "Name");
        let package_id = PackageId::from_ts_ident(ts_ident);
        assert_eq!(package_id.0, "Author-Name");
    }
}
