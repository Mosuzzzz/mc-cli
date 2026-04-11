use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct PaperProjectResponse {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct PaperVersionResponse {
    builds: Vec<u32>,
}

#[derive(Deserialize)]
struct PaperBuildResponse {
    downloads: std::collections::HashMap<String, PaperDownload>,
}

#[derive(Deserialize)]
struct PaperDownload {
    name: String,
    sha256: String,
}

pub async fn list_versions() -> Result<Vec<String>> {
    let url = "https://api.papermc.io/v2/projects/paper";
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?.error_for_status()?;
    let data: PaperProjectResponse = response
        .json()
        .await
        .context("Failed to parse PaperMC API response")?;
    Ok(data.versions)
}

pub async fn download(version: &str, dest: &std::path::Path) -> Result<()> {
    // 1. Get latest build
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}",
        version
    );
    let res: PaperVersionResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let build = res
        .builds
        .into_iter()
        .max()
        .context("No builds found for version")?;

    // 2. Get build details
    let url = format!(
        "https://api.papermc.io/v2/projects/paper/versions/{}/builds/{}",
        version, build
    );
    let res: PaperBuildResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let download_info = res
        .downloads
        .get("application")
        .context("No application download found")?;

    // 3. Download file
    let download_url = format!("{}/downloads/{}", url, download_info.name);
    let response = client.get(&download_url).send().await?.error_for_status()?;

    // Validate hash while writing
    use sha2::{Digest, Sha256};
    use std::io::Write;

    let mut file = std::fs::File::create(dest)?;
    let mut hasher = Sha256::new();

    let bytes = response.bytes().await?;
    hasher.update(&bytes);

    let hash = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    if hash != download_info.sha256 {
        anyhow::bail!(
            "SHA256 mismatch! Expected {}, got {}",
            download_info.sha256,
            hash
        );
    }

    file.write_all(&bytes)?;

    Ok(())
}
