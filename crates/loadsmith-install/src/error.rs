use std::path::PathBuf;

/// Errors that can occur while interacting with the loadsmith-install crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Wraps an [`std::io::Error`].
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Wraps a [`zip::result::ZipError`].
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    /// Wraps a [`camino::FromPathBufError`].
    #[error(transparent)]
    InvalidUtf8(#[from] camino::FromPathBufError),

    /// Wraps a [`walkdir::Error`].
    #[error(transparent)]
    Walkdir(#[from] walkdir::Error),

    /// A file already exists at the target path and the conflict strategy is [`ConflictStrategy::Error`](crate::ConflictStrategy::Error).
    #[error("file already exists: {0}")]
    FileAlreadyExists(PathBuf),

    /// A path inside a zip archive could not be extracted (e.g. it escapes the archive root).
    #[error("invalid zip path: {0}")]
    InvalidZipPath(String),

    /// A zip file index is out of bounds for the archive.
    #[error("zip file index out of bounds: {index} (len={len})")]
    ZipFileOutOfBounds { index: usize, len: usize },
}

/// Convenience alias for [`std::result::Result`] with the crate-level [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
