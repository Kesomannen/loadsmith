mod error;

pub use error::{Error, Result};

pub trait Registry {
    fn id(&self) -> &'static str;
}
