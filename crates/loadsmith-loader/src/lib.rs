use std::{borrow::Cow, fmt::Debug};

use camino::{Utf8Path, Utf8PathBuf};
use loadsmith_core::LaunchArgs;
use loadsmith_install::InstallRuleset;

mod doorstop;
mod error;
mod loaders;

pub use error::{Error, Result};
pub use loaders::*;

pub trait Loader: Debug {
    fn id(&self) -> &'static str;

    fn package_install_rules(&self) -> InstallRuleset<'_>;
    fn loader_install_rules(&self) -> InstallRuleset<'_>;

    fn get_launch_args(&self, ctx: LaunchContext) -> Result<LaunchArgs>;
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct LaunchContext<'a> {
    pub profile: Cow<'a, Utf8Path>,
    pub game_path: Cow<'a, Utf8Path>,
    pub is_proton: bool,
}

impl<'a> LaunchContext<'a> {
    pub fn new(
        profile: impl Into<Cow<'a, Utf8Path>>,
        game_path: impl Into<Cow<'a, Utf8Path>>,
        is_proton: bool,
    ) -> Self {
        Self {
            profile: profile.into(),
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

    fn format_proton_path<'b>(&self, path: &'b Utf8Path) -> Cow<'b, Utf8Path> {
        if self.is_proton {
            let path = format!("Z:{path}");
            Cow::Owned(Utf8PathBuf::from(path))
        } else {
            Cow::Borrowed(path)
        }
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
