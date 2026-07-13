use std::{borrow::Cow, fmt::Debug, fs, path::Path};

use camino::{Utf8Path, Utf8PathBuf};
use globset::GlobSet;
use loadsmith_util::hash_file;
use tracing::trace;
use walkdir::WalkDir;

use crate::Result;

#[derive(Debug, Clone)]
pub struct LaunchContext<'a> {
    profile_path: Cow<'a, Utf8Path>,
    game_path: Cow<'a, Utf8Path>,
    is_proton: bool,
}

impl<'a> LaunchContext<'a> {
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

    pub fn profile_path(&self) -> &Utf8Path {
        &self.profile_path
    }

    pub fn game_path(&self) -> &Utf8Path {
        &self.game_path
    }

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

    pub fn format_proton_path(&self, path: impl Into<Utf8PathBuf>) -> Utf8PathBuf {
        if self.is_proton {
            let path = format!("Z:{}", path.into());
            Utf8PathBuf::from(path)
        } else {
            path.into()
        }
    }

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
            if hash_file(&profile_path)? == hash_file(&game_path)? {
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

        if let Some(parent) = game_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(profile_path, game_path)?;

        Ok(())
    }

    pub fn copy_glob_to_game(&self, patterns: &GlobSet) -> Result<()> {
        self.copy_files_to_game(|path| patterns.is_match(path))
    }

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
