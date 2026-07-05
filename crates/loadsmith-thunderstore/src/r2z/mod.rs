use std::{
    fmt::Display,
    path::{Component, Path},
};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use thunderstore::PackageIdent;

use crate::{Error, Result};

mod export;
mod import;

pub use export::ExportFile;
pub use import::ImportFile;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileManifest<T = ()> {
    pub profile_name: String,
    pub mods: Vec<Mod>,
    #[serde(flatten)]
    pub extra: T,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub name: PackageIdent,
    pub version: Version,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl From<Version> for loadsmith_core::Version {
    fn from(value: Version) -> Self {
        Self::new(value.major, value.minor, value.patch)
    }
}

impl From<loadsmith_core::Version> for Version {
    fn from(value: loadsmith_core::Version) -> Self {
        Self::new(value.major, value.minor, value.patch)
    }
}

impl<T> ProfileManifest<T> {
    pub fn new(name: impl Into<String>, mods: Vec<Mod>, extra: T) -> Self {
        Self {
            profile_name: name.into(),
            mods,
            extra,
        }
    }
}

impl Mod {
    pub fn new(name: impl Into<PackageIdent>, version: impl Into<Version>, enabled: bool) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            enabled,
        }
    }
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl From<(u64, u64, u64)> for Version {
    fn from((major, minor, patch): (u64, u64, u64)) -> Self {
        Self::new(major, minor, patch)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Normalizes a path so it can be compared against a globset in a predictable way.
/// Removes `.` and `..` components and converts all separators to `/`.
fn normalize_zip_file_path(path: impl AsRef<Path>) -> Result<Utf8PathBuf> {
    let path = path.as_ref();
    return inner(path).ok_or_else(|| Error::InvalidZipFilePath(path.to_path_buf()));

    fn inner(path: &Path) -> Option<Utf8PathBuf> {
        let mut normalized = Utf8PathBuf::new();

        for component in path.components() {
            match component {
                Component::Normal(os_str) => {
                    normalized.push(os_str.to_str()?);
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use std::{assert_matches, io::Cursor};

    use super::*;

    #[test]
    fn normalize_path_normalizes() {
        assert_eq!(normalize_zip_file_path("test.txt").unwrap(), "test.txt");
        assert_eq!(normalize_zip_file_path("./test.txt").unwrap(), "test.txt");
        assert_eq!(normalize_zip_file_path("././test.txt").unwrap(), "test.txt");
        assert_eq!(
            normalize_zip_file_path("./test.txt/./.").unwrap(),
            "test.txt"
        );
        assert_eq!(
            normalize_zip_file_path("nested/test.txt").unwrap(),
            "nested/test.txt"
        );
        assert_eq!(
            normalize_zip_file_path("nested/../test.txt").unwrap(),
            "test.txt"
        );
    }

    #[test]
    fn normalize_path_rejects_invalid() {
        assert_matches!(
            normalize_zip_file_path("../text.txt"),
            Err(Error::InvalidZipFilePath(_))
        );
        assert_matches!(
            normalize_zip_file_path("nested/../../test.txt"),
            Err(Error::InvalidZipFilePath(_))
        );
        #[cfg(unix)]
        assert_matches!(
            normalize_zip_file_path("/absolute/path/test.txt"),
            Err(Error::InvalidZipFilePath(_))
        );
        #[cfg(target_os = "windows")]
        assert_matches!(
            normalize_zip_file_path("C:\\absolute\\path\\test.txt"),
            Err(Error::InvalidZipFilePath(_))
        );
    }

    #[test]
    fn import_export_roundtrip() {
        let manifest = ProfileManifest::new(
            "My Profile",
            vec![Mod::new(
                PackageIdent::new("BepInEx", "BepInExPack"),
                (5, 4, 2100),
                true,
            )],
            (),
        );
        let mut export = ExportFile::create(Cursor::new(Vec::new()), &manifest).unwrap();
        export
            .write_file("config/BepInEx.cfg", &b"Some bytes"[..])
            .unwrap();
        let data = export.finish().unwrap();

        let mut import = ImportFile::open(data).unwrap();
        let read_manifest = import.read_manifest().unwrap();
        assert_eq!(read_manifest, manifest);

        import
            .read_files(|path, file| {
                assert_eq!(path, Path::new("config/BepInEx.cfg"));

                let mut str = String::new();
                file.read_to_string(&mut str)?;
                assert_eq!(str, "Some bytes");

                Ok(())
            })
            .unwrap();
    }
}
