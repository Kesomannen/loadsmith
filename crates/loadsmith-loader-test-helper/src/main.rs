use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use bytes::Bytes;
use loadsmith_core::PackageRef;
use loadsmith_thunderstore::PackageRefExt;
use serde::Deserialize;
use tokio::fs;
use tracing::{Level, debug, info, warn};

#[derive(Debug, Deserialize)]
struct Packages(HashMap<String, Vec<PackageRef>>);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let out_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("out"));

    let packages_json = include_str!("../packages.json");
    let packages: Packages = serde_json::from_str(packages_json)?;

    let http = reqwest::Client::new();

    for (category, package_list) in packages.0 {
        let category_path = out_path.join(&category);
        fs::create_dir_all(&category_path).await?;

        for pkg in package_list {
            let bytes = download_package(&http, &pkg)
                .await
                .with_context(|| format!("failed to download {pkg}"))?;

            debug!(category, %pkg, len = bytes.len(), "downloaded package");

            let file_list = extract_file_list(bytes)
                .await
                .with_context(|| format!("failed to extract file list for {pkg}"))?;
            let contents = file_list.join("\n");

            let package_path = category_path.join(format!("{pkg}.txt"));
            fs::write(&package_path, contents).await?;

            info!(
                %pkg,
                category,
                file_count = file_list.len(),
                path = %package_path.display(),
                "wrote package list"
            );
        }
    }

    Ok(())
}

pub async fn download_package(http: &reqwest::Client, pkg: &PackageRef) -> Result<Bytes> {
    let ident = pkg.to_owned().into_ts_ident()?;

    let url = format!("https://thunderstore.io/package/download/{}/", ident.path());

    debug!(%pkg, %url, "downloading package");

    for _ in 0..3 {
        match try_download(http, &url).await {
            Ok(bytes) => return Ok(bytes),
            Err(err) => {
                warn!(%pkg, %url, error = %err, "failed to download package, retrying");
            }
        }
    }

    bail!("failed to download package {pkg} after 3 attempts");
}

async fn try_download(http: &reqwest::Client, url: &str) -> Result<Bytes> {
    let response = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    Ok(response)
}

pub async fn extract_file_list(bytes: Bytes) -> Result<Vec<String>> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader)?;

    let mut file_list = (0..archive.len())
        .filter_map(|i| match archive.by_index(i) {
            Ok(entry) if entry.is_dir() => None,
            Ok(entry) => Some(Ok(entry.mangled_name().to_string_lossy().into_owned())),
            Err(err) => Some(Err(anyhow!(err).context("failed to read zip entry"))),
        })
        .collect::<Result<Vec<_>>>()?;

    file_list.sort();

    Ok(file_list)
}
