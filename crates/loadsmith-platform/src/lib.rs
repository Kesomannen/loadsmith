//! Platform and game detection for the loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.

mod epic_games;
mod error;
mod steam;
mod xbox_store;

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use camino::Utf8PathBuf;
use tracing::warn;
use walkdir::WalkDir;

pub use epic_games::EpicGamesGame;
pub use error::{Error, Result};
pub use steam::{SteamGame, SteamInstallation, SteamInstallationKind};
pub use xbox_store::XboxStoreGame;

/// A game distribution on a specific storefront.
///
/// This enum handles two main use cases:
///
/// - Locating the game's directory in the storefront's local library.
/// - Creating a launch command for the game via the storefront.
///
/// This is a wrapper around the platform-specific game types, such as [`SteamGame`], [`EpicGamesGame`], and [`XboxStoreGame`],
/// and is intended to give a unified interface with sane defaults for locating and launching games across different platforms.
/// If you seek more control over the platform-specific behavior, use those types directly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum GameDistribution {
    /// A Steam game. See [`SteamGame`] for more details.
    Steam(steam::SteamGame),
    /// An Epic Games Store game. See [`EpicGamesGame`] for more details.
    EpicGames(epic_games::EpicGamesGame),
    /// An Xbox / Microsoft Store game. See [`XboxStoreGame`] for more details.
    XboxStore(xbox_store::XboxStoreGame),
}

impl GameDistribution {
    /// Returns a human-readable name for the storefront associated with this game distribution.
    ///
    /// # Example
    ///
    /// ```
    /// # use loadsmith_platform::{GameDistribution, SteamGame};
    /// let distribution = GameDistribution::Steam(SteamGame::new(730));
    /// assert_eq!(distribution.storefront(), "Steam");
    /// ```
    pub fn storefront(&self) -> &'static str {
        match self {
            GameDistribution::Steam { .. } => "Steam",
            GameDistribution::EpicGames { .. } => "Epic Games",
            GameDistribution::XboxStore { .. } => "Xbox Store",
        }
    }

    /// Locate the install directory for this game distribution with sane defaults.
    ///
    /// The method to accomplish this varies by storefront, see the documentation for each
    /// platform-specific type for more details.
    pub fn locate_game(&self) -> Result<Utf8PathBuf> {
        match self {
            GameDistribution::Steam(game) => steam::SteamInstallation::locate()?.locate_game(game),
            GameDistribution::EpicGames(game) => game.locate(),
            GameDistribution::XboxStore(game) => game.locate(),
        }
    }

    /// Creates a launch command for this game distribution with sane defaults.
    ///
    /// The method to accomplish this varies by storefront, see the documentation for each
    /// platform-specific type for more details.
    pub fn create_launch_command(&self) -> Result<Option<Command>> {
        match self {
            GameDistribution::Steam(game) => steam::SteamInstallation::locate()?
                .launch_command(game)
                .map(Some),
            GameDistribution::EpicGames(game) => game.launch_command(),
            GameDistribution::XboxStore(_) => Ok(None),
        }
    }
}

/// Guess whether a game is running under Proton or a similar Windows translation layer.
///
/// On Windows this always returns `false`.
///
/// On Linux, it checks for a `.forceproton` marker file, or the presence
/// of `.exe` files in the game directory.
///
/// Errors during the process cause the function to return `false`.
/// See [`try_guess_proton`] for the fallible version.
pub fn guess_proton(game_path: impl AsRef<Path>) -> bool {
    try_guess_proton(game_path).unwrap_or_else(|err| {
        warn!(%err, "failed to guess if game is running under Proton");
        false
    })
}

/// Guess whether a game is running under Proton or a similar Windows translation layer.
///
/// On Windows this always returns `false`.
///
/// On Linux, it checks for a `.forceproton` marker file, or the presence
/// of `.exe` files in the game directory.
///
/// For an infallible version that returns `false` on errors, see [`guess_proton`].
pub fn try_guess_proton(#[allow(unused)] game_path: impl AsRef<Path>) -> Result<bool> {
    #[cfg(target_os = "windows")]
    {
        Ok(false)
    }

    #[cfg(target_os = "linux")]
    {
        use tracing::{debug, trace};

        let game_path = game_path.as_ref();

        trace!("checking for .forceproton file in game directory");

        if game_path.join(".forceproton").exists() {
            debug!(".forceproton file found");
            return Ok(true);
        }

        let exe = find_executables(game_path).map(|mut executable| {
            executable.find(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "exe")
            })
        })?;

        if let Some(exe) = exe {
            debug!(exe = %exe.display(), "found .exe file in game directory, assuming proton");
            Ok(true)
        } else {
            debug!("no .exe file found in game directory");
            Ok(false)
        }
    }
}

/// Walk the given game directory and find all executable files.
///
/// This filters out known non-game executables like crash handlers, and only returns files
/// with extensions that are likely to be game executables.
pub fn find_executables(game_path: &Path) -> impl Iterator<Item = PathBuf> + use<> {
    const IGNORED_FILES: &[&str] = &[
        "crashpad_handler.exe",
        "UnityCrashHandler32.exe",
        "UnityCrashHandler64.exe",
    ];

    #[cfg(target_os = "windows")]
    const EXTENSIONS: &[&str] = &["exe"];

    #[cfg(target_os = "linux")]
    const EXTENSIONS: &[&str] = &["x86_64", "x86", "sh", "exe"];

    WalkDir::new(game_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            EXTENSIONS.contains(&extension) && !IGNORED_FILES.contains(&file_name)
        })
}
