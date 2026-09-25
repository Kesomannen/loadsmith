use std::process::Command;

#[cfg(target_os = "windows")]
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// An Epic Games Store game, identified by its internal launcher identifier.
///
/// The internal identifier is sometimes a UUID while in other cases a human-readable string.
/// The identifier can be found in the Epic Games Launcher manifest file, located at
/// `C:\ProgramData\Epic\UnrealEngineLauncher\LauncherInstalled.dat`.
///
/// As of writing, the Epic Games Launcher is only supported on Windows, and as such the
/// methods on this struct will always return an error on other platforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EpicGamesGame(pub String);

impl EpicGamesGame {
    /// Creates a new [`EpicGamesGame`] from the internal launcher identifier.
    pub fn new(identifier: impl Into<String>) -> Self {
        Self(identifier.into())
    }

    /// Attempts to locate the installation directory for this Epic Games Store game.
    ///
    /// This is done by reading the Epic Games Launcher JSON manifest file, located at
    /// `C:\ProgramData\Epic\UnrealEngineLauncher\LauncherInstalled.dat`.
    ///
    /// On non-Windows platforms, this method always returns an [`Error::UnsupportedEnvironment`].
    pub fn locate(&self) -> Result<Utf8PathBuf> {
        #[cfg(target_os = "windows")]
        {
            use serde::Deserialize;
            use std::path::PathBuf;
            use tracing::debug;

            let dat_path =
                PathBuf::from("C:\\ProgramData\\Epic\\UnrealEngineLauncher\\LauncherInstalled.dat");

            #[derive(Debug, Deserialize)]
            #[serde(rename_all = "PascalCase")]
            struct ListItem {
                install_location: Utf8PathBuf,
                app_name: String,
            }

            debug!(
                path = %dat_path.display(),
                "reading Epic Games installations",
            );

            let s = std::fs::read_to_string(&dat_path)?;
            let list: Vec<ListItem> = serde_json::from_str(&s)?;

            list.into_iter()
                .find(|item| item.app_name == self.0)
                .map(|item| item.install_location)
                .ok_or(Error::GameNotFound)
        }

        #[cfg(target_os = "linux")]
        {
            Err(Error::UnsupportedEnvironment)
        }
    }

    /// Creates a [`Command`] to launch this Epic Games Store game.
    ///
    /// The command will open an Epic Games Launcher deep link url, which will then prompt
    /// the launcher to start the game. The mechanism used to open the URL is the [open] crate,
    /// see the documentation for that crate for more details.
    ///
    /// On non-Windows platforms, this method always returns an [`Error::UnsupportedEnvironment`].
    pub fn launch_command(&self) -> Result<Option<Command>> {
        #[cfg(target_os = "windows")]
        {
            let url = format!(
                "com.epicgames.launcher://apps/{}?action=launch&silent=true",
                self.0
            );

            Ok(open::commands(url).into_iter().next())
        }

        #[cfg(target_os = "linux")]
        {
            Err(Error::UnsupportedEnvironment)
        }
    }
}
