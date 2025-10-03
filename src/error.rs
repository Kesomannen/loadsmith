use std::{
    io,
    path::{Path, PathBuf},
};

/// Errors that may occur while interacting with loadsmith.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error at {} while {}: {}", path.display(), description, err)]
    Io {
        /// The inner [`std::io::Error`] that occured.
        err: io::Error,
        /// The path where the operation was attempted.
        path: PathBuf,
        /// A description of the operation, meant to be diplayed as `error while ...`.
        description: &'static str,
    },

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

    #[error("multiple GDWeave mod roots found")]
    MultipleGDWeaveModRoots,

    #[error("no GDWeave mod roots found")]
    NoGDWeaveModRoots,

    #[error("path must be UTF-8: {}", _0.display())]
    NonUTF8Path(PathBuf),
}

/// An alias for [`std::result::Result`] with the error type set to [`crate::Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn wrap_io(
        err: io::Error,
        path: impl AsRef<Path>,
        description: &'static str,
    ) -> Self {
        Self::Io {
            err,
            path: path.as_ref().into(),
            description,
        }
    }
}

pub(crate) trait IoResultExt<T> {
    fn wrap_err(self, path: impl AsRef<Path>, description: &'static str) -> Result<T>;
}

impl<T> IoResultExt<T> for io::Result<T> {
    fn wrap_err(self, path: impl AsRef<Path>, description: &'static str) -> Result<T> {
        self.map_err(|err| Error::wrap_io(err, path, description))
    }
}
