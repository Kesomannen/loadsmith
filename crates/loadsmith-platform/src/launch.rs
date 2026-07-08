use std::{path::PathBuf, process::Command};
use tracing::{debug, trace};

use crate::{Error, Platform, Result};

pub fn create_launch_command(platform: &Platform) -> Result<Option<Command>> {
    match platform {
        Platform::Steam { id } => create_steam_command(*id).map(Some),
        Platform::EpicGames { identifier } => create_epic_command(identifier),
        _ => Ok(None),
    }
}

fn create_steam_command(id: u32) -> Result<Command> {
    let mut command = create_base_steam_command()?;

    command.arg("-applaunch").arg(id.to_string());

    Ok(command)
}

fn create_base_steam_command() -> Result<Command> {
    let steam = steamlocate::SteamDir::locate()?;

    debug!(
        path = %steam.path().display(),
        "found steam installation"
    );

    #[cfg(target_os = "linux")]
    let options = {
        if let Some(command) = create_flatpak_steam_command()? {
            return Ok(command);
        }

        vec![
            steam.path().join("steam.sh"),
            steam.path().join("steam"),
            PathBuf::from("/usr/bin/steam"),
        ]
    };

    #[cfg(target_os = "windows")]
    let options = vec![
        steam.path().join("steam.exe"),
        PathBuf::from("C:\\Program Files (x86)\\Steam\\steam.exe"),
    ];

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
        .map(|path| Command::new(path))
        .ok_or_else(|| Error::SteamExecutableNotFound)
}

#[cfg(target_os = "linux")]
fn create_flatpak_steam_command() -> Result<Option<Command>> {
    use std::process::Stdio;
    use tracing::warn;

    let mut check_command = Command::new("flatpak");
    check_command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(["info", "com.valvesoftware.Steam"]);

    debug!(
        command = ?check_command,
        "checking for steam flatpak installation"
    );

    match check_command.status() {
        Ok(status) if status.success() => {
            debug!("steam flatpak installation found");

            let mut command = Command::new("flatpak");
            command.args(["run", "com.valvesoftware.Steam"]);

            return Ok(Some(command));
        }
        Ok(status) => {
            debug!(?status, "steam flatpak installation not found",);
            return Ok(None);
        }
        Err(err) => {
            warn!(%err, "failed to check for steam flatpak installation");

            return Ok(None);
        }
    }
}

fn create_epic_command(identifier: &str) -> Result<Option<Command>> {
    let url = format!("com.epicgames.launcher://apps/{identifier}?action=launch&silent=true");

    Ok(open::commands(url).into_iter().next())
}
