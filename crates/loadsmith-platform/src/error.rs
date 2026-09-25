use std::process::ExitStatus;

/// Errors that can occur while interacting with the loadsmith-platform crate.
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

    #[error("could not find steam executable")]
    SteamExecutableNotFound,

    #[error("Xbox Store query failed with status {status}")]
    XboxStoreQueryFailed { status: ExitStatus },

    #[error("Xbox Store query returned non-UTF-8 output")]
    XboxNonUtf8 {
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error("the game was not found in the storefront's local library")]
    GameNotFound,

    #[error("game detection is not supported for this platform and/or OS")]
    UnsupportedEnvironment,
}

/// Convenience alias for `Result<T, loadsmith_platform::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
