use std::process::ExitStatus;

/// Errors that can occur during platform and game detection.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An underlying I/O error.
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    /// An error parsing or serialising JSON (e.g. Epic Games manifest).
    #[error("JSON error")]
    Json(#[from] serde_json::Error),

    /// A path could not be converted to valid UTF-8.
    #[error("path is not valid UTF-8")]
    Utf8(#[from] camino::FromPathBufError),

    /// An error from the steamlocate library.
    #[error("steamlocate error")]
    SteamLocate(#[from] steamlocate::Error),

    /// The requested app ID was not found in any Steam library.
    #[error("could not find app in steam library")]
    SteamAppNotFound,

    /// The Steam executable could not be located.
    #[error("could not find steam executable")]
    SteamExecutableNotFound,

    /// A PowerShell query for Xbox Store games failed with a non-zero exit
    /// status.
    #[error("Xbox Store query failed with status {status}")]
    XboxStoreQueryFailed { status: ExitStatus },

    /// A PowerShell query for Xbox Store games returned non-UTF-8 output.
    #[error("Xbox Store query returned non-UTF-8 output")]
    XboxNonUtf8 {
        #[source]
        source: std::string::FromUtf8Error,
    },

    /// The game was not found in the Epic Games Launcher installations.
    #[error("game not found in Epic Games Launcher installations")]
    EpicGameNotFound,

    /// The game path could not be determined and no override was provided.
    #[error("game path could not be determined and no override was provided")]
    NoGamePathOverride,
}

/// Alias for `Result<T, Error>` in the loadsmith-platform crate.
pub type Result<T> = std::result::Result<T, Error>;
