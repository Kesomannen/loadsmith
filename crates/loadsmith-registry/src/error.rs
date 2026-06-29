use std::{error::Error as StdError, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("thunderstore client error: {0}")]
    Thunderstore(#[from] thunderstore::Error),

    #[error("registry not found: {0}")]
    RegistryNotFound(String),

    #[error("registry implementation returned an uncategorized error: {0}")]
    Other(Box<dyn StdError + Send + Sync>),

    #[error("registry requires metadata, but none was provided")]
    MissingMetadata,

    #[error("registry got invalid metadata: {error}")]
    InvalidMetadata { error: serde_json::Error },

    #[error("file not found: {0}")]
    FileNotFound(PathBuf),
}

impl Error {
    pub fn other_registry<E: StdError + Send + Sync + 'static>(err: E) -> Self {
        Self::Other(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
