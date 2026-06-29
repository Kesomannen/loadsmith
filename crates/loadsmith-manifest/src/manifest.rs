use std::collections::HashMap;

use loadsmith_core::{PackageId, Version};
use loadsmith_registry::RegistrySet;

use crate::{Result, lockfile::Lockfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependencies(pub(crate) HashMap<PackageId, Dependency>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    version: Version,
    source: Option<String>,
    registry_metadata: Option<serde_json::Value>,
}

impl Dependencies {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub async fn resolve(
        &self,
        registries: &RegistrySet,
        existing_lockfile: Option<&Lockfile>,
    ) -> Result<Lockfile> {
        crate::resolve::resolve(self.0.clone(), registries, existing_lockfile).await
    }
}

impl FromIterator<(PackageId, Dependency)> for Dependencies {
    fn from_iter<T: IntoIterator<Item = (PackageId, Dependency)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl From<HashMap<PackageId, Dependency>> for Dependencies {
    fn from(map: HashMap<PackageId, Dependency>) -> Self {
        Self(map)
    }
}

impl Dependency {
    pub fn new(version: impl Into<Version>) -> Self {
        Self {
            version: version.into(),
            source: None,
            registry_metadata: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_registry_metadata(mut self, metadata: impl Into<serde_json::Value>) -> Self {
        self.registry_metadata = Some(metadata.into());
        self
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn registry_metadata(&self) -> Option<&serde_json::Value> {
        self.registry_metadata.as_ref()
    }

    pub fn into_registry_metadata(self) -> Option<serde_json::Value> {
        self.registry_metadata
    }
}
