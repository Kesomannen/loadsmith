use std::{
    fmt::Debug,
    fs::{self},
    path::Path,
};

use camino::{Utf8Path, Utf8PathBuf};
use loadsmith_core::{InstalledPackage, PackageRef};
use tracing::{debug, trace};
use walkdir::WalkDir;

use crate::error::{Error, Result};

#[tracing::instrument]
pub fn install(
    package: PackageRef,
    source: &Path,
    profile: &Path,
    mut options: InstallOptions,
) -> Result<InstalledPackage> {
    let mut files = Vec::new();
    let walkdir = WalkDir::new(source).follow_links(false).into_iter();

    for entry in walkdir {
        let entry = entry?;

        let relative_path = entry
            .path()
            .strip_prefix(source)
            .expect("entry path should be relative to source");

        let target_path = Utf8PathBuf::try_from(profile.join(relative_path))?;

        if entry.file_type().is_dir() {
            fs::create_dir(&target_path)?;
            trace!(?relative_path, "create directory")
        } else {
            let link = options.should_link(&target_path);

            if target_path.exists() {
                match options.conflict_strategy(&target_path) {
                    ConflictStrategy::Overwrite => {
                        trace!(?relative_path, "overwriting existing file");
                        // fs::copy already overwrites the file, no need to remove it first
                        if link {
                            fs::remove_file(&target_path)?;
                        }
                    }
                    ConflictStrategy::Skip => {
                        trace!(?relative_path, "skipping existing file");
                        continue;
                    }
                    ConflictStrategy::Error => {
                        return Err(Error::FileAlreadyExists(target_path.into_std_path_buf()));
                    }
                }
            }

            if link {
                fs::hard_link(entry.path(), &target_path)?;
                trace!(?relative_path, "link file");
            } else {
                fs::copy(entry.path(), &target_path)?;
                trace!(?relative_path, "copy file");
            }

            files.push(target_path);
        }
    }

    Ok(InstalledPackage::now(package, files))
}

#[tracing::instrument]
pub fn uninstall(package: InstalledPackage, profile: &Path) -> Result<()> {
    for file in &package.files {
        let target_path = profile.join(file);

        if target_path.exists() {
            fs::remove_file(&target_path)?;
            trace!(?file, "remove file");
        } else {
            debug!(?file, "file does not exist, skipping");
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    Overwrite,
    Skip,
    Error,
}

#[derive(Debug, Default)]
pub struct InstallOptions<'a, 'b> {
    link: InstallOption<'a, bool>,
    conflict: InstallOption<'b, ConflictStrategy>,
}

pub enum InstallOption<'a, T> {
    Default,
    Overwrite(T),
    Dynamic(&'a mut dyn FnMut(&Utf8Path) -> T),
}

impl<'a, 'b> InstallOptions<'a, 'b> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn link(mut self, strategy: impl Into<InstallOption<'a, bool>>) -> Self {
        self.link = strategy.into();
        self
    }

    pub fn on_conflict(mut self, strategy: impl Into<InstallOption<'b, ConflictStrategy>>) -> Self {
        self.conflict = strategy.into();
        self
    }

    fn should_link(&mut self, path: impl AsRef<Utf8Path>) -> bool {
        self.link.eval(path.as_ref()).unwrap_or(false)
    }

    fn conflict_strategy(&mut self, path: impl AsRef<Utf8Path>) -> ConflictStrategy {
        self.conflict
            .eval(path.as_ref())
            .unwrap_or(ConflictStrategy::Error)
    }
}

impl<'a, T, F> From<&'a mut F> for InstallOption<'a, T>
where
    F: FnMut(&Utf8Path) -> T,
{
    fn from(value: &'a mut F) -> Self {
        InstallOption::Dynamic(value)
    }
}

impl<'a, T: Clone> InstallOption<'a, T> {
    fn eval(&mut self, path: &Utf8Path) -> Option<T> {
        match self {
            InstallOption::Overwrite(value) => Some(value.clone()),
            InstallOption::Dynamic(f) => Some(f(path)),
            InstallOption::Default => None,
        }
    }
}

impl<T> Default for InstallOption<'_, T> {
    fn default() -> Self {
        InstallOption::Default
    }
}

impl<T: Debug> Debug for InstallOption<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallOption::Default => f.debug_tuple("InstallStrategy").field(&"Default").finish(),
            InstallOption::Overwrite(value) => {
                f.debug_tuple("InstallStrategy").field(value).finish()
            }
            InstallOption::Dynamic(_) => {
                f.debug_tuple("InstallStrategy").field(&"Dynamic").finish()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_options_default() {
        let mut options = InstallOptions::default();
        assert_eq!(options.should_link("test"), false);
        assert_eq!(options.conflict_strategy("test"), ConflictStrategy::Error);
    }

    #[test]
    fn install_options_constant() {
        let mut options = InstallOptions::new()
            .link(InstallOption::Overwrite(true))
            .on_conflict(InstallOption::Overwrite(ConflictStrategy::Skip));

        assert_eq!(options.should_link("test"), true);
        assert_eq!(options.conflict_strategy("test"), ConflictStrategy::Skip);
    }

    #[test]
    fn install_options_dynamic() {
        let mut link_strategy = |path: &Utf8Path| path.file_name().unwrap() == "link.txt";
        let mut conflict_strategy = |path: &Utf8Path| {
            if path.file_name().unwrap() == "skip.txt" {
                ConflictStrategy::Skip
            } else {
                ConflictStrategy::Error
            }
        };

        let mut options = InstallOptions::new()
            .link(&mut link_strategy)
            .on_conflict(&mut conflict_strategy);

        assert_eq!(options.should_link("link.txt"), true);
        assert_eq!(options.should_link("file.txt"), false);
        assert_eq!(
            options.conflict_strategy("skip.txt"),
            ConflictStrategy::Skip
        );
        assert_eq!(
            options.conflict_strategy("file.txt"),
            ConflictStrategy::Error
        );
    }
}
