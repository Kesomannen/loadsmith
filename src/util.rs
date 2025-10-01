use std::{
    borrow::Cow,
    cmp::Ordering,
    fs::{self, File},
    io::{self, Read, Seek},
    path::Path,
};

use itertools::Itertools;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::{Error, Result};

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
pub(super) fn extract<R, F>(
    mut archive: ZipArchive<R>,
    output_path: impl AsRef<Path>,
    mut map_file: F,
) -> Result<()>
where
    R: Read + Seek,
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

        fs::create_dir_all(parent).map_err(|err| Error::wrap_io(err, &target_path))?;

        let mut target_file =
            File::create(&target_path).map_err(|err| Error::wrap_io(err, &target_path))?;

        io::copy(&mut source_file, &mut target_file)
            .map_err(|err| Error::wrap_io(err, &target_path))?;

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

pub enum InstallOpt<'a> {
    Const(bool),
    Fn(&'a mut dyn FnMut(&Path) -> bool),
}

impl InstallOpt<'_> {
    fn eval(&mut self, path: &Path) -> bool {
        match self {
            InstallOpt::Const(value) => *value,
            InstallOpt::Fn(f) => f(path),
        }
    }
}

pub struct InstallOptions<'a> {
    link: InstallOpt<'a>,
    overwrite: InstallOpt<'a>,
    on_write: Option<&'a mut dyn FnMut(&Path)>,
}

impl Default for InstallOptions<'_> {
    fn default() -> Self {
        Self {
            link: InstallOpt::Const(false),
            overwrite: InstallOpt::Const(true),
            on_write: None,
        }
    }
}

impl<'a> InstallOptions<'a> {
    pub fn should_link(mut self, f: InstallOpt<'a>) -> Self {
        self.link = f;
        self
    }

    pub fn should_overwrite(mut self, g: InstallOpt<'a>) -> Self {
        self.overwrite = g;
        self
    }

    pub fn on_write(mut self, f: &'a mut dyn FnMut(&Path)) -> Self {
        self.on_write = Some(f);
        self
    }
}

pub fn install<'a>(
    profile_root: &Path,
    source_root: &Path,
    mut options: InstallOptions<'a>,
) -> io::Result<()> {
    let entries = WalkDir::new(source_root)
        .into_iter()
        .filter_ok(|entry| entry.file_type().is_file());

    for entry in entries {
        let entry = entry?;

        let relative_path = entry
            .path()
            .strip_prefix(source_root)
            .expect("source file should be child of the source root");

        install_package_file(profile_root, source_root, &relative_path, &mut options)?;
    }

    Ok(())
}

pub fn install_package_file<'a>(
    profile_root: &Path,
    source_root: &Path,
    relative_path: &Path,
    options: &mut InstallOptions<'a>,
) -> io::Result<()> {
    let source_path = source_root.join(relative_path);
    let target_path = profile_root.join(relative_path);

    if target_path.exists() && !options.overwrite.eval(relative_path) {
        return Ok(());
    }

    fs::create_dir_all(
        target_path
            .parent()
            .expect("new_path should have parent directory"),
    )?;

    if options.link.eval(relative_path) {
        fs::hard_link(source_path, target_path)?;
    } else {
        fs::copy(source_path, target_path)?;
    }

    if let Some(on_write) = options.on_write.as_mut() {
        on_write(relative_path);
    }

    Ok(())
}
