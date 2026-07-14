use std::{
    fmt::Debug,
    fs::{self},
    path::Path,
};

use camino::Utf8PathBuf;
use loadsmith_core::{InstalledFile, InstalledPackage, PackageRef};
use tracing::{debug, trace};
use walkdir::WalkDir;

use crate::{
    InstallRule, InstallRuleset,
    error::{Error, Result},
};

pub fn install(
    package: PackageRef,
    ruleset: InstallRuleset,
    source: impl AsRef<Path>,
    profile: impl AsRef<Path>,
    no_links: bool,
) -> Result<(InstalledPackage, Vec<Utf8PathBuf>)> {
    let source = source.as_ref();
    let profile = profile.as_ref();

    let mut installed_files = Vec::new();
    let mut overriden_files = Vec::new();

    let walkdir = WalkDir::new(source).follow_links(false).into_iter();

    for entry in walkdir {
        let entry = entry?;

        if entry.file_type().is_dir() {
            continue; // we create the necessary dirs when creating files instead
        }

        let relative_path = entry
            .path()
            .strip_prefix(source)
            .expect("entry path should be relative to source");

        let relative_path = Utf8PathBuf::try_from(relative_path.to_path_buf())?;

        let Some((mapped, rule)) = ruleset.map_file_and_return_rule(&relative_path, &package)
        else {
            debug!(%relative_path, "files left unmapped by ruleset, skipping");
            continue;
        };

        let target_path = profile.join(mapped);

        loadsmith_util::create_parent_dirs(&target_path)?;

        let (installed, overwrote) = install_file(
            entry.path(),
            &target_path,
            relative_path.clone(),
            rule,
            no_links,
        )?;

        if let Some(installed) = installed {
            installed_files.push(installed);
        }

        if overwrote {
            overriden_files.push(relative_path);
        }
    }

    Ok((
        InstalledPackage::now(package, installed_files),
        overriden_files,
    ))
}

fn install_file(
    source: &Path,
    target: &Path,
    relative_path: Utf8PathBuf,
    rule: &InstallRule,
    no_links: bool,
) -> Result<(Option<InstalledFile>, bool)> {
    let link = !no_links && rule.use_links();
    let mut overwrote = false;

    if target.exists() {
        match rule.conflict_strategy() {
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

pub fn uninstall(package: &InstalledPackage, profile: impl AsRef<Path>) -> Result<()> {
    let profile = profile.as_ref();

    for file in &package.files {
        let target_path = profile.join(&file.relative_path);

        if target_path.exists() {
            fs::remove_file(&target_path)?;
            trace!(?file, "remove file");

            loadsmith_util::remove_empty_parents(target_path)?;
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
