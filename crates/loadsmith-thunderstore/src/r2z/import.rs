use std::{
    fs,
    io::{self, Read, Seek},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::de::DeserializeOwned;
use tracing::trace;
use zip::ZipArchive;

use crate::{Error, Result, r2z::ProfileManifest};

pub struct ImportFile<R> {
    zip: ZipArchive<R>,
}

impl<R: Read + Seek> ImportFile<R> {
    pub fn open(reader: R) -> Result<Self> {
        Ok(Self {
            zip: ZipArchive::new(reader)?,
        })
    }

    pub fn read_manifest<T: DeserializeOwned>(&mut self) -> Result<ProfileManifest<T>> {
        let file = self.zip.by_name("export.r2x").map_err(|err| match err {
            zip::result::ZipError::FileNotFound => Error::ProfileManifestNotFound,
            err => Error::Zip(err),
        })?;

        let manifest: ProfileManifest<T> = serde_yaml_ng::from_reader(file)?;

        Ok(manifest)
    }

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
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut file = fs::File::create(target_path)?;
            io::copy(reader, &mut file)?;

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
