use std::borrow::Cow;

use camino::{Utf8Path, Utf8PathBuf};
use globset::{Glob, GlobBuilder};
use loadsmith_core::PackageRef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobRule {
    pattern: Glob,
    pub(super) target: Cow<'static, Utf8Path>,
    strip_levels: usize,
    pub(super) use_links: bool,
}

impl GlobRule {
    pub fn new(pattern: Glob, target: impl Into<Cow<'static, Utf8Path>>) -> Self {
        Self {
            pattern,
            target: target.into(),
            strip_levels: 0,
            use_links: false,
        }
    }

    pub fn try_from_pattern(
        pattern: impl AsRef<str>,
        target: impl Into<Cow<'static, Utf8Path>>,
    ) -> Result<Self, globset::Error> {
        let pattern = GlobBuilder::new(pattern.as_ref()).build()?;
        Ok(Self::new(pattern, target))
    }

    pub fn strip_top_level(mut self, strip_top_level: bool) -> Self {
        self.strip_levels = if strip_top_level { 1 } else { 0 };
        self
    }

    pub fn strip_levels(mut self, strip_levels: usize) -> Self {
        self.strip_levels = strip_levels;
        self
    }

    pub fn use_links(mut self, use_links: bool) -> Self {
        self.use_links = use_links;
        self
    }

    pub fn matches(&self, path: impl AsRef<Utf8Path>) -> bool {
        self.pattern.compile_matcher().is_match(path.as_ref())
    }

    pub fn map_file(
        &self,
        path: impl AsRef<Utf8Path>,
        _package: &PackageRef,
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

        Some(self.target.as_ref().join(suffix))
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::PackageId;

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

        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

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
        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

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
        let package = PackageRef::new(PackageId::new("Author-Name"), (1, 0, 0));

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
