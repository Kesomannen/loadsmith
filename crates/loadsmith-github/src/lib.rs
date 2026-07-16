use std::{pin::Pin, str::FromStr};

use globset::Glob;
use loadsmith_core::{Checksum, PackageId, PackageRef, Version};
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
        let (owner, repo) = metadata.owner_and_repo(package_id)?;

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
        pkg: &PackageRef,
        metadata: &Metadata,
    ) -> Result<loadsmith_registry::ResolvedVersion> {
        let release = self.release_by_tag(metadata, pkg).await?;

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

        let checksum = asset
            .digest
            .as_ref()
            .map(|digest| {
                Checksum::from_str(digest).map_err(|err| Error::InvalidAssetDigest {
                    checksum: digest.clone(),
                    err,
                })
            })
            .transpose()?;

        let resolved_version = loadsmith_registry::ResolvedVersion {
            url: asset.browser_download_url.to_string(),
            size: Some(asset.size as u64),
            checksum,
            deps: Vec::new(),
        };

        Ok(resolved_version)
    }

    async fn release_by_tag(
        &self,
        metadata: &Metadata,
        pkg: &PackageRef,
    ) -> Result<octocrab::models::repos::Release> {
        let (owner, repo) = metadata.owner_and_repo(pkg.id())?;
        let tag = metadata.tag(pkg.version());

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
    #[serde(rename = "repo")]
    repository: Option<String>,
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

    fn owner_and_repo<'a>(&'a self, package_id: &'a PackageId) -> Result<(&'a str, &'a str)> {
        if let Some(repo) = &self.repository {
            let mut parts = repo.split('/');
            let (owner, name) = match (parts.next(), parts.next(), parts.next()) {
                (Some(owner), Some(repo), None) => (owner, repo),
                _ => {
                    return Err(Error::InvalidRepositoryFormat(repo.clone()));
                }
            };

            Ok((owner, name))
        } else {
            package_id
                .as_str()
                .split_once('-')
                .ok_or(Error::InvalidPackageIdFormat)
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            tag_template: "v{version}".to_string(),
            asset_glob: Glob::new("*.zip").expect("constant glob should be valid"),
            repository: None,
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
        ref_: &'a PackageRef,
        metadata: Option<&'a serde_json::Value>,
    ) -> Pin<Box<dyn Future<Output = RegistryResult<loadsmith_registry::ResolvedVersion>> + 'a>>
    {
        Box::pin(async move {
            let metadata = loadsmith_registry::read_metadata_or_default(metadata)?;
            let resolved = self.resolve(ref_, &metadata).await?;
            Ok(resolved)
        })
    }
}

#[cfg(test)]
mod tests {
    use loadsmith_core::PackageRef;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn version_info_lethal_lib() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .unwrap();

        let registry = GithubRegistry::default();
        let id = PackageId::new("Evaisa-LethalLib");

        let versions = registry
            .version_info(
                &id,
                &Metadata {
                    tag_template: "v{version}".to_string(),
                    asset_glob: Glob::new("*.zip").unwrap(),
                    repository: Some("EvaisaDev/LethalLib".to_string()),
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
        let ref_ = PackageRef::new("Evaisa-LethalLib".to_string(), (0, 13, 1));

        let resolved = registry
            .resolve(
                &ref_,
                &Metadata {
                    tag_template: "v{version}".to_string(),
                    asset_glob: Glob::new("*.zip").unwrap(),
                    repository: Some("EvaisaDev/LethalLib".to_string()),
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
    fn metadata_tag_multiple_placeholders() {
        let metadata = Metadata {
            tag_template: "v{version}-beta-{version}".to_string(),
            ..Default::default()
        };

        let version = Version::new(1, 2, 3);
        let tag = metadata.tag(&version);
        assert_eq!(tag, "v1.2.3-beta-{version}");
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

    #[test]
    fn metadata_owner_and_repo() {
        let package_id = PackageId::new("some-package");

        let metadata = Metadata {
            repository: Some("owner/repo".to_string()),
            ..Default::default()
        };
        let (owner, repo) = metadata.owner_and_repo(&package_id).unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");

        let metadata_no_repo = Metadata::default();
        let (owner, repo) = metadata_no_repo.owner_and_repo(&package_id).unwrap();
        assert_eq!(owner, "some");
        assert_eq!(repo, "package");
    }
}
