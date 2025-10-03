use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    ModLoader, PackageInstaller, Result,
    extract::ExtractInstaller,
    rule::{Rule, RuleInstaller},
};

#[derive(Debug, Clone)]
pub struct ReturnOfModding {
    loader_installer: ExtractInstaller,
    plugin_installer: RuleInstaller,
}

impl ReturnOfModding {
    pub fn new() -> Self {
        let loader_installer = ExtractInstaller::new(["*.dll"]).flatten_top_level(true);

        let plugin_installer = RuleInstaller::new(vec![
            Rule::separated("plugins", Path::new("ReturnOfModding/plugins")),
            Rule::separated("plugins_data", Path::new("ReturnOfModding/plugins_data")),
            Rule::separated("config", Path::new("ReturnOfModding/config")).with_mutability(true),
        ])
        .with_default(0);

        Self {
            loader_installer,
            plugin_installer,
        }
    }
}

impl ModLoader for ReturnOfModding {
    fn to_str(&self) -> &'static str {
        "ReturnOfModding"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        Ok(vec![
            "--rom_modding_root_folder".into(),
            profile_root.into(),
        ])
    }

    fn package_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.plugin_installer
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }

    fn prepare_launch(&self, profile_root: &Path, game_root: &Path) -> Result<()> {
        crate::util::copy_matching_files(profile_root, game_root, &["*.dll"])
    }

    fn mod_config_dirs(&self, _profile_root: &Path) -> Vec<PathBuf> {
        vec!["ReturnOfModding/config".into()]
    }
}
