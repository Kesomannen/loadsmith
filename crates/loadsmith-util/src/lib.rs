//! Internal utility functions (hashing, file helpers) for the loadsmith
//! mod-manager library.
//!
//! This is an internal crate of the [`loadsmith`] workspace. Most consumers
//! should depend on the `loadsmith` facade crate instead of using this
//! crate directly.

use std::path::{Path, PathBuf};

use tracing::{trace, warn};

/// Remove empty ancestor directories starting from the parent of the given
/// path, stopping at the first non-empty or non-removable directory.
///
/// Walks upward from the given path, removing each directory that is empty
/// (or that has already been removed by a prior iteration). Stops when it
/// encounters a directory that is not empty, not found, or permission-denied.
///
/// ```no_run
/// use std::fs;
/// use loadsmith_util::remove_empty_parents;
///
/// let dir = std::env::temp_dir().join("a").join("b").join("c");
/// fs::create_dir_all(&dir).unwrap();
/// remove_empty_parents(dir.join("file.txt")).unwrap();
/// ```
pub fn remove_empty_parents(path: impl Into<PathBuf>) -> std::io::Result<()> {
    use std::io::ErrorKind;

    let mut path = path.into();

    while path.pop() {
        match std::fs::remove_dir(&path) {
            Ok(_) => {
                trace!(path = %path.display(), "removed empty directory");
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::DirectoryNotEmpty | ErrorKind::NotFound
                ) =>
            {
                break;
            }
            Err(err) if err.kind() == ErrorKind::PermissionDenied => {
                warn!(path = %path.display(), "permission denied while removing empty directories");
                break;
            }

            Err(err) => return Err(err.into()),
        }
    }

    Ok(())
}

/// Create all parent directories for a file path.
///
/// Equivalent to `std::fs::create_dir_all` on the parent component of the
/// path. Does nothing if the path has no parent (e.g. a root or relative
/// single-component path).
///
/// ```no_run
/// use loadsmith_util::create_parent_dirs;
///
/// create_parent_dirs("/tmp/mods/my-mod/config.toml").unwrap();
/// // /tmp/mods/my-mod/ now exists
/// ```
pub fn create_parent_dirs(path: impl AsRef<Path>) -> std::io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Recursively copy a directory tree from `src` to `dst`.
///
/// Creates the destination directory and all subdirectories as needed, then
/// copies each file. Uses [`walkdir::WalkDir`] to traverse `src`.
///
/// ```no_run
/// use loadsmith_util::copy_dir;
///
/// copy_dir("/path/to/source", "/path/to/dest").unwrap();
/// ```
pub fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let relative_path = entry
            .path()
            .strip_prefix(src)
            .expect("child should be descendant of parent path");

        let target_path = dst.join(relative_path);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target_path)?;
        } else {
            std::fs::copy(entry.path(), &target_path)?;
        }
    }

    Ok(())
}

/// Build a [`VersionReq`](semver::VersionReq) that matches exactly one
/// version.
///
/// Creates a requirement equivalent to `"=x.y.z"` in semver notation. The
/// returned requirement will match only the exact major.minor.patch
/// combination.
///
/// ```
/// use loadsmith_util::exact_version_eq;
///
/// let ver = semver::Version::new(1, 2, 3);
/// let req = exact_version_eq(&ver);
///
/// assert!(req.matches(&ver));
/// assert!(!req.matches(&semver::Version::new(1, 2, 4)));
/// assert!(!req.matches(&semver::Version::new(2, 0, 0)));
/// ```
pub fn exact_version_eq(version: &semver::Version) -> semver::VersionReq {
    semver::VersionReq {
        comparators: vec![semver::Comparator {
            op: semver::Op::Exact,
            major: version.major,
            minor: Some(version.minor),
            patch: Some(version.patch),
            pre: semver::Prerelease::EMPTY,
        }],
    }
}
