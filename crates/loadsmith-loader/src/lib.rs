use std::{
    borrow::Cow,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use camino::{Utf8Path, Utf8PathBuf};
use globset::GlobSet;
use loadsmith_core::LaunchArgs;
use loadsmith_install::InstallRuleset;

mod doorstop;
mod error;
mod loaders;

pub use error::{Error, Result};
pub use loaders::*;
use walkdir::WalkDir;

pub trait Loader: Debug + Send + Sync {
    fn id(&self) -> &'static str;

    fn package_install_rules(&self) -> InstallRuleset<'_>;
    fn loader_install_rules(&self) -> InstallRuleset<'_>;

    fn prepare_launch(&self, ctx: &LaunchContext) -> Result<()> {
        let _ = ctx;
        Ok(())
    }
    fn get_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs>;

    fn mod_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
    }
    fn log_file(&self) -> Option<PathBuf> {
        None
    }
    fn proxy_dll(&self) -> Option<PathBuf> {
        None
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct LaunchContext<'a> {
    pub profile_path: Cow<'a, Utf8Path>,
    pub game_path: Cow<'a, Utf8Path>,
    pub is_proton: bool,
}

impl<'a> LaunchContext<'a> {
    pub fn new(
        profile_path: impl Into<Cow<'a, Utf8Path>>,
        game_path: impl Into<Cow<'a, Utf8Path>>,
        is_proton: bool,
    ) -> Self {
        Self {
            profile_path: profile_path.into(),
            game_path: game_path.into(),
            is_proton,
        }
    }

    #[cfg(test)]
    pub(crate) fn in_tempdir(tempdir: &tempfile::TempDir, is_proton: bool) -> Result<Self> {
        use std::fs;

        let path = Utf8Path::from_path(tempdir.path()).expect("temp path should be UTF-8");

        let profile = path.join("profile");
        let game = path.join("game");

        fs::create_dir_all(&profile)?;
        fs::create_dir_all(&game)?;

        Ok(Self::new(profile, game, is_proton))
    }

    pub fn format_proton_path<'b>(&self, path: &'b Utf8Path) -> Cow<'b, Utf8Path> {
        if self.is_proton {
            let path = format!("Z:{path}");
            Cow::Owned(Utf8PathBuf::from(path))
        } else {
            Cow::Borrowed(path)
        }
    }

    pub fn copy_file_to_game(
        &self,
        relative_source: impl AsRef<Path>,
        relative_target: impl AsRef<Path>,
    ) -> Result<()> {
        let profile_path = self.profile_path.as_std_path().join(relative_source);
        let game_path = self.game_path.as_std_path().join(relative_target);

        if let Some(parent) = game_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(profile_path, game_path)?;

        Ok(())
    }

    pub fn copy_glob_to_game(&self, patterns: &GlobSet) -> Result<()> {
        self.copy_files_to_game(|path| patterns.is_match(path))
    }

    pub fn copy_files_to_game<F>(&self, filter: F) -> Result<()>
    where
        F: Fn(&Path) -> bool,
    {
        WalkDir::new(&*self.profile_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| {
                entry
                    .into_path()
                    .strip_prefix(&*self.profile_path)
                    .expect("profile path should be a prefix of the file path")
                    .to_path_buf()
            })
            .filter(|relative_path| filter(relative_path))
            .try_for_each(|relative_path| self.copy_file_to_game(&relative_path, &relative_path))
    }
}

#[cfg(test)]
mod test_util {
    use camino::{Utf8Path, Utf8PathBuf};
    use loadsmith_core::PackageRef;
    use loadsmith_install::InstallRuleset;

    use crate::Loader;

    #[macro_export]
    macro_rules! assert_map {
        ($tester:expr, $file:literal, $expected:literal) => {
            assert_eq!(
                $tester.map($file),
                Some(camino::Utf8PathBuf::from($expected))
            );
        };
        ($tester:expr, $file:literal, None) => {
            assert_eq!($tester.map($file), None);
        };
    }

    #[macro_export]
    macro_rules! assert_maps {
        ($tester:expr, [$($file:literal => $expected:tt),* $(,)?]) => {
            {
                let _tester = $tester;
                $(
                    assert_map!(_tester, $file, $expected);
                )*
            }
        };
    }

    pub struct MapFileTester<T> {
        loader: T,
        package: PackageRef,
        loader_package: bool,
    }

    impl<T: Loader> MapFileTester<T> {
        pub fn new(loader: T, package: PackageRef, loader_package: bool) -> Self {
            Self {
                package,
                loader_package,
                loader,
            }
        }

        pub fn ruleset(&self) -> InstallRuleset<'_> {
            if self.loader_package {
                self.loader.loader_install_rules()
            } else {
                self.loader.package_install_rules()
            }
        }

        pub fn map(&self, file: impl AsRef<Utf8Path>) -> Option<Utf8PathBuf> {
            self.ruleset().map_file(file, &self.package)
        }
    }
}
