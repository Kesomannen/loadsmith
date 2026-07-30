use std::{fs, path::Path};

use loadsmith_core::InstalledPackage;
use tracing::{debug, trace};

use crate::Result;

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
