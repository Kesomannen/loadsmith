use std::error::Error as StdError;

use camino::Utf8PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("thunderstore client error")]
    Thunderstore(#[from] thunderstore::Error),

    #[error("zip error")]
    Zip(#[from] zip::result::ZipError),

    #[error("json error")]
    Json(#[from] serde_json::Error),

    #[error("registry not found: {0}")]
    RegistryNotFound(String),

    #[error(transparent)]
    Other(Box<dyn StdError + Send + Sync>),

    #[error("registry requires metadata, but none was provided")]
    MissingMetadata,

    #[error("registry got invalid metadata: {error}")]
    InvalidMetadata { error: serde_json::Error },

    #[error("file not found: {0}")]
    FileNotFound(Utf8PathBuf),

    #[error("invalid file type: {0}")]
    InvalidFileType(Utf8PathBuf),

    #[error("package not found")]
    PackageNotFound,

    #[error("package version not found")]
    VersionNotFound,
}

impl Error {
    pub fn other<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::Other(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
