use std::process::ExitStatus;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    #[error("path is not valid UTF-8")]
    Utf8(#[from] camino::FromPathBufError),

    #[error("steamlocate error")]
    SteamLocate(#[from] steamlocate::Error),

    #[error("could not find app in steam library")]
    SteamAppNotFound,

    #[error("could not find steam executable")]
    SteamExecutableNotFound,

    #[error("Xbox Store query failed with status {status}")]
    XboxStoreQueryFailed { status: ExitStatus },

    #[error("Xbox Store query returned non-UTF-8 output")]
    XboxNonUtf8 {
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error("game not found in Epic Games Launcher installations")]
    EpicGameNotFound,

    #[error("game path could not be determined and no override was provided")]
    NoGamePathOverride,
}

pub type Result<T> = std::result::Result<T, Error>;
