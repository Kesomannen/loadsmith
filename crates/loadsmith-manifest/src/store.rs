use std::{
    fmt::Display,
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
};

use camino::Utf8PathBuf;
use loadsmith_core::{Checksum, InstalledPackage, PackageId, PackageRef, Version};
use loadsmith_install::InstallRuleset;
use walkdir::WalkDir;

use crate::{Error, Result};

/// Manages a central store of package files
#[derive(Debug, Clone)]
pub struct PackageStore {
    base_path: Arc<Path>,
    no_links: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageStoreEntry {
    package: PackageRef,
    checksum: Option<Checksum>,
}

impl PackageStore {
    pub fn open(base_path: impl Into<Arc<Path>>) -> Result<Self> {
        let base_path = base_path.into();
        if !base_path.is_dir() {
            std::fs::create_dir_all(&base_path)?;
        }
        Ok(Self {
            base_path,
            no_links: false,
        })
    }

    pub fn without_links(mut self) -> Self {
        self.no_links = true;
        self
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    pub fn path_of(&self, entry: &PackageStoreEntry) -> PathBuf {
        self.base_path.join(entry.path())
    }

    pub fn contains(&self, entry: &PackageStoreEntry) -> bool {
        self.path_of(entry).exists()
    }

    pub fn add<R: Read + Seek>(
        &self,
        entry: &PackageStoreEntry,
        reader: R,
    ) -> Result<Vec<PathBuf>> {
        let target = self.path_of(entry);
        if target.exists() {
            return Err(Error::PackageStoreEntryAlreadyExists);
        }

        std::fs::create_dir_all(&target)?;
        let files = loadsmith_install::extract(reader, &target)?;
        Ok(files)
    }

    pub fn install(
        &self,
        entry: PackageStoreEntry,
        ruleset: InstallRuleset,
        profile: impl AsRef<Path>,
    ) -> Result<(InstalledPackage, Vec<Utf8PathBuf>)> {
        let source = self.path_of(&entry);

        let (install, overriden_files) = loadsmith_install::install(
            entry.package,
            ruleset,
            source,
            profile,
            self.no_links,
            entry.checksum,
        )?;

        Ok((install, overriden_files))
    }

    pub fn remove(&self, entry: &PackageStoreEntry) -> Result<()> {
        let path = self.path_of(entry);
        std::fs::remove_dir_all(&path)?;
        loadsmith_util::remove_empty_parents(path)?;
        Ok(())
    }

    pub fn entry_from_path(&self, path: impl AsRef<Path>) -> Result<PackageStoreEntry> {
        let path = path.as_ref();
        let relative_path = path
            .strip_prefix(&self.base_path)
            .map_err(|_| Error::InvalidPackageStoreEntryPath)?;

        PackageStoreEntry::try_from_path(relative_path)
    }

    pub fn unused_entries(&self) -> impl Iterator<Item = Result<PackageStoreEntry>> + '_ {
        self.entries().filter(|entry| match entry {
            Ok(entry) => self.is_unused(entry),
            Err(_) => true,
        })
    }

    pub fn entries(&self) -> impl Iterator<Item = Result<PackageStoreEntry>> + '_ {
        WalkDir::new(&self.base_path)
            .follow_links(false)
            .min_depth(3)
            .max_depth(3)
            .into_iter()
            .filter_map(|entry| match entry {
                Ok(entry) => {
                    if !entry.file_type().is_dir() {
                        return None;
                    }

                    Some(self.entry_from_path(entry.path()))
                }
                Err(err) => Some(Err(err.into())),
            })
    }

    fn is_unused(&self, entry: &PackageStoreEntry) -> bool {
        WalkDir::new(self.path_of(entry))
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .all(|#[allow(unused)] file| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;

                    let Some(meta) = file.metadata().ok() else {
                        return false;
                    };

                    meta.nlink() <= 1
                }

                #[cfg(not(unix))]
                {
                    false
                }
            })
    }
}

impl PackageStoreEntry {
    pub fn new(package: impl Into<PackageRef>, checksum: Option<Checksum>) -> Self {
        Self {
            package: package.into(),
            checksum,
        }
    }

    pub fn package(&self) -> &PackageRef {
        &self.package
    }

    pub fn checksum(&self) -> Option<&Checksum> {
        self.checksum.as_ref()
    }

    fn path(&self) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(self.prefix());
        path.push(self.package.id().as_str());

        let version_str = match &self.checksum {
            Some(checksum) => format!(
                "{}+{}_{}",
                self.package.version(),
                checksum.algorithm(),
                checksum.without_algorithm()
            ),
            None => self.package.version().to_string(),
        };

        path.push(version_str);

        path
    }

    fn prefix(&self) -> String {
        self.package
            .id()
            .as_str()
            .chars()
            .take(2)
            .collect::<String>()
            .to_lowercase()
    }

    fn try_from_path(path: impl AsRef<Path>) -> Result<Self> {
        use std::path::Component;

        let path = path.as_ref();

        let mut components = path.components();
        let Some(Component::Normal(prefix)) = components.next() else {
            return Err(Error::InvalidPackageStoreEntryPath);
        };

        let prefix = prefix.to_str().ok_or(Error::NonUtf8Path)?;

        let id = match components.next() {
            Some(Component::Normal(os_str)) => {
                PackageId::new(os_str.to_str().ok_or(Error::NonUtf8Path)?)
            }
            _ => return Err(Error::InvalidPackageStoreEntryPath),
        };

        let (version, checksum) = match components.next() {
            Some(Component::Normal(os_str)) => {
                let str = os_str.to_str().ok_or(Error::NonUtf8Path)?;

                let (version, checksum) = match str.split_once('+') {
                    Some((version, checksum)) => {
                        let checksum: Checksum = checksum
                            .replace('_', ":")
                            .parse()
                            .map_err(Error::InvalidPackageStoreEntryChecksum)?;

                        (version, Some(checksum))
                    }
                    None => (str, None),
                };

                let version: Version = version
                    .parse()
                    .map_err(Error::InvalidPackageStoreEntryVersion)?;

                (version, checksum)
            }
            _ => return Err(Error::InvalidPackageStoreEntryPath),
        };

        if components.next().is_some() {
            return Err(Error::InvalidPackageStoreEntryPath);
        }

        let package = PackageRef::new(id, version);

        let this = Self { package, checksum };

        if this.prefix() != prefix {
            return Err(Error::InvalidPackageStoreEntryPath);
        }

        Ok(this)
    }
}

impl Display for PackageStoreEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(checksum) = &self.checksum {
            write!(f, "{}+{}", self.package, checksum)
        } else {
            write!(f, "{}", self.package)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // blake3 hash of "Hello, world!"
    const BLAKE3_HELLO_WORLD: &str =
        "ede5c0b10f2ec4979c69b52f61e42ff5b413519ce09be0f14d098dcfe5f6f98d";

    #[test]
    fn entry_path() {
        let entry =
            PackageStoreEntry::new(PackageRef::new("Author-Name", Version::new(0, 1, 0)), None);
        assert_eq!(entry.path(), PathBuf::from("au/Author-Name/0.1.0"));

        let entry = PackageStoreEntry::new(PackageRef::new("A", Version::new(0, 1, 0)), None);
        assert_eq!(entry.path(), PathBuf::from("a/A/0.1.0"));

        let hash = blake3::hash(b"Hello, world!");
        let entry = PackageStoreEntry::new(
            PackageRef::new("Author-Name", Version::new(0, 1, 0)),
            Some(hash.into()),
        );
        assert_eq!(
            entry.path(),
            PathBuf::from(format!("au/Author-Name/0.1.0+blake3_{BLAKE3_HELLO_WORLD}"))
        );
    }

    #[test]
    fn entry_from_path() {
        let entry = PackageStoreEntry::try_from_path("au/Author-Name/0.1.0").unwrap();
        assert_eq!(
            entry,
            PackageStoreEntry {
                package: PackageRef::new("Author-Name", Version::new(0, 1, 0)),
                checksum: None
            }
        );

        let entry = PackageStoreEntry::try_from_path(format!(
            "au/Author-Name/0.1.0+blake3_{BLAKE3_HELLO_WORLD}"
        ))
        .unwrap();

        let hash = blake3::hash(b"Hello, world!");
        assert_eq!(
            entry,
            PackageStoreEntry {
                package: PackageRef::new("Author-Name", Version::new(0, 1, 0)),
                checksum: Some(hash.into())
            }
        );
    }

    #[test]
    fn entry_from_path_invalid() {
        assert!(PackageStoreEntry::try_from_path("").is_err());
        assert!(PackageStoreEntry::try_from_path("au").is_err());
        assert!(PackageStoreEntry::try_from_path("au/Author-Name").is_err());
        assert!(PackageStoreEntry::try_from_path("au/Author-Name/0.1.0/extra").is_err());
        assert!(PackageStoreEntry::try_from_path("/au/Author-Name/0.1.0").is_err());
        assert!(PackageStoreEntry::try_from_path("C:/au/Author-Name/0.1.0").is_err());
        assert!(PackageStoreEntry::try_from_path("au/Author-Name/0.1.0+invalid_checksum").is_err());
        assert!(PackageStoreEntry::try_from_path("au/Author-Name/invalid_version").is_err());
        assert!(PackageStoreEntry::try_from_path("other-prefix/Author-Name/0.1.0").is_err());
    }

    #[test]
    fn entry_path_roundtrip() {
        let hash = blake3::hash(b"Hello, world!");

        let entry = PackageStoreEntry::new(
            PackageRef::new("Author-Name", Version::new(0, 1, 0)),
            Some(hash.into()),
        );
        let entry2 = PackageStoreEntry::try_from_path(entry.path()).unwrap();

        assert_eq!(entry, entry2);
    }
}
