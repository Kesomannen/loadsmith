#[macro_export]
macro_rules! glob {
    ($pattern:expr) => {
        Glob::new($pattern).expect("constant pattern should be valid")
    };
}

#[macro_export]
macro_rules! glob_rule {
    ($pattern:expr, $destination:expr, $strip_top_level:expr) => {
        InstallRule::Glob(
            loadsmith_install::GlobRule::new(crate::glob!($pattern), Utf8Path::new($destination))
                .with_strip_top_level($strip_top_level),
        )
    };
}

#[macro_export]
macro_rules! glob_rules {
    [$(($pattern:literal => $destination:literal, $strip_top_level:expr)),* $(,)?] => {
        vec![
            $(
                $crate::glob_rule!($pattern, $destination, $strip_top_level),
            )*
        ]
    };
}
