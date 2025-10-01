use std::{
    borrow::Cow,
    cmp::Ordering,
    fs::{self, File},
    io::{self, Read, Seek},
    path::Path,
};

use itertools::Itertools;
use zip::ZipArchive;

use crate::{Result, wrap_io_err};

pub fn cmp_ignore_case(a: impl AsRef<str>, b: impl AsRef<str>) -> Ordering {
    a.as_ref()
        .chars()
        .flat_map(char::to_lowercase)
        .zip_longest(b.as_ref().chars().flat_map(char::to_lowercase))
        .map(|ab| match ab {
            itertools::EitherOrBoth::Left(_) => Ordering::Greater,
            itertools::EitherOrBoth::Right(_) => Ordering::Less,
            itertools::EitherOrBoth::Both(a, b) => a.cmp(&b),
        })
        .find(|&ordering| ordering != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}

/// Extract a package archive to `output_path`, mapping files using `map_file`.
///
/// `map_file` is called with each file's relative path. It should return
/// the (relative) output path where the file should be copied to. `Ok(None)`
/// skips the file entirely.
///
/// Directories are created as needed.
pub(super) fn extract<S, F>(
    mut archive: ZipArchive<S>,
    output_path: impl AsRef<Path>,
    mut map_file: F,
) -> Result<()>
where
    S: Read + Seek,
    F: FnMut(&Path) -> Option<Cow<Path>>,
{
    for i in 0..archive.len() {
        let mut source_file = archive.by_index(i)?;

        if source_file.is_dir() {
            continue; // we create the necessary dirs when copying files instead
        }

        let Some(path) = source_file.enclosed_name() else {
            // log::warn!(
            //     "file at {} escapes archive root, skipping",
            //     source_file.name()
            // );
            continue;
        };

        let Some(relative_target) = map_file(&path) else {
            continue;
        };

        let target_path = output_path.as_ref().join(relative_target);
        let parent = target_path.parent().unwrap();

        fs::create_dir_all(parent).map_err(|err| wrap_io_err(err, &target_path))?;

        let mut target_file =
            File::create(&target_path).map_err(|err| wrap_io_err(err, &target_path))?;

        io::copy(&mut source_file, &mut target_file)
            .map_err(|err| wrap_io_err(err, &target_path))?;

        #[cfg(unix)]
        set_unix_mode(&source_file, &target_path)?;
    }

    Ok(())
}

#[cfg(unix)]
fn set_unix_mode(file: &zip::read::ZipFile, target_path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut mode = match file.unix_mode() {
        Some(mode) => mode,
        None => fs::metadata(target_path)?.permissions().mode(),
    };

    if target_path.extension().is_some_and(|ext| ext == "sh") {
        // set all scripts to be executable by everyone
        mode |= 0o111;
    }

    fs::set_permissions(target_path, PermissionsExt::from_mode(mode));

    Ok(())
}

pub fn install_package_file(
    profile_root: &Path,
    source_root: &Path,
    file_relative: &Path,
    overwrite: bool,
    use_links: bool,
) -> io::Result<()> {
    let source_path = source_root.join(file_relative);
    let target_path = profile_root.join(file_relative);

    if !overwrite && target_path.exists() {
        return Ok(());
    }

    fs::create_dir_all(
        target_path
            .parent()
            .expect("new_path should have parent directory"),
    )?;

    if use_links {
        fs::hard_link(source_path, target_path)?;
    } else {
        fs::copy(source_path, target_path)?;
    }

    Ok(())
}
