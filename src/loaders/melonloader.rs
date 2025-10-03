use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    ModLoader, PackageInstaller, Result,
    extract::ExtractInstaller,
    rule::{Rule, RuleInstaller},
};

#[derive(Debug, Default)]
pub struct MelonLoaderBuilder {
    extra_installer_rules: Vec<Rule>,
    custom_state_file_path: Option<PathBuf>,
}

impl MelonLoaderBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_extra_rules(mut self, extra_rules: Vec<Rule>) -> Self {
        self.extra_installer_rules = extra_rules;
        self
    }

    pub fn with_state_file_name(mut self, state_file_path: impl Into<PathBuf>) -> Self {
        self.custom_state_file_path = Some(state_file_path.into());
        self
    }

    pub fn build(self) -> MelonLoader {
        let rules = [
            Rule::tracked("UserLibs", Path::new("UserLibs")).with_extensions(["lib.dll"]),
            Rule::tracked("Managed", Path::new("MelonLoader/Managed"))
                .with_extensions(["managed.dll"]),
            Rule::tracked("Mods", Path::new("Mods")).with_extensions(["dll"]),
            Rule::separated("ModManager", Path::new("UserData/ModManager")),
            Rule::tracked("MelonLoader", Path::new("MelonLoader")),
            Rule::tracked("Libs", Path::new("MelonLoader/Libs")),
        ]
        .into_iter()
        .chain(self.extra_installer_rules.into_iter())
        .collect();

        let mut plugin_installer = RuleInstaller::new(rules);

        if let Some(path) = self.custom_state_file_path {
            plugin_installer = plugin_installer.with_state_file_path(path);
        }

        let loader_installer = ExtractInstaller::new([
            "dobby.dll",
            "version.dll",
            "MelonLoader/Dependencies/*",
            "MelonLoader/Documentation/*",
            "MelonLoader/net6/*",
            "MelonLoader/net35/*",
        ]);

        MelonLoader {
            plugin_installer,
            loader_installer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MelonLoader {
    plugin_installer: RuleInstaller,
    loader_installer: ExtractInstaller,
}

impl MelonLoader {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for MelonLoader {
    fn default() -> Self {
        MelonLoaderBuilder::default().build()
    }
}

impl ModLoader for MelonLoader {
    fn to_str(&self) -> &'static str {
        "MelonLoader"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        let mut vec = vec!["--melonloader.basedir".into(), profile_root.into()];

        let mono_assembly_exists = profile_root
            .join("MelonLoader/Managed/Assembly-CSharp.dll")
            .exists();
        let il2cpp_assembly_exists = profile_root
            .join("MelonLoader/Il2CppAssemblies/Assembly-CSharp.dll")
            .exists();

        if !mono_assembly_exists && !il2cpp_assembly_exists {
            vec.push("--melonloader.agfregenerate".into());
        }

        Ok(vec)
    }

    fn default_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
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
        crate::util::copy_matching_files(profile_root, game_root, &["*.dll"])
    }
}
