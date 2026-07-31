use std::path::PathBuf;

/// Errors that can occur during thunderstore operations.
///
/// Covers I/O, serialization, SQLite, zip archive, glob, and API errors,
/// as well as domain-specific errors such as missing manifests or
/// unsupported loaders and platforms.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::Error;
///
/// let err = Error::IndexNotComplete;
/// assert_eq!(err.to_string(), "index is not built");
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("zip error")]
    Zip(#[from] zip::result::ZipError),

    #[error("YAML error")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    #[error("SQLite error")]
    Sqlite(#[from] rusqlite::Error),

    #[error("glob error")]
    Glob(#[from] globset::Error),

    #[error("thunderstore client error")]
    Thunderstore(#[from] thunderstore::Error),

    #[error("non UTF-8 path: {0}")]
    NonUtf8Path(PathBuf),

    #[error("profile manifest not found in zip archive")]
    ProfileManifestNotFound,

    #[error("invalid zip file path: {0}")]
    InvalidZipFilePath(PathBuf),

    #[error("invalid thunderstore identifier: {0}")]
    InvalidIdent(thunderstore::Error),

    #[error("index is not built")]
    IndexNotComplete,

    #[error("distribution is missing identifier")]
    DistributionIsMissingIdentifier,

    #[error("invalid steam id")]
    InvalidSteamId(#[source] std::num::ParseIntError),

    #[error("unsupported tracking method: {0:?}")]
    UnsupportedTrackingMethod(thunderstore::models::schema::TrackingMethod),

    #[error("unsupported loader: {0:?}")]
    UnsupportedLoader(thunderstore::models::schema::Loader),

    #[error("unsupported platform: {0:?}")]
    UnsupportedPlatform(thunderstore::models::schema::Platform),
}

/// A specialized [`Result`] type for thunderstore operations.
///
/// This type alias uses [`Error`] as the error variant.
///
/// # Examples
///
/// ```
/// use loadsmith_thunderstore::Result;
///
/// fn example() -> Result<()> {
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;
