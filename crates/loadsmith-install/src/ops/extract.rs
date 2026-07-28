use std::{
    fs::{self, File},
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};

use tracing::{trace, warn};

use crate::{
    error::Result,
    zip::{Zip, ZipFile},
};

pub fn extract<R: Read + Seek>(reader: R, target: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut zip = zip::ZipArchive::new(reader)?;
    extract_zip(&mut zip, target.as_ref())
}

fn extract_zip<Z: Zip>(zip: &mut Z, target: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let t = crate::zip::private::Token;

    let target = target.as_ref();
    let mut files = Vec::new();

    for i in 0..zip.len(t) {
        let mut source_file = zip.by_index(i, t)?;

        if source_file.is_dir(t) {
            continue; // we create the necessary dirs when creating files instead
        }

        let relative_path = source_file.path(t)?;

        let target_path = target.join(&relative_path);

        if target_path.exists() {
            warn!(%relative_path, "file already exists, skipping extraction");
            continue;
        }

        trace!(%relative_path, "extract file");

        loadsmith_util::create_parent_dirs(&target_path)?;

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
    use crate::zip::mock::MockZip;

    use super::*;

    #[test]
    fn simple_glob() {
        let included_files = &["file1", "nested/file2", "nested/../file3", "./././file4"];

        let mut zip = MockZip::default();

        for file in included_files {
            zip = zip.with_empty_file(file);
        }

        let dir = tempfile::tempdir().unwrap();

        extract_zip(&mut zip, &dir).unwrap();

        for file in included_files {
            assert!(dir.path().join(file).exists());
        }
    }
}
