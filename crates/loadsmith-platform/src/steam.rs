use std::{path::PathBuf, process::Command};

use camino::Utf8PathBuf;
use tracing::{debug, trace};

use crate::{Error, Result};

/// A Steam installation on the current system.
///
/// This is backed by the [steamlocate] crate, which is used to locate the Steam installation directory
/// and libraries across the system.
///
/// Additionally, this struct tracks the kind of Steam installation, either a standard binary one, or
/// a Linux Flatpak installation. This is used to determine how to launch games, as Flatpak installations
/// require a different command to invoke Steam correctly.
///
/// The easiest way to get started is to use [`SteamInstallation::locate`] to automatically find the Steam
/// installation and determine its kind.
///
/// # Example
///
/// ```no_run
/// # use loadsmith_platform::{SteamInstallation, SteamGame};
/// let steam = SteamInstallation::locate().unwrap();
/// let game = SteamGame::new(730); // CS:GO
///
/// let game_path = steam.locate_game(&game).unwrap();
/// assert!(game_path.exists());
///
/// let launch_command = steam.launch_command(&game).unwrap();
/// assert_eq!(launch_command.get_args().collect::<Vec<_>>(), vec!["-applaunch", "730"]);
/// ```
#[derive(Debug)]
pub struct SteamInstallation {
    steam_dir: steamlocate::SteamDir,
    kind: SteamInstallationKind,
}

/// A type of Steam installation: either a standard binary installation, or a Linux Flatpak installation.
///
/// This is used to determine how to launch games, as Flatpak installations require a different command to
/// invoke Steam correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamInstallationKind {
    /// A standard Steam installation, with the path to the Steam executable (e.g. `steam.exe` on Windows
    /// or `steam` on Linux).
    ///
    /// This is always the case on Windows, and on Linux if Steam is installed as a system package.
    Binary(PathBuf),

    /// A Linux Flatpak installation of Steam.
    Flatpak,
}

/// A Steam game, identified by its Steam App ID.
///
/// To use this type, a `SteamInstallation` must be acquired first, either by calling
/// [`SteamInstallation::locate`] or [`SteamInstallation::create`]. You can then use
/// the [`locate_game`](SteamInstallation::locate_game) and [`launch_command`](SteamInstallation::launch_command)
/// methods to find the game's directory and create a command to launch the game, respectively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamGame(pub u32);

impl From<u32> for SteamGame {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<SteamGame> for u32 {
    fn from(game: SteamGame) -> Self {
        game.0
    }
}

impl SteamGame {
    /// Creates a new [`SteamGame`] from a Steam App ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

impl SteamInstallation {
    /// Creates a new [`SteamInstallation`] from an already constructed [`steamlocate::SteamDir`]
    /// and a [`SteamInstallationKind`].
    ///
    /// The path of the SteamDir and the kind of installation don't necessarily have to match,
    /// for instance you may choose to use a separate script to launch Steam, or have a Flatpak
    /// installation make use of a non-standard Steam directory.
    ///
    /// See [`create`](Self::create) and [`locate`](Self::locate) for more convenient (but less flexible)
    /// ways to construct this type.
    pub fn new(steam_dir: steamlocate::SteamDir, kind: SteamInstallationKind) -> Self {
        Self { steam_dir, kind }
    }

    /// Returns the kind of Steam installation.
    pub fn kind(&self) -> &SteamInstallationKind {
        &self.kind
    }

    /// Returns the underlying [`steamlocate::SteamDir`] for this installation.
    pub fn steam_dir(&self) -> &steamlocate::SteamDir {
        &self.steam_dir
    }

    /// Creates a new [`SteamInstallation`] from a path to Steam's installation directory and a
    /// [`SteamInstallationKind`].
    ///
    /// The `path` here must be the path to the **Steam installation directory**, not a Steam library
    /// or executable. This directory is typically located at:
    ///
    /// - Windows: `C:\Program Files (x86)\Steam`
    /// - Linux: `~/.local/share/Steam`
    ///
    /// To automatically locate the installation and determine the kind, use [`locate`](Self::locate) instead.
    pub fn create(path: impl Into<PathBuf>, kind: SteamInstallationKind) -> Result<Self> {
        let path = path.into();
        let steam_dir = steamlocate::SteamDir::from_dir(&path)?;

        Ok(Self::new(steam_dir, kind))
    }

    /// Attemps to locate the Steam installation on the current system, and automatically determine the kind
    /// of installation.
    ///
    /// The [steamlocate] crate is used on both Windows and Linux, refer to its documentation for more details
    /// on how the Steam installation is located.
    ///
    /// On Linux, this will first check for a Flatpak installation via `flatpak info ...` before falling
    /// back to standard checks.
    pub fn locate() -> Result<Self> {
        let steam_dir = steamlocate::SteamDir::locate()?;

        #[cfg(target_os = "linux")]
        let options = {
            if Self::detect_flatpak()? {
                return Ok(Self::new(steam_dir, SteamInstallationKind::Flatpak));
            }

            vec![
                steam_dir.path().join("steam"),
                PathBuf::from("/usr/bin/steam"),
            ]
        };

        #[cfg(target_os = "windows")]
        let options = {
            vec![
                steam_dir.path().join("steam.exe"),
                PathBuf::from("C:\\Program Files (x86)\\Steam\\steam.exe"),
            ]
        };

        options
            .into_iter()
            .find(|path| {
                let found = path.exists();

                trace!(
                    path = %path.display(),
                    found,
                    "check for steam executable"
                );

                found
            })
            .ok_or_else(|| Error::SteamExecutableNotFound)
            .map(|path| Self::new(steam_dir, SteamInstallationKind::Binary(path)))
    }

    #[cfg(target_os = "linux")]
    fn detect_flatpak() -> Result<bool> {
        let mut check_command = Command::new("flatpak");
        check_command.args(["info", "com.valvesoftware.Steam"]);

        debug!(
            command = ?check_command,
            "checking for steam flatpak installation"
        );

        match check_command.output().map(|out| out.status) {
            Ok(status) if status.success() => {
                // let mut command = Command::new("flatpak");
                // command.args(["run", "com.valvesoftware.Steam"]);
                debug!("steam flatpak installation found");
                return Ok(true);
            }
            Ok(status) => {
                debug!(?status, "flatpak steam was not installed",);
                return Ok(false);
            }
            Err(err) => {
                debug!(?err, "failed to check for steam flatpak installation");
                return Ok(false);
            }
        }
    }

    /// Locates the installation directory of a Steam game.
    ///
    /// The [steamlocate] crate is used as the underlying mechanism, refer to
    /// [`steamlocate::SteamDir::find_app`] for more details on how the game is located.
    pub fn locate_game(&self, game: &SteamGame) -> Result<Utf8PathBuf> {
        let (app, lib) = self
            .steam_dir
            .find_app(game.0)?
            .ok_or(Error::GameNotFound)?;

        debug!(
            name = app.name,
            library_path = %lib.path().display(),
            "found game in steam library"
        );

        let path = lib.resolve_app_dir(&app);
        let utf8_path = Utf8PathBuf::try_from(path)?;

        Ok(utf8_path)
    }

    /// Creates a [`Command`] to launch a Steam game.
    ///
    /// On Flatpak installations, this will invoke `flatpak run com.valvesoftware.Steam -applaunch <appid>`.
    ///
    /// Otherwise, it will directly call the Steam executable with `<executable> -applaunch <appid>`.
    pub fn launch_command(&self, game: &SteamGame) -> Result<Command> {
        let mut command = match &self.kind {
            SteamInstallationKind::Binary(path) => Command::new(path),
            SteamInstallationKind::Flatpak => {
                let mut command = Command::new("flatpak");
                command.args(["run", "com.valvesoftware.Steam"]);
                command
            }
        };

        command.arg("-applaunch").arg(game.0.to_string());
        Ok(command)
    }
}
