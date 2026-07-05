use std::{collections::HashMap, ffi::OsString, fmt::Display, path::PathBuf, str::FromStr};

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

impl FromStr for PackageId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(s.to_string().into())
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
        let (id, version) = s.rsplit_once('@').ok_or(Error::InvalidPackageRefFormat)?;

        let id = PackageId::from_str(id)?;
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LaunchArgs {
    pub args: Vec<OsString>,
    pub env: HashMap<OsString, OsString>,
    pub wrapper: Option<PathBuf>,
}

impl LaunchArgs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn wrapper(mut self, wrapper: impl Into<PathBuf>) -> Self {
        self.wrapper = Some(wrapper.into());
        self
    }

    pub fn apply(self, command: &mut std::process::Command) {
        if let Some(wrapper) = self.wrapper {
            let mut new_command = std::process::Command::new(wrapper);
            new_command
                .arg(command.get_program())
                .args(command.get_args());

            for (key, value) in command.get_envs() {
                if let Some(value) = value {
                    new_command.env(key, value);
                } else {
                    new_command.env_remove(key);
                }
            }

            *command = new_command;
        }

        for arg in self.args {
            command.arg(arg);
        }

        for (key, value) in self.env {
            command.env(key, value);
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
    fn package_ref_from_str_invalid() {
        let res: Result<PackageRef> = "author-name-1.2.3".parse();
        assert_matches!(res, Err(Error::InvalidPackageRefFormat));
    }

    #[test]
    fn package_id_from_str() {
        let package_id: PackageId = "author-name".parse().expect("failed to parse package id");
        assert_eq!(package_id.as_str(), "author-name");
    }
}
