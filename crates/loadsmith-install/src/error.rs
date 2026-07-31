use std::path::PathBuf;

/// Errors that can occur during mod installation, extraction, or removal.
///
/// # Examples
///
/// ```rust
/// use loadsmith_install::Error;
///
/// let err = Error::FileAlreadyExists("C:\\mods\\file.dll".into());
/// assert!(err.to_string().contains("file already exists"));
/// ```
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Wraps an [`std::io::Error`].
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Wraps a [`zip::result::ZipError`].
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    /// Wraps a [`camino::FromPathBufError`] when a non-UTF-8 path is encountered.
    #[error(transparent)]
    InvalidUtf8(#[from] camino::FromPathBufError),

    /// Wraps a [`walkdir::Error`] when walking directories.
    #[error(transparent)]
    Walkdir(#[from] walkdir::Error),

    /// A file already exists at the target path and the conflict strategy is [`Error`](crate::ConflictStrategy::Error).
    #[error("file already exists: {0}")]
    FileAlreadyExists(PathBuf),

    /// A path inside a zip archive could not be extracted (e.g. it escapes the archive root).
    #[error("invalid zip path: {0}")]
    InvalidZipPath(String),

    /// A zip file index is out of bounds for the archive.
    #[error("zip file index out of bounds: {index} (len={len})")]
    ZipFileOutOfBounds { index: usize, len: usize },
}

/// Convenience alias for [`Error`] results.
pub type Result<T> = std::result::Result<T, Error>;
