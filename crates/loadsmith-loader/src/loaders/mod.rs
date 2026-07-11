mod bep_in_ex;
mod melon_loader;
mod shimloader;

mod macros;

pub use bep_in_ex::BepInEx;
use globset::{Glob, GlobBuilder};
pub use melon_loader::MelonLoader;
pub use shimloader::Shimloader;

fn top_level_dll_glob() -> Glob {
    GlobBuilder::new("*.dll")
        .literal_separator(true)
        .build()
        .expect("constant glob should be valid")
}
