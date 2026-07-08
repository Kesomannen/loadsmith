use std::{fmt::Display, str::FromStr};

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod error;
mod version;

pub use error::{Error, Result};
pub use version::{Version, VersionRange};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct PackageId(pub String);

impl PackageId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
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

impl Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "&str")]
pub struct PackageRef {
    pub id: PackageId,
    pub version: Version,
}

impl PackageRef {
    pub fn new(id: impl Into<PackageId>, version: impl Into<Version>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackage {
    #[serde(rename = "package")]
    pub ref_: PackageRef,
    pub files: Vec<InstalledFile>,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFile {
    pub relative_path: Utf8PathBuf,
    pub linked: bool,
}

impl InstalledPackage {
    pub fn now(ref_: PackageRef, files: Vec<InstalledFile>) -> Self {
        Self {
            ref_,
            files,
            date: Utc::now(),
        }
    }
}

impl InstalledFile {
    pub fn new(relative_path: impl Into<Utf8PathBuf>, linked: bool) -> Self {
        Self {
            relative_path: relative_path.into(),
            linked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub id: PackageId,
    pub version_range: VersionRange,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_metadata: Option<serde_json::Value>,
}

impl Dependency {
    pub fn new(
        id: impl Into<PackageId>,
        version_range: impl Into<VersionRange>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            version_range: version_range.into(),
            source: source.into(),
            registry_metadata: None,
        }
    }

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
