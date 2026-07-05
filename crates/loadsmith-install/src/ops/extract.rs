use std::{
    fs::{self, File},
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};

use loadsmith_core::PackageRef;
use tracing::{trace, warn};

#[cfg(unix)]
use crate::zip::ZipFile;
use crate::{error::Result, rule::InstallRuleset, zip::Zip};

pub fn extract<R: Read + Seek>(
    reader: R,
    package: &PackageRef,
    ruleset: InstallRuleset,
    target: impl AsRef<Path>,
) -> Result<Vec<PathBuf>> {
    let mut zip = zip::ZipArchive::new(reader)?;
    extract_zip(&mut zip, package, ruleset, target.as_ref())
}

pub fn extract_zip<Z: Zip>(
    zip: &mut Z,
    package: &PackageRef,
    ruleset: InstallRuleset,
    target: &Path,
) -> Result<Vec<PathBuf>> {
    let t = crate::zip::private::Token;

    let mut files = Vec::new();

    for i in 0..zip.len(t) {
        let mut source_file = zip.by_index(i, t)?;

        if source_file.is_dir(t) {
            continue; // we create the necessary dirs when creating files instead
        }

        let source_path = source_file.path(t)?;

        let Some(mapped_path) = ruleset.map_file(&source_path, package) else {
            trace!(%source_path, "no matching rule");
            continue;
        };

        let target_path = target.join(&mapped_path);

        if target_path.exists() {
            warn!(%source_path, %mapped_path, "file already exists, skipping extraction");
            continue;
        }

        trace!(%source_path, %mapped_path, "extract file");

        fs::create_dir_all(
            target_path
                .parent()
                .expect("path should have target as parent"),
        )?;

        let mut target_file = File::create(&target_path)?;

        io::copy(&mut source_file, &mut target_file)?;

        #[cfg(unix)]
        set_unix_mode(&source_file, &target_path)?;

        files.push(target_path);
    }

    Ok(files)
}

#[cfg(unix)]
fn set_unix_mode<F: ZipFile>(file: &F, path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = file.unix_mode(crate::zip::private::Token) {
        fs::set_permissions(path, PermissionsExt::from_mode(mode))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use camino::Utf8Path;

    use crate::{GlobRule, InstallRule, zip::mock::MockZip};

    use super::*;

    const TEST_PACKAGE_ID: &str = "Author-Name";

    fn test_extract(zip: &mut MockZip, ruleset: InstallRuleset) -> tempfile::TempDir {
        let package = loadsmith_core::PackageRef::new(TEST_PACKAGE_ID.to_string(), (1, 0, 0));
        let tempfile = tempfile::tempdir().unwrap();

        extract_zip(zip, &package, ruleset, tempfile.path()).unwrap();

        tempfile
    }

    #[test]
    fn simple_glob() {
        let mut zip = MockZip::default()
            .with_empty_file("file1")
            .with_empty_file("file2");

        let rules = [InstallRule::Glob(
            GlobRule::try_from_pattern("file1", Utf8Path::new(".")).unwrap(),
        )];

        let ruleset = InstallRuleset::new(&rules, None);

        let dir = test_extract(&mut zip, ruleset);

        assert!(dir.path().join("file1").exists());
        assert!(!dir.path().join("file2").exists());
    }
}
