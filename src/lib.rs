//! # Loadsmith
//!
//! A crate for handling mod extraction and (un)installation with a range of mod loaders.
//!
//! This crate is primarily focused on [Thunderstore](https://thunderstore.io) compatibility,
//! and was originally a part of the mod manager [Gale](https://github.com/Kesomannen/gale).
//! The supported mod loaders is those found on Thunderstore's registry, including:
//!
//! * [BepInEx](crate::loaders::BepInEx)
//! * [MelonLoader](crate::loaders::MelonLoader)
//! * [Shimloader](crate::loaders::Shimloader)
//! * [ReturnOfModding](crate::loaders::ReturnOfModding)
//! * [GDWeave](crate::loaders::GDWeave)
//! * [Lovely](crate::loaders::Lovely)
//! * [Northstar](crate::loaders::Northstar)
//! * [BepisLoader](crate::loaders::BepisLoader)
//!
//! ## Usage
//!
//! The main two traits are [`ModLoader`] and [`PackageInstaller`].
//!
//! A [`ModLoader`] holds methods related to one modding framework and sometimes pieces of game-specific configuration.
//! It is implemented by all the structs listed above.
//!
//! A [`PackageInstaller`] is responsible for extracting, installing and bookkeeping the installed mods in a directory.
//! Each [`ModLoader`] has a pair of installers: [`ModLoader::package_installer()`] and [`ModLoader::loader_installer()`].
//! The former used to install regular packages/mods, and the later for the mod loader itself.
//!
//! ### Install a mod
//!
//! Installation happens in two steps:
//!
//! * Firstly, the mod's zip archive is extracted according to the mod loader's packaging rules.
//! * Secondly, the contents of the extracted directory are copied/linked to the destination directory (called the mod profile).
//! Here the installer may do some extra operations, for example writing to a [state file](#state-file).
//!
//! This can be used to easily and efficiently cache mods, as you can extract the archive to some directory,
//! then install it into as many profiles you want. If you want a simpler flow however, there is also
//! [`PackageInstaller::extract_and_install`], which extracts to a temporary directory.
//!
//! ```rust
//! use std::path::Path;
//! use loadsmith::loaders::BepInEx;
//!
//! // create a BepInEx loader with the default config
//! let loader = BepInEx::new();
//! let profile_path = Path::new("profiles/Default");
//!
//! // our mod archive, maybe downloaded from Thunderstore
//! // you can also use the `zip` crate directly
//! let zip = loadsmith::open_zip("mods/MoreCompany.zip")?;
//!
//! // the second argument is a unique identifier for the mod (for this profile specifically)
//! // in a Thunderstore context, this is commonly set to `AUTHOR-NAME`
//! loader.extract_and_install(zip, "notnotnotswipez-MoreCompany", profile_path)?;
//!
//! // then install BepInEx itself
//! let zip = loadsmith::open_zip("mods/BepInExPack.zip")?;
//! loader.loader_installer().extract_and_install(zip, "BepInEx-BepInExPack", profile_path)?;
//! ```
//!
//! ### Uninstall a mod
//!
//! ```rust
//! use std::path::Path;
//! use loadsmith::loaders::BepInEx;
//!
//! let loader = BepInEx::new();
//! let profile_path = Path::new("profiles/Default");
//!
//! // second argument must be the same as when the mod was installed
//! loader.uninstall(profile_path, "notnotnotswipez-MoreCompany")?;
//! ```
//!
//! ### List files belonging to a mod
//!
//! ```rust
//! use std::path::Path;
//! use loadsmith::loaders::MelonLoader;
//!
//! let loader = MelonLoader::new();
//! let profile_path = Path::new("profiles/Default");
//!
//! // again, this name must be consistent for each mod
//! let file_paths = loader.package_files(profile_path, "MedicalMess-MultiplayerPlus")?;
//!
//! for path in file_paths {
//!     // will print an absolute path
//!     println!("{}", path.display());
//! }
//! ```
//!
//! ### Launch the (modded) game
//!
//! ```rust
//! use std::{path::Path, process::Command};
//! use loadsmith::loaders::MelonLoader;
//!
//! let loader = GDWeave::new();
//! let profile_path = Path::new("profiles/Default");
//! let game_path = Path::new("C:/Program Files/Steam/steamapps/common/WEBFISHING");
//!
//! // this usually copies modloader files to the game directory
//! loader.prepare_launch(profile_path, game_path);
//!
//! let args = loader.get_launch_args(profile_path)?;
//!
//! Command::new("steam") // this requires Steam to be on $PATH
//!     .args(["-applaunch", "3146520"]) // app ID for WEBFISHING
//!     .args(args)
//!     .spawn()?;
//! ```
//!

use std::{
    env,
    ffi::OsString,
    fs::{self, File},
    io::{BufReader, Read, Seek},
    path::{Path, PathBuf},
};

use tempfile::TempDir;
use zip::ZipArchive;

mod error;
pub mod extract;
pub mod loaders;
pub mod rule;
mod state;
mod util;

pub use error::*;

use crate::util::InstallOptions;

/// Any type that can be used as a reader for [`zip::ZipArchive`].
pub trait AnyZipReader: Read + Seek {}
impl<T> AnyZipReader for T where T: Read + Seek {}

/// A [`zip::ZipArchive`] wrapping a boxed reader.
///
/// You can construct this from a file path with [`open_zip`].
///
/// This is used in the methods on [`PackageInstaller`] to erase the [`zip::ZipArchive`]
/// generics and thus make the trait be dyn-compatible.
pub type AnyZipArchive = ZipArchive<Box<dyn AnyZipReader>>;

/// An instance of a mod loader that can be used to install a mod loader, mods and launch the game.
///
/// For many mod loaders, this carries no actual data or state and thus can be shared across your application.
/// Even for those with game-specific configuration like [`BepInEx`](crate::loaders::BepInEx), since methods only take `&self`,
/// this can easily be shared by wrapping it in a smart-pointer like [`LazyLock`](std::sync::LazyLock) or [`Arc`](std::sync::Arc).
pub trait ModLoader {
    /// The name of this mod loader.
    fn to_str(&self) -> &'static str;

    /// Prepares a game to be launched using this mod loader with the specificed profile.
    ///
    /// This usually copies essential files to the game directory for the mod loader itself to launch.
    /// For example, proxy DLLs that are used in the injection process.
    ///
    /// It is important to call this method before [`get_launch_args`](ModLoader::get_launch_args) and actually executing the game.
    fn prepare_launch(&self, _profile_root: &Path, _game_root: &Path) -> Result<()> {
        Ok(())
    }

    /// Creates the command arguments that should be ran against the game in order to invoke the mod loader.
    ///
    /// **Example**
    ///
    /// ```rust
    /// use std::{path::Path, process::Command};
    ///
    /// // use the working directory as our profile
    /// let profile_path = Path::new(".");
    /// let game_path = Path::new("path/to/game/install");
    ///
    /// let loader = BepInEx::new();
    ///
    /// // don't forget this!
    /// loader.prepare_launch(profile_path, game_path)?;
    ///
    /// let args = loader.get_launch_args(profile_path)?;
    ///
    /// Command::new("steam") // this requires steam to be on $PATH
    ///     .args(["-applaunch", "3146520"])
    ///     .args(&args) // steam will forward these extra arguments to the game executable
    ///     .spawn()?;
    ///
    /// // alternatively launch the game directly:
    ///
    /// let exe_path = game_path.join("game.exe");
    ///
    /// Command::new(exe_path)
    ///     .args(args)
    ///     .spawn()?;
    /// ```
    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>>;

    /// The [`PackageInstaller`] that should be used to manage standard mods.
    ///
    /// You rarely need to call this explicitly, since the implementation of
    /// [`PackageInstaller`] on [`ModLoader`] itself forwards every call to this.
    fn package_installer<'a>(&'a self) -> &'a dyn PackageInstaller;

    /// The [`PackageInstaller`] that should be used to manage the mod loader itself.
    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller;

    /// Returns an optional, absolute path to the mod loader's log file inside the specified profile.
    ///
    /// Note that this may return `Some` even for a file that doesn't exist.
    fn log_path(&self, _profile_root: &Path) -> Option<PathBuf> {
        None
    }

    /// The directories to search for mod-specific configuration files.
    ///
    /// This returns absolute paths but may be empty.
    fn mod_config_dirs(&self, _profile_root: &Path) -> Vec<PathBuf> {
        Vec::new()
    }
}

impl<T> PackageInstaller for T
where
    T: ModLoader,
{
    fn extract(
        &self,
        archive: AnyZipArchive,
        package_name: &str,
        output_path: &Path,
    ) -> Result<()> {
        self.package_installer()
            .extract(archive, package_name, output_path)
    }

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<Vec<PathBuf>> {
        self.package_installer()
            .package_files(install_root, package_name)
    }

    fn install(
        &self,
        profile_root: &Path,
        source_root: &Path,
        package_name: &str,
        use_links: bool,
    ) -> Result<()> {
        self.package_installer()
            .install(profile_root, source_root, package_name, use_links)
    }

    fn uninstall(&self, profile_root: &Path, package_name: &str) -> Result<()> {
        self.package_installer()
            .uninstall(profile_root, package_name)
    }

    fn extract_and_install(
        &self,
        archive: AnyZipArchive,
        package_name: &str,
        profile_root: &Path,
    ) -> Result<()> {
        self.package_installer()
            .extract_and_install(archive, package_name, profile_root)
    }
}

/// A struct responsible for extracting, installing and keeping track of installed mod files
/// according to some ruleset (either defined by the mod loader itself or Thunderstore package convention).
///
/// You'll most often get an instance of this trait from [`ModLoader`], or use [`ModLoader`] directly as it
/// also implements this type (by forwarding calls to [`ModLoader::package_installer`]).
///
/// Each method in this trait takes a `package_name`, which must be a unique name for each mod that
/// distinguises it from others within the same profile. In Thunderstore contexts, this is commonly set
/// to `AUTHOR-NAME` (omitting the version, since you normally dissallow multiple versions of the same mod being installed).
///
/// ## State
///
/// This trait is designed to be stateless, thus only taking `&self` references. However, some implementors
/// may do their bookkeeping by maintaining a "state file" inside the profile directory. A notable example is
/// [`RuleInstaller`](rule::RuleInstaller)'s [`RuleMode::Track`](rule::RuleMode::Track) rules, used by
/// [`MelonLoader`](loaders::MelonLoader) among others.
///
/// See [`RuleMode::Track`](rule::RuleMode::Track) for more information.
pub trait PackageInstaller {
    /// Extracts a mod archive to the specified `output_path`, according to this implementors
    /// specific unpacking rules.
    ///
    /// The files will be placed exactly as they will once installed in a profile.
    ///
    /// This should be used in an empty directory since it will overwite existing files.
    fn extract(&self, archive: AnyZipArchive, package_name: &str, output_path: &Path)
    -> Result<()>;

    /// Lists the files within `install_root` that "belong" to the specified package.
    ///
    /// Normally this shouldn't return an non-existent paths, however, if the install directory has been
    /// mutated from outside loadsmith, this may become out of sync.
    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<Vec<PathBuf>>;

    /// Installs a mod into a profile from a source directory.
    ///
    /// This is mostly a straight copy from `source_root` to `profile_root`, except that the
    /// implementor may want to do some extra book-keeping, such as writing to a state file.
    ///
    /// ## Linking
    ///
    /// `use_links` determines whether the files will be properly copied (`false`), or linked
    /// using [`std::fs::hard_link`] (`true`). The later consumes less disk space and is much faster,
    /// but not supported on some storage devices and configurations. Also note that some files
    /// (for example config files) will always be copied, as loadsmith expects them be changed on a
    /// per-profile basis, which linking would disallow.
    fn install(
        &self,
        profile_root: &Path,
        source_root: &Path,
        _package_name: &str,
        use_links: bool,
    ) -> Result<()> {
        util::install(
            profile_root,
            source_root,
            InstallOptions::default().should_link(util::InstallOpt::Const(use_links)),
        )
    }

    /// Uninstalls a mod from a profile.
    ///
    /// This will attempt to find and delete every file that was installed with the mod, but that is not always
    /// possible (such as when using [`RuleMode::None`](rule::RuleMode::None)).
    ///
    /// This also clears the profile from empty directories (which aren't deleted as a part of the core
    /// uninstallation process).
    fn uninstall(&self, profile_root: &Path, package_name: &str) -> Result<()> {
        for file in self.package_files(profile_root, package_name)? {
            fs::remove_file(&file).wrap_err(file, "removing package file")?;
        }

        util::delete_empty_dirs(profile_root)?;

        Ok(())
    }

    /// Returns the directory most "assossicated" with an installed mod.
    ///
    /// This may return a non-existent path.
    fn package_dir(&self, _install_root: &Path, _package_name: &str) -> Result<Option<PathBuf>> {
        Ok(None)
    }

    /// Extracts a mod zip to a temporary folder and then installs it into a profile.
    ///
    /// See [`extract`](PackageInstaller::extract) and [`install`](PackageInstaller::install) for more details.
    fn extract_and_install(
        &self,
        archive: AnyZipArchive,
        package_name: &str,
        profile_root: &Path,
    ) -> Result<()> {
        let tempdir = TempDir::new().wrap_err(env::temp_dir(), "creating temporary directory")?;

        self.extract(archive, package_name, tempdir.path())?;
        self.install(profile_root, tempdir.path(), package_name, true)?;

        Ok(())
    }
}

/// Opens a [`zip::ZipArchive`] from the specified path.
///
/// The file reader is boxed, so that it can be used with the methods on [`PackageInstaller`]
/// while keeping the trait dyn-compatible.
pub fn open_zip(path: impl AsRef<Path>) -> Result<AnyZipArchive> {
    let reader: Box<dyn AnyZipReader> = File::open(&path)
        .wrap_err(path.as_ref(), "opening zip file")
        .map(BufReader::new)
        .map(Box::new)?;

    let zip = ZipArchive::new(reader)?;

    Ok(zip)
}
