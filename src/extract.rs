use std::{
    borrow::Cow,
    fmt::Debug,
    path::{Path, PathBuf},
};

use super::PackageInstaller;

use crate::Result;

#[derive(Debug, Clone)]
pub struct ExtractInstaller {
    include_patterns: Vec<&'static str>,
    flatten_top_level: bool,
}

impl ExtractInstaller {
    pub fn new(include_patterns: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            include_patterns: include_patterns.into_iter().collect(),
            flatten_top_level: false,
        }
    }

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
