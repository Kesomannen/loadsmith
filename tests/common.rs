use std::path::{Path, PathBuf};

use loadsmith::AnyZipArchive;
use tempfile::TempDir;

pub fn open_zip(name: &str) -> AnyZipArchive {
    let path: PathBuf = format!("{}/tests/data/{name}.zip", env!("CARGO_MANIFEST_DIR")).into();
    loadsmith::open_zip(path).expect("error opening test zip archive")
}

pub fn assert_exists(tempdir: &TempDir, path: impl AsRef<Path>) {
    assert!(
        tempdir.path().join(&path).exists(),
        "{} did not exist!",
        path.as_ref().display()
    );
}
