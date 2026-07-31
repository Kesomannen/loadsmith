use std::{fs, path::Path};

use loadsmith_core::InstalledPackage;
use tracing::{debug, trace};

use crate::Result;

/// Remove all files that belong to an installed package from a profile directory.
///
/// Removes each file recorded in `package.files()` from the `profile` directory
/// and cleans up any empty parent directories along the way.
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
pub fn uninstall(package: &InstalledPackage, profile: impl AsRef<Path>) -> Result<()> {
    let profile = profile.as_ref();

    for file in package.files() {
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
