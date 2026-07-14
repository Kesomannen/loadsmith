use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use bytes::Bytes;
use loadsmith::{InstallRuleset, InstalledPackage, PackageRef, thunderstore::PackageRefExt};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use walkdir::WalkDir;

async fn get_loader(
    client: &thunderstore::Client,
    name: &str,
) -> anyhow::Result<Box<dyn loadsmith::Loader>> {
    let schema = client.get_schema("dev").await?;
    let lethal_company = schema
        .games
        .get(name)
        .ok_or_else(|| anyhow!("{name} not found in schema"))?;
    let r2_config = lethal_company
        .r2modman
        .as_ref()
        .and_then(|vec| vec.first())
        .ok_or_else(|| anyhow!("r2modman config not found in schema"))?;
    let loader = loadsmith::thunderstore::r2_config_to_loader(r2_config)
        .context("failed to create loader")?;

    Ok(loader)
}

async fn download_package(
    client: &thunderstore::Client,
    package: &PackageRef,
) -> anyhow::Result<Bytes> {
    let bytes = client.download(package.clone().into_ts_ident()?).await?;

    Ok(bytes)
}

fn extract_and_install_package(
    bytes: &[u8],
    package: PackageRef,
    ruleset: InstallRuleset,
) -> anyhow::Result<(TempDir, InstalledPackage)> {
    let extract_dir = tempfile::tempdir()?;
    loadsmith::extract(Cursor::new(bytes), &extract_dir)?;

    let install_dir = tempfile::tempdir()?;
    let (install_manifest, _overwritten_files) =
        loadsmith::install(package, ruleset, &extract_dir, &install_dir, false)?;

    Ok((install_dir, install_manifest))
}

#[derive(Debug, Serialize, Deserialize)]
struct ListedFile {
    file: PathBuf,
    hash: String,
}

impl ListedFile {
    fn from_path(path: PathBuf, root: &Path) -> anyhow::Result<Self> {
        let relative_path = path.strip_prefix(root)?.to_path_buf();
        let hash = loadsmith_util::hash_file_to_string(path)?;

        Ok(ListedFile {
            file: relative_path,
            hash,
        })
    }
}

fn list_files(dir: impl AsRef<Path>) -> anyhow::Result<Vec<ListedFile>> {
    let dir = dir.as_ref();

    let mut files = WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            if entry.file_type().is_dir() {
                return None;
            }

            let path = entry.path().to_path_buf();
            Some(ListedFile::from_path(path, dir))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    files.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(files)
}

async fn test_install_flow(
    community: &str,
    package: PackageRef,
    is_loader: bool,
) -> anyhow::Result<()> {
    let client = thunderstore::Client::new();

    let loader = get_loader(&client, community)
        .await
        .context("failed to get loader")?;

    let bytes = download_package(&client, &package)
        .await
        .context("failed to download package")?;

    let (profile, install_manifest) = extract_and_install_package(
        &bytes,
        package,
        if is_loader {
            loader.loader_install_rules()
        } else {
            loader.package_install_rules()
        },
    )
    .context("failed to extract and install package")?;

    let files = list_files(&profile).context("failed to list files")?;

    insta::assert_yaml_snapshot!(install_manifest.ref_.to_string(), files);

    loadsmith::uninstall(&install_manifest, &profile).context("failed to uninstall package")?;

    let files = list_files(&profile).context("failed to list files after uninstall")?;

    assert!(
        files.is_empty(),
        "profile directory should be empty after uninstall"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn install_lethal_lib() -> anyhow::Result<()> {
    let package = PackageRef::new("Evaisa-LethalLib".to_string(), (1, 2, 0));
    test_install_flow("lethal-company", package, false).await
}

#[tokio::test]
#[ignore]
async fn install_bep_in_ex() -> anyhow::Result<()> {
    let package = PackageRef::new("BepInEx-BepInExPack_ROUNDS".to_string(), (5, 4, 1901));
    test_install_flow("rounds", package, true).await
}
