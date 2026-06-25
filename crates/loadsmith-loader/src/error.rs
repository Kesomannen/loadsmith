use camino::Utf8PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("invalid doorstop version format: {0}")]
    InvalidDoorstopVersionFormat(String),

    #[error("unsupported doorstop version: {0}")]
    UnsupportedDoorstopVersion(u32),

    #[error("BepInEx core directory is missing")]
    BepInExCoreDirectoryMissing {
        #[source]
        source: std::io::Error,
    },

    #[error("BepInEx preloader not found in core directory at {core_directory}")]
    BepInExPreloaderNotFound { core_directory: Utf8PathBuf },
}

pub type Result<T> = std::result::Result<T, Error>;
