use std::{
    borrow::Cow,
    fmt::Debug,
    path::{Path, PathBuf},
};

use super::PackageInstaller;

use crate::Result;

/// A [`PackageInstaller`] that follows standard zip extraction logic.
///
/// Files are simply placed into the target directory according to their relative path inside of the
/// zip archive.
///
/// Only files that match any of the glob patterns given to [`new()`](ExtractInstaller::new) get
/// extracted and included in the mod installation.
#[derive(Debug, Clone)]
pub struct ExtractInstaller {
    include_patterns: Vec<&'static str>,
    flatten_top_level: bool,
}

impl ExtractInstaller {
    /// Creates a new [`ExtractInstaller`] that will manage files which match **any** of
    /// the specified glob patterns.
    ///
    /// Patterns are checked using the [glob_match](https://crates.io/crates/glob-match) crate.
    ///
    /// # Important!
    ///
    /// The patterns will also be used to resolve [`PackageInstaller::package_files`],
    /// meaning files installed by other mods that happen to match any of the patterns will be
    /// incorrectly bundled with the previously installed mod (bad!). That, along with its lack
    /// of separation, is why this installer should only be used for mod loaders, where we only
    /// expect one package per profile to be installed with this installer.
    pub fn new(include_patterns: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            include_patterns: include_patterns.into_iter().collect(),
            flatten_top_level: false,
        }
    }

    /// Whether to flatten top level directories and ignore top level files in mod archives.
    ///
    /// This is applied *before* matching file patterns, so these two are equivalent
    /// (unless other top level directories than `TopLevel` exist):
    ///
    /// ```rust
    /// let a = ExtractInstaller::new(["TopLevel/Other/*"]).flatten_top_level(false);
    /// let b = ExtractInstaller::new(["Other/*"]).flatten_top_level(true);
    /// ```
    ///
    /// Defaults to `false`.
    pub fn flatten_top_level(mut self, value: bool) -> Self {
        self.flatten_top_level = value;
        self
    }
}

impl PackageInstaller for ExtractInstaller {
    fn extract(
        &self,
        archive: crate::AnyZipArchive,
        _package_name: &str,
        output_path: &Path,
    ) -> Result<()> {
        crate::util::extract(archive, output_path, |relative_path| {
            let mut components = relative_path.components();

            if self.flatten_top_level {
                components.next();
            }

            let path = components.as_path();
            let str = path.to_string_lossy();

            self.include_patterns
                .iter()
                .any(|pattern| glob_match::glob_match(pattern, &*str))
                .then_some(Cow::Borrowed(path))
        })
    }

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        _package_name: &'a str,
    ) -> Result<Vec<PathBuf>> {
        crate::util::match_files_in_dir(install_root, &self.include_patterns)
    }
}
