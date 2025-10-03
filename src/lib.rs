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
mod loaders;
pub mod rule;
mod state;
mod util;

pub use error::*;
pub use loaders::*;

use crate::util::InstallOptions;

pub trait AnyZipReader: Read + Seek {}
impl<T> AnyZipReader for T where T: Read + Seek {}

pub type AnyZipArchive = ZipArchive<Box<dyn AnyZipReader>>;

pub trait ModLoader {
    fn to_str(&self) -> &'static str;

    fn prepare_launch(&self, _profile_root: &Path, _game_root: &Path) -> Result<()> {
        Ok(())
    }
    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>>;

    fn default_installer<'a>(&'a self) -> &'a dyn PackageInstaller;
    fn loader_installer<'a>(&'a self) -> &'a dyn PackageInstaller;

    fn log_path(&self, _profile_root: &Path) -> Option<PathBuf> {
        None
    }
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
        self.default_installer()
            .extract(archive, package_name, output_path)
    }

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<Vec<PathBuf>> {
        self.default_installer()
            .package_files(install_root, package_name)
    }

    fn install(
        &self,
        profile_root: &Path,
        source_root: &Path,
        package_name: &str,
        use_links: bool,
    ) -> Result<()> {
        self.default_installer()
            .install(profile_root, source_root, package_name, use_links)
    }

    fn uninstall(&self, profile_root: &Path, package_name: &str) -> Result<()> {
        self.default_installer()
            .uninstall(profile_root, package_name)
    }

    fn extract_and_install(
        &self,
        archive: AnyZipArchive,
        package_name: &str,
        profile_root: &Path,
    ) -> Result<()> {
        self.default_installer()
            .extract_and_install(archive, package_name, profile_root)
    }
}

pub trait PackageInstaller {
    fn extract(&self, archive: AnyZipArchive, package_name: &str, output_path: &Path)
    -> Result<()>;

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<Vec<PathBuf>>;

    fn install(
        &self,
        profile_root: &Path,
        source_root: &Path,
        _package_name: &str,
        _use_links: bool,
    ) -> Result<()> {
        util::install(profile_root, source_root, InstallOptions::default())
    }

    fn uninstall(&self, profile_root: &Path, package_name: &str) -> Result<()> {
        for file in self.package_files(profile_root, package_name)? {
            fs::remove_file(&file).wrap_err(file, "removing package file")?;
        }

        util::delete_empty_dirs(profile_root)?;

        Ok(())
    }

    fn package_dir(&self, _install_root: &Path, _package_name: &str) -> Result<Option<PathBuf>> {
        Ok(None)
    }

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

pub fn open_zip(path: impl AsRef<Path>) -> Result<AnyZipArchive> {
    let reader: Box<dyn AnyZipReader> = File::open(&path)
        .wrap_err(path.as_ref(), "opening zip file")
        .map(BufReader::new)
        .map(Box::new)?;

    let zip = ZipArchive::new(reader)?;

    Ok(zip)
}
