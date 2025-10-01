use std::{
    borrow::Cow,
    collections::HashMap,
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use itertools::Itertools;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::{
    PackageInstaller, Result,
    state::{ProfileState, ProfileStateHandle},
};

type StaticCow<T> = Cow<'static, T>;

#[derive(Debug, Clone)]
pub struct RuleInstaller {
    rules: Vec<Rule>,
    default_rule: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: StaticCow<str>,
    pub target: StaticCow<Path>,
    pub mode: RuleMode,
    pub mutable: bool,
    pub extensions: Vec<StaticCow<str>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RuleMode {
    Separate,
    #[default]
    SeparateFlatten,
    Track,
    None,
}

impl Rule {
    pub fn new(
        name: impl Into<StaticCow<str>>,
        target: impl Into<StaticCow<Path>>,
        mode: RuleMode,
    ) -> Self {
        Self {
            name: name.into(),
            target: target.into(),
            mode,
            mutable: false,
            extensions: Vec::new(),
        }
    }

    pub fn with_extensions<T: Into<StaticCow<str>>>(
        mut self,
        extensions: impl IntoIterator<Item = T>,
    ) -> Self {
        self.extensions = extensions.into_iter().map(|item| item.into()).collect();
        self
    }

    pub fn separated(name: impl Into<StaticCow<str>>, target: impl Into<StaticCow<Path>>) -> Self {
        Self::new(name, target, RuleMode::Separate)
    }

    pub fn flat_separated(
        name: impl Into<StaticCow<str>>,
        target: impl Into<StaticCow<Path>>,
    ) -> Self {
        Self::new(name, target, RuleMode::SeparateFlatten)
    }

    pub fn tracked(name: impl Into<StaticCow<str>>, target: impl Into<StaticCow<Path>>) -> Self {
        Self::new(name, target, RuleMode::Track)
    }

    pub fn untracked(name: impl Into<StaticCow<str>>, target: impl Into<StaticCow<Path>>) -> Self {
        Self::new(name, target, RuleMode::None)
    }

    pub fn mutable(mut self) -> Self {
        self.mutable = true;
        self
    }
}

enum RulePackageFiles {
    None,
    Track(<HashMap<PathBuf, String> as IntoIter>::IntoIter),
    WalkDir(walkdir::IntoIter),
}

impl<'a> Iterator for RulePackageFiles {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RulePackageFiles::WalkDir(walkdir) => walkdir.find_map(|result| match result {
                Ok(entry) if entry.file_type().is_file() => Some(entry.into_path()),
                _ => None,
            }),
            RulePackageFiles::Track => None,
            RulePackageFiles::None => None,
        }
    }
}

impl Rule {
    fn package_files<'a>(
        &'a self,
        profile_root: &Path,
        package_name: &str,
    ) -> Result<RulePackageFiles> {
        let iter = match self.mode {
            RuleMode::Separate | RuleMode::SeparateFlatten => {
                let directory = profile_root.join(&*self.target).join(package_name);
                RulePackageFiles::WalkDir(WalkDir::new(directory).into_iter())
            }
            RuleMode::Track => {
                let file_map = ProfileStateHandle::new(profile_root);
                let iter = file_map.state.file_map.into_iter();

                RulePackageFiles::Track(iter)
            }
            RuleMode::None => RulePackageFiles::None,
        };

        Ok(iter)
    }
}

impl RuleInstaller {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self {
            rules,
            default_rule: None,
        }
    }

    pub fn with_default(mut self, index: usize) -> Self {
        self.default_rule = Some(index);
        self
    }

    fn match_component(&'_ self, component: &str) -> Option<&'_ Rule> {
        self.rules.iter().find(|rule| {
            if crate::util::cmp_ignore_case(component, &rule.name).is_eq() {
                return true;
            }

            let Some((_, extension)) = component.split_once('.') else {
                return false;
            };

            rule.extensions.iter().any(|rule_ext| rule_ext == extension)
        })
    }

    pub fn rule_from_installed_file(&'_ self, relative_path: impl AsRef<Path>) -> &'_ Rule {
        self.rules
            .iter()
            .find(|rule| relative_path.as_ref().starts_with(&*rule.target))
            .expect("file should be in a rule")
    }

    /// Map a file in the mod archive (at relative_path) to the target path relative to the profile's root.
    /// Ok(None) means a file should be ignored
    pub(crate) fn map_file<'p>(
        &self,
        relative_path: &'p Path,
        package_name: &str,
    ) -> Option<Cow<'p, Path>> {
        use std::path::Component;

        // find a subdir in the file path, ex.
        // MyFolder/plugins/MyMod.dll
        //          ^-----^
        // `prev` holds the path up to the subdir component (or the whole path if it doesn't contain one)
        // `components` holds the remaining components of the path

        let mut prev = PathBuf::new();
        let mut components = relative_path.components();

        let rule = loop {
            match components.next() {
                Some(Component::Normal(name)) => {
                    prev.push(name);
                    if let Some(name) = name.to_str() {
                        if let Some(subdir) = self.match_component(name) {
                            break subdir; // found a subdir
                        }
                    }
                }
                // remove the previous parent
                Some(Component::ParentDir) => {
                    prev.pop();
                }
                // we don't care/don't expect any of these
                Some(Component::RootDir | Component::Prefix(_) | Component::CurDir) => continue,
                // default when the whole path is exhausted
                None => match self.default_rule {
                    Some(index) => break &self.rules[index],
                    None => return None,
                },
            }
        };

        // whether to separete mod files by source mod
        let separate = matches!(rule.mode, RuleMode::Separate | RuleMode::SeparateFlatten);
        // whether to flatten the target path (i.e. placing the files directly in the subdir)
        let flatten = matches!(
            rule.mode,
            RuleMode::SeparateFlatten | RuleMode::Track | RuleMode::None
        );
        // this means the path didn't contain any subdirs
        let defaulted = components.clone().next().is_none();

        // ex. BepInEx/plugins
        let mut target = rule.target.to_path_buf();

        if separate {
            // ex. BepInEx/plugins/Author-ModName
            target.push(package_name);
        }

        if defaulted {
            if flatten {
                // place the file directly into the default directory
                let file_name = prev.file_name()?;

                // ex. icon.png -> BepInEx/plugins/icon.png
                target.push(file_name);
            } else {
                // place the file along with its parent directories into the default directory
                // ex. MyIcons/icon.png -> BepInEx/plugins/MyIcons/icon.png
                target.push(&prev);
            }
        } else {
            if !flatten {
                let mut prev = prev.components();
                // remove the subdir component itself
                prev.next_back();

                target.push(prev);
            }

            // ex. relative_path: MyFolder/plugins/MyOtherFolder/Plugin.dll
            //    (with flatten): BepInEx/plugins/MyOtherFolder/Plugin.dll
            // (without flatten): BepInEx/plugins/MyFolder/MyOtherFolder/Plugin.dll
            target.push(components);
        }

        Some(Cow::Owned(target))
    }
}

impl PackageInstaller for RuleInstaller {
    fn extract<R: Read + Seek>(
        &self,
        archive: ZipArchive<R>,
        package_name: &str,
        output_path: &Path,
    ) -> Result<()> {
        crate::util::extract(archive, output_path, move |relative_path| {
            self.map_file(relative_path, package_name.as_ref())
        })
    }

    fn package_files<'a>(
        &'a self,
        install_root: &'a Path,
        package_name: &'a str,
    ) -> Result<impl Iterator<Item = Result<PathBuf>> + 'a> {
        let iter = self
            .rules
            .iter()
            .map(|rule| rule.package_files(install_root, package_name))
            .flatten_ok();

        Ok(iter)
    }

    fn is_mutable(&self, relative_path: impl AsRef<Path>) -> bool {
        self.rule_from_installed_file(relative_path).mutable
    }

    fn should_overwrite(&self, relative_path: impl AsRef<Path>) -> bool {
        self.rule_from_installed_file(relative_path).mode == RuleMode::Track
    }
}
