use std::{
    fs,
    io::{self, Cursor, Read, Seek},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};
use loadsmith_core::{Checksum, ChecksumAlgorithm};
use serde::de::DeserializeOwned;
use tracing::trace;
use zip::ZipArchive;

use crate::{Error, Result, r2z::ProfileManifest};

/// Reads an r2z (r2modman zip) profile export.
///
/// Provides access to the manifest and config files stored in the archive.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// use loadsmith_thunderstore::r2z::{ImportFile, ExportFile, ProfileManifest, Mod, Version};
/// use thunderstore::PackageIdent;
///
/// let manifest = ProfileManifest::new(
///     "Test",
///     vec![Mod::new(PackageIdent::new("A", "B"), Version::new(1, 0, 0), true)],
///     (),
/// );
/// let data = ExportFile::create(Cursor::new(Vec::new()), &manifest).unwrap().finish().unwrap();
///
/// let mut import = ImportFile::open(data).unwrap();
/// let read = import.read_manifest::<()>().unwrap();
/// assert_eq!(read.profile_name, "Test");
/// ```
pub struct ImportFile<R> {
    zip: ZipArchive<R>,
}

impl<R: Read + Seek> ImportFile<R> {
    /// Opens an r2z archive for reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use loadsmith_thunderstore::r2z::ImportFile;
    ///
    /// let import = ImportFile::open(Cursor::new(Vec::new()));
    /// ```
    pub fn open(reader: R) -> Result<Self> {
        Ok(Self {
            zip: ZipArchive::new(reader)?,
        })
    }

    /// Reads and deserialises the `export.r2x` manifest from the archive.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use loadsmith_thunderstore::r2z::{ImportFile, ExportFile, ProfileManifest};
    ///
    /// let manifest = ProfileManifest::new("Test", vec![], ());
    /// let data = ExportFile::create(Cursor::new(Vec::new()), &manifest).unwrap().finish().unwrap();
    ///
    /// let mut import = ImportFile::open(data).unwrap();
    /// let read: ProfileManifest<()> = import.read_manifest().unwrap();
    /// assert_eq!(read.profile_name, "Test");
    /// ```
    pub fn read_manifest<T: DeserializeOwned>(&mut self) -> Result<ProfileManifest<T>> {
        let file = self.zip.by_name("export.r2x").map_err(|err| match err {
            zip::result::ZipError::FileNotFound => Error::ProfileManifestNotFound,
            err => Error::Zip(err),
        })?;

        let manifest: ProfileManifest<T> = serde_yaml_ng::from_reader(file)?;

        Ok(manifest)
    }

    /// Iterates over every file in the archive (except `export.r2x`)
    /// and invokes `callback` with the file path and a reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use loadsmith_thunderstore::r2z::{ImportFile, ExportFile, ProfileManifest, Mod, Version};
    /// use thunderstore::PackageIdent;
    ///
    /// let manifest = ProfileManifest::new(
    ///     "Test",
    ///     vec![Mod::new(PackageIdent::new("A", "B"), Version::new(1, 0, 0), true)],
    ///     (),
    /// );
    /// let mut export = ExportFile::create(Cursor::new(Vec::new()), &manifest).unwrap();
    /// export.write_file("config.cfg", &b"enabled=true"[..]).unwrap();
    /// let data = export.finish().unwrap();
    ///
    /// let mut import = ImportFile::open(data).unwrap();
    /// import.read_files(|path, _reader| {
    ///     assert_eq!(path, std::path::Path::new("config.cfg"));
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn read_files<F>(&mut self, mut callback: F) -> Result<()>
    where
        F: FnMut(PathBuf, &mut dyn Read) -> Result<()>,
    {
        for i in 0..self.zip.len() {
            let mut file = self.zip.by_index(i)?;
            if file.name() == "export.r2x" {
                continue;
            }

            let path = file.mangled_name();
            callback(path, &mut file)?;
        }

        Ok(())
    }

    /// Extracts config files from the archive into a target directory.
    ///
    /// When `filter` is `true`, executable and script file types (`.dll`,
    /// `.exe`, `.bat`, etc.) as well as `export.r2x` and `mods.yml` are
    /// skipped. Files under a `config/` prefix are mapped to `BepInEx/config/`.
    /// Files that already exist with identical content are skipped.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io::Cursor;
    /// use loadsmith_thunderstore::r2z::{ImportFile, ExportFile, ProfileManifest};
    ///
    /// let manifest = ProfileManifest::new("Test", vec![], ());
    /// let data = ExportFile::create(Cursor::new(Vec::new()), &manifest).unwrap().finish().unwrap();
    ///
    /// let mut import = ImportFile::open(data).unwrap();
    /// import.import_config_files("./output", true).unwrap();
    /// ```
    pub fn import_config_files(&mut self, target: impl AsRef<Path>, filter: bool) -> Result<()> {
        let target = target.as_ref();
        self.read_files(|relative_path, reader| {
            let mut relative_path = Utf8PathBuf::from(relative_path.to_string_lossy().into_owned());

            if relative_path.starts_with("config") {
                relative_path = Utf8PathBuf::from("BepInEx").join(relative_path);
            }

            if filter && is_excluded(&relative_path) {
                trace!(path = %relative_path, "skipping excluded file");
                return Ok(());
            }

            let target_path = target.join(&relative_path);

            let mut buf = Vec::new();
            reader.read_to_end(&mut buf)?;

            if target_path.exists() {
                if Checksum::compute_from_path(&target_path, ChecksumAlgorithm::Blake3)?
                    == Checksum::compute(Cursor::new(&buf), ChecksumAlgorithm::Blake3)?
                {
                    trace!(path = %relative_path, "skipping identical file");
                    return Ok(());
                }
            }

            trace!(path = %relative_path, "importing file");

            loadsmith_util::create_parent_dirs(&target_path)?;

            let mut file = fs::File::create(target_path)?;
            io::copy(&mut Cursor::new(buf), &mut file)?;

            Ok(())
        })
    }
}

fn is_excluded(relative_path: impl AsRef<Utf8Path>) -> bool {
    static EXCLUDE_SET: LazyLock<GlobSet> = LazyLock::new(|| {
        GlobSetBuilder::new()
            .add(Glob::new("export.r2x").unwrap())
            .add(Glob::new("mods.yml").unwrap())
            .add(Glob::new("*.{dll,exe,scr,com,pif,bat,cmd,ps1,vbs,vbe,js,jse,wsf,wsh,hta,msi,msix,sys,drv,cpl,ocx,lnk,reg,inf}").unwrap())
            .build()
            .unwrap()
    });

    EXCLUDE_SET.is_match(relative_path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_excluded() {
        assert!(is_excluded("export.r2x"));
        assert!(is_excluded("mods.yml"));
        assert!(!is_excluded("nested/mods.yml"));
        assert!(is_excluded("some/path/to/file.dll"));
        assert!(!is_excluded("some/path/to/file.txt"));
        assert!(!is_excluded("some/path/to/file.json"));
    }
}
