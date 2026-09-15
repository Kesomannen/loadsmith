use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobBuilder};
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

/// A glob-based install rule.
///
/// A `GlobRule` matches files by a glob pattern and maps them into a target
/// directory, with optional path stripping and subdirectory creation.
///
/// The glob implementation used is the [globset] crate, refer to its documentation for details.
/// This struct is best used when that crate is in your dependencies.
///
/// ## Examples
///
/// ```
/// # use loadsmith_core::{PackageRef, PackageId, Version};
/// # use loadsmith_install::rule::GlobRule;
///
/// let rule = GlobRule::try_from_pattern("*.dll", "BepInEx/plugins").unwrap();
///
/// assert!(rule.matches("MyPlugin.dll"));
/// assert!(rule.matches("plugins/MyPlugin.dll"));
/// assert!(!rule.matches("config.txt"));
///
/// let pkg = PackageRef::new(PackageId::new("x753-More_Suits"), Version::new(1, 0, 3));
///
/// assert_eq!(
///     rule.map_file("MyPlugin.dll", &pkg),
///     Some("BepInEx/plugins/MyPlugin.dll".into())
/// );
///
/// assert_eq!(
///     rule.map_file("plugins/MyPlugin.dll", &pkg),
///     Some("BepInEx/plugins/plugins/MyPlugin.dll".into())
/// );
///
/// // Paths that don't match the glob pattern may still be mapped.
/// assert_eq!(
///     rule.map_file("config.txt", &pkg),
///     Some("BepInEx/plugins/config.txt".into())
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobRule {
    pattern: Glob,
    pub(crate) target: Utf8PathBuf,
    strip_levels: usize,
    subdir: bool,
    use_links: bool,
}

impl GlobRule {
    /// Creates a new `GlobRule` from a compiled [`Glob`] pattern and a target path.
    pub fn new(pattern: Glob, target: impl Into<Utf8PathBuf>) -> Self {
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
        Ok(Self::new(pattern, target.into()))
    }

    /// Sets whether the top-level directory component of paths should be stripped.
    ///
    /// Equivalent to calling [`strip_levels`](GlobRule::strip_levels) with a value of `0` or `1`.
    /// See [`strip_levels`](GlobRule::strip_levels) for more information.
    pub fn strip_top_level(mut self, strip_top_level: bool) -> Self {
        self.strip_levels = if strip_top_level { 1 } else { 0 };
        self
    }

    /// Strips the given number of leading path components before mapping.
    ///
    /// If a path contains fewer components than the number to strip, it will be mapped to `None`.
    ///
    /// ## Example
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, PackageId, Version};
    /// # use loadsmith_install::rule::GlobRule;
    /// // Map all files (`*`) to the current directory, but strip the top-level directory component.
    /// let strip_rule = GlobRule::try_from_pattern("*", ".").unwrap().strip_top_level(true);
    ///
    /// strip_rule.matches("README.md");
    /// strip_rule.matches("config/settings.json");
    ///
    /// let pkg = PackageRef::new(PackageId::new("x753-More_Suits"), Version::new(1, 0, 3));
    ///
    /// // The file is outside any directory, so stripping removes all components and results in no mapping.
    /// assert_eq!(
    ///     strip_rule.map_file("README.md", &pkg),
    ///     None
    /// );
    ///
    /// assert_eq!(
    ///     strip_rule.map_file("config/settings.json", &pkg),
    ///     Some("./settings.json".into())
    /// );
    ///
    /// assert_eq!(
    ///     strip_rule.map_file("nested/config/settings.json", &pkg),
    ///     Some("./config/settings.json".into())
    /// );
    ///
    /// // Current directory components (`.`) are ignored when stripping.
    /// // Both `.` components are ignored, leaving only `README.md` which is stripped away.
    /// assert_eq!(
    ///     strip_rule.map_file("././README.md", &pkg),
    ///     None
    /// );
    ///
    /// assert_eq!(
    ///     strip_rule.map_file("././config/./settings.json", &pkg),
    ///     Some("./settings.json".into())
    /// );
    /// ```
    pub fn strip_levels(mut self, strip_levels: usize) -> Self {
        self.strip_levels = strip_levels;
        self
    }

    /// Sets whether the rule should prefer hard links over file copies.
    ///
    /// Note that the actual behavior depends on the implementation of the install operation;
    /// this is just a hint to the installer.
    ///
    /// This option is disabled by default.
    pub fn with_links(mut self, with_links: bool) -> Self {
        self.use_links = with_links;
        self
    }

    /// Sets whether to create a subdirectory named after the package ID inside
    /// the target directory.
    ///
    /// ```
    /// # use loadsmith_core::{PackageRef, PackageId, Version};
    /// # use loadsmith_install::rule::GlobRule;
    /// let subdir_rule = GlobRule::try_from_pattern("*.dll", "BepInEx/plugins").unwrap().with_subdir(true);
    ///
    /// let pkg = PackageRef::new(PackageId::new("x753-More_Suits"), Version::new(1, 0, 3));
    ///
    /// assert_eq!(
    ///     subdir_rule.map_file("MyPlugin.dll", &pkg),
    ///     Some("BepInEx/plugins/x753-More_Suits/MyPlugin.dll".into())
    /// );
    ///
    /// assert_eq!(
    ///     subdir_rule.map_file("some/nesting/MyPlugin.dll", &pkg),
    ///     Some("BepInEx/plugins/x753-More_Suits/some/nesting/MyPlugin.dll".into())
    /// );
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
    /// Returns `None` if stripping removes all path components.
    ///
    /// When `subdir` is enabled the package ID is inserted as an intermediate directory.
    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        package: &PackageRef,
    ) -> Option<Utf8PathBuf> {
        let path = path.as_ref();

        let suffix = {
            let mut components = path.components();

            // Component iterators ignore every `CurDir` component except the first one.
            // We want to treat *all* `CurDir` components as no-ops, so we skip the first one if it exists.
            if let Some(camino::Utf8Component::CurDir) = components.clone().next() {
                components.next();
            }

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

    /// Returns `true` if the rule prefers hard links over file copies.
    ///
    /// Note that the actual behavior depends on the implementation of the install operation;
    /// this is just a hint to the installer.
    pub fn use_links(&self) -> bool {
        self.use_links
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::{PackageId, Version};

    use super::*;

    #[test]
    fn matches() {
        let rule = GlobRule::new(Glob::new("*.rs").unwrap(), "target");

        assert!(rule.matches("main.rs"));
        assert!(rule.matches("src/main.rs"));
        assert!(!rule.matches("main.txt"));
        assert!(!rule.matches("src/main.txt"));
    }

    #[test]
    fn map_file() {
        let rule = GlobRule::try_from_pattern("*.rs", "target").unwrap();

        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        assert_eq!(
            rule.map_file("main.rs", &package),
            Some("target/main.rs".into())
        );

        assert_eq!(
            rule.map_file("src/main.rs", &package),
            Some("target/src/main.rs".into())
        );

        assert_eq!(
            rule.map_file("main.txt", &package),
            Some("target/main.txt".into())
        );
    }

    #[test]
    fn map_file_strip_top() {
        let rule = GlobRule::try_from_pattern("in/**", "out")
            .unwrap()
            .strip_top_level(true);
        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        assert_eq!(
            rule.map_file("in/file.txt", &package),
            Some("out/file.txt".into())
        );

        assert_eq!(
            rule.map_file("in/nested/file.txt", &package),
            Some("out/nested/file.txt".into())
        );

        assert_eq!(
            rule.map_file("in/nested/deep/file.txt", &package),
            Some("out/nested/deep/file.txt".into())
        );

        assert_eq!(rule.map_file("other.txt", &package), None);
    }

    #[test]
    fn map_file_strip_multiple() {
        let rule = GlobRule::try_from_pattern("UE4SS/Mods/*", "shimloader/mod")
            .unwrap()
            .strip_levels(2);
        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        assert_eq!(rule.map_file("file.dll", &package), None);

        assert_eq!(rule.map_file("UE4SS/file.dll", &package), None);

        assert_eq!(
            rule.map_file("UE4SS/Mods/file.dll", &package),
            Some("shimloader/mod/file.dll".into())
        );

        assert_eq!(
            rule.map_file("UE4SS/Mods/nested/file.dll", &package),
            Some("shimloader/mod/nested/file.dll".into())
        );

        assert_eq!(
            rule.map_file("UE4SS/Mods/nested/deep/file.dll", &package),
            Some("shimloader/mod/nested/deep/file.dll".into())
        );
    }

    #[test]
    fn map_file_strip_current_dirs() {
        let rule = GlobRule::try_from_pattern("*", "target")
            .unwrap()
            .strip_levels(1);
        let package = PackageRef::new(PackageId::new("Author-Name"), Version::new(1, 0, 0));

        // `.` components are always ignored
        assert_eq!(rule.map_file("file.dll", &package), None);
        assert_eq!(rule.map_file("./file.dll", &package), None);
        assert_eq!(rule.map_file("././file.dll", &package), None);
        assert_eq!(rule.map_file("./././file.dll", &package), None);

        // `nested` ends up as the first real path component, so it is stripped
        assert_eq!(
            rule.map_file("./././nested/file.dll", &package),
            Some("target/file.dll".into())
        );
        assert_eq!(
            rule.map_file("./././nested/./././file.dll", &package),
            Some("target/file.dll".into())
        );
        assert_eq!(
            rule.map_file("nested/./././file.dll", &package),
            Some("target/file.dll".into())
        );
    }
}
