//! Platform and game detection for the loadsmith mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.

mod error;
mod launch;
mod locate;

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    process::Command,
};

use camino::{Utf8Path, Utf8PathBuf};
pub use error::{Error, Result};
use loadsmith_loader::LaunchContext;
use tracing::warn;

/// A game distribution platform.
///
/// Each variant identifies a specific storefront or launcher. The
/// [`Platform`] type is used to locate game installations and create launch
/// commands on the host system.
///
/// This enum is `#[non_exhaustive]`; new platforms may be added without a
/// breaking change.
///
/// ```
/// use loadsmith_platform::Platform;
///
/// let steam = Platform::Steam { id: 230230 };
/// assert_eq!(steam.name(), "Steam");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Platform {
    /// A Steam game, identified by its Steam App ID.
    Steam { id: u32 },
    /// An Epic Games Store game, identified by its launcher identifier.
    EpicGames { identifier: String },
    /// An Oculus / Meta Store game.
    Oculus,
    /// An Origin (EA App) game.
    Origin,
    /// An Xbox / Microsoft Store game, identified by its package name.
    XboxStore { identifier: String },
    /// Any other platform that is not explicitly handled.
    Other,
}

impl Platform {
    /// Return the human-readable display name for this platform.
    ///
    /// ```
    /// use loadsmith_platform::Platform;
    ///
    /// assert_eq!(Platform::Steam { id: 1 }.name(), "Steam");
    /// assert_eq!(Platform::EpicGames { identifier: "x".into() }.name(), "Epic Games");
    /// assert_eq!(Platform::Oculus.name(), "Oculus");
    /// assert_eq!(Platform::Origin.name(), "Origin");
    /// assert_eq!(Platform::XboxStore { identifier: "x".into() }.name(), "Xbox Store");
    /// assert_eq!(Platform::Other.name(), "Other");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Steam { .. } => "Steam",
            Platform::EpicGames { .. } => "Epic Games",
            Platform::Oculus => "Oculus",
            Platform::Origin => "Origin",
            Platform::XboxStore { .. } => "Xbox Store",
            Platform::Other => "Other",
        }
    }

    /// Locate the game directory for this platform's game, if possible.
    ///
    /// Returns `Ok(None)` when the platform does not support automatic
    /// detection or when the game cannot be found.
    ///
    /// ```no_run
    /// use loadsmith_platform::Platform;
    ///
    /// let platform = Platform::Steam { id: 730 }; // CS:GO / CS2
    /// match platform.locate_game() {
    ///     Ok(Some(path)) => println!("Game found at: {path}"),
    ///     _ => println!("Game not found"),
    /// }
    /// ```
    pub fn locate_game(&self) -> Result<Option<Utf8PathBuf>> {
        locate::locate_game(self)
    }

    /// Create a platform-specific [`Command`] to launch the game, if
    /// supported.
    ///
    /// Returns `Ok(None)` for platforms where launch-command generation is
    /// not implemented.
    ///
    /// ```no_run
    /// use loadsmith_platform::Platform;
    ///
    /// let platform = Platform::Steam { id: 730 };
    /// if let Ok(Some(cmd)) = platform.create_launch_command() {
    ///     println!("Launch command: {cmd:?}");
    /// }
    /// ```
    pub fn create_launch_command(&self) -> Result<Option<Command>> {
        launch::create_launch_command(self)
    }

    /// Build a [`LaunchContext`] for this platform, resolving the game path
    /// and guessing whether Proton is being used.
    ///
    /// If `override_game_path` is `Some`, it is used directly; otherwise the
    /// game is located via `locate_game`.
    ///
    /// ```no_run
    /// use loadsmith_platform::Platform;
    /// use camino::Utf8PathBuf;
    ///
    /// let platform = Platform::Steam { id: 730 };
    /// let ctx = platform.create_launch_context(Utf8PathBuf::from("./profiles"), None);
    /// ```
    pub fn create_launch_context<'a>(
        &'a self,
        profile_path: impl Into<Cow<'a, Utf8Path>>,
        override_game_path: Option<Utf8PathBuf>,
    ) -> Result<LaunchContext<'a>> {
        let game_path = match override_game_path {
            Some(path) => path,
            None => match self.locate_game()? {
                Some(path) => path,
                None => return Err(Error::NoGamePathOverride),
            },
        };

        let is_proton = crate::try_guess_proton(&*game_path)?;

        Ok(LaunchContext::new(profile_path, game_path, is_proton))
    }
}

/// Walk a game directory tree and yield paths to executable files.
///
/// Filters by platform-appropriate extensions (`.exe` on Windows; `.exe`,
/// `.sh`, `.x86_64`, `.x86` on Linux) and skips known crash-handler binaries.
///
/// ```no_run
/// use loadsmith_platform::find_executables;
///
/// for exe in find_executables("/path/to/game").unwrap() {
///     println!("Found executable: {}", exe.display());
/// }
/// ```
pub fn find_executables(game_path: impl AsRef<Path>) -> Result<impl Iterator<Item = PathBuf>> {
    Ok(locate::find_executables(game_path.as_ref()))
}

/// Guess whether a game is running under Proton (Linux Steam Play).
///
/// On Windows this always returns `false`. On Linux, it checks for a
/// `.forceproton` marker file or the presence of `.exe` files in the game
/// directory. Errors are silently swallowed and return `false`.
///
/// ```no_run
/// use loadsmith_platform::guess_proton;
///
/// let is_proton = guess_proton("/path/to/game");
/// println!("Proton: {is_proton}");
/// ```
pub fn guess_proton(game_path: impl AsRef<Path>) -> bool {
    try_guess_proton(game_path).unwrap_or_else(|err| {
        warn!(%err, "failed to guess if game is running under Proton");
        false
    })
}

/// Guess whether a game is running under Proton (Linux Steam Play),
/// returning errors.
///
/// This is the fallible version of [`guess_proton`]. On Windows it always
/// returns `Ok(false)`.
///
/// ```no_run
/// use loadsmith_platform::try_guess_proton;
///
/// match try_guess_proton("/path/to/game") {
///     Ok(true) => println!("Likely running under Proton"),
///     Ok(false) => println!("Native Linux or Windows"),
///     Err(e) => eprintln!("Error checking Proton: {e}"),
/// }
/// ```
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
