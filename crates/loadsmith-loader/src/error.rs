use camino::Utf8PathBuf;

/// Errors that can occur during loader setup and launch-argument generation.
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::Error;
///
/// let err = Error::UnsupportedDoorstopVersion(5);
/// assert_eq!(err.to_string(), "unsupported doorstop version: 5");
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O error occurred.
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    /// The `.doorstop_version` file contains an unparseable value.
    #[error("invalid doorstop version format: {0}")]
    InvalidDoorstopVersionFormat(String),

    /// A Doorstop version other than 3 or 4 was requested.
    #[error("unsupported doorstop version: {0}")]
    UnsupportedDoorstopVersion(u32),

    /// The BepInEx core directory could not be opened.
    #[error("BepInEx core directory is missing")]
    BepInExCoreDirectoryMissing {
        #[source]
        source: std::io::Error,
    },

    /// The BepInEx preloader assembly was not found in the core directory.
    #[error("BepInEx preloader not found in core directory at {core_directory}")]
    BepInExPreloaderNotFound { core_directory: Utf8PathBuf },
}

/// Convenience alias for `Result<T, loadsmith_loader::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
