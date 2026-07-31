//! Core types, traits, and errors for the loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.
//!
//! # Examples
//!
//! ```rust
//! # use loadsmith_core::*;
//! let pkg = PackageRef::new("denikson-BepInExPack_Valheim", Version::new(5, 4, 2202));
//! assert_eq!(pkg.to_string(), "denikson-BepInExPack_Valheim@5.4.2202");
//!
//! let parsed: PackageRef = "x753-More_Suits@1.4.0".parse()?;
//! assert_eq!(parsed.id().as_str(), "x753-More_Suits");
//! assert_eq!(parsed.version().to_string(), "1.4.0");
//! # Ok::<_, loadsmith_core::Error>(())
//! ```

use std::{fmt::Display, str::FromStr};

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod checksum;
mod error;
mod url;

pub use checksum::{Checksum, ChecksumAlgorithm};
pub use error::{Error, Result};
/// Re-export of [`semver::Version`] for convenience.
///
/// Consumers can use this directly instead of adding `semver` to their own
/// `Cargo.toml`.
pub use semver::{Version, VersionReq};
pub use url::FileUrl;

/// A unique identifier for a package (e.g. `"denikson-BepInExPack_Valheim"`).
///
/// This is a plain string wrapper that does not include a version.
/// Pair it with [`PackageRef`] when you need both.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct PackageId(String);

impl PackageId {
    /// Create a new `PackageId` from anything that can become a `String`.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner identifier as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the `PackageId` and return the inner `String`.
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

/// A reference to a specific version of a package, formatted as `"<id>@<version>"`.
///
/// ```rust
/// # use loadsmith_core::{PackageRef, Version};
/// let pkg = PackageRef::new("denikson-BepInExPack_Valheim", Version::new(5, 4, 2202));
/// assert_eq!(pkg.to_string(), "denikson-BepInExPack_Valheim@5.4.2202");
/// assert_eq!(pkg.id().as_str(), "denikson-BepInExPack_Valheim");
///
/// assert_eq!(pkg.version().major, 5);
/// assert_eq!(pkg.version().minor, 4);
/// assert_eq!(pkg.version().patch, 2202);
///
/// let (id, ver) = pkg.into_split();
/// assert_eq!(id.as_str(), "denikson-BepInExPack_Valheim");
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
    pub fn new(id: impl Into<PackageId>, version: impl Into<Version>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
    }

    /// Borrow the package identifier.
    pub fn id(&self) -> &PackageId {
        &self.id
    }

    /// Borrow the package version.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Consume the reference and return the identifier.
    pub fn into_id(self) -> PackageId {
        self.id
    }

    /// Consume the reference and return the version.
    pub fn into_version(self) -> Version {
        self.version
    }

    /// Consume the reference and return both parts as a tuple.
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
/// Tracks the package reference, install date, file inventory, and an
/// optional checksum for integrity verification.
///
/// ```rust
/// # use loadsmith_core::*;
/// let pkg = PackageRef::new("denikson-BepInExPack_Valheim", Version::new(5, 4, 2202));
/// let file = InstalledFile::new("BepInEx/plugins/MyMod.dll", true);
///
/// let record = InstalledPackage::now(pkg, vec![file], None);
/// assert_eq!(record.ref_().id().as_str(), "denikson-BepInExPack_Valheim");
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
    /// Create a new install record with the current timestamp.
    ///
    /// ```rust
    /// # use loadsmith_core::*;
    /// let record = InstalledPackage::now(
    ///     PackageRef::new("x753-More_Suits", Version::new(1, 4, 0)),
    ///     vec![InstalledFile::new("BepInEx/plugins/More_Suits.dll", false)],
    ///     None,
    /// );
    /// assert_eq!(record.ref_().id().as_str(), "x753-More_Suits");
    /// assert!(record.date() <= &chrono::Utc::now());
    /// ```
    pub fn now(ref_: PackageRef, files: Vec<InstalledFile>, checksum: Option<Checksum>) -> Self {
        Self {
            ref_,
            files,
            date: Utc::now(),
            checksum,
        }
    }

    /// Borrow the package reference.
    pub fn ref_(&self) -> &PackageRef {
        &self.ref_
    }

    /// Borrow the list of installed files.
    pub fn files(&self) -> &[InstalledFile] {
        &self.files
    }

    /// Mutate the list of installed files.
    pub fn files_mut(&mut self) -> &mut Vec<InstalledFile> {
        &mut self.files
    }

    /// Borrow the installation timestamp.
    pub fn date(&self) -> &DateTime<Utc> {
        &self.date
    }

    /// Borrow the optional checksum.
    pub fn checksum(&self) -> Option<&Checksum> {
        self.checksum.as_ref()
    }
}

impl InstalledFile {
    /// Create a new installed file entry.
    ///
    /// `relative_path` is expected to be relative to the package install root.
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

    /// Whether this file is a symlink rather than a copy.
    pub fn linked(&self) -> bool {
        self.linked
    }
}

/// A declared dependency on another package.
///
/// ```rust
/// # use loadsmith_core::{Dependency, VersionReq};
/// let dep = Dependency::new(
///     "x753-More_Suits",
///     VersionReq::parse(">=1.0").unwrap(),
///     "thunderstore",
/// );
/// assert_eq!(dep.id.as_str(), "x753-More_Suits");
///
/// let dep = dep.with_registry_metadata(
///     serde_json::json!({"website_url": "https://thunderstore.io/c/valheim/"}),
/// );
/// assert!(dep.registry_metadata.is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub id: PackageId,
    pub version_req: VersionReq,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_metadata: Option<serde_json::Value>,
}

impl Dependency {
    /// Create a new dependency with the given identifier, version
    /// requirement, and source string.
    pub fn new(
        id: impl Into<PackageId>,
        version_req: impl Into<VersionReq>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            version_req: version_req.into(),
            source: source.into(),
            registry_metadata: None,
        }
    }

    /// Attach registry-specific metadata to this dependency.
    ///
    /// ```rust
    /// # use loadsmith_core::{Dependency, VersionReq};
    /// let dep = Dependency::new("denikson-BepInExPack_Valheim", VersionReq::STAR, "thunderstore")
    ///     .with_registry_metadata(serde_json::json!({"key": "val"}));
    /// assert_eq!(dep.registry_metadata.unwrap()["key"], "val");
    /// ```
    pub fn with_registry_metadata(mut self, metadata: impl Into<serde_json::Value>) -> Self {
        self.registry_metadata = Some(metadata.into());
        self
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
