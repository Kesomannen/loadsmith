use std::path::PathBuf;

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

    #[error("zip file index out of bounds: {index} (len={len})")]
    ZipFileOutOfBounds { index: usize, len: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
