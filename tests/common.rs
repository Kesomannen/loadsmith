use std::{collections::HashSet, path::PathBuf};

use anyhow::Context;
use loadsmith::{AnyZipArchive, PackageInstaller};
use tempfile::TempDir;

pub fn open_zip(name: &str) -> AnyZipArchive {
    let path: PathBuf = format!("{}/tests/data/{name}.zip", env!("CARGO_MANIFEST_DIR")).into();
    loadsmith::open_zip(path).expect("error opening test zip archive")
}

pub fn test_mod_operations(
    installer: &dyn PackageInstaller,
    zip_path: &'static str,
    mod_name: &'static str,
    expected_files: Vec<&'static str>,
    dont_list: Vec<&'static str>,
    expected_not_present: Vec<&'static str>,
) -> anyhow::Result<()> {
    let tempdir = TempDir::new()?;

    installer
        .extract_and_install(open_zip(zip_path), mod_name, tempdir.path())
        .context("failed to extract and install package")?;

    for file in &expected_files {
        assert!(
            tempdir.path().join(file).exists(),
            "file at {file} wasn't present!",
        );
    }

    for file in &expected_not_present {
        assert!(
            !tempdir.path().join(file).exists(),
            "file at {file} shouldn't be present, but was!",
        );
    }

    let expected_listed_files = expected_files
        .into_iter()
        .filter(|file| !dont_list.contains(file))
        .map(|file| tempdir.path().join(file))
        .collect::<HashSet<_>>();

    let actual_installed_files = installer
        .package_files(tempdir.path(), mod_name)
        .context("failed to list package files")?
        .into_iter()
        .collect::<HashSet<_>>();

    assert_eq!(actual_installed_files, expected_listed_files);

    installer
        .uninstall(tempdir.path(), mod_name)
        .context("failed to uninstall package")?;

    for file in &expected_listed_files {
        assert!(
            !file.exists(),
            "file at {} wasn't removed!",
            file.strip_prefix(tempdir.path()).unwrap().display()
        );
    }

    assert!(
        installer
            .package_files(tempdir.path(), mod_name)
            .context("failed to list package files after uninstallation")?
            .is_empty(),
        "package files weren't empty after uninstallation!"
    );

    Ok(())
}
