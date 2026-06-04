use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Version {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    url: String, // added url
}

#[derive(Deserialize)]
struct ManifestResponse {
    versions: Vec<Version>,
}

#[derive(Deserialize)]
struct VersionJson {
    downloads: Option<Downloads>,
}

#[derive(Deserialize)]
struct Downloads {
    server: Option<ServerDownload>,
}

#[derive(Deserialize)]
struct ServerDownload {
    url: String,
    sha1: String,
}

fn api_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("mc-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

pub async fn list_versions() -> Result<Vec<String>> {
    let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let client = api_client()?;
    let response = client.get(url).send().await?.error_for_status()?;
    let data: ManifestResponse = response
        .json()
        .await
        .context("Failed to parse Mojang API response")?;

    // Return release versions, API returns newest-first already
    let releases = data
        .versions
        .into_iter()
        .filter(|v| v.version_type == "release")
        .map(|v| v.id)
        .collect();

    Ok(releases)
}

pub async fn download(version: &str, dest: &std::path::Path) -> Result<()> {
    // 1. Fetch version manifest
    let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let client = reqwest::Client::builder()
        .user_agent(concat!("mc-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    let res: ManifestResponse = client
        .get(manifest_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    // 2. Find the version URL
    let ver = res
        .versions
        .into_iter()
        .find(|v| v.id == version)
        .context("Version not found in vanilla manifest")?;

    // 3. Fetch version JSON
    let ver_res: VersionJson = client
        .get(&ver.url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let server_dl = ver_res
        .downloads
        .and_then(|d| d.server)
        .context("No server download found for this version")?;

    // 4. Download file
    let response = client
        .get(&server_dl.url)
        .send()
        .await?
        .error_for_status()?;

    // Validate hash while writing
    use sha1::{Digest, Sha1};
    use std::io::Write;

    let mut file = std::fs::File::create(dest)?;
    let mut hasher = Sha1::new();

    let bytes = response.bytes().await?;
    println!("  {:.1} MB — verifying SHA-1...", bytes.len() as f64 / 1_048_576.0);
    hasher.update(&bytes);

    let hash = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    if hash != server_dl.sha1 {
        anyhow::bail!("SHA1 mismatch! Expected {}, got {}", server_dl.sha1, hash);
    }

    file.write_all(&bytes)?;

    Ok(())
}
