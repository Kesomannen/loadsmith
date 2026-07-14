use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use loadsmith_core::{InstalledPackage, PackageId, PackageRef};
use loadsmith_install::InstallRuleset;
use serde::{Deserialize, Serialize};

use crate::{Diff, Diffable, Error, LockedPackage, Lockfile, Result};

#[derive(Debug, Clone)]
pub struct ProfileState {
    path: PathBuf,
    data: ProfileStateData,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProfileStateData {
    packages: BTreeMap<PackageId, InstalledPackage>,
}

impl Diffable for InstalledPackage {
    fn version(&self) -> &loadsmith_core::Version {
        self.ref_.version()
    }
}

impl ProfileState {
    pub fn new(path: impl Into<PathBuf>, data: ProfileStateData) -> Self {
        Self {
            path: path.into(),
            data,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn data(&self) -> &ProfileStateData {
        &self.data
    }

    pub fn packages(&self) -> &BTreeMap<PackageId, InstalledPackage> {
        &self.data.packages
    }

    fn packages_mut(&mut self) -> &mut BTreeMap<PackageId, InstalledPackage> {
        &mut self.data.packages
    }

    fn id_to_package_map(&self) -> HashMap<&PackageId, &InstalledPackage> {
        self.packages().iter().collect()
    }

    pub fn diff<'a>(
        &'a self,
        other: &'a ProfileState,
    ) -> Diff<'a, InstalledPackage, InstalledPackage> {
        Diff::compute(self.id_to_package_map(), other.id_to_package_map())
    }

    pub fn diff_lockfile<'a>(
        &'a self,
        lockfile: &'a Lockfile,
    ) -> Diff<'a, InstalledPackage, LockedPackage> {
        Diff::compute(self.id_to_package_map(), lockfile.id_to_package_map())
    }

    pub fn install(
        &mut self,
        package: PackageRef,
        ruleset: InstallRuleset,
        source: impl AsRef<Path>,
        no_links: bool,
    ) -> Result<()> {
        if self.packages().contains_key(package.id()) {
            return Err(Error::PackageAlreadyInstalled);
        }

        let (install, overwritten_files) =
            loadsmith_install::install(package, ruleset, source, &self.path, no_links)?;
        self.add(install, overwritten_files);
        Ok(())
    }

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
                .files
                .retain(|file| !overwritten_files.contains(&file.relative_path));
        }

        self.packages_mut()
            .insert(install.ref_.id().clone(), install);
    }
}

impl ProfileStateData {
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
