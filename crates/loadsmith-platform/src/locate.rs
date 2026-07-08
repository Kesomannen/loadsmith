use std::path::{Path, PathBuf};

use camino::Utf8PathBuf;
use tracing::debug;
use walkdir::WalkDir;

use crate::{Error, Platform, Result};

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

pub fn locate_game(platform: &Platform) -> Result<Option<Utf8PathBuf>> {
    match platform {
        Platform::Steam { id } => locate_steam_game(*id),
        #[cfg(target_os = "windows")]
        Platform::XboxStore { identifier } => xbox_game_dir(identifier).map(Some),
        #[cfg(target_os = "windows")]
        Platform::EpicGames { identifier } => epic_game_dir(identifier).map(Some),
        _ => Ok(None),
    }
}

fn locate_steam_game(id: u32) -> Result<Option<Utf8PathBuf>> {
    let steam = steamlocate::SteamDir::locate()?;

    debug!(
        path = %steam.path().display(),
        "found steam installation"
    );

    let (app, lib) = steam.find_app(id)?.ok_or(Error::SteamAppNotFound)?;

    debug!(
        name = app.name,
        library_path = %lib.path().display(),
        "found game in steam library"
    );

    let path = lib.resolve_app_dir(&app);
    let utf8_path = Utf8PathBuf::try_from(path)?;

    Ok(Some(utf8_path))
}

#[cfg(target_os = "windows")]
fn xbox_game_dir(identifier: &str) -> Result<Utf8PathBuf> {
    use std::process::Command;

    let mut query = Command::new("powershell.exe");
    query.args([
        "get-appxpackage",
        "-Name",
        identifier,
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

    let s = String::from_utf8(out.stdout).map_err(|source| Error::XboxNonUtf8 { source })?;
    let path = Utf8PathBuf::from(s.trim());

    Ok(path)
}

#[cfg(target_os = "windows")]
fn epic_game_dir(identifier: &str) -> Result<Utf8PathBuf> {
    use serde::Deserialize;

    let dat_path: PathBuf =
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
        .find(|item| item.app_name == identifier)
        .map(|item| item.install_location)
        .ok_or(Error::EpicGameNotFound)
}
