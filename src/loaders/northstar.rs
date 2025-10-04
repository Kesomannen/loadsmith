use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    Error, ModLoader, PackageInstaller, Result,
    extract::ExtractInstaller,
    rule::{Rule, RuleInstaller},
};

/// [`ModLoader`] for the [Northstar](https://github.com/R2Northstar/Northstar) Titanfall 2 modding framework.
#[derive(Debug, Clone)]
pub struct Northstar {
    loader_installer: ExtractInstaller,
    plugin_installer: RuleInstaller,
}

impl Northstar {
    pub fn new() -> Self {
        let loader_installer = ExtractInstaller::new([
            "Northstar.dll",
            "NorthstarLauncher.exe",
            "r2ds.bat",
            "LEGAL.txt",
            "bin/*",
            "R2Northstar/mods/Northstar.*/*",
            "R2Northstar/mods/md5sum.txt",
            "R2Northstar/mods/LICENSE",
            "R2Northstar/plugins/*",
        ])
        .flatten_top_level(true);

        let plugin_installer =
            RuleInstaller::new(vec![Rule::tracked("mods", Path::new("R2Northstar/mods"))]);

        Self {
            loader_installer,
            plugin_installer,
        }
    }
}

impl ModLoader for Northstar {
    fn to_str(&self) -> &'static str {
        "Northstar"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        let path = profile_root.join("R2Northstar");
        let path = path
            .to_str()
            .ok_or_else(|| Error::NonUTF8Path(profile_root.to_path_buf()))?;

        let arg = format!("-profile={path}");

        Ok(vec!["-northstar".into(), arg.into()])
    }

    fn package_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.plugin_installer
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }

    fn prepare_launch(&self, profile_root: &Path, game_root: &Path) -> Result<()> {
        crate::util::copy_matching_files(profile_root, game_root, &["*.dll", "*.exe", "*.bat"])
    }

    fn mod_config_dirs(&self, _profile_root: &Path) -> Vec<PathBuf> {
        Vec::new()
    }
}
