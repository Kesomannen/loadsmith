//! Core types, traits, and errors for the loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the [`loadsmith`](https://crates.io/crates/loadsmith) facade crate instead of using this
//! crate directly.

use std::{fmt::Display, str::FromStr};

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod checksum;
mod error;
mod url;

pub use checksum::{Checksum, ChecksumAlgorithm};
pub use error::{Error, Result};
pub use semver::{Version, VersionReq};
pub use url::FileUrl;

/// An identifier for a package.
///
/// Designed to be used with any package registry, and does not enforce any specific format or naming convention.
/// This is currently a plain string wrapper, but may in the future require validation to, for example, forbid invalid path characters.
///
/// Does not include a version; pair with [`PackageRef`] for a specific version of a package.
///
/// # Examples
///
/// ```
/// # use loadsmith_core::PackageId;
/// let id = PackageId::new("BepInEx-BepInExPack");
/// assert_eq!(id.as_str(), "BepInEx-BepInExPack");
///
/// let id = PackageId::from(String::new("MyCoolModpack"));
/// assert_eq!(id.as_str(), "MyCoolModpack");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct PackageId(String);

impl PackageId {
    /// Create a new [`PackageId`] from a type that implements [`Into<String>`].
    ///
    /// This does not copy the `String` as long as the [`Into<String>`] implementation does not.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the [`PackageId`] and return the inner [`String`].
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for PackageId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for PackageId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for PackageId {
    fn from(id: &str) -> Self {
        id.to_string().into()
    }
}

impl Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An identifier of a specific version of a package.
///
/// This is created by combining a [`PackageId`] with a semver-[`Version`], and is formatted as `{id}@{version}`.
///
/// # Example
///
/// ```
/// # use loadsmith_core::{PackageRef, Version};
/// let pkg = PackageRef::new("denikson-BepInExPack_Valheim", Version::new(5, 4, 2202));
/// assert_eq!(pkg.to_string(), "denikson-BepInExPack_Valheim@5.4.2202");
/// assert_eq!(pkg.id().as_str(), "denikson-BepInExPack_Valheim");
/// assert_eq!(pkg.version().to_string(), "5.4.2202");
///
/// let (id, ver) = pkg.into_split();
/// assert_eq!(id.as_str(), "denikson-BepInExPack_Valheim");
/// assert_eq!(ver.to_string(), "5.4.2202");
///
/// let parsed: PackageRef = "Team17-Valheim@0.220.3".parse().unwrap();
/// assert_eq!(parsed.id().as_str(), "Team17-Valheim");
/// assert_eq!(parsed.version().to_string(), "0.220.3");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(into = "String", try_from = "&str")]
pub struct PackageRef {
    id: PackageId,
    version: Version,
}

impl PackageRef {
    /// Create a new reference from a package identifier and version.
    ///
    /// As long as the [`Into<PackageId>`] and [`Into<Version>`] implementations do not copy, this does not copy either value.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// let pkg = PackageRef::new("denikson-BepInExPack_Valheim", Version::new(5, 4, 2202));
    /// assert_eq!(pkg.id().as_str(), "denikson-BepInExPack_Valheim");
    /// assert_eq!(pkg.version().to_string(), "5.4.2202");
    /// ```
    pub fn new(id: impl Into<PackageId>, version: impl Into<Version>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
    }

    /// Borrow the package identifier, excluding the version.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// let pkg = PackageRef::new("EvaisaDev-LethalLib", Version::new(1, 1, 13));
    /// assert_eq!(pkg.id().as_str(), "EvaisaDev-LethalLib");
    /// ```
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    /// Borrow the package version.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// let pkg = PackageRef::new("EvaisaDev-LethalLib", Version::new(1, 1, 13));
    /// assert_eq!(pkg.version().to_string(), "1.1.13");
    /// ```
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Consume the [`PackageRef`] and return the package identifier.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// let pkg = PackageRef::new("EvaisaDev-LethalLib", Version::new(1, 1, 13));
    /// let id = pkg.into_id();
    /// assert_eq!(id.as_str(), "EvaisaDev-LethalLib");
    /// ```
    pub fn into_id(self) -> PackageId {
        self.id
    }

    /// Consume the [`PackageRef`] and return the version.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// let pkg = PackageRef::new("EvaisaDev-LethalLib", Version::new(1, 1, 13));
    /// let version = pkg.into_version();
    /// assert_eq!(version.to_string(), "1.1.13");
    /// ```
    pub fn into_version(self) -> Version {
        self.version
    }

    /// Consume the [`PackageRef`] and return both parts as a tuple.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, Version};
    /// let pkg = PackageRef::new("EvaisaDev-LethalLib", Version::new(1, 1, 13));
    /// let (id, version) = pkg.into_split();
    /// assert_eq!(id.as_str(), "EvaisaDev-LethalLib");
    /// assert_eq!(version.to_string(), "1.1.13");
    /// ```
    pub fn into_split(self) -> (PackageId, Version) {
        (self.id, self.version)
    }
}

impl Display for PackageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

impl FromStr for PackageRef {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut split = s.split('@');

        let id = split.next().ok_or(Error::InvalidPackageRefFormat)?;
        let version = split.next().ok_or(Error::InvalidPackageRefFormat)?;

        if split.next().is_some() {
            return Err(Error::InvalidPackageRefFormat);
        }

        let id = PackageId::new(id);
        let version = Version::from_str(version)?;

        Ok(Self::new(id, version))
    }
}

impl From<PackageRef> for String {
    fn from(package_ref: PackageRef) -> Self {
        package_ref.to_string()
    }
}

impl TryFrom<&str> for PackageRef {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        s.parse()
    }
}

/// A record of a package installation at a specific point in time.
///
/// Tracks the package reference (a [`PackageRef`]), install date, file inventory, and an
/// optional [`Checksum`] for integrity verification.
///
/// # Example
///
/// ```
/// # use loadsmith_core::*;
/// let pkg = PackageRef::new("MyMod", Version::new(5, 4, 2202));
/// let file = InstalledFile::new("BepInEx/plugins/MyMod.dll", true);
///
/// let record = InstalledPackage::now(pkg, vec![file], None);
/// assert_eq!(record.ref_().id().as_str(), "MyMod");
/// assert_eq!(record.files().len(), 1);
/// assert!(record.checksum().is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackage {
    #[serde(rename = "package")]
    ref_: PackageRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    checksum: Option<Checksum>,
    date: DateTime<Utc>,
    files: Vec<InstalledFile>,
}

/// A single file belonging to an installed package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFile {
    relative_path: Utf8PathBuf,
    linked: bool,
}

impl InstalledPackage {
    /// Create a new install record with the given timestamp.
    pub fn new(
        ref_: impl Into<PackageRef>,
        files: Vec<InstalledFile>,
        checksum: Option<Checksum>,
        date: DateTime<Utc>,
    ) -> Self {
        Self {
            ref_: ref_.into(),
            files,
            checksum,
            date,
        }
    }

    /// Create a new install record with the current timestamp.
    pub fn now(
        ref_: impl Into<PackageRef>,
        files: Vec<InstalledFile>,
        checksum: Option<Checksum>,
    ) -> Self {
        Self::new(ref_, files, checksum, Utc::now())
    }

    /// Borrows the package reference.
    pub fn ref_(&self) -> &PackageRef {
        &self.ref_
    }

    /// Borrows the list of installed files belonging to this package.
    pub fn files(&self) -> &[InstalledFile] {
        &self.files
    }

    /// Mutate the list of installed files.
    pub fn files_mut(&mut self) -> &mut Vec<InstalledFile> {
        &mut self.files
    }

    /// Borrows the installation timestamp.
    pub fn date(&self) -> &DateTime<Utc> {
        &self.date
    }

    /// Borrows the optional checksum.
    pub fn checksum(&self) -> Option<&Checksum> {
        self.checksum.as_ref()
    }
}

impl InstalledFile {
    /// Create a new installed file entry.
    ///
    /// - `relative_path` is expected to be relative to the package install root, commonly referred to as "profile".
    /// - `linked` indicates whether the file is a link to some central location rather than a full copy of the file.
    pub fn new(relative_path: impl Into<Utf8PathBuf>, linked: bool) -> Self {
        Self {
            relative_path: relative_path.into(),
            linked,
        }
    }

    /// Borrow the relative path of the file.
    pub fn relative_path(&self) -> &Utf8PathBuf {
        &self.relative_path
    }

    /// Whether this file is a link to some central location rather than a full copy of the file.
    pub fn linked(&self) -> bool {
        self.linked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[test]
    fn package_ref_from_str() {
        let package_ref: PackageRef = "author-name@1.2.3"
            .parse()
            .expect("failed to parse package ref");
        assert_eq!(package_ref.id.as_str(), "author-name");
        assert_eq!(package_ref.version, Version::new(1, 2, 3));
    }

    #[test]
    fn package_ref_from_str_two_ats() {
        let res: Result<PackageRef> = "author-name@1.2.3@4.5.6".parse();
        assert_matches!(res, Err(Error::InvalidPackageRefFormat));
    }

    #[test]
    fn package_ref_from_str_invalid() {
        let res: Result<PackageRef> = "author-name-1.2.3".parse();
        assert_matches!(res, Err(Error::InvalidPackageRefFormat));
    }

    #[test]
    fn package_id_from_str() {
        let package_id: PackageId = PackageId::new("author-name");
        assert_eq!(package_id.as_str(), "author-name");
    }
}
