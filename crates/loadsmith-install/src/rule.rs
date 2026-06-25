use std::borrow::Cow;

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobBuilder};
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

use crate::ConflictStrategy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallRule {
    Glob(GlobRule),
    Route(RouteRule),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobRule {
    pattern: Glob,
    target: Cow<'static, Utf8Path>,
    strip_top_level: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRule {
    name: Cow<'static, str>,
    route: Cow<'static, Utf8Path>,
    file_extensions: Vec<Cow<'static, str>>,
    subdir: bool,
    flatten: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallRuleset<'a> {
    pub rules: &'a [InstallRule],
    pub default_rule: Option<&'a InstallRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedInstallRuleset {
    rules: Vec<InstallRule>,
    default_rule: Option<usize>,
}

impl InstallRule {
    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        match self {
            InstallRule::Glob(glob) => glob.matches(path),
            InstallRule::Route(route) => route.matches(path),
        }
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        match self {
            InstallRule::Glob(glob) => Some(glob.map_file(path, package)),
            InstallRule::Route(route) => route.map_file(path, package),
        }
    }

    pub(crate) fn should_link(&self) -> bool {
        todo!()
    }

    pub(crate) fn conflict_strategy(&self) -> ConflictStrategy {
        todo!()
    }
}

impl From<GlobRule> for InstallRule {
    fn from(glob: GlobRule) -> Self {
        InstallRule::Glob(glob)
    }
}

impl From<RouteRule> for InstallRule {
    fn from(route: RouteRule) -> Self {
        InstallRule::Route(route)
    }
}

impl GlobRule {
    pub fn new(pattern: Glob, target: impl Into<Cow<'static, Utf8Path>>) -> Self {
        Self {
            pattern,
            target: target.into(),
            strip_top_level: false,
        }
    }

    pub fn with_strip_top_level(mut self, strip_top_level: bool) -> Self {
        self.strip_top_level = strip_top_level;
        self
    }

    pub fn try_from_pattern(
        pattern: impl AsRef<str>,
        target: impl Into<Cow<'static, Utf8Path>>,
    ) -> Result<Self, globset::Error> {
        let pattern = GlobBuilder::new(pattern.as_ref()).build()?;
        Ok(Self::new(pattern, target))
    }

    fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.pattern.compile_matcher().is_match(path.as_ref())
    }

    fn map_file(&self, path: impl AsRef<Utf8Path>, _package: &PackageRef) -> Utf8PathBuf {
        let path = path.as_ref();

        let suffix = {
            let mut components = path.components();
            if self.strip_top_level {
                components.next();
            }

            if components.clone().next().is_none() {
                path
            } else {
                components.as_path()
            }
        };

        self.target.as_ref().join(suffix)
    }
}

impl RouteRule {
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        route: impl Into<Cow<'static, Utf8Path>>,
    ) -> Self {
        Self {
            name: name.into(),
            route: route.into(),
            file_extensions: Vec::new(),
            flatten: true,
            subdir: true,
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

    fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.matches_extension(path.as_ref()) || self.split_path(path.as_ref()).is_some()
    }

    fn map_file(&self, path: impl AsRef<Utf8Path>, package: &PackageRef) -> Option<Utf8PathBuf> {
        let path = path.as_ref();
        let (prefix, suffix) = self.split_path(path).map(Some).unwrap_or_else(|| {
            let mut components = path.components();
            let file_name = components.next_back()?;
            let prefix = components.collect();

            Some((prefix, Utf8PathBuf::from(file_name.as_str())))
        })?;

        let mut target_path = Utf8PathBuf::from(self.route.as_ref());

        if self.subdir {
            target_path.push(package.id.as_str());
        }

        if !self.flatten {
            target_path.push(prefix);
        }

        target_path.push(suffix);

        Some(target_path)
    }
}

impl<'a> InstallRuleset<'a> {
    pub const fn new(rules: &'a [InstallRule], default_rule: Option<&'a InstallRule>) -> Self {
        Self {
            rules,
            default_rule,
        }
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        let path = path.as_ref();
        self.rules
            .into_iter()
            .find(|rule| rule.matches(path))
            .or(self.default_rule)
            .and_then(|rule| rule.map_file(path, package))
    }
}

impl OwnedInstallRuleset {
    pub fn with_rules(rules: Vec<InstallRule>, default_rule_index: Option<usize>) -> Option<Self> {
        if default_rule_index.is_some_and(|index| !(0..rules.len()).contains(&index)) {
            return None;
        }

        Some(Self {
            rules,
            default_rule: default_rule_index,
        })
    }

    pub fn add(&mut self, rule: InstallRule) {
        self.rules.push(rule);
    }

    pub fn rules(&self) -> &[InstallRule] {
        &self.rules
    }

    pub fn default_rule(&self) -> Option<&InstallRule> {
        self.default_rule.map(|index| &self.rules[index])
    }

    pub fn as_ref(&self) -> InstallRuleset<'_> {
        InstallRuleset::new(&self.rules, self.default_rule())
    }
}

impl Default for OwnedInstallRuleset {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            default_rule: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::PackageId;

    use super::*;

    #[test]
    fn glob_matches() {
        let rule = GlobRule::new(
            Glob::new("*.rs").unwrap(),
            Cow::Borrowed(Utf8Path::new("target")),
        );

        assert!(rule.matches("main.rs"));
        assert!(rule.matches("src/main.rs"));
        assert!(!rule.matches("main.txt"));
        assert!(!rule.matches("src/main.txt"));
    }

    #[test]
    fn glob_map_file() {
        let rule = GlobRule::try_from_pattern("*.rs", Utf8Path::new("target")).unwrap();

        let package = PackageRef::new(PackageId::new("Author-Name"), "1.0.0");

        assert_eq!(
            rule.map_file("main.rs", &package),
            Utf8PathBuf::from("target/main.rs")
        );

        assert_eq!(
            rule.map_file("src/main.rs", &package),
            Utf8PathBuf::from("target/src/main.rs")
        );

        assert_eq!(
            rule.map_file("main.txt", &package),
            Utf8PathBuf::from("target/main.txt")
        );
    }

    #[test]
    fn glob_map_file_strip_top() {
        let rule = GlobRule::try_from_pattern("in/**", Utf8Path::new("out"))
            .unwrap()
            .with_strip_top_level(true);
        let package = PackageRef::new(PackageId::new("Author-Name"), "1.0.0");

        assert_eq!(
            rule.map_file("in/file.txt", &package),
            Utf8PathBuf::from("out/file.txt")
        );

        assert_eq!(
            rule.map_file("in/nested/file.txt", &package),
            Utf8PathBuf::from("out/nested/file.txt")
        );

        assert_eq!(
            rule.map_file("in/nested/deep/file.txt", &package),
            Utf8PathBuf::from("out/nested/deep/file.txt")
        );

        assert_eq!(
            rule.map_file("other.txt", &package),
            Utf8PathBuf::from("out/other.txt")
        );
    }

    #[test]
    fn route_matches_path() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins"));

        assert!(rule.matches("BepInEx/plugins/myplugin.dll"));
        assert!(rule.matches("plugins/myplugin.dll"));
        assert!(rule.matches("Plugins/myplugin.dll"));
        assert!(rule.matches("Nested/BepInEx/plugins/myplugin.dll"));
        assert!(!rule.matches("BepInEx/core/myplugin.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn route_matches_extension() {
        let rule = RouteRule::new("monomod", Utf8Path::new("BepInEx/monomod"))
            .with_file_extensions(vec![Cow::Borrowed("mm.dll")]);

        assert!(rule.matches("BepInEx/monomod/myplugin.mm.dll"));
        assert!(rule.matches("BepInEx/monomod/myplugin.dll"));
        assert!(rule.matches("myplugin.mm.dll"));
        assert!(!rule.matches("myplugin.dll"));
    }

    #[test]
    fn route_map_file() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins"))
            .with_flatten(false)
            .with_file_extensions(vec![Cow::Borrowed("plugin")]);

        let package = PackageRef::new(PackageId::new("Author-Name"), "1.0.0");

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
    fn route_map_file_flatten() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins"))
            .with_flatten(true)
            .with_file_extensions(vec![Cow::Borrowed("plugin")]);

        let package = PackageRef::new(PackageId::new("Author-Name"), "1.0.0");

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
    fn route_map_file_empty() {
        let rule = RouteRule::new("plugins", Utf8Path::new("BepInEx/plugins")).with_flatten(false);

        assert_eq!(
            rule.map_file("", &PackageRef::new(PackageId::new("Author-Name"), "1.0.0")),
            None
        );
    }
}
