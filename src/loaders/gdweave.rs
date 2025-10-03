use std::{
    borrow::Cow,
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use itertools::Itertools;
use walkdir::WalkDir;

use crate::{AnyZipArchive, Error, ModLoader, PackageInstaller, Result, extract::ExtractInstaller};

#[derive(Debug, Clone)]
pub struct GDWeave {
    package_installer: GDWeavePackageIntaller,
    loader_installer: ExtractInstaller,
}

impl GDWeave {
    pub fn new() -> Self {
        let loader_installer = ExtractInstaller::new(["winmm.dll", "GDWeave/core/*"]);

        Self {
            package_installer: GDWeavePackageIntaller,
            loader_installer,
        }
    }
}

impl ModLoader for GDWeave {
    fn to_str(&self) -> &'static str {
        "GDWeave"
    }

    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>> {
        let path = profile_root.join("GDWeave");
        let path = path
            .to_str()
            .ok_or_else(|| Error::NonUTF8Path(profile_root.to_path_buf()))?;

        let arg = format!("--gdweave-folder-override={path}");

        Ok(vec![arg.into()])
    }

    fn default_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.package_installer
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }

    fn prepare_launch(&self, profile_root: &Path, game_root: &Path) -> Result<()> {
        crate::util::copy_matching_files(profile_root, game_root, &["winmm.dll"])
    }

    fn log_path(&self, _profile_root: &Path) -> Option<PathBuf> {
        Some("GDWeave/GDWeave.log".into())
    }

    fn mod_config_dirs(&self, _profile_root: &Path) -> Vec<PathBuf> {
        vec!["GDWeave/configs".into()]
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct GDWeavePackageIntaller;

impl GDWeavePackageIntaller {
    pub fn new() -> Self {
        Self::default()
    }
}

fn mod_install_root(package_name: &str) -> PathBuf {
    ["GDWeave", "mods", package_name].iter().collect()
}

impl PackageInstaller for GDWeavePackageIntaller {
    fn extract(
        &self,
        mut archive: AnyZipArchive,
        package_name: &str,
        output_path: &Path,
    ) -> Result<()> {
        let mut roots: Vec<PathBuf> = Vec::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;

            let Some(path) = file.enclosed_name() else {
                continue;
            };

            let mut components = path.components();

            match components.next_back() {
                Some(Component::Normal(name))
                    if name == "manifest.json" && components.clone().count() > 0 =>
                {
                    roots.push(components.collect());
                }
                _ => (),
            }
        }

        let root = match roots.len() {
            0 => return Err(Error::NoGDWeaveModRoots),
            1 => roots.into_iter().next().unwrap(),
            _ => return Err(Error::MultipleGDWeaveModRoots),
        };

        let target_root = mod_install_root(package_name);

        crate::util::extract(archive, output_path, |relative_path| {
            relative_path
                .strip_prefix(&root)
                .map(|relative_to_root| Cow::Owned(target_root.join(relative_to_root)))
                .ok()
        })
    }

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<Vec<PathBuf>> {
        let mod_root = install_root.join(mod_install_root(package_name));

        if !mod_root.exists() {
            return Ok(Vec::new());
        }

        WalkDir::new(mod_root)
            .into_iter()
            .filter_ok(|entry| entry.file_type().is_file())
            .map(|entry| match entry {
                Ok(entry) => Ok(entry.into_path()),
                Err(err) => Err(Error::Walkdir(err)),
            })
            .collect()
    }

    fn package_dir(&self, install_root: &Path, package_name: &str) -> Result<Option<PathBuf>> {
        let mod_root = install_root.join(mod_install_root(package_name));

        Ok(Some(mod_root))
    }
}
