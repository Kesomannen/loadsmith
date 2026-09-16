//! Mod loader definitions and launch-argument generation for the loadsmith
//! mod-manager library.
//!
//! This is an internal crate of the `loadsmith` workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.

use std::{fmt::Debug, path::PathBuf, sync::LazyLock};

use camino::Utf8PathBuf;
use globset::{Glob, GlobBuilder, GlobSet};
use loadsmith_core::PackageRef;
use loadsmith_install::rule::InstallRuleset;

mod args;
mod context;
pub mod doorstop;
mod error;
mod loaders;

pub use args::LaunchArgs;
pub use context::LaunchContext;
pub use error::{Error, Result};
pub use loaders::*;

/// A mod loader definition.
///
/// This is a central trait of the loadsmith library. The idea is to instantiate one loader
/// for each supported game, which can then be used as an abstraction over the game- and
/// loader-sensitive logic, such as package installation and game launching.
///
/// Each loader implementation describes the following:
///
/// * How its own loader-pack files are placed ([`loader_install_rules`](Loader::loader_install_rules)).
/// * How mod package files are placed ([`package_install_rules`](Loader::package_install_rules)).
/// * What CLI arguments / environment variables are needed at launch ([`generate_launch_args`](Loader::generate_launch_args)).
/// * How to prepare the game directory for launch ([`prepare_launch`](Loader::prepare_launch)).
///
/// Along with miscellaneous other metadata useful for various mod-manager operations.
///
/// Current implementations from within loadsmith include:
///
/// * [`BepInEx`]
/// * [`BepisLoader`]
/// * [`GDWeave`]
/// * [`Lovely`]
/// * [`MelonLoader`]
/// * [`Northstar`]
/// * [`ReturnOfModding`]
/// * [`Rivet`]
/// * [`Shimloader`]
pub trait Loader: Debug + Send + Sync {
    /// A unique, human-readable identifier for this loader (e.g. `"BepInEx"`).
    fn id(&self) -> &'static str;

    /// Returns the installation rules for the loader's own files (the "loader pack").
    fn package_install_rules(&self) -> InstallRuleset<'_>;
    /// Returns the installation rules for end-user mod packages.
    fn loader_install_rules(&self) -> InstallRuleset<'_>;

    /// Prepares the game directory for a launch.
    ///
    /// Many, if not all loaders, require some files to be present in the game directory before
    /// the game is launched. This usually includes proxy DLLs used to hook into the game's process,
    /// but may also include other files such as required libraries or config definitions. To aid
    /// with this, the `LaunchContext` struct provides the convenient [`copy_glob_to_game`](LaunchContext::copy_glob_to_game).
    ///
    /// This method is not constrained to only copy files; it may perform any necessary preparation steps
    /// based on the incoming `LaunchContext` and the loader's requirements.
    ///
    /// The default implementation, however, simply copies all top-level `*.dll` files from the
    /// profile directory directly into the game directory.
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
    /// launch the game with this loader in the given [`LaunchContext`].
    fn generate_launch_args(&self, ctx: &LaunchContext) -> Result<LaunchArgs>;

    /// Returns the directory where a package's files are placed, if any.
    fn package_dir(&self, package: &PackageRef) -> Option<PathBuf> {
        self.package_install_rules()
            .default_rule()
            .and_then(|default_rule| default_rule.map_file("", package))
            .map(Utf8PathBuf::into_std_path_buf)
    }

    /// Directories that contain mutable per-package configuration.
    ///
    /// Defaults to an empty list.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use std::path::PathBuf;
    /// # use loadsmith_loader::{Loader, BepInEx, Rivet};
    /// let bepinex = BepInEx::with_default_rules();
    /// assert_eq!(
    ///     bepinex.package_config_dirs(),
    ///     vec![PathBuf::from("BepInEx/config")]
    /// );
    ///
    /// let rivet = Rivet::new();
    /// assert!(rivet.package_config_dirs().is_empty());
    /// ```
    fn package_config_dirs(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Path to the loader's log file, relative to the profile directory.
    ///
    /// Defaults to `None`, indicating the loader does not have a log file.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use loadsmith_loader::{Loader, BepInEx, MelonLoader};
    /// let bepinex = BepInEx::with_default_rules();
    /// assert_eq!(bepinex.log_file(), Some("BepInEx/BepInEx.log".into()));
    ///
    /// let melonloader = MelonLoader::with_default_legacy_rules();
    /// assert_eq!(melonloader.log_file(), Some("MelonLoader/Latest.log".into()));
    /// ```
    fn log_file(&self) -> Option<PathBuf> {
        None
    }

    /// Filename of the proxy DLL used by this loader, if any.
    ///
    /// This can be used by mod managers to set up proxy DLL specific configuration,
    /// such as including it in the [`WINEDLLOVERRIDES` environment variable](https://protonscr.com/option/WINEDLLOVERRIDES).
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
    use loadsmith_install::rule::InstallRuleset;

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
