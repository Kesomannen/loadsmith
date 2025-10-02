use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::{
    AnyZipArchive, ModLoader, PackageInstaller, Result,
    rule::{Rule, RuleInstaller},
};

#[derive(Debug, Clone)]
pub struct Shimloader {
    loader_installer: ShimloaderInstaller,
    plugin_installer: RuleInstaller,
}

impl Shimloader {
    pub fn new() -> Self {
        let plugin_installer = RuleInstaller::new(vec![
            Rule::flat_separated("mod", Path::new("shimloader/mod")),
            Rule::flat_separated("pak", Path::new("shimloader/pak")),
            Rule::untracked("cfg", Path::new("shimloader/cfg")),
        ])
        .with_default(0);

        Self {
            loader_installer: ShimloaderInstaller,
            plugin_installer,
        }
    }
}

impl ModLoader for Shimloader {
    fn to_str(&self) -> &'static str {
        "Shimloader"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        let path = profile_root.join("shimloader");

        Ok(vec![
            "--mod-dir".into(),
            path.join("mod").into(),
            "--pak-dir".into(),
            path.join("pak").into(),
            "--cfg-dir".into(),
            path.join("cfg").into(),
        ])
    }

    fn default_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.plugin_installer
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ShimloaderInstaller;

impl PackageInstaller for ShimloaderInstaller {
    fn extract(
        &self,
        archive: AnyZipArchive,
        package_name: &str,
        output_path: &Path,
    ) -> Result<()> {
        todo!()
    }

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<Vec<PathBuf>> {
        todo!()
    }
}
