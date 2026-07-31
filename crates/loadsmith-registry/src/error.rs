use std::error::Error as StdError;

use camino::Utf8PathBuf;

/// Errors returned by registry operations.
///
/// Wraps I/O errors, Thunderstore client errors, zip errors, JSON errors, and
/// several registry-specific error conditions such as missing packages,
/// missing or invalid metadata, and file-not-found.
///
/// # Examples
///
/// ```rust
/// use loadsmith_registry::Error;
///
/// let err = Error::PackageNotFound;
/// assert_eq!(err.to_string(), "package not found");
///
/// let io_err = Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
/// assert!(io_err.to_string().contains("I/O error"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O error occurred while reading a file or directory.
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    /// An error from the Thunderstore API client.
    #[error("thunderstore client error")]
    Thunderstore(#[from] thunderstore::Error),

    /// An error while reading a zip archive.
    #[error("zip error")]
    Zip(#[from] zip::result::ZipError),

    /// An error while serialising or deserialising JSON metadata.
    #[error("json error")]
    Json(#[from] serde_json::Error),

    /// A registry with the given identifier was not found in the set.
    #[error("registry not found: {0}")]
    RegistryNotFound(String),

    /// An opaque, boxed error from an external source.
    #[error(transparent)]
    Other(Box<dyn StdError + Send + Sync>),

    /// A registry operation required metadata but none was provided.
    #[error("registry requires metadata, but none was provided")]
    MissingMetadata,

    /// The metadata provided to the registry was not valid for the expected schema.
    #[error("registry got invalid metadata")]
    InvalidMetadata(#[source] serde_json::Error),

    /// A required file does not exist at the given path.
    #[error("file not found: {0}")]
    FileNotFound(Utf8PathBuf),

    /// The file at the given path is not a recognised type.
    #[error("invalid file type: {0}")]
    InvalidFileType(Utf8PathBuf),

    /// The requested package could not be found in the registry.
    #[error("package not found")]
    PackageNotFound,

    /// The requested package version could not be found.
    #[error("package version not found")]
    VersionNotFound,

    /// A local package source is missing its `manifest.json`.
    #[error("local package has no manifest")]
    LocalManifestMissing,

    /// A local package's version could not be determined and no `source_version` was supplied.
    #[error(
        "version could not be determined from local package, please specify a source_version in metadata"
    )]
    LocalVersionMissing,
}

impl Error {
    /// Wrap an arbitrary error into the [`Error::Other`] variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use loadsmith_registry::Error;
    ///
    /// let custom = "something went wrong".to_string();
    /// let err = Error::other(std::io::Error::new(std::io::ErrorKind::Other, custom));
    /// assert!(err.to_string().contains("something went wrong"));
    /// ```
    pub fn other<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::Other(Box::new(err))
    }
}

/// Convenience alias for [`std::result::Result`] whose error type is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;
