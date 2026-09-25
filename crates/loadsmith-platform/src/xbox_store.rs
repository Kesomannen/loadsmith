use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// An Xbox / Microsoft Store game, identified by the package name of its Appx/MSIX package.
///
/// The package name is the identifier matched by `Get-AppxPackage -Name <identifier>` in
/// PowerShell; you can list the packages installed on your system by running `Get-AppxPackage`.
/// For example, Valheim is registered under the name `CoffeeStainStudios.Valheim`.
///
/// As of writing, the Microsoft Store is only supported on Windows, and as such the methods on
/// this struct will always return an error on other platforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct XboxStoreGame(pub String);

impl XboxStoreGame {
    /// Creates a new [`XboxStoreGame`] from the package name of the game's Appx/MSIX package.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Attempts to locate the installation directory for this Xbox / Microsoft Store game.
    ///
    /// This is done by invoking `powershell.exe` and running
    /// `Get-AppxPackage -Name <identifier> | select -expand InstallLocation`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use loadsmith_platform::XboxStoreGame;
    /// let game = XboxStoreGame::new("CoffeeStainStudios.Valheim");
    ///
    /// let game_path = game.locate().unwrap();
    /// println!("game located at {game_path}");
    /// ```
    pub fn locate(&self) -> Result<Utf8PathBuf> {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;

            use tracing::debug;

            let mut query = Command::new("powershell.exe");
            query.args([
                "get-appxpackage",
                "-Name",
                self.0.as_str(),
                "|",
                "select",
                "-expand",
                "InstallLocation",
            ]);

            debug!(
                command = ?query,
                "running powershell query for Xbox Store game directory"
            );

            let out = query.output()?;

            if !out.status.success() {
                return Err(Error::XboxStoreQueryFailed { status: out.status });
            }

            let s =
                String::from_utf8(out.stdout).map_err(|source| Error::XboxNonUtf8 { source })?;

            if s.trim().is_empty() {
                return Err(Error::GameNotFound);
            }

            let path = Utf8PathBuf::from(s.trim());

            Ok(path)
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err(Error::UnsupportedEnvironment)
        }
    }
}
