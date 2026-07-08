use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    #[error(transparent)]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

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
}

pub type Result<T> = std::result::Result<T, Error>;
