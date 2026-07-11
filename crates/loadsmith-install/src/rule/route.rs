use std::borrow::Cow;

use camino::{Utf8Path, Utf8PathBuf};
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

use crate::ConflictStrategy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRule {
    name: Cow<'static, str>,
    pub(super) target: Cow<'static, Utf8Path>,
    file_extensions: Vec<Cow<'static, str>>,
    subdir: bool,
    flatten: bool,
    mutable: bool,
}

impl RouteRule {
    pub fn new_static(path: &'static str) -> Self {
        Self::new(Cow::Borrowed(Utf8Path::new(path)))
    }

    pub fn new(path: impl Into<Cow<'static, Utf8Path>>) -> Self {
        let path = path.into();

        let name = match path {
            Cow::Borrowed(borrowed) => borrowed.file_name().map(Cow::Borrowed),
            Cow::Owned(_) => path.file_name().map(|s| Cow::Owned(s.to_string())),
        }
        .unwrap_or_default();

        Self::new_with_target(name, path)
    }

    pub fn new_with_target(
        name: impl Into<Cow<'static, str>>,
        target: impl Into<Cow<'static, Utf8Path>>,
    ) -> Self {
        Self {
            name: name.into(),
            target: target.into(),
            file_extensions: Vec::new(),
            flatten: true,
            subdir: true,
            mutable: false,
        }
    }

    pub fn with_file_extension(mut self, extension: impl Into<Cow<'static, str>>) -> Self {
        self.file_extensions.push(extension.into());
        self
    }

    pub fn with_file_extensions(mut self, extensions: Vec<Cow<'static, str>>) -> Self {
        self.file_extensions = extensions;
        self
    }

    pub fn with_flatten(mut self, flatten: bool) -> Self {
        self.flatten = flatten;
        self
    }

    pub fn with_subdir(mut self, subdir: bool) -> Self {
        self.subdir = subdir;
        self
    }

    pub fn with_mutable(mut self, mutable: bool) -> Self {
        self.mutable = mutable;
        self
    }

    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        self.matches_extension(path) || self.matches_path(path)
    }

    pub fn matches_extension(&self, path: impl AsRef<Utf8Path>) -> bool {
        // check the whole extension, that is everything after the first dot in the file name
        path.as_ref()
            .file_name()
            .and_then(|name| name.split_once('.'))
            .map(|(_, ext)| self.file_extensions.iter().any(|e| e == ext))
            .unwrap_or(false)
    }

    pub fn matches_path(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.split_path(path.as_ref()).is_some()
    }

    fn split_path(&self, path: &Utf8Path) -> Option<(Utf8PathBuf, Utf8PathBuf)> {
        // eat components until we find the route name
        let Some(route_name_index) = path
            .components()
            .position(|comp| comp.as_str().eq_ignore_ascii_case(self.name.as_ref()))
        else {
            // no match
            return None;
        };

        let prefix = path.components().take(route_name_index).collect();
        let suffix = path.components().skip(route_name_index + 1).collect();

        Some((prefix, suffix))
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        let path = path.as_ref();
        let (prefix, suffix) = self.split_path(path).unwrap_or_else(|| {
            let mut components = path.components();
            let file_name = components
                .next_back()
                .map(|file_name| file_name.as_str())
                .unwrap_or_default();
            let prefix = components.collect();

            (prefix, Utf8PathBuf::from(file_name))
        });

        let mut target_path = Utf8PathBuf::from(self.target.as_ref());

        if self.subdir {
            target_path.push(package.id.as_str());
        }

        if !self.flatten {
            target_path.push(prefix);
        }

        target_path.push(suffix);

        Some(target_path)
    }

    pub fn use_links(&self) -> bool {
        !self.mutable
    }

    pub fn conflict_strategy(&self) -> ConflictStrategy {
        ConflictStrategy::Skip
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::PackageId;

    use super::*;

    #[test]
    fn new() {
        let rule = RouteRule::new(Utf8Path::new("BepInEx/plugins"));

        assert_eq!(rule.name, "plugins");
        assert_eq!(rule.target, Utf8Path::new("BepInEx/plugins"));

        let rule = RouteRule::new(Utf8Path::new("MelonLoader"));

        assert_eq!(rule.name, "MelonLoader");
        assert_eq!(rule.target, Utf8Path::new("MelonLoader"));

        let rule = RouteRule::new(Utf8Path::new(""));

        assert_eq!(rule.name, "");
        assert_eq!(rule.target, Utf8Path::new(""));
    }

    #[test]
    fn defaults() {
        let rule = RouteRule::new_static("BepInEx/plugins");

        assert!(rule.flatten);
        assert!(rule.subdir);
        assert!(!rule.mutable);
    }

    #[test]
    fn matches_path() {
        let rule = RouteRule::new_static("BepInEx/plugins");

        assert!(rule.matches("BepInEx/plugins/myplugin.dll"));
        assert!(rule.matches("plugins/myplugin.dll"));
        assert!(rule.matches("Plugins/myplugin.dll"));
        assert!(rule.matches("Nested/BepInEx/plugins/myplugin.dll"));
        assert!(!rule.matches("BepInEx/core/myplugin.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn matches_extension() {
        let rule = RouteRule::new_static("BepInEx/monomod").with_file_extension("mm.dll");

        assert!(rule.matches("BepInEx/monomod/myplugin.mm.dll"));
        assert!(rule.matches("BepInEx/monomod/myplugin.dll"));
        assert!(rule.matches("myplugin.mm.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn match_extension_with_dot() {
        let rule1 = RouteRule::new_static("plugins").with_file_extension("dll");

        assert!(rule1.matches("myplugin.dll"));
        assert!(!rule1.matches("myplugin.mm.dll"));
        assert!(!rule1.matches("myplugin.dll.mm"));
        assert!(!rule1.matches("myplugin.dll.dll"));

        let rule2 = RouteRule::new_static("monomod").with_file_extension("mm.dll");

        assert!(!rule2.matches("myplugin.dll"));
        assert!(rule2.matches("myplugin.mm.dll"));
        assert!(!rule2.matches("myplugin.mm"));
        assert!(!rule2.matches("myplugin.mm.mm.dll"));
    }

    macro_rules! assert_map {
        ($rule:expr, $file:expr, $package:expr => None) => {
            assert_eq!($rule.map_file($file, $package), None);
        };
        ($rule:expr, $file:expr, $package:expr => $expected:expr) => {
            assert_eq!(
                $rule.map_file($file, $package),
                Some(Utf8PathBuf::from($expected))
            );
        };
    }

    #[test]
    fn map_file() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_file_extension("plugin")
            .with_subdir(true)
            .with_flatten(true);

        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

        // defaults correctly when name is not present in the path
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // removes the matched "plugins" component
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // removes the matched "pluguins" component and flattens the "BepInEx" component
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // case-insensitive match of "plugins"
        assert_map!(rule, "BepInEx/Plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // flattens the "Nested" component
        assert_map!(rule, "Nested/plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // flattens the "Nested" component
        assert_map!(rule, "Nested/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // retains nesting after the matched "plugins" component but flattens before
        assert_map!(rule, "Before/plugins/After/MyPlugin.plugin", &package => "BepInEx/plugins/Author-Name/After/MyPlugin.plugin");
    }

    #[test]
    fn map_file_flatten() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_flatten(true)
            .with_subdir(true)
            .with_file_extension("plugin");

        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

        // Flattened routing keeps only the file name and package folder.
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // When the route name is already present, it is removed from the output path.
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // A fully qualified route path still collapses down to the target folder.
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/MyPlugin.dll");

        // Extra nesting after the route name is preserved even when flattening.
        assert_map!(rule, "plugins/Nested/MyPlugin.dll", &package => "BepInEx/plugins/Author-Name/Nested/MyPlugin.dll");

        // File extension matching does not change the mapped directory layout.
        assert_map!(rule, "plugins/Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Author-Name/Nested/MyPlugin.plugin");

        // Paths without the route name still map relative to the file name.
        assert_map!(rule, "Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Author-Name/MyPlugin.plugin");
    }

    #[test]
    fn map_file_no_subdir_flatten() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_subdir(false)
            .with_flatten(true)
            .with_file_extension("plugin");

        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

        // Without subdir support, the package id is never inserted.
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // The route prefix is still stripped when it appears in the input path.
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Flattening keeps fully qualified paths at the target root.
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Nested content stays nested when flattening does not remove it.
        assert_map!(rule, "plugins/Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Nested/MyPlugin.plugin");
    }

    #[test]
    fn map_file_no_subdir_no_flatten() {
        let rule = RouteRule::new_static("BepInEx/plugins")
            .with_subdir(false)
            .with_flatten(false)
            .with_file_extension("plugin");

        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

        // No package folder is inserted when subdir is disabled.
        assert_map!(rule, "MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Nested routes are preserved even when the route name doesn't appear.
        assert_map!(rule, "other/MyPlugin.dll", &package => "BepInEx/plugins/other/MyPlugin.dll");

        // A matching route component is removed before the remainder is appended.
        assert_map!(rule, "plugins/MyPlugin.dll", &package => "BepInEx/plugins/MyPlugin.dll");

        // Without flattening, the unmatched prefix is preserved in the output path.
        assert_map!(rule, "BepInEx/plugins/MyPlugin.dll", &package => "BepInEx/plugins/BepInEx/MyPlugin.dll");

        // Nested content remains nested when the route name is stripped.
        assert_map!(rule, "plugins/Nested/MyPlugin.plugin", &package => "BepInEx/plugins/Nested/MyPlugin.plugin");
    }

    #[test]
    fn map_file_empty() {
        let rule = RouteRule::new_static("BepInEx/plugins").with_flatten(false);

        // An empty path falls back to the package directory when flattening is off.
        assert_map!(
            rule,
            "",
            &PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0)) => "BepInEx/plugins/Author-Name"
        );
    }
}
