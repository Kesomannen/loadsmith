use std::{fs, path::Path};

use loadsmith_core::InstalledFile;
use tracing::{debug, trace};

use crate::Result;

/// Remove all files that belong to an installed package from a profile directory.
///
/// Removes each file in `files` from the `profile` directory. If a file does not exist, it is skipped.
/// Whenever a file is successfully removed, its parent directories are also removed if they are empty.
///
/// # Examples
///
/// ```rust,no_run
/// use loadsmith_core::InstalledPackage;
/// use loadsmith_install::uninstall;
///
/// // Load a previously installed package, then uninstall it.
/// # let package: InstalledPackage = unimplemented!();
/// uninstall(&package, "C:\\games\\Valheim\\profile").unwrap();
/// ```
pub fn uninstall<'a, I>(files: I, profile: impl AsRef<Path>) -> Result<()>
where
    I: IntoIterator<Item = &'a InstalledFile>,
{
    let profile = profile.as_ref();

    for file in files {
        let target_path = profile.join(file.relative_path());

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
