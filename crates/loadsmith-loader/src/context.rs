use std::{borrow::Cow, fmt::Debug, fs, path::Path};

use camino::{Utf8Path, Utf8PathBuf};
use globset::GlobSet;
use loadsmith_core::{Checksum, ChecksumAlgorithm};
use tracing::trace;
use walkdir::WalkDir;

use crate::Result;

/// Information about the game and profile directories used when launching a
/// loader.
///
/// A `LaunchContext` holds the paths to the profile directory (where mod files
/// are staged) and the game directory (where the game executable lives),
/// together with a flag indicating whether the game runs under Proton.
///
/// # Examples
///
/// ```rust
/// use loadsmith_loader::LaunchContext;
/// use camino::Utf8Path;
///
/// let ctx = LaunchContext::new(
///     Utf8Path::new("/tmp/profile"),
///     Utf8Path::new("/tmp/game"),
///     false,
/// );
///
/// assert_eq!(ctx.profile_path(), Utf8Path::new("/tmp/profile"));
/// assert_eq!(ctx.game_path(), Utf8Path::new("/tmp/game"));
/// assert!(!ctx.is_proton());
/// ```
#[derive(Debug, Clone)]
pub struct LaunchContext<'a> {
    profile_path: Cow<'a, Utf8Path>,
    game_path: Cow<'a, Utf8Path>,
    is_proton: bool,
}

impl<'a> LaunchContext<'a> {
    /// Creates a new launch context.
    ///
    /// * `profile_path` - directory where mod and loader files are staged.
    /// * `game_path` - directory containing the game executable.
    /// * `is_proton` - whether the game runs via Proton (used for path
    ///   translation).
    pub fn new(
        profile_path: impl Into<Cow<'a, Utf8Path>>,
        game_path: impl Into<Cow<'a, Utf8Path>>,
        is_proton: bool,
    ) -> Self {
        Self {
            profile_path: profile_path.into(),
            game_path: game_path.into(),
            is_proton,
        }
    }

    /// Returns the profile directory path.
    pub fn profile_path(&self) -> &Utf8Path {
        &self.profile_path
    }

    /// Returns the game directory path.
    pub fn game_path(&self) -> &Utf8Path {
        &self.game_path
    }

    /// Returns `true` when the game runs under Proton.
    pub fn is_proton(&self) -> bool {
        self.is_proton
    }

    #[cfg(test)]
    pub(crate) fn in_tempdir(tempdir: &tempfile::TempDir, is_proton: bool) -> Result<Self> {
        use std::fs;

        let path = Utf8Path::from_path(tempdir.path()).expect("temp path should be UTF-8");

        let profile = path.join("profile");
        let game = path.join("game");

        fs::create_dir_all(&profile)?;
        fs::create_dir_all(&game)?;

        Ok(Self::new(profile, game, is_proton))
    }

    /// Formats a path for use with Proton, prepending `Z:` when
    /// [`is_proton`](LaunchContext::is_proton) is `true`.
    pub fn format_proton_path(&self, path: impl Into<Utf8PathBuf>) -> Utf8PathBuf {
        if self.is_proton {
            let path = format!("Z:{}", path.into());
            Utf8PathBuf::from(path)
        } else {
            path.into()
        }
    }

    /// Copies a single file from the profile to the game directory.
    ///
    /// Skips the copy when the destination already exists and has the same
    /// content (determined by hash).
    pub fn copy_file_to_game(
        &self,
        relative_source: impl AsRef<Path>,
        relative_target: impl AsRef<Path>,
    ) -> Result<()> {
        let src = relative_source.as_ref();
        let target = relative_target.as_ref();

        let profile_path = self.profile_path.as_std_path().join(src);
        let game_path = self.game_path.as_std_path().join(target);

        if game_path.is_file() {
            if Checksum::compute_from_path(&profile_path, ChecksumAlgorithm::Blake3)?
                == Checksum::compute_from_path(&game_path, ChecksumAlgorithm::Blake3)?
            {
                trace!(
                    src = %src.display(),
                    target = %target.display(),
                    "skipping copy, file already exists and is identical"
                );

                return Ok(());
            }
        }

        trace!(
            src = %src.display(),
            target = %target.display(),
            "copy file to game directory"
        );

        loadsmith_util::create_parent_dirs(&game_path)?;

        fs::copy(profile_path, game_path)?;

        Ok(())
    }

    /// Copies all files in the profile matching a glob set to the game
    /// directory, preserving relative paths.
    pub fn copy_glob_to_game(&self, patterns: &GlobSet) -> Result<()> {
        self.copy_files_to_game(|path| patterns.is_match(path))
    }

    /// Copies all files in the profile for which `filter` returns `true` to
    /// the game directory, preserving relative paths.
    pub fn copy_files_to_game<F>(&self, filter: F) -> Result<()>
    where
        F: Fn(&Path) -> bool,
    {
        WalkDir::new(&*self.profile_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| {
                entry
                    .into_path()
                    .strip_prefix(&*self.profile_path)
                    .expect("profile path should be a prefix of the file path")
                    .to_path_buf()
            })
            .filter(|relative_path| filter(relative_path))
            .try_for_each(|relative_path| self.copy_file_to_game(&relative_path, &relative_path))
    }
}
