#[macro_export]
macro_rules! glob {
    ($pattern:expr) => {
        globset::Glob::new($pattern).expect("constant pattern should be valid")
    };
}

#[macro_export]
macro_rules! glob_rule {
    ($pattern:expr => $destination:expr) => {
        loadsmith_install::GlobRule::new(
            crate::glob!($pattern),
            camino::Utf8Path::new($destination),
        )
    };
}
