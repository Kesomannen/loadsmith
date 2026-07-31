use loadsmith_core::PackageId;
use thunderstore::{PackageIdent, VersionIdent};

use crate::{Error, Result};

/// Conversion between [`PackageId`] and [`PackageIdent`].
///
/// # Examples
///
/// ```
/// use loadsmith_core::PackageId;
/// use loadsmith_thunderstore::PackageIdExt;
/// use thunderstore::PackageIdent;
///
/// let id = PackageId::new("Author-Package");
/// let ident = id.into_ts_ident().unwrap();
/// assert_eq!(ident.namespace(), "Author");
/// assert_eq!(ident.name(), "Package");
///
/// let back = PackageId::from_ts_ident(ident);
/// assert_eq!(back.as_str(), "Author-Package");
/// ```
pub trait PackageIdExt {
    fn into_ts_ident(self) -> Result<PackageIdent>;
    fn from_ts_ident(ident: PackageIdent) -> Self;
}

impl PackageIdExt for PackageId {
    fn into_ts_ident(self) -> Result<PackageIdent> {
        PackageIdent::try_from(self.into_string()).map_err(Error::InvalidIdent)
    }

    fn from_ts_ident(ident: PackageIdent) -> Self {
        PackageId::from(ident.into_string())
    }
}

/// Conversion between [`PackageRef`](loadsmith_core::PackageRef) and [`VersionIdent`].
///
/// # Examples
///
/// ```
/// use loadsmith_core::{PackageId, PackageRef, Version};
/// use loadsmith_thunderstore::{PackageIdExt, PackageRefExt};
/// use thunderstore::VersionIdent;
///
/// let package_ref = PackageRef::new(
///     PackageId::new("Author-Package"),
///     Version::new(1, 2, 3),
/// );
///
/// let ident = package_ref.into_ts_ident().unwrap();
/// assert_eq!(ident.to_string(), "Author-Package-1.2.3");
///
/// let back = PackageRef::from_ts_ident(ident);
/// assert_eq!(back.to_string(), "Author-Package@1.2.3");
/// ```
pub trait PackageRefExt {
    fn into_ts_ident(self) -> Result<VersionIdent>;
    fn from_ts_ident(ident: VersionIdent) -> Self;
}

impl PackageRefExt for loadsmith_core::PackageRef {
    fn into_ts_ident(self) -> Result<VersionIdent> {
        let (id, version) = self.into_split();

        id.into_ts_ident()
            .map(|package| package.with_version(version.to_string()))
    }

    fn from_ts_ident(ident: VersionIdent) -> Self {
        Self::new(
            PackageId::from_ts_ident(ident.package_id()),
            ident.parsed_version(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_id_into_ts_ident() {
        let package_id = PackageId::new("Author-Name");
        let ts_ident = package_id.into_ts_ident().unwrap();
        assert_eq!(ts_ident.namespace(), "Author");
        assert_eq!(ts_ident.name(), "Name");
        assert_eq!(ts_ident.as_str(), "Author-Name");
    }

    #[test]
    fn package_id_into_ts_ident_hyphen_in_namespace() {
        let package_id = PackageId::new("Author-Name-Hyphen");
        let ts_ident = package_id.into_ts_ident().unwrap();
        assert_eq!(ts_ident.namespace(), "Author-Name");
        assert_eq!(ts_ident.name(), "Hyphen");
        assert_eq!(ts_ident.as_str(), "Author-Name-Hyphen");
    }

    #[test]
    fn package_id_from_ts_ident() {
        let ts_ident = PackageIdent::new("Author", "Name");
        let package_id = PackageId::from_ts_ident(ts_ident);
        assert_eq!(package_id.as_str(), "Author-Name");
    }
}
