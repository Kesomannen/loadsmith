use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{Error, ModLoader, PackageInstaller, Result, extract::ExtractInstaller, rule::Rule};

#[derive(Debug, Default)]
pub struct BepisLoaderBuilder {
    extra_installer_rules: Vec<Rule>,
    custom_state_file_path: Option<PathBuf>,
    doorstop_version_override: Option<u32>,
}

impl BepisLoaderBuilder {
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

    pub fn with_doorstop_version(mut self, version: u32) -> Self {
        self.doorstop_version_override = Some(version);
        self
    }

    pub fn build(mut self) -> BepisLoader {
        self.extra_installer_rules.push(Rule::flat_separated(
            "Renderer",
            Path::new("Renderer/BepInEx/plugins"),
        ));

        let mut bepinex = super::BepInExBuilder::new().with_extra_rules(self.extra_installer_rules);

        if let Some(state_file_path) = self.custom_state_file_path {
            bepinex = bepinex.with_state_file_name(state_file_path);
        }

        let loader_installer = ExtractInstaller::new([
            "BepisLoader.*",
            "hookfxr.*",
            "LinuxBootstrap.sh",
            "BepInEx/core/*",
            "BepInEx/icon.ico",
            "Renderer/BepInEx/core/*",
        ])
        .flatten_top_level(true);

        BepisLoader {
            doorstop_version_override: self.doorstop_version_override,
            inner: bepinex.build(),
            loader_installer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BepisLoader {
    doorstop_version_override: Option<u32>,
    loader_installer: ExtractInstaller,
    inner: super::BepInEx,
}

impl BepisLoader {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for BepisLoader {
    fn default() -> Self {
        BepisLoaderBuilder::default().build()
    }
}

impl ModLoader for BepisLoader {
    fn to_str(&self) -> &'static str {
        "BepisLoader"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        let bepinex_path = profile_root.join("BepInEx");

        let mut args = vec![
            "--hookfxr-enable".into(),
            "--bepinex-target".into(),
            bepinex_path.into(),
        ];

        let preloader_result = super::bepinex::find_preloader(&profile_root.join("Renderer"))
            .and_then(|option| option.ok_or(Error::BepInExPreloaderNotFound));

        if let Ok(preloader_path) = preloader_result {
            const DEFAULT_DOORSTOP_VERSION: u32 = 3;

            let doorstop_version = match self.doorstop_version_override {
                Some(version) => version,
                None => super::bepinex::read_doorstop_version(profile_root)?
                    .unwrap_or(DEFAULT_DOORSTOP_VERSION),
            };

            let doorstop_args =
                super::bepinex::make_doorstop_args(doorstop_version, true, preloader_path)?;

            args.extend_from_slice(&doorstop_args);
        }

        Ok(args)
    }

    fn package_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        self.inner.package_installer()
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }

    fn log_path(&self, profile_root: &Path) -> Option<PathBuf> {
        self.inner.log_path(profile_root)
    }

    fn mod_config_dirs(&self, profile_root: &Path) -> Vec<PathBuf> {
        let mut inner = self.inner.mod_config_dirs(profile_root);
        inner.push("Renderer/BepInEx/config".into());
        inner
    }

    fn prepare_launch(&self, profile_root: &Path, game_root: &Path) -> Result<()> {
        self.inner.prepare_launch(profile_root, game_root)
    }
}
