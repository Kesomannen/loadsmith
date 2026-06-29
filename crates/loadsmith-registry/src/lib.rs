use std::{collections::HashMap, fmt::Display, pin::Pin};

use loadsmith_core::{PackageId, Version};

mod error;

mod registries;

pub use error::{Error, Result};
pub use registries::*;

#[derive(Debug, Clone)]
pub struct Package {
    pub versions: Vec<PackageVersion>,
}

#[derive(Debug, Clone)]
pub struct PackageVersion {
    pub version: Version,
    pub download_url: String,
    pub checksum: Option<String>,
    pub deps: Vec<PackageId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegistryId(pub &'static str);

impl Display for RegistryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait Registry {
    fn id(&self) -> RegistryId;

    fn get_package<'a>(
        &'a self,
        id: &'a PackageId,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Package>>> + 'a>>;
}

pub struct RegistrySet {
    default: Option<RegistryId>,
    registries: HashMap<&'static str, Box<dyn Registry>>,
}

impl RegistrySet {
    pub fn new() -> Self {
        Self {
            default: None,
            registries: HashMap::new(),
        }
    }

    pub fn set_default_registry_id(&mut self, id: RegistryId) {
        self.default = Some(id);
    }

    pub fn add_registry<R: Registry + 'static>(&mut self, registry: R) {
        let id = registry.id();
        self.registries.insert(id.0, Box::new(registry));
    }

    pub fn add_default_registry<R: Registry + 'static>(&mut self, registry: R) {
        let id = registry.id();
        self.set_default_registry_id(id);
        self.add_registry(registry);
    }

    pub fn default_registry(&self) -> Option<&dyn Registry> {
        self.default.and_then(|id| self.get_registry(id.0))
    }

    pub fn get_registry(&self, id: &str) -> Option<&dyn Registry> {
        self.registries.get(id).map(|r| r.as_ref())
    }
}
