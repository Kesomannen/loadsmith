use std::path::PathBuf;

/// Errors that can occur while interacting with the loadsmith-install crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    #[error(transparent)]
    InvalidUtf8(#[from] camino::FromPathBufError),

    #[error(transparent)]
    Walkdir(#[from] walkdir::Error),

    #[error("file already exists: {0}")]
    FileAlreadyExists(PathBuf),

    #[error("invalid zip path: {0}")]
    InvalidZipPath(String),
}

/// Convenience alias for [`std::result::Result`] with the crate-level [`Error`] type.
pub type Result<T> = std::result::Result<T, Error>;
