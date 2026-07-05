use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    pin::Pin,
};

use loadsmith_core::{Dependency, PackageId, Version};

mod error;

mod registries;

pub use error::{Error, Result};
pub use registries::*;

#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: Version,
}

#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub url: String,
    pub size: Option<u64>,
    pub checksum: Option<String>,
    pub deps: Vec<Dependency>,
}

pub trait Registry: Debug {
    fn version_info<'a>(
        &'a self,
        id: &'a PackageId,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<VersionInfo>>> + 'a>>;

    fn resolve<'a>(
        &'a self,
        id: &'a PackageId,
        version: &'a Version,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion>> + 'a>>;
}

#[derive(Debug)]
pub struct RegistrySet {
    registries: HashMap<String, Box<dyn Registry>>,
}

impl RegistrySet {
    pub fn new() -> Self {
        Self {
            registries: HashMap::new(),
        }
    }

    pub fn add<R: Registry + 'static>(&mut self, id: impl Into<String>, registry: R) {
        self.registries.insert(id.into(), Box::new(registry));
    }

    pub fn get(&self, id: &str) -> Option<&dyn Registry> {
        self.registries.get(id).map(|r| r.as_ref())
    }
}
