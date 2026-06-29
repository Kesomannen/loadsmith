use std::pin::Pin;

use loadsmith_core::PackageId;

use crate::{Package, PackageVersion, Registry, RegistryId, Result};

#[derive(Debug, Clone)]
pub struct ThunderstoreRegistry {
    _client: thunderstore::Client,
    packages: Vec<thunderstore::models::PackageV1>,
}

impl ThunderstoreRegistry {
    pub async fn create(community: impl AsRef<str>) -> Result<Self> {
        Self::create_with(thunderstore::Client::default(), community).await
    }

    pub async fn create_with(
        client: thunderstore::Client,
        community: impl AsRef<str>,
    ) -> Result<Self> {
        let packages = client.get_package_index_v1(community).await?;

        Ok(Self {
            _client: client,
            packages,
        })
    }

    fn find_package(&self, id: &PackageId) -> Option<&thunderstore::models::PackageV1> {
        self.packages
            .iter()
            .find(|package| package.ident.as_str() == id.as_str())
    }
}

impl Registry for ThunderstoreRegistry {
    fn id(&self) -> RegistryId {
        RegistryId("thunderstore")
    }

    fn get_package<'a>(
        &'a self,
        id: &'a PackageId,
        _metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Package>>> + 'a>> {
        Box::pin(async move {
            let Some(package) = self.find_package(id) else {
                return Ok(None);
            };

            let versions = package
                .versions
                .iter()
                .map(|version| {
                    let version_number = version.number.clone().into();
                    let deps = version
                        .dependencies
                        .iter()
                        .map(|dep| PackageId::from(dep.package_id().into_string()))
                        .collect();

                    Ok(PackageVersion {
                        version: version_number,
                        download_url: version.download_url.to_string(),
                        checksum: None,
                        deps,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(Some(Package { versions }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn get_package() {
        let client = ThunderstoreRegistry::create("rounds").await.unwrap();
        let package = client
            .get_package(&PackageId::new("BepInEx-BepInExPack_ROUNDS"), None)
            .await
            .unwrap()
            .unwrap();

        println!("{package:#?}");
    }
}
