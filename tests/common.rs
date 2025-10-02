use std::path::PathBuf;

use loadsmith::AnyZipArchive;

pub fn open_zip(name: &str) -> AnyZipArchive {
    let path: PathBuf = format!("{}/tests/data/{name}.zip", env!("CARGO_MANIFEST_DIR")).into();
    loadsmith::open_zip(path).expect("error opening test zip archive")
}
