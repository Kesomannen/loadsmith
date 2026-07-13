use std::{fmt::Debug, path::PathBuf, sync::LazyLock};

use camino::Utf8PathBuf;
use globset::{Glob, GlobBuilder, GlobSet};
use loadsmith_core::PackageRef;
use loadsmith_install::InstallRuleset;

mod args;
mod context;
mod doorstop;
mod error;
mod loaders;

pub use args::LaunchArgs;
pub use context::LaunchContext;
pub use error::{Error, Result};
pub use loaders::*;

pub trait Loader: Debug + Send + Sync {
    fn id(&self) -> &'static str;

    fn package_install_rules(&self) -> InstallRuleset<'_>;
    fn loader_install_rules(&self) -> InstallRuleset<'_>;

    fn prepare_launch(&self, ctx: &LaunchContext) -> Result<()> {
        static GLOB_SET: LazyLock<GlobSet> = LazyLock::new(|| {
            GlobSet::builder()
                .add(top_level_dll_glob())
                .build()
                .expect("constant globs should be valid")
        });

        ctx.copy_glob_to_game(&GLOB_SET)
    }

    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs>;

    fn package_dir(&self, package: &PackageRef) -> Option<PathBuf> {
        self.package_install_rules()
            .default_rule()
            .and_then(|default_rule| default_rule.map_file("", package))
            .map(Utf8PathBuf::into_std_path_buf)
    }

    fn package_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
    }
    fn log_file(&self) -> Option<PathBuf> {
        None
    }
    fn proxy_dll(&self) -> Option<PathBuf> {
        None
    }
}

fn top_level_dll_glob() -> Glob {
    GlobBuilder::new("*.dll")
        .literal_separator(true)
        .build()
        .expect("constant glob should be valid")
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
