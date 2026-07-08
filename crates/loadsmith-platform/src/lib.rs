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

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Platform {
    Steam { id: u32 },
    EpicGames { identifier: String },
    Oculus,
    Origin,
    XboxStore { identifier: String },
    Other,
}

impl Platform {
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

    pub fn locate_game(&self) -> Result<Option<Utf8PathBuf>> {
        locate::locate_game(self)
    }

    pub fn create_launch_command(&self) -> Result<Option<Command>> {
        launch::create_launch_command(self)
    }

    pub fn create_launch_context<'a>(
        &'a self,
        profile_path: impl Into<Cow<'a, Utf8Path>>,
        fallback_game_path: Option<Utf8PathBuf>,
    ) -> Result<LaunchContext<'a>> {
        let game_path = match self.locate_game()? {
            Some(path) => path,
            None => fallback_game_path.ok_or(Error::NoFallbackGamePath)?,
        };

        let is_proton = crate::try_guess_proton(&*game_path)?;

        Ok(LaunchContext::new(profile_path, game_path, is_proton))
    }
}

pub fn find_executables(game_path: impl AsRef<Path>) -> Result<impl Iterator<Item = PathBuf>> {
    Ok(locate::find_executables(game_path.as_ref()))
}

pub fn guess_proton(game_path: impl AsRef<Path>) -> bool {
    try_guess_proton(game_path).unwrap_or_else(|err| {
        warn!(%err, "failed to guess if game is running under Proton");
        false
    })
}

pub fn try_guess_proton(game_path: impl AsRef<Path>) -> Result<bool> {
    #[cfg(target_os = "windows")]
    {
        Ok(false)
    }

    #[cfg(target_os = "linux")]
    {
        use tracing::debug;

        let game_path = game_path.as_ref();

        if game_path.join(".forceproton").exists() {
            debug!(".forceproton file found");
            return Ok(true);
        } else {
            debug!(".forceproton file not found");
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
