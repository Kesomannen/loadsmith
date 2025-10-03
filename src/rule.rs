use std::{
    borrow::Cow,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{
    AnyZipArchive, IoResultExt, PackageInstaller, Result,
    state::ProfileStateHandle,
    util::{InstallOpt, InstallOptions},
};

type StaticCow<T> = Cow<'static, T>;

#[derive(Debug, Clone)]
pub struct RuleInstaller {
    rules: Vec<Rule>,
    default_rule: Option<usize>,
    state_file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: StaticCow<str>,
    pub target: StaticCow<Path>,
    pub mode: RuleMode,
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
}

enum RulePackageFiles<'a> {
    None,
    Track {
        iter: <HashMap<PathBuf, String> as IntoIterator>::IntoIter,
        package_name: &'a str,
        profile_root: &'a Path,
    },
    WalkDir(walkdir::IntoIter),
}

impl<'a> Iterator for RulePackageFiles<'a> {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RulePackageFiles::WalkDir(walkdir) => walkdir.find_map(|result| match result {
                Ok(entry) if entry.file_type().is_file() => Some(entry.into_path()),
                _ => None,
            }),
            RulePackageFiles::Track {
                iter,
                package_name,
                profile_root,
            } => iter
                .find_map(|(path, package)| (package == *package_name).then_some(path))
                .map(|path| profile_root.join(path)),
            RulePackageFiles::None => None,
        }
    }
}

impl Rule {
    fn package_files<'a>(
        &'a self,
        profile_root: &'a Path,
        package_name: &'a str,
        state_file_path: &Path,
    ) -> Result<RulePackageFiles<'a>> {
        let iter = match self.mode {
            RuleMode::Separate | RuleMode::SeparateFlatten => {
                let directory = profile_root.join(&*self.target).join(package_name);
                RulePackageFiles::WalkDir(WalkDir::new(directory).into_iter())
            }
            RuleMode::Track => {
                let file_map = ProfileStateHandle::new(profile_root.join(state_file_path));
                let iter = file_map.state.file_map.into_iter();

                RulePackageFiles::Track {
                    iter,
                    package_name,
                    profile_root,
                }
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
            state_file_path: ["_state", "profile.json"].iter().collect(),
        }
    }

    pub fn with_default(mut self, index: usize) -> Self {
        if index >= self.rules.len() {
            panic!("default rule index is out of bounds");
        }

        self.default_rule = Some(index);
        self
    }

    pub fn with_state_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.state_file_path = path.into();
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

    pub fn rule_from_relative_path(&'_ self, relative_path: impl AsRef<Path>) -> Option<&'_ Rule> {
        self.rules
            .iter()
            .find(|rule| relative_path.as_ref().starts_with(&*rule.target))
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
    fn extract(
        &self,
        archive: AnyZipArchive,
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
    ) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();
        let mut read_tracked_files = false;

        for rule in &self.rules {
            if rule.mode == RuleMode::Track {
                // tracked files aren't distinguised by rule, which means
                // scanning one tracked rule will scan all tracked rule files
                if read_tracked_files {
                    continue;
                }

                read_tracked_files = true;
            }

            let files = rule.package_files(install_root, package_name, &self.state_file_path)?;

            result.extend(files);
        }

        Ok(result)
    }

    fn package_dir(&self, install_root: &Path, package_name: &str) -> Result<Option<PathBuf>> {
        let path = self.default_rule.map(|index| {
            install_root
                .join(&self.rules[index].target)
                .join(package_name)
        });

        Ok(path)
    }

    fn install(
        &self,
        profile_root: &Path,
        source_root: &Path,
        package_name: &str,
        use_links: bool,
    ) -> Result<()> {
        let mut profile_state: Option<ProfileStateHandle> = None;

        crate::util::install(
            profile_root,
            source_root,
            InstallOptions::default()
                .should_overwrite(InstallOpt::Const(false))
                .should_link(InstallOpt::Fn(&mut |path| {
                    if !use_links {
                        return false;
                    }

                    let Some(rule) = self.rule_from_relative_path(path) else {
                        // TODO: warning?
                        return false;
                    };

                    matches!(rule.mode, RuleMode::Track | RuleMode::None)
                }))
                .on_write(&mut |path| {
                    let Some(rule) = self.rule_from_relative_path(path) else {
                        return;
                    };

                    if rule.mode != RuleMode::Track {
                        return;
                    }

                    let handle = profile_state.get_or_insert_with(|| {
                        ProfileStateHandle::new(profile_root.join(&self.state_file_path))
                    });

                    handle
                        .state
                        .file_map
                        .insert(path.to_path_buf(), package_name.to_string());
                }),
        )?;

        if let Some(handle) = profile_state {
            handle.commit()?;
        }

        Ok(())
    }

    fn uninstall(&self, profile_root: &Path, package_name: &str) -> Result<()> {
        for file in self.package_files(profile_root, package_name)? {
            fs::remove_file(&file).wrap_err(file, "removing package file")?;
        }

        crate::util::delete_empty_dirs(profile_root)?;

        if self.rules.iter().any(|rule| rule.mode == RuleMode::Track) {
            let mut state = ProfileStateHandle::new(profile_root.join(&self.state_file_path));

            state
                .state
                .file_map
                .retain(|_, package| package != package_name);

            state.commit()?;
        }

        Ok(())
    }
}
