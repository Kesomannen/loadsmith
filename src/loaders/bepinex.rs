use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Error, IoResultExt, ModLoader, PackageInstaller, Result,
    extract::ExtractInstaller,
    rule::{Rule, RuleInstaller},
};

/// Builder for the [`BepInEx`] struct.
#[derive(Debug, Default)]
pub struct BepInExBuilder {
    extra_installer_rules: Vec<Rule>,
    custom_state_file_path: Option<PathBuf>,
}

impl BepInExBuilder {
    /// Creates a new [`BepInExBuilder`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds extra rules to the loader's inner [`RuleInstaller`].
    ///
    /// Read more in the [rule module documentation](crate::rule).
    pub fn with_extra_rules(mut self, extra_rules: Vec<Rule>) -> Self {
        self.extra_installer_rules = extra_rules;
        self
    }

    /// Sets a custom path for the profile state file.
    ///
    /// Read more in the [rule module documentation](crate::rule).
    pub fn with_state_file_name(mut self, state_file_path: impl Into<PathBuf>) -> Self {
        self.custom_state_file_path = Some(state_file_path.into());
        self
    }

    /// Creates a [`BepInEx`] instance from the builder's configuration.
    pub fn build(self) -> BepInEx {
        let rules = [
            Rule::flat_separated("plugins", Path::new("BepInEx/plugins")),
            Rule::flat_separated("patchers", Path::new("BepInEx/patchers")),
            Rule::flat_separated("monomod", Path::new("BepInEx/monomod"))
                .with_extensions(["mm.dll"]),
            Rule::flat_separated("core", Path::new("BepInEx/core")),
            Rule::untracked("config", Path::new("BepInEx/config")),
        ]
        .into_iter()
        .chain(self.extra_installer_rules.into_iter())
        .collect();

        let mut plugin_installer = RuleInstaller::new(rules).with_default(0);

        if let Some(path) = self.custom_state_file_path {
            plugin_installer = plugin_installer.with_state_file_path(path);
        }

        let loader_installer = ExtractInstaller::new([
            "*.dll",
            "*.sh",
            "doorstop_config.ini",
            ".doorstop_version",
            "BepInEx/core/*",
            "BepInEx/patchers/*",
            "BepInEx/config/BepInEx.cfg",
            "doorstop_libs/*",
            "dotnet/*",
        ])
        .flatten_top_level(true);

        BepInEx {
            plugin_installer,
            loader_installer,
        }
    }
}

/// [`ModLoader`] for the [BepInEx](https://github.com/BepInEx/BepInEx) Unity modding framework.
///
/// This type can either be created with [`BepInEx::default()`] or [`BepInEx::new()`] for a default
/// configuration, or via [`BepInExBuilder`] for more configuration options.
///
/// Supports both the Mono and IL2CPP editions of both BepInEx 5 and 6.
#[derive(Debug, Clone)]
pub struct BepInEx {
    loader_installer: ExtractInstaller,
    plugin_installer: RuleInstaller,
}

impl BepInEx {
    /// Creates a new [`BepInEx`] instance with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for BepInEx {
    fn default() -> Self {
        BepInExBuilder::default().build()
    }
}

impl ModLoader for BepInEx {
    fn to_str(&self) -> &'static str {
        "BepInEx"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        const DEFAULT_DOORSTOP_VERSION: u32 = 3;

        let preloader_path =
            find_preloader(profile_root)?.ok_or(Error::BepInExPreloaderNotFound)?;

        let doorstop_version =
            read_doorstop_version(profile_root)?.unwrap_or(DEFAULT_DOORSTOP_VERSION);

        make_doorstop_args(doorstop_version, true, preloader_path)
    }

    fn package_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.plugin_installer
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }

    fn log_path(&self, profile_root: &Path) -> Option<PathBuf> {
        Some(profile_root.join("BepInEx").join("LogOutput.log"))
    }

    fn mod_config_dirs(&self, profile_root: &Path) -> Vec<PathBuf> {
        vec![profile_root.join("BepInEx").join("config")]
    }

    fn prepare_launch(&self, profile_root: &Path, game_root: &Path) -> Result<()> {
        crate::util::copy_matching_files(
            profile_root,
            game_root,
            &[
                "doorstop_libs/*",
                "dotnet/*",
                "*.dll",
                "*.sh",
                ".doorstop_version",
                "doorstop_config.ini",
            ],
        )
    }
}

pub(super) fn read_doorstop_version(profile_root: &Path) -> Result<Option<u32>> {
    let path = profile_root.join(".doorstop_version");

    path.exists()
        .then(|| {
            let version = fs::read_to_string(&path)
                .wrap_err(path, "reading doorstop version file")?
                .split('.') // read only the major version number
                .next()
                .and_then(|str| str.parse().ok())
                .ok_or(Error::InvalidDoorstopVersionFormat)?;

            Ok(version)
        })
        .transpose()
}

pub(super) fn make_doorstop_args(
    doorstop_version: u32,
    enabled: bool,
    target_assembly: PathBuf,
) -> Result<Vec<OsString>> {
    let (enable_arg, target_arg) = match doorstop_version {
        3 => ("--doorstop-enable", "--doorstop-target"),
        4 => ("--doorstop-enabled", "--doorstop-target-assembly"),
        vers => return Err(Error::UnsupportedDoorstopVersion(vers)),
    };

    Ok(vec![
        enable_arg.into(),
        enabled.to_string().into(),
        target_arg.into(),
        target_assembly.into(),
    ])
}

pub(super) fn find_preloader(profile_root: &Path) -> Result<Option<PathBuf>> {
    const PRELOADER_NAMES: &[&str] = &[
        "BepInEx.Unity.Mono.Preloader.dll",
        "BepInEx.Unity.IL2CPP.dll",
        "BepInEx.Preloader.dll",
        "BepInEx.IL2CPP.dll",
    ];

    let core_path = profile_root.join("BepInEx").join("core");

    let path = core_path
        .read_dir()
        .wrap_err(&core_path, "reading core directory")?
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            let file_name = entry.file_name();
            PRELOADER_NAMES.iter().any(|name| file_name == **name)
        })
        .map(|entry| entry.path());

    Ok(path)
}

#[cfg(test)]
mod test {
    use tempfile::TempDir;

    use super::*;
    use std::borrow::Cow;

    fn assert_map_plugin_file(
        bepinex: &BepInEx,
        relative_path: &str,
        package_name: &str,
        expected: &str,
    ) {
        assert_eq!(
            bepinex
                .plugin_installer
                .map_file(Path::new(relative_path), package_name),
            Some(Cow::Borrowed(Path::new(expected)))
        );
    }

    #[test]
    fn it_works() {
        let bepinex = BepInEx::new();

        assert_map_plugin_file(&bepinex, "mod.dll", "mod", "BepInEx/plugins/mod/mod.dll");
        assert_map_plugin_file(
            &bepinex,
            "plugins/mod.dll",
            "mod",
            "BepInEx/plugins/mod/mod.dll",
        );
        assert_map_plugin_file(&bepinex, "core/mod.dll", "mod", "BepInEx/core/mod/mod.dll");
        assert_map_plugin_file(
            &bepinex,
            "patchers/mod.dll",
            "mod",
            "BepInEx/patchers/mod/mod.dll",
        );
        assert_map_plugin_file(
            &bepinex,
            "monomod/mod.dll",
            "mod",
            "BepInEx/monomod/mod/mod.dll",
        );
        assert_map_plugin_file(
            &bepinex,
            "mod.mm.dll",
            "mod",
            "BepInEx/monomod/mod/mod.mm.dll",
        );
        assert_map_plugin_file(&bepinex, "config/mod.cfg", "mod", "BepInEx/config/mod.cfg");
    }

    #[test]
    fn it_flattens_unnested_folders() {
        let bepinex = BepInEx::new();

        assert_map_plugin_file(
            &bepinex,
            "folder/icon.png",
            "mod",
            "BepInEx/plugins/mod/icon.png",
        );
        assert_map_plugin_file(
            &bepinex,
            "folder/core/icon.png",
            "mod",
            "BepInEx/core/mod/icon.png",
        );
        assert_map_plugin_file(
            &bepinex,
            "folder/mod.mm.dll",
            "mod",
            "BepInEx/monomod/mod/mod.mm.dll",
        );
    }

    #[test]
    fn it_retains_nested_folders() {
        let bepinex = BepInEx::new();

        assert_map_plugin_file(
            &bepinex,
            "plugins/folder/icon.png",
            "mod",
            "BepInEx/plugins/mod/folder/icon.png",
        );
        assert_map_plugin_file(
            &bepinex,
            "core/folder/icon.png",
            "mod",
            "BepInEx/core/mod/folder/icon.png",
        );
    }

    #[test]
    fn it_reads_doorstop_version() {
        let tempdir = TempDir::new().unwrap();

        let version_file = tempdir.path().join(".doorstop_version");

        fs::write(&version_file, "3.0.1").unwrap();
        assert_eq!(read_doorstop_version(tempdir.path()).unwrap(), Some(3));

        fs::write(version_file, "4.2.0").unwrap();
        assert_eq!(read_doorstop_version(tempdir.path()).unwrap(), Some(4));
    }

    #[test]
    fn it_doesnt_make_up_doorstop_version() {
        let tempdir = TempDir::new().unwrap();

        assert_eq!(read_doorstop_version(tempdir.path()).unwrap(), None);
    }

    #[test]
    fn it_makes_doorstop_args() {
        let target_assembly = PathBuf::from("path/to/preloader.dll");

        let args = make_doorstop_args(3, true, target_assembly.clone()).unwrap();
        assert_eq!(
            args,
            vec![
                OsString::from("--doorstop-enable"),
                OsString::from("true"),
                OsString::from("--doorstop-target"),
                target_assembly.clone().into_os_string()
            ]
        );

        let args = make_doorstop_args(4, false, target_assembly.clone()).unwrap();
        assert_eq!(
            args,
            vec![
                OsString::from("--doorstop-enabled"),
                OsString::from("false"),
                OsString::from("--doorstop-target-assembly"),
                target_assembly.into_os_string()
            ]
        );
    }

    #[test]
    fn it_returns_error_for_unsupported_doorstop_version() {
        let target_assembly = PathBuf::from("path/to/preloader.dll");

        let tempdir = TempDir::new().unwrap();

        let version_file = tempdir.path().join(".doorstop_version");

        fs::write(&version_file, "5.0.0").unwrap();

        let err = make_doorstop_args(5, true, target_assembly).unwrap_err();
        match err {
            Error::UnsupportedDoorstopVersion(5) => {}
            _ => panic!("Expected UnsupportedDoorstopVersion error"),
        }
    }

    #[test]
    fn it_returns_error_for_invalid_doorstop_version_file() {
        let tempdir = TempDir::new().unwrap();

        let version_file = tempdir.path().join(".doorstop_version");

        fs::write(&version_file, "notanumber").unwrap();

        let err = read_doorstop_version(tempdir.path()).unwrap_err();
        match err {
            Error::InvalidDoorstopVersionFormat => {}
            _ => panic!("Expected InvalidDoorstopVersionFormat error"),
        }
    }
}
