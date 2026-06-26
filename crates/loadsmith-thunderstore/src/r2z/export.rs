use std::{
    fs::File,
    io::{self, Read, Seek, Write},
    sync::LazyLock,
};

use camino::Utf8Path;
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Serialize;
use walkdir::WalkDir;
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::{Error, Result, r2z::ProfileManifest};

pub struct ExportFile<W: Write + Seek> {
    zip: ZipWriter<W>,
}

impl<W: Write + Seek> ExportFile<W> {
    pub fn create<T: Serialize>(writer: W, manifest: &ProfileManifest<T>) -> Result<Self> {
        let mut zip = ZipWriter::new(writer);

        zip.start_file("export.r2x", SimpleFileOptions::default())?;
        serde_yaml_ng::to_writer(&mut zip, manifest)?;

        Ok(Self { zip })
    }

    pub fn finish(self) -> Result<W> {
        self.zip.finish().map_err(Error::Zip)
    }

    pub fn write_file(
        &mut self,
        zip_path: impl AsRef<Utf8Path>,
        mut reader: impl Read,
    ) -> Result<()> {
        let path = super::normalize_zip_file_path(zip_path.as_ref())?;
        let path = path.into_string().replace("\\", "/");
        self.zip.start_file(path, SimpleFileOptions::default())?;

        io::copy(&mut reader, &mut self.zip)?;

        Ok(())
    }

    pub fn export_from_dir(&mut self, directory: impl AsRef<Utf8Path>, filter: bool) -> Result<()> {
        let directory = directory.as_ref();
        WalkDir::new(directory)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .try_for_each(|entry| {
                let source_path = Utf8Path::from_path(entry.path())
                    .ok_or_else(|| Error::NonUtf8Path(entry.path().to_path_buf()))?;

                let relative_path = source_path.strip_prefix(directory).unwrap_or(source_path);
                let normalized_path = super::normalize_zip_file_path(relative_path)?;

                if filter && !is_included(&normalized_path) {
                    return Ok(());
                }

                let mut file = File::open(source_path)?;
                self.write_file(relative_path, &mut file)
            })
    }
}

fn is_included(relative_path: impl AsRef<Utf8Path>) -> bool {
    static INCLUDE_SET: LazyLock<GlobSet> = LazyLock::new(|| {
        GlobSetBuilder::new()
            .add(Glob::new("BepInEx/config/*").unwrap())
            .add(Glob::new("*.{cfg,txt,json,yml,yaml,ini}").unwrap())
            .build()
            .unwrap()
    });

    static EXCLUDE_SET: LazyLock<GlobSet> = LazyLock::new(|| {
        GlobSetBuilder::new()
            .add(Glob::new("{dotnet,_state,MelonLoader}/*").unwrap())
            .add(Glob::new("dotnet/*").unwrap())
            .add(Glob::new("GDWeave/{GDWeave.log,core/*,mods/*}").unwrap())
            .add(Glob::new("mods.yml").unwrap())
            .add(
                GlobBuilder::new("BepInEx/plugins/*/manifest.json")
                    .literal_separator(true)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap()
    });

    let path = relative_path.as_ref();
    INCLUDE_SET.is_match(path) && !EXCLUDE_SET.is_match(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_included() {
        assert!(is_included("config.ini"));
        assert!(is_included("other/test.json"));
        assert!(is_included("BepInEx/config/test.png"));
        assert!(is_included("BepInEx/config/test.cfg"));

        assert!(!is_included("test.dll"));
        assert!(!is_included("nested/latest.log"));
        assert!(!is_included("mods.yml"));
        assert!(!is_included("random/test.exe"));
        assert!(!is_included("dotnet/test.cfg"));

        assert!(is_included("GDWeave/test.cfg"));
        assert!(!is_included("GDWeave/GDWeave.log"));
        assert!(!is_included("GDWeave/core/test.cfg"));
        assert!(!is_included("GDWeave/mods/test.cfg"));

        assert!(is_included("BepInEx/plugins/test.txt"));
        assert!(!is_included("BepInEx/plugins/Author-Name/manifest.json"));
        assert!(is_included(
            "BepInEx/plugins/Author-Name/nested/manifest.json"
        ));
    }
}
