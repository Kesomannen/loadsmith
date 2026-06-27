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
    pub fn new(
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

    fn matches_extension(&self, path: impl AsRef<Utf8Path>) -> bool {
        // check the whole extension, that is everything after the first dot in the file name
        path.as_ref()
            .file_name()
            .and_then(|name| name.split_once('.'))
            .map(|(_, ext)| self.file_extensions.iter().any(|e| e == ext))
            .unwrap_or(false)
    }

    fn split_path<'a>(&self, path: &'a Utf8Path) -> Option<(Utf8PathBuf, Utf8PathBuf)> {
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

    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.matches_extension(path.as_ref()) || self.split_path(path.as_ref()).is_some()
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        let path = path.as_ref();
        let (prefix, suffix) = self.split_path(path).map(Some).unwrap_or_else(|| {
            let mut components = path.components();
            let file_name = components.next_back()?;
            let prefix = components.collect();

            Some((prefix, Utf8PathBuf::from(file_name.as_str())))
        })?;

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
        if self.subdir {
            ConflictStrategy::Error
        } else {
            ConflictStrategy::Skip
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use loadsmith_core::PackageId;

    use super::*;

    #[test]
    fn matches_path() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins"));

        assert!(rule.matches("BepInEx/plugins/myplugin.dll"));
        assert!(rule.matches("plugins/myplugin.dll"));
        assert!(rule.matches("Plugins/myplugin.dll"));
        assert!(rule.matches("Nested/BepInEx/plugins/myplugin.dll"));
        assert!(!rule.matches("BepInEx/core/myplugin.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn matches_extension() {
        let rule = RouteRule::new("monomod", Utf8Path::new("BepInEx/monomod"))
            .with_file_extensions(vec![Cow::Borrowed("mm.dll")]);

        assert!(rule.matches("BepInEx/monomod/myplugin.mm.dll"));
        assert!(rule.matches("BepInEx/monomod/myplugin.dll"));
        assert!(rule.matches("myplugin.mm.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn map_file() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins"))
            .with_flatten(false)
            .with_file_extensions(vec![Cow::Borrowed("plugin")]);

        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

        assert_eq!(
            rule.map_file("MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("plugins/MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("BepInEx/plugins/MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/BepInEx/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("BepInEx/Plugins/MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/BepInEx/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("Nested/plugins/MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/Nested/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("Nested/MyPlugin.plugin", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/Nested/MyPlugin.plugin"
            ))
        );
    }

    #[test]
    fn map_file_flatten() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins"))
            .with_flatten(true)
            .with_file_extensions(vec![Cow::Borrowed("plugin")]);

        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

        assert_eq!(
            rule.map_file("MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("plugins/MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("BepInEx/plugins/MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("plugins/Nested/MyPlugin.dll", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/Nested/MyPlugin.dll"
            ))
        );

        assert_eq!(
            rule.map_file("plugins/Nested/MyPlugin.plugin", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/Nested/MyPlugin.plugin"
            ))
        );

        assert_eq!(
            rule.map_file("Nested/MyPlugin.plugin", &package),
            Some(Utf8PathBuf::from(
                "BepInEx/plugins/Author-Name/MyPlugin.plugin"
            ))
        );
    }

    #[test]
    fn map_file_empty() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins")).with_flatten(false);

        assert_eq!(
            rule.map_file(
                "",
                &PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0))
            ),
            None
        );
    }
}
