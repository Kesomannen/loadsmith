use std::{
    fmt::Debug,
    fs::{self},
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use loadsmith_core::{InstalledFile, InstalledPackage, PackageRef};
use tracing::{debug, trace, warn};
use walkdir::WalkDir;

use crate::{
    InstallRule, InstallRuleset,
    error::{Error, Result},
};

pub fn install(
    package: PackageRef,
    ruleset: InstallRuleset,
    source: impl AsRef<Path> + Debug,
    profile: impl AsRef<Path> + Debug,
    no_links: bool,
) -> Result<(InstalledPackage, Vec<Utf8PathBuf>)> {
    let source = source.as_ref();
    let profile = profile.as_ref();

    let mut files = Vec::new();
    let mut overwrote_files = Vec::new();
    let walkdir = WalkDir::new(source).follow_links(false).into_iter();

    for entry in walkdir {
        let entry = entry?;

        let relative_path = entry
            .path()
            .strip_prefix(source)
            .expect("entry path should be relative to source");

        let relative_path = Utf8PathBuf::try_from(relative_path.to_path_buf())?;

        let target_path = profile.join(&relative_path);

        if entry.file_type().is_dir() {
            if target_path.is_dir() {
                trace!(?relative_path, "directory already exists, skipping");
            } else {
                fs::create_dir(&target_path)?;
                trace!(?relative_path, "create directory")
            }
        } else {
            let rule = ruleset.find_rule_for_mapped_path(&relative_path);
            let (file, overwrote) = install_file(
                entry.path(),
                target_path,
                relative_path.clone(),
                rule,
                no_links,
            )?;

            if let Some(file) = file {
                files.push(file);
            }

            if overwrote {
                overwrote_files.push(relative_path);
            }
        }
    }

    Ok((InstalledPackage::now(package, files), overwrote_files))
}

fn install_file(
    source: impl AsRef<Path>,
    target: impl AsRef<Path> + Into<PathBuf>,
    relative_path: Utf8PathBuf,
    rule: Option<&InstallRule>,
    no_links: bool,
) -> Result<(Option<InstalledFile>, bool)> {
    let target = target.as_ref();

    let link = !no_links && rule.map(|r| r.use_links()).unwrap_or(false);
    let mut overwrote = false;

    if target.exists() {
        match rule.map(|r| r.conflict_strategy()).unwrap_or_else(|| {
            debug!(
                ?relative_path,
                "no matching rule, defaulting to overwrite strategy"
            );
            ConflictStrategy::Overwrite
        }) {
            ConflictStrategy::Overwrite => {
                trace!(?relative_path, "overwriting existing file");
                overwrote = true;
                // fs::copy already overwrites the file, no need to remove it first
                if link {
                    fs::remove_file(&target)?;
                }
            }
            ConflictStrategy::Skip => {
                trace!(?relative_path, "skipping existing file");
                return Ok((None, false));
            }
            ConflictStrategy::Error => {
                return Err(Error::FileAlreadyExists(target.into()));
            }
        }
    }

    if link {
        fs::hard_link(source, &target)?;
        trace!(?relative_path, "link file");
    } else {
        fs::copy(source, &target)?;
        trace!(?relative_path, "copy file");
    }

    Ok((Some(InstalledFile::new(relative_path, link)), overwrote))
}

pub fn uninstall(package: InstalledPackage, profile: impl AsRef<Path> + Debug) -> Result<()> {
    let profile = profile.as_ref();

    for file in &package.files {
        let target_path = profile.join(&file.relative_path);

        if target_path.exists() {
            fs::remove_file(&target_path)?;
            trace!(?file, "remove file");

            remove_empty_parents(target_path)?;
        } else {
            debug!(?file, "file does not exist, skipping");
        }
    }

    Ok(())
}

fn remove_empty_parents(mut path: PathBuf) -> Result<()> {
    while path.pop() {
        match fs::remove_dir(&path) {
            Ok(_) => {
                trace!(path = %path.display(), "removed empty directory");
            }
            Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => {
                break;
            }
            Err(err) => return Err(err.into()),
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
