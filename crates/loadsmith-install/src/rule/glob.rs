use std::borrow::Cow;

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobBuilder};
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

/// A glob-based install rule that maps matching files to a target directory.
///
/// A `GlobRule` matches files by a glob pattern and maps them into a target
/// directory, with optional path stripping and subdirectory creation.
///
/// # Examples
///
/// ```rust
/// use std::borrow::Cow;
/// use camino::{Utf8Path, Utf8PathBuf};
/// use globset::Glob;
/// use loadsmith_core::{PackageRef, PackageId, Version};
/// use loadsmith_install::GlobRule;
///
/// let rule = GlobRule::try_from_pattern("*.dll", Utf8Path::new("BepInEx/plugins")).unwrap();
/// let pkg = PackageRef::new(PackageId::new("x753-More_Suits"), Version::new(1, 0, 3));
///
/// assert!(rule.matches("MyPlugin.dll"));
/// assert!(!rule.matches("config.txt"));
/// assert_eq!(
///     rule.map_file("MyPlugin.dll", &pkg),
///     Some(Utf8PathBuf::from("BepInEx/plugins/MyPlugin.dll"))
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobRule {
    pattern: Glob,
    pub(crate) target: Cow<'static, Utf8Path>,
    strip_levels: usize,
    subdir: bool,
    pub(super) use_links: bool,
}

impl GlobRule {
    /// Creates a new `GlobRule` from a compiled [`Glob`] pattern and a target path.
    pub fn new(pattern: Glob, target: impl Into<Cow<'static, Utf8Path>>) -> Self {
        Self {
            pattern,
            target: target.into(),
            strip_levels: 0,
            subdir: false,
            use_links: false,
        }
    }

    /// Creates a new `GlobRule` by compiling a glob pattern string.
    ///
    /// Returns a [`globset::Error`] if the pattern is invalid.
    pub fn try_from_pattern(
        pattern: impl AsRef<str>,
        target: impl Into<Utf8PathBuf>,
    ) -> Result<Self, globset::Error> {
        let pattern = GlobBuilder::new(pattern.as_ref()).build()?;
        Ok(Self::new(pattern, Cow::Owned(target.into())))
    }

    /// Strips the top-level directory component from paths before mapping.
    pub fn strip_top_level(mut self, strip_top_level: bool) -> Self {
        self.strip_levels = if strip_top_level { 1 } else { 0 };
        self
    }

    /// Strips the given number of leading path components before mapping.
    pub fn strip_levels(mut self, strip_levels: usize) -> Self {
        self.strip_levels = strip_levels;
        self
    }

    /// Sets whether to use hard links instead of copying files.
    pub fn use_links(mut self, use_links: bool) -> Self {
        self.use_links = use_links;
        self
    }

    /// Sets whether to create a subdirectory named after the package ID inside
    /// the target directory.
    pub fn with_subdir(mut self, subdir: bool) -> Self {
        self.subdir = subdir;
        self
    }

    /// Returns `true` if the path matches this rule's glob pattern.
    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.pattern.compile_matcher().is_match(path.as_ref())
    }

    /// Maps a file path to its install destination under the rule's target directory.
    ///
    /// Returns `None` if stripping removes all path components. When `subdir` is
    /// enabled the package ID is inserted as an intermediate directory.
    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        let path = path.as_ref();

        let suffix = {
            let mut components = path.components();
            for _ in 0..self.strip_levels {
                components.next();
            }

            if components.clone().next().is_none() {
                return None;
            } else {
                components.as_path()
            }
        };

        let mut target = self.target.to_path_buf();
        if self.subdir {
            target.push(package.id().as_str());
        }
        target.push(suffix);

        Some(target)
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::{PackageId, Version};

    use super::*;

    #[test]
    fn matches() {
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
    fn map_file() {
        let rule = GlobRule::try_from_pattern("*.rs", Utf8Path::new("target")).unwrap();

        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        assert_eq!(
            rule.map_file("main.rs", &package),
            Some(Utf8PathBuf::from("target/main.rs"))
        );

        assert_eq!(
            rule.map_file("src/main.rs", &package),
            Some(Utf8PathBuf::from("target/src/main.rs"))
        );

        assert_eq!(
            rule.map_file("main.txt", &package),
            Some(Utf8PathBuf::from("target/main.txt"))
        );
    }

    #[test]
    fn map_file_strip_top() {
        let rule = GlobRule::try_from_pattern("in/**", Utf8Path::new("out"))
            .unwrap()
            .strip_top_level(true);
        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        assert_eq!(
            rule.map_file("in/file.txt", &package),
            Some(Utf8PathBuf::from("out/file.txt"))
        );

        assert_eq!(
            rule.map_file("in/nested/file.txt", &package),
            Some(Utf8PathBuf::from("out/nested/file.txt"))
        );

        assert_eq!(
            rule.map_file("in/nested/deep/file.txt", &package),
            Some(Utf8PathBuf::from("out/nested/deep/file.txt"))
        );

        assert_eq!(rule.map_file("other.txt", &package), None);
    }

    #[test]
    fn map_file_strip_multiple() {
        let rule = GlobRule::try_from_pattern("UE4SS/Mods/*", Utf8Path::new("shimloader/mod"))
            .unwrap()
            .strip_levels(2);
        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        assert_eq!(rule.map_file("file.dll", &package), None);

        assert_eq!(rule.map_file("UE4SS/file.dll", &package), None);

        assert_eq!(
            rule.map_file("UE4SS/Mods/file.dll", &package),
            Some(Utf8PathBuf::from("shimloader/mod/file.dll"))
        );

        assert_eq!(
            rule.map_file("UE4SS/Mods/nested/file.dll", &package),
            Some(Utf8PathBuf::from("shimloader/mod/nested/file.dll"))
        );

        assert_eq!(
            rule.map_file("UE4SS/Mods/nested/deep/file.dll", &package),
            Some(Utf8PathBuf::from("shimloader/mod/nested/deep/file.dll"))
        );
    }
}
