use std::{
    borrow::Cow,
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use crate::{
    AnyZipArchive, ModLoader, PackageInstaller, Result,
    rule::{Rule, RuleInstaller},
};

/// [`ModLoader`] for the [Shimloader](https://github.com/thunderstore-io/unreal-shimloader) Unreal Engine mod loader.
#[derive(Debug, Clone)]
pub struct Shimloader {
    internal_game_name: String,
    loader_installer: ShimloaderInstaller,
    plugin_installer: RuleInstaller,
}

impl Shimloader {
    pub fn new(internal_game_name: impl Into<String>) -> Self {
        let plugin_installer = RuleInstaller::new(vec![
            Rule::flat_separated("mod", Path::new("shimloader/mod")),
            Rule::flat_separated("pak", Path::new("shimloader/pak")),
            Rule::untracked("cfg", Path::new("shimloader/cfg")),
        ])
        .with_default(0);

        Self {
            internal_game_name: internal_game_name.into(),
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

    fn package_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.plugin_installer
    }

    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller {
        &self.loader_installer
    }

    fn prepare_launch(&self, profile_root: &Path, game_root: &Path) -> Result<()> {
        let target_dir = game_root
            .join(&self.internal_game_name)
            .join("Binaries")
            .join("Win64");

        crate::util::copy_matching_files(
            profile_root,
            &target_dir,
            &["*.dll", "UE4SS-settings.ini"],
        )
    }

    fn mod_config_dirs(&self, _profile_root: &Path) -> Vec<PathBuf> {
        vec!["shimloader/cfg".into()]
    }
}

/// A [`PackageInstaller`] for the Unreal Shimloader mod loader package.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct ShimloaderInstaller;

impl PackageInstaller for ShimloaderInstaller {
    fn extract(
        &self,
        archive: AnyZipArchive,
        _package_name: &str,
        output_path: &Path,
    ) -> Result<()> {
        crate::util::extract(archive, output_path, |relative_path| {
            let mut components = relative_path.components();
            let in_ue4ss = relative_path.starts_with("UE4SS");

            if in_ue4ss {
                components.next(); // flatten the UE4SS folder
            }

            let Some(Component::Normal(next)) = components.clone().next() else {
                return None;
            };

            match next.to_str() {
                Some("dwmapi.dll") => {
                    // the Shimloader package has 2 dwmapi.dll files, the one inside UE4SS doesn't seem to work.
                    if in_ue4ss {
                        return None;
                    }

                    Some(Cow::Borrowed(components.as_path()))
                }
                Some("UE4SS.dll" | "UE4SS-settings.ini") => {
                    Some(Cow::Borrowed(components.as_path()))
                }
                Some("Mods") => {
                    // place built-in mods (under UE4SS/Mods into their own mod folders)
                    components.next();

                    let mut path: PathBuf = ["shimloader", "mod"].iter().collect();
                    path.push(components);

                    Some(Cow::Owned(path))
                }
                _ => None,
            }
        })
    }

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        _package_name: &'a str,
    ) -> Result<Vec<PathBuf>> {
        // we can't include the builtin mods in a good way, since Mods/* would include *every* mods' files
        crate::util::match_files_in_dir(install_root, &["*.dll", "UE4SS-settings.ini"])
    }
}
