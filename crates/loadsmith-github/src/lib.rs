use std::pin::Pin;

use globset::Glob;
use loadsmith_core::{PackageId, Version};
use loadsmith_registry::{Registry, Result as RegistryResult, VersionInfo};
use octocrab::Octocrab;
use serde::Deserialize;

mod error;

pub use error::{Error, Result};

#[derive(Debug)]
pub struct GithubRegistry {
    github: Octocrab,
}

impl GithubRegistry {
    pub fn new() -> Self {
        Self::with_github(Octocrab::default())
    }

    pub fn with_github(github: Octocrab) -> Self {
        Self { github }
    }

    async fn version_info(
        &self,
        package_id: &PackageId,
        metadata: &Metadata,
    ) -> Result<Vec<VersionInfo>> {
        let (owner, repo) = split_package_id(package_id)?;

        let releases = self
            .github
            .repos(owner, repo)
            .releases()
            .list()
            .send()
            .await
            .map_err(|err| Error::map_github_404(err, || Error::PackageNotFound))?;

        let versions = releases
            .into_iter()
            .map(|release| {
                let version = metadata.parse_tag(&release.tag_name)?;

                Ok(loadsmith_registry::VersionInfo { version })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(versions)
    }

    async fn resolve(
        &self,
        package_id: &PackageId,
        version: &Version,
        metadata: &Metadata,
    ) -> Result<loadsmith_registry::ResolvedVersion> {
        let tag = metadata.tag(version);
        let release = self.release_by_tag(package_id, tag).await?;

        let matching_assets = release
            .assets
            .iter()
            .filter(|asset| metadata.matches_asset(&asset.name))
            .collect::<Vec<_>>();

        let asset = match matching_assets.as_slice() {
            [asset] => asset,
            [] => {
                return Err(Error::NoMatchingAssets {
                    assets: release.assets.into_iter().map(|a| a.name).collect(),
                });
            }
            _ => {
                return Err(Error::MultipleMatchingAssets {
                    matching_assets: matching_assets
                        .into_iter()
                        .map(|a| a.name.clone())
                        .collect(),
                });
            }
        };

        let resolved_version = loadsmith_registry::ResolvedVersion {
            url: asset.browser_download_url.to_string(),
            size: Some(asset.size as u64),
            checksum: asset.digest.clone(),
            deps: Vec::new(),
        };

        Ok(resolved_version)
    }

    async fn release_by_tag(
        &self,
        package_id: &PackageId,
        tag: impl AsRef<str>,
    ) -> Result<octocrab::models::repos::Release> {
        let (owner, repo) = split_package_id(package_id)?;

        let release = self
            .github
            .repos(owner, repo)
            .releases()
            .get_by_tag(tag.as_ref())
            .await
            .map_err(|err| Error::map_github_404(err, || Error::VersionNotFound))?;

        Ok(release)
    }
}

impl Default for GithubRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Metadata {
    #[serde(rename = "tag")]
    tag_template: String,
    #[serde(rename = "asset")]
    asset_glob: Glob,
}

impl Metadata {
    fn tag(&self, version: &Version) -> String {
        self.tag_template
            .replacen("{version}", &version.to_string(), 1)
    }

    fn parse_tag(&self, tag: &str) -> Result<Version> {
        let prefix = self.tag_template.split("{version}").next().unwrap_or("");
        let suffix = self.tag_template.split("{version}").nth(1).unwrap_or("");

        if !tag.starts_with(prefix) || !tag.ends_with(suffix) {
            return Err(Error::InvalidReleaseTagFormat {
                tag: tag.to_string(),
            });
        }

        let version_str = &tag[prefix.len()..tag.len() - suffix.len()];
        let version: Version =
            version_str
                .parse()
                .map_err(|err| Error::InvalidReleaseTagVersion {
                    tag: tag.to_string(),
                    err,
                })?;

        Ok(version)
    }

    fn matches_asset(&self, asset_name: impl AsRef<str>) -> bool {
        self.asset_glob
            .compile_matcher()
            .is_match(asset_name.as_ref())
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            tag_template: "v{version}".to_string(),
            asset_glob: Glob::new("*.zip").expect("constant glob should be valid"),
        }
    }
}

impl Registry for GithubRegistry {
    fn version_info<'a>(
        &'a self,
        id: &'a PackageId,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<Vec<loadsmith_registry::VersionInfo>>> + 'a>>
    {
        Box::pin(async move {
            let metadata = loadsmith_registry::read_metadata_or_default(metadata)?;
            let versions = self.version_info(id, &metadata).await?;
            Ok(versions)
        })
    }

    fn resolve<'a>(
        &'a self,
        id: &'a PackageId,
        version: &'a Version,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<loadsmith_registry::ResolvedVersion>> + 'a>>
    {
        Box::pin(async move {
            let metadata = loadsmith_registry::read_metadata_or_default(metadata)?;
            let resolved = self.resolve(id, version, &metadata).await?;
            Ok(resolved)
        })
    }
}

fn split_package_id(id: &PackageId) -> Result<(&str, &str)> {
    id.as_str()
        .split_once('/')
        .ok_or(Error::InvalidPackageIdFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn version_info_lethal_lib() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .unwrap();

        let registry = GithubRegistry::default();
        let id = PackageId::new("EvaisaDev/LethalLib");

        let versions = registry
            .version_info(
                &id,
                &Metadata {
                    tag_template: "v{version}".to_string(),
                    asset_glob: Glob::new("*.zip").unwrap(),
                },
            )
            .await
            .unwrap();

        assert!(!versions.is_empty());

        println!("{versions:#?}");
    }

    #[tokio::test]
    #[ignore]
    async fn resolve_lethal_lib() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .unwrap();
        let registry = GithubRegistry::default();
        let id = PackageId::new("EvaisaDev/LethalLib");
        let version = Version::new(0, 13, 1);

        let resolved = registry
            .resolve(
                &id,
                &version,
                &Metadata {
                    tag_template: "v{version}".to_string(),
                    asset_glob: Glob::new("*.zip").unwrap(),
                },
            )
            .await
            .unwrap();

        println!("{resolved:#?}");
    }

    #[test]
    fn metadata_tag() {
        let metadata = Metadata {
            tag_template: "v{version}".to_string(),
            ..Default::default()
        };

        let version = Version::new(1, 2, 3);
        let tag = metadata.tag(&version);
        assert_eq!(tag, "v1.2.3");
    }

    #[test]
    fn metadata_parse_tag() {
        let metadata = Metadata {
            tag_template: "v{version}".to_string(),
            ..Default::default()
        };

        let tag = "v1.2.3";
        let version = metadata.parse_tag(tag).unwrap();
        assert_eq!(version, Version::new(1, 2, 3));
    }

    #[test]
    fn metadata_parse_tag_with_suffix() {
        let metadata = Metadata {
            tag_template: "v{version}-beta".to_string(),
            ..Default::default()
        };

        let tag = "v1.2.3-beta";
        let version = metadata.parse_tag(tag).unwrap();
        assert_eq!(version, Version::new(1, 2, 3));
    }

    #[test]
    fn metadata_parse_tag_base_template() {
        let metadata = Metadata {
            tag_template: "{version}".to_string(),
            ..Default::default()
        };

        let tag = "1.2.3";
        let version = metadata.parse_tag(tag).unwrap();
        assert_eq!(version, Version::new(1, 2, 3));
    }

    #[test]
    fn metadata_parse_tag_invalid() {
        let metadata = Metadata {
            tag_template: "v{version}".to_string(),
            ..Default::default()
        };

        let tag = "1.2.3";
        let result = metadata.parse_tag(tag);
        assert!(result.is_err());
    }

    #[test]
    fn metadata_asset_zip_default() {
        let metadata = Metadata::default();
        assert_eq!(metadata.asset_glob.glob(), "*.zip");
    }

    #[test]
    fn metadata_asset_glob_custom() {
        let metadata = Metadata {
            asset_glob: Glob::new("*.tar.gz").unwrap(),
            ..Default::default()
        };

        assert!(metadata.matches_asset("file.tar.gz"));
        assert!(!metadata.matches_asset("file.zip"));
    }
}
