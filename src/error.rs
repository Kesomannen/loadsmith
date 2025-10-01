use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("io error at {}: {}", _1.display(), _0)]
    IoWithPath(io::Error, PathBuf),

    #[error(transparent)]
    Walkdir(#[from] walkdir::Error),

    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("invalid .doorstop_version file format")]
    InvalidDoorstopVersionFormat,

    #[error("unsupported doorstop version: {_0}")]
    UnsupportedDoorstopVersion(u32),

    #[error("could not find BepInEx preloader")]
    BepInExPreloaderNotFound,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn wrap_io(err: io::Error, path: impl Into<PathBuf>) -> Self {
        Self::IoWithPath(err, path.into())
    }
}
