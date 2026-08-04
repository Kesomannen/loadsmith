use std::{fmt::Display, str::FromStr};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Error, Result};

/// A URL or local file path that points to a package source.
///
/// URL sources are formatted as is, whereas local file paths are prefixed with `file://`.
///
/// # Example
///
/// ```
/// # use loadsmith_core::FileUrl;
/// # use std::assert_matches;
/// let url: FileUrl = "https://thunderstore.io/package/download/x753/Mimics/0.7.0"
///     .parse()
///     .unwrap();
/// assert_matches!(url, FileUrl::Url(_));
///
/// let local: FileUrl = "file:///home/user/.config/cache/x753-Mimics-0.7.0"
///     .parse()
///     .unwrap();
/// assert_matches!(local, FileUrl::Path(_));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum FileUrl {
    Url(Url),
    Path(Utf8PathBuf),
}

impl FileUrl {
    /// Parse a remote URL string into a [`FileUrl`].
    ///
    /// ```
    /// # use loadsmith_core::FileUrl;
    /// # use std::assert_matches;
    /// let url = FileUrl::try_from_url("https://thunderstore.io/api/v1/package/");
    /// assert_matches!(url, Ok(FileUrl::Url(_)));
    ///
    /// // Does not support local file paths, use `FileUrl::from_str` instead.
    /// let url = FileUrl::try_from_url("file://x753-Mimics-0.7.0");
    /// assert_matches!(url, Err(_));
    /// ```
    pub fn try_from_url(s: &str) -> Result<Self> {
        let url = Url::parse(s)?;
        Ok(Self::Url(url))
    }
}

impl Display for FileUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileUrl::Url(url) => write!(f, "{url}"),
            FileUrl::Path(path) => write!(f, "file://{path}"),
        }
    }
}

impl From<FileUrl> for String {
    fn from(source: FileUrl) -> Self {
        source.to_string()
    }
}

impl FromStr for FileUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Some(path) = s.strip_prefix("file://") {
            Ok(FileUrl::Path(Utf8PathBuf::from(path)))
        } else {
            FileUrl::try_from_url(s)
        }
    }
}

impl TryFrom<String> for FileUrl {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        s.parse()
    }
}

impl From<Url> for FileUrl {
    fn from(url: Url) -> Self {
        Self::Url(url)
    }
}

impl From<Utf8PathBuf> for FileUrl {
    fn from(path: Utf8PathBuf) -> Self {
        Self::Path(path)
    }
}
