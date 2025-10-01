use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, BufReader, Read, Seek},
    iter,
    path::{Path, PathBuf},
};

use tempdir::TempDir;
use zip::ZipArchive;

mod error;
mod loaders;
mod rule;
mod state;
#[cfg(test)]
mod tests;
mod util;

pub use error::*;
pub use loaders::*;

pub trait ModLoader {
    fn to_str(&self) -> &'static str;

    fn prepare_launch(&self, _profile_root: &Path, _game_root: &Path) -> Result<()> {
        Ok(())
    }
    fn get_launch_args(&self, profile_root: &Path) -> Result<Vec<OsString>>;

    fn default_installer<'a>(&'a self) -> &'a impl PackageInstaller;
    fn loader_installer<'a>(&'a self) -> &'a impl PackageInstaller;

    fn log_path(&self, _profile_root: &Path) -> Option<PathBuf> {
        None
    }
    fn mod_config_dirs(&self, _profile_root: &Path) -> impl Iterator<Item = PathBuf> {
        iter::empty()
    }
}

impl<T> PackageInstaller for T
where
    T: ModLoader,
{
    fn extract<R: Read + Seek>(
        &self,
        archive: ZipArchive<R>,
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
    ) -> Result<impl Iterator<Item = Result<PathBuf>> + 'a> {
        self.default_installer()
            .package_files(install_root, package_name)
    }

    fn is_mutable(&self, relative_path: impl AsRef<Path>) -> bool {
        self.default_installer().is_mutable(relative_path)
    }

    fn should_overwrite(&self, relative_path: impl AsRef<Path>) -> bool {
        self.default_installer().should_overwrite(relative_path)
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

    fn extract_and_install<R: Read + Seek>(
        &self,
        archive: ZipArchive<R>,
        package_name: &str,
        profile_root: &Path,
    ) -> Result<()> {
        self.default_installer()
            .extract_and_install(archive, package_name, profile_root)
    }
}

pub trait PackageInstaller {
    fn extract<R: Read + Seek>(
        &self,
        archive: ZipArchive<R>,
        package_name: &str,
        output_path: &Path,
    ) -> Result<()>;

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<impl Iterator<Item = Result<PathBuf>> + 'a>;

    #[allow(unused_variables)]
    fn is_mutable(&self, relative_path: impl AsRef<Path>) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn should_overwrite(&self, relative_path: impl AsRef<Path>) -> bool {
        false
    }

    fn install(
        &self,
        profile_root: &Path,
        source_root: &Path,
        package_name: &str,
        use_links: bool,
    ) -> Result<()> {
        for file in self.package_files(source_root, package_name)? {
            let file = file?;

            let relative_path = file
                .strip_prefix(source_root)
                .expect("source file should be child of the source root");
            let use_link = use_links && !self.is_mutable(relative_path);
            let overwite = self.should_overwrite(relative_path);

            util::install_package_file(
                profile_root,
                source_root,
                &relative_path,
                overwite,
                use_link,
            )?;
        }

        Ok(())
    }

    fn uninstall(&self, profile_root: &Path, package_name: &str) -> Result<()> {
        for file in self.package_files(profile_root, package_name)? {
            let file = file?;
            if !file.exists() {
                continue;
            }
            fs::remove_file(file)?;
        }

        Ok(())
    }

    fn extract_and_install<R: Read + Seek>(
        &self,
        archive: ZipArchive<R>,
        package_name: &str,
        profile_root: &Path,
    ) -> Result<()> {
        let tempdir = TempDir::new("loadsmith")?;

        self.extract(archive, package_name, tempdir.path())?;
        self.install(profile_root, tempdir.path(), package_name, true)?;

        tempdir.close()?;
        Ok(())
    }
}

// const KNOWN_GENERATED_FILES: &[&str] = &[
//     "profile.yml",
//     "_state",
//     "profile.json",
//     "snapshots",
//     "tempest.toml",
//     "tempest.lock",
//     ".tempest",
// ];

// fn is_known_generated(file_name: &str) -> bool {
//     KNOWN_GENERATED_FILES.contains(&file_name)
// }

pub fn open_zip(path: impl AsRef<Path>) -> io::Result<ZipArchive<BufReader<File>>> {
    let reader = File::open(path).map(BufReader::new)?;
    let zip = ZipArchive::new(reader)?;
    Ok(zip)
}
