//! Mod loader definitions and launch-argument generation for the loadsmith
//! mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.
//!
//! # Examples
//!
//! ```rust
//! use loadsmith_loader::{BepInEx, Loader};
//!
//! let loader = BepInEx::with_default_rules();
//! assert_eq!(loader.id(), "BepInEx");
//! ```

use std::{fmt::Debug, path::PathBuf, sync::LazyLock};

use camino::Utf8PathBuf;
use globset::{Glob, GlobBuilder, GlobSet};
use loadsmith_core::PackageRef;
use loadsmith_install::InstallRuleset;

mod args;
mod context;
pub mod doorstop;
mod error;
mod loaders;

pub use args::LaunchArgs;
pub use context::LaunchContext;
pub use error::{Error, Result};
pub use loaders::*;

/// A mod loader definition that knows how to install packages and generate
/// launch arguments for a target game.
///
/// Each loader implementation describes:
///
/// * How its own loader-pack files are placed (`loader_install_rules`).
/// * How mod packages are placed (`package_install_rules`).
/// * What CLI arguments / environment variables are needed at launch
///   (`generate_launch_args`).
///
/// By default [`prepare_launch`](Loader::prepare_launch) copies all top-level
/// `*.dll` files from the profile into the game directory.
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::{BepInEx, Loader};
///
/// let loader = BepInEx::with_default_rules();
/// assert_eq!(loader.id(), "BepInEx");
/// ```
pub trait Loader: Debug + Send + Sync {
    /// A unique, human-readable identifier for this loader (e.g. `"BepInEx"`).
    fn id(&self) -> &'static str;

    /// Returns the install rules for the loader's own files (the "loader pack").
    fn package_install_rules(&self) -> InstallRuleset<'_>;
    /// Returns the install rules for end-user mod packages.
    fn loader_install_rules(&self) -> InstallRuleset<'_>;

    /// Prepares the game directory by copying files from the profile.
    ///
    /// The default implementation copies every top-level `*.dll` file.
    fn prepare_launch(&self, ctx: &LaunchContext) -> Result<()> {
        static GLOB_SET: LazyLock<GlobSet> = LazyLock::new(|| {
            GlobSet::builder()
                .add(top_level_dll_glob())
                .build()
                .expect("constant globs should be valid")
        });

        ctx.copy_glob_to_game(&GLOB_SET)
    }

    /// Builds the command-line arguments and environment variables needed to
    /// launch the game with this loader.
    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs>;

    /// Returns the directory where a package's files are placed, if any.
    fn package_dir(&self, package: &PackageRef) -> Option<PathBuf> {
        self.package_install_rules()
            .default_rule()
            .and_then(|default_rule| default_rule.map_file("", package))
            .map(Utf8PathBuf::into_std_path_buf)
    }

    /// Directories that contain mutable per-package configuration.
    fn package_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
    }
    /// Path to the loader's log file, relative to the game directory.
    fn log_file(&self) -> Option<PathBuf> {
        None
    }
    /// Filename of the proxy DLL used by this loader, if any.
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
