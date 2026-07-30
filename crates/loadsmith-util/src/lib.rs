use std::{
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use tracing::{trace, warn};

pub fn hash_reader(mut reader: impl Read) -> std::io::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut reader, &mut hasher)?;

    Ok(hasher.finalize())
}

pub fn hash_file(path: impl AsRef<Path>) -> std::io::Result<blake3::Hash> {
    let mut file = std::fs::File::open(path).map(BufReader::new)?;
    hash_reader(&mut file)
}

pub fn hash_file_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    hash_file(path).map(|hash| hash.to_hex().to_string())
}

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

pub fn create_parent_dirs(path: impl AsRef<Path>) -> std::io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

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
