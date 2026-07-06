use std::{cmp::Ordering, fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Copy, Hash)]
#[serde(into = "String", try_from = "&str")]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl From<(u64, u64, u64)> for Version {
    fn from((major, minor, patch): (u64, u64, u64)) -> Self {
        Self::new(major, minor, patch)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s
            .split('.')
            .map(|s| s.parse::<u64>().map_err(Error::InvalidVersionPart));

        let major = parts.next().ok_or(Error::InvalidVersionFormat)??;
        let minor = parts.next().ok_or(Error::InvalidVersionFormat)??;
        let patch = parts.next().ok_or(Error::InvalidVersionFormat)??;

        if parts.next().is_some() {
            return Err(Error::InvalidVersionFormat);
        }

        Ok(Self::new(major, minor, patch))
    }
}

impl From<Version> for String {
    fn from(version: Version) -> Self {
        version.to_string()
    }
}

impl TryFrom<&str> for Version {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<semver::Version> for Version {
    fn from(version: semver::Version) -> Self {
        Self::new(version.major, version.minor, version.patch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "&str")]
pub enum VersionRange {
    Any,
    Exact(Version),
}

impl VersionRange {
    pub fn any() -> Self {
        VersionRange::Any
    }

    pub fn exact(version: impl Into<Version>) -> Self {
        VersionRange::Exact(version.into())
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionRange::Any => true,
            VersionRange::Exact(v) => v == version,
        }
    }

    pub fn is_any(&self) -> bool {
        matches!(self, VersionRange::Any)
    }
}

impl From<Version> for VersionRange {
    fn from(version: Version) -> Self {
        VersionRange::Exact(version)
    }
}

impl FromStr for VersionRange {
    type Err = Error;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            Ok(VersionRange::Any)
        } else {
            if s.starts_with('=') {
                s = &s[1..];
            }

            let version = s.parse::<Version>()?;
            Ok(VersionRange::Exact(version))
        }
    }
}

impl TryFrom<&str> for VersionRange {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Display for VersionRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionRange::Any => write!(f, "*"),
            VersionRange::Exact(v) => write!(f, "={v}"),
        }
    }
}

impl From<VersionRange> for String {
    fn from(range: VersionRange) -> Self {
        range.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_matches;

    #[test]
    fn version_from_str() {
        let version: Version = "1.2.3".parse().unwrap();
        assert_eq!(version, Version::new(1, 2, 3));
    }

    #[test]
    fn version_from_str_invalid() {
        let version: Result<Version, _> = "1.2".parse();
        assert_matches!(version, Err(Error::InvalidVersionFormat));
    }

    #[test]
    fn version_from_str_invalid_part() {
        let version: Result<Version, _> = "1.2.x".parse();
        assert_matches!(version, Err(Error::InvalidVersionPart(_)));
    }

    #[test]
    fn version_from_str_extra_part() {
        let version: Result<Version, _> = "1.2.3.4".parse();
        assert_matches!(version, Err(Error::InvalidVersionFormat));
    }

    #[test]
    fn version_ordering() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);
        let v3 = Version::new(1, 3, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);

        assert!(v1 < v3);
        assert!(v2 < v3);

        assert!(v1 < v4);
        assert!(v3 < v4);
        assert!(v2 < v4);
    }

    #[test]
    fn version_from_semver() {
        let semver = semver::Version::new(1, 2, 3);
        let version: Version = semver.into();
        assert_eq!(version, Version::new(1, 2, 3));
    }

    #[test]
    fn version_range_matches() {
        let range_any = VersionRange::Any;
        let range_exact = VersionRange::Exact(Version::new(1, 2, 3));

        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);

        assert!(range_any.matches(&v1));
        assert!(range_any.matches(&v2));

        assert!(range_exact.matches(&v1));
        assert!(!range_exact.matches(&v2));
    }

    #[test]
    fn version_range_display() {
        let range_any = VersionRange::Any;
        let range_exact = VersionRange::Exact(Version::new(1, 2, 3));

        assert_eq!(range_any.to_string(), "*");
        assert_eq!(range_exact.to_string(), "=1.2.3");
    }

    #[test]
    fn version_range_from_str() {
        let range: VersionRange = "*".parse().unwrap();
        assert_eq!(range, VersionRange::Any);

        let range: VersionRange = "1.2.3".parse().unwrap();
        assert_eq!(range, VersionRange::Exact(Version::new(1, 2, 3)));

        let range: VersionRange = "=1.2.3".parse().unwrap();
        assert_eq!(range, VersionRange::Exact(Version::new(1, 2, 3)));
    }
}
