use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use loadsmith_core::{Checksum, InstalledPackage, PackageId, PackageRef};
use loadsmith_install::InstallRuleset;
use serde::{Deserialize, Serialize};

use crate::{
    Diff, Diffable, Error, LockedPackage, Lockfile, PackageStore, PackageStoreEntry, Result,
};

/// Tracks the installation state of packages for a single profile directory.
///
/// A profile state records which packages are installed, their checksums, and
/// the files they own. It also supports installing new packages (from disk or
/// from a [`PackageStore`](crate::PackageStore)) and uninstalling existing ones.
///
/// # Examples
///
/// ```rust
/// use std::collections::BTreeMap;
/// use loadsmith_manifest::{ProfileState, ProfileStateData};
///
/// let data = ProfileStateData::new(BTreeMap::new());
/// let state = ProfileState::new("/tmp/my-profile", data);
///
/// assert_eq!(state.path().to_string_lossy(), "/tmp/my-profile");
/// assert!(state.packages().is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct ProfileState {
    path: PathBuf,
    data: ProfileStateData,
}

/// The serializable portion of [`ProfileState`].
///
/// This holds the mapping of package IDs to installed packages and can be
/// serialised/deserialised independently of the profile path.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProfileStateData {
    packages: BTreeMap<PackageId, InstalledPackage>,
}

impl Diffable for InstalledPackage {
    /// Delegates to [`InstalledPackage::checksum`].
    fn checksum(&self) -> Option<&Checksum> {
        self.checksum()
    }

    /// Delegates to the version of the package reference.
    fn version(&self) -> &loadsmith_core::Version {
        self.ref_().version()
    }
}

impl ProfileState {
    /// Create a new profile state at the given path with the given data.
    pub fn new(path: impl Into<PathBuf>, data: ProfileStateData) -> Self {
        Self {
            path: path.into(),
            data,
        }
    }

    /// Borrow the profile directory path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Borrow the inner [`ProfileStateData`].
    pub fn data(&self) -> &ProfileStateData {
        &self.data
    }

    /// Borrow the map of installed packages.
    pub fn packages(&self) -> &BTreeMap<PackageId, InstalledPackage> {
        &self.data.packages
    }

    fn packages_mut(&mut self) -> &mut BTreeMap<PackageId, InstalledPackage> {
        &mut self.data.packages
    }

    fn id_to_package_map(&self) -> HashMap<&PackageId, &InstalledPackage> {
        self.packages().iter().collect()
    }

    /// Compute the difference between this profile state and another.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::BTreeMap;
    /// use loadsmith_manifest::{ProfileState, ProfileStateData};
    ///
    /// let a = ProfileState::new("/tmp/a", ProfileStateData::new(BTreeMap::new()));
    /// let b = ProfileState::new("/tmp/b", ProfileStateData::new(BTreeMap::new()));
    ///
    /// assert!(a.diff(&b).is_empty());
    /// ```
    pub fn diff<'a>(
        &'a self,
        other: &'a ProfileState,
    ) -> Diff<'a, InstalledPackage, InstalledPackage> {
        Diff::compute(self.id_to_package_map(), other.id_to_package_map())
    }

    /// Compute the difference between this profile state and a lockfile.
    ///
    /// This is the primary way to determine what needs to be installed,
    /// updated, or removed to bring a profile in line with a resolved lockfile.
    pub fn diff_lockfile<'a>(
        &'a self,
        lockfile: &'a Lockfile,
    ) -> Diff<'a, InstalledPackage, LockedPackage> {
        Diff::compute(self.id_to_package_map(), lockfile.id_to_package_map())
    }

    fn check_already_installed(&self, package: &PackageRef) -> Result<()> {
        if self.packages().contains_key(package.id()) {
            Err(Error::PackageAlreadyInstalled)
        } else {
            Ok(())
        }
    }

    /// Install a package into this profile from a local source path.
    ///
    /// The `source` may point to a directory (plain copy), a `.zip` file
    /// (extracted), or any other file (copied as-is).
    ///
    /// Returns an error if the package is already installed.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use std::collections::BTreeMap;
    /// use loadsmith_core::{PackageRef, Version};
    /// use loadsmith_install::InstallRuleset;
    /// use loadsmith_manifest::{ProfileState, ProfileStateData};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut state = ProfileState::new("/tmp/my-profile", ProfileStateData::new(BTreeMap::new()));
    /// let pkg = PackageRef::new("Author-Mod", Version::new(1, 0, 0));
    /// let ruleset = InstallRuleset::new(&[]);
    /// state.install(pkg, ruleset, "/tmp/package-source", false, None)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn install(
        &mut self,
        package: PackageRef,
        ruleset: InstallRuleset,
        source: impl AsRef<Path>,
        no_links: bool,
        checksum: Option<Checksum>,
    ) -> Result<()> {
        self.check_already_installed(&package)?;

        let (install, overwritten_files) =
            loadsmith_install::install(package, ruleset, source, &self.path, no_links, checksum)?;
        self.add(install, overwritten_files);
        Ok(())
    }

    /// Install a package from the central [`PackageStore`](crate::PackageStore)
    /// into this profile.
    ///
    /// Returns an error if the package is already installed.
    pub fn install_from_store(
        &mut self,
        entry: PackageStoreEntry,
        ruleset: InstallRuleset,
        store: &PackageStore,
    ) -> Result<()> {
        self.check_already_installed(entry.package())?;

        let (install, overwritten_files) = store.install(entry, ruleset, &self.path)?;
        self.add(install, overwritten_files);
        Ok(())
    }

    /// Uninstall a package from this profile by its [`PackageId`].
    ///
    /// Returns an error if the package is not installed. On I/O failure the
    /// package is re-inserted into the state so it is not silently lost.
    pub fn uninstall(&mut self, package: &PackageId) -> Result<()> {
        let install = self
            .packages_mut()
            .remove(package)
            .ok_or(Error::PackageNotInstalled)?;

        if let Err(err) = loadsmith_install::uninstall(&install, &self.path) {
            self.packages_mut().insert(package.clone(), install);

            Err(err.into())
        } else {
            Ok(())
        }
    }

    fn add(&mut self, install: InstalledPackage, overwritten_files: Vec<Utf8PathBuf>) {
        for other in self.packages_mut().values_mut() {
            other
                .files_mut()
                .retain(|file| !overwritten_files.contains(file.relative_path()));
        }

        self.packages_mut()
            .insert(install.ref_().id().clone(), install);
    }
}

impl ProfileStateData {
    /// Create profile state data from a map of installed packages.
    ///
    /// Typically you would call [`ProfileStateData::default`] for an empty
    /// state and then use [`ProfileState::install`] to populate it.
    pub fn new(packages: BTreeMap<PackageId, InstalledPackage>) -> Self {
        Self { packages }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_lockfile_empty() {
        let state = ProfileState::new("/tmp/test", ProfileStateData::default());
        let lockfile = Lockfile::default();

        let diff = state.diff_lockfile(&lockfile);

        assert!(diff.is_empty());
    }
}
