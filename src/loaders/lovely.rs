use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    IoResultExt, ModLoader, PackageInstaller, Result,
    extract::ExtractInstaller,
    rule::{Rule, RuleInstaller},
};

/// [`ModLoader`] for the [Lovely](https://github.com/ethangreen-dev/lovely-injector) LÖVE 2d modding framework.
#[derive(Debug, Clone)]
pub struct Lovely {
    loader_installer: ExtractInstaller,
    package_installer: RuleInstaller,
}

impl Lovely {
    pub fn new() -> Self {
        let loader_installer = ExtractInstaller::new(["version.dll"]);
        let package_installer = RuleInstaller::new(vec![Rule::separated("", Path::new("mods"))]);

        Self {
            loader_installer,
            package_installer,
        }
    }
}

impl ModLoader for Lovely {
    fn to_str(&self) -> &'static str {
        "Lovely"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        Ok(vec!["--mod-dir".into(), profile_root.join("mods").into()])
    }

    fn package_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.package_installer
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }

    fn prepare_launch(&self, profile_root: &Path, game_root: &Path) -> Result<()> {
        let source_path = profile_root.join("version.dll");
        let target_path = game_root.join("version.dll");

        fs::copy(&source_path, &target_path)
            .wrap_err(target_path, "copying proxy dll to game directory")?;

        Ok(())
    }

    fn log_path(&self, _profile_root: &Path) -> Option<PathBuf> {
        Some("mods/lovely/log".into())
    }

    fn mod_config_dirs(&self, _profile_root: &Path) -> Vec<PathBuf> {
        Vec::new()
    }
}
