//! Contains the [`RuleInstaller`], a highly customizable [`PackageInstaller`].
//!
//! This closely mimics the logic used in r2modman's `BepInEx` and `MelonLoader` extraction implementation (described
//! [in this wiki article](https://github.com/ebkr/r2modmanPlus/wiki/Structuring-your-Thunderstore-package)),
//! with some extra flexibility to allow it to be reused in a wider range of mod loaders.
//!
//! Most [`ModLoader`](crate::ModLoader) implementors return an instance of this type when you call
//! [`package_installer()`](crate::ModLoader::package_installer).
//!
//! ## Rules
//!
//! The installer operates on a number of [`Rule`]s, which map certain file paths to certain directories
//! within the output directory. Each [`Rule`] has a `name` and `target`. When a file path is found to contain a
//! segment matching a rule's name, it is placed into the rule's target directory.
//!
//! Depending on the [`RuleMode`], files are placed differently:
//!
//! * `RuleMode::Separate`: files are placed inside a subdirectory matching the source package's name.
//! This keeps files from different mods separated and avoids collisions. To list the installed files belonging
//! to a package, the `Rule` simply walks the subdirectory with the package's name. Any path segments in the original
//! file's path leading up to the rule name are kept as nested directories.
//! * `RuleMode::SeparateFlatten`: similar to the prior mode, but ignores path segments before the rule name.
//! * `RuleMode::Track`: does not use subdirectories to separate mods. Instead, this mode writes a [state file]()
//! that keeps track of which mod owns each file. File collisions can happen, but are easier to resolve easier due to
//! the file tracking from the state file.
//! * `RuleMode::None`: like the previous rule, but with state disabled. In turn, this means any files written
//! here will not be tracked after installation and thus **won't be uninstalled**! File collisions are also impossible to
//! handle in a graceful way.
//!
//! ### State file
//!
//! The [`Track`](RuleMode::Track) rule mode writes to a json file within the profile directory keeping track of
//! the installed files. The file looks something like this:
//!
//! ```json
//! {
//!   "fileMap": {
//!     "MelonLoader/Managed/file6.managed.dll": "modname",
//!     "MelonLoader/Libs/file11.txt": "modname",
//!     "MelonLoader/Managed/file5.txt": "modname",
//!     "MelonLoader/file10.txt": "modname",
//!     "Mods/file1.dll": "modname",
//!     "Mods/file2.txt": "modname",
//!   }
//! }
//! ```
//!
//! By default, the file is created at `_state/profile.json` (mimicking r2modman's convention), but it can be changed with
//! [`RuleInstaller::with_state_file_path`]. Note that the file (and any parent directories) are only created once necessary.
//! Some mod loaders using the `RuleInstaller`, like BepInEx, may not have any rules marked as `Track`, and thus the state
//! file won't be created
//!
//! ### Mutability
//!
//! When [`RuleInstaller::install`] is called with `use_links = true`, the installer will choose which files to copy and
//! which to link depending on its rules. By default, the `Track` and `None` rule modes are assumed to be mutable and thus
//! copied for each profile, while the other two modes are assumed immutable and linked. This behaviour can be overriden
//! on a per-rule basis with [`Rule::with_mutability`].
//!
//! ### Extensions
//!
//! In addition to a name, rules can have a number of file extensions that direct files to them.
//!
//! The file extension is checked *after* no matches were found while checking its parent directories, meaning rule names
//! take precedence over extensions.
//!
//! Also, unlike the file extension methods in [`std::fs`], loadsmith defines a file's extension as everything following
//! the first dot (or `None` if it doesn't contain one). This means you can match on file extensions like `lib.dll` instead of
//! only `dll`.
//!
//! ### Default rules
//!
//! By default, files whose paths or extension don't match any rule will not be extracted or installed at all. However, the
//! [`RuleInstaller`] can optionally receive a default rule index, where unmatched files will end up.
//!
//! ## Examples
//!
//! This table summarizes the traits of each `RuleMode` where the input path is "dir1/`RULE_TARGET`/dir2/file.txt":
//!
//! |                   | Output path                                     | Mutable by default? | Is tracked? | Avoids collisions? |
//! |-------------------|-------------------------------------------------|---------------------|-------------|--------------------|
//! | `Separate`        | `RULE_TARGET`/`PACKAGE_NAME`/dir1/dir2/file.txt |                     | X           | X                  |
//! | `SeparateFlatten` | `RULE_TARGET`/`PACKAGE_NAME`/dir2/file.txt      |                     | X           | X                  |
//! | `Track`           | `RULE_TARGET`/dir2/file.txt                     | X                   | X           | *                  |
//! | `None`            | `RULE_TARGET`/dir2/file.txt                     | X                   |             |                    |

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

/// A highly configurable [`PackageInstaller`] implementation based on [`Rule`]s.
///
/// Read more in the [module level documentation](crate::rule).
#[derive(Debug, Clone)]
pub struct RuleInstaller {
    rules: Vec<Rule>,
    default_rule: Option<usize>,
    state_file_path: PathBuf,
}

/// A rule used to configure the [`RuleInstaller`].
///
/// Read more in the [module level documentation](crate::rule).
#[derive(Debug, Clone)]
pub struct Rule {
    name: StaticCow<str>,
    target: StaticCow<Path>,
    mode: RuleMode,
    extensions: Vec<StaticCow<str>>,
    override_mutability: Option<bool>,
}

/// Determines how a [`Rule`] should place its files.
///
/// Read more in the [module level documentation](crate::rule).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RuleMode {
    Separate,
    #[default]
    SeparateFlatten,
    Track,
    None,
}

impl Rule {
    /// Creates a new [`Rule`] with the specified name, target and mode.
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
            override_mutability: None,
        }
    }

    /// Sets the extensions that, in addition to this rule's name, will match files to this rule.
    pub fn with_extensions<T: Into<StaticCow<str>>>(
        mut self,
        extensions: impl IntoIterator<Item = T>,
    ) -> Self {
        self.extensions = extensions.into_iter().map(|item| item.into()).collect();
        self
    }

    /// Creates a new [`Rule`] with the [`RuleMode::Separate`] mode.
    pub fn separated(name: impl Into<StaticCow<str>>, target: impl Into<StaticCow<Path>>) -> Self {
        Self::new(name, target, RuleMode::Separate)
    }

    /// Creates a new [`Rule`] with the [`RuleMode::SeparateFlatten`] mode.
    pub fn flat_separated(
        name: impl Into<StaticCow<str>>,
        target: impl Into<StaticCow<Path>>,
    ) -> Self {
        Self::new(name, target, RuleMode::SeparateFlatten)
    }

    /// Creates a new [`Rule`] with the [`RuleMode::Track`] mode.
    pub fn tracked(name: impl Into<StaticCow<str>>, target: impl Into<StaticCow<Path>>) -> Self {
        Self::new(name, target, RuleMode::Track)
    }

    /// Creates a new [`Rule`] with the [`RuleMode::None`] mode.
    pub fn untracked(name: impl Into<StaticCow<str>>, target: impl Into<StaticCow<Path>>) -> Self {
        Self::new(name, target, RuleMode::None)
    }

    /// Overrides the mutability of files placed in this rule.
    ///
    /// This changes how files are installed when calling [`RuleInstaller::install`] with
    /// `use_links = true`. By default, only rules with [`RuleMode::Separate`] and [`RuleMode::SeparateFlatten`]
    /// will have their files linked and others copied. However, setting this option to `false` will always use
    /// links for files that belong to this rule, and vice versa.
    pub fn with_mutability(mut self, value: bool) -> Self {
        self.override_mutability = Some(value);
        self
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
    /// Creates a new [`RuleInstaller`] with the specified rules.
    ///
    /// Read more in the [module level documentation](crate::rule).
    pub fn new(rules: Vec<Rule>) -> Self {
        Self {
            rules,
            default_rule: None,
            state_file_path: ["_state", "profile.json"].iter().collect(),
        }
    }

    /// Sets the default rule index to use for unmatched files.
    ///
    /// Read more in the [module level documentation](crate::rule).
    pub fn with_default(mut self, index: usize) -> Self {
        if index >= self.rules.len() {
            panic!("default rule index is out of bounds");
        }

        self.default_rule = Some(index);
        self
    }

    /// Sets the (relative) path to create the profile state file at.
    ///
    /// Defaults to `_state/profile.json`.
    ///
    /// Read more in the [module level documentation](crate::rule).
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

    /// Finds which rule the file at `relative_path` belongs to.
    ///
    /// This applies to already extracted or installed files, not those in the source zip archive.
    pub fn rule_from_relative_path(&'_ self, relative_path: impl AsRef<Path>) -> Option<&'_ Rule> {
        self.rules
            .iter()
            .find(|rule| relative_path.as_ref().starts_with(&*rule.target))
    }

    /// Map a file in the source archive (at `relative_path`) to the target path relative to the profile's root.
    ///
    /// A return value of `None` means a file should be ignored.
    pub fn map_file<'p>(
        &self,
        relative_path: &'p Path,
        package_name: &str,
    ) -> Option<Cow<'p, Path>> {
        use std::path::Component;

        // find a rule in the file path, ex.
        // MyFolder/plugins/MyMod.dll
        //          ^-----^
        // `prev` holds the path up to the rule component (or the whole path if it doesn't contain one)
        // `components` holds the remaining components of the path

        let mut prev = PathBuf::new();
        let mut components = relative_path.components();

        let rule = loop {
            match components.next() {
                Some(Component::Normal(name)) => {
                    prev.push(name);
                    if let Some(name) = name.to_str() {
                        if let Some(rule) = self.match_component(name) {
                            break rule; // found a rule
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
        // whether to flatten directories that come before the rule name in the file path
        let flatten = matches!(
            rule.mode,
            RuleMode::SeparateFlatten | RuleMode::Track | RuleMode::None
        );
        // this means the path didn't match any rules
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
                // remove the rule name component
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
                .should_overwrite(InstallOpt::Fn(&mut |path| {
                    let Some(rule) = self.rule_from_relative_path(path) else {
                        return false;
                    };

                    rule.mode == RuleMode::Track
                }))
                .should_link(InstallOpt::Fn(&mut |path| {
                    if !use_links {
                        return false;
                    }

                    let Some(rule) = self.rule_from_relative_path(path) else {
                        return false;
                    };

                    if let Some(mutable) = rule.override_mutability {
                        return !mutable;
                    }

                    !matches!(rule.mode, RuleMode::Track | RuleMode::None)
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
