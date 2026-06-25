use std::{collections::HashMap, ffi::OsString, fmt::Display, path::PathBuf};

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageRef {
    pub id: PackageId,
    pub version: String,
}

impl PackageRef {
    pub fn new(id: impl Into<PackageId>, version: impl Into<String>) -> Self {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPackage {
    pub package: PackageRef,
    pub files: Vec<Utf8PathBuf>,
    pub date: DateTime<Utc>,
}

impl InstalledPackage {
    pub fn now(package: PackageRef, files: Vec<Utf8PathBuf>) -> Self {
        Self {
            package,
            files,
            date: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoaderId(pub &'static str);

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
}
