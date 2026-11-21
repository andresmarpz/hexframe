use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use greyscreen_core::update::{self, Asset, EXE_ASSET, Release};
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(crate) const API_BASE: &str = "https://api.github.com";
pub(crate) const DOWNLOAD_PREFIX: &str =
    "https://github.com/andresmarpz/greyscreen/releases/download/";
const RELEASES_PER_PAGE: u32 = 20;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const LIST_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MZ_MAGIC: &[u8] = b"MZ";

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("request to {url} failed: {reason}")]
    Request { url: String, reason: String },
    #[error("GitHub is limiting requests right now")]
    RateLimited,
    #[error("bad response from {url}: {reason}")]
    Bad { url: String, reason: String },
    #[error("the download failed its check: {0}")]
    Verify(String),
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone)]
pub struct GithubReleases {
    api_base: String,
    download_prefix: String,
    user_agent: String,
}

impl GithubReleases {
    pub fn new(version: &str) -> Self {
        Self::with_bases(API_BASE, DOWNLOAD_PREFIX, version)
    }

    pub fn with_bases(api_base: &str, download_prefix: &str, version: &str) -> Self {
        Self {
            api_base: api_base.trim_end_matches('/').to_owned(),
            download_prefix: download_prefix.to_owned(),
            user_agent: format!("greyscreen/{version}"),
        }
    }

    fn agent(&self, timeout: Duration) -> ureq::Agent {
        ureq::Agent::config_builder()
            .user_agent(self.user_agent.clone())
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(timeout))
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .build()
            .into()
    }

    pub fn releases(&self) -> Result<Vec<Release>, UpdateError> {
        let url = format!(
            "{}/repos/{}/releases?per_page={RELEASES_PER_PAGE}",
            self.api_base,
            update::REPO
        );
        let mut response = self
            .agent(LIST_TIMEOUT)
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|err| request_error(&url, err))?;
        let body = response
            .body_mut()
            .read_to_vec()
            .map_err(|err| request_error(&url, err))?;
        let raw: Vec<RawRelease> =
            serde_json::from_slice(&body).map_err(|err| UpdateError::Bad {
                url: url.clone(),
                reason: err.to_string(),
            })?;
        Ok(raw
            .into_iter()
            .filter_map(RawRelease::into_release)
            .collect())
    }

    pub fn download(&self, asset: &Asset, dest: &Path) -> Result<(), UpdateError> {
        if file_is_valid(dest, asset) {
            return Ok(());
        }
        if !asset.url.starts_with(&self.download_prefix) {
            return Err(UpdateError::Bad {
                url: asset.url.clone(),
                reason: "the download address is not a release asset".into(),
            });
        }
        let partial = dest.with_extension("part");
        let result = self.fetch_to(asset, &partial);
        if result.is_err() {
            fs::remove_file(&partial).ok();
            return result;
        }
        fs::rename(&partial, dest).map_err(|source| UpdateError::Write {
            path: dest.to_owned(),
            source,
        })
    }

    fn fetch_to(&self, asset: &Asset, partial: &Path) -> Result<(), UpdateError> {
        let write_error = |source| UpdateError::Write {
            path: partial.to_owned(),
            source,
        };
        if let Some(dir) = partial.parent() {
            fs::create_dir_all(dir).map_err(write_error)?;
        }
        let mut response = self
            .agent(DOWNLOAD_TIMEOUT)
            .get(&asset.url)
            .call()
            .map_err(|err| request_error(&asset.url, err))?;
        let mut reader = response.body_mut().as_reader().take(asset.size + 1);
        let mut file = fs::File::create(partial).map_err(write_error)?;
        let mut hasher = Sha256::new();
        let mut total = 0u64;
        let mut head = Vec::new();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let read = reader
                .read(&mut buf)
                .map_err(|err| request_error(&asset.url, ureq::Error::Io(err)))?;
            if read == 0 {
                break;
            }
            if head.len() < MZ_MAGIC.len() {
                head.extend_from_slice(&buf[..read.min(MZ_MAGIC.len())]);
            }
            hasher.update(&buf[..read]);
            file.write_all(&buf[..read]).map_err(write_error)?;
            total += read as u64;
        }
        file.sync_all().map_err(write_error)?;
        verify(asset, total, &head, &hex(&hasher.finalize()))
    }
}

fn verify(asset: &Asset, size: u64, head: &[u8], sha256: &str) -> Result<(), UpdateError> {
    if size != asset.size {
        return Err(UpdateError::Verify(format!(
            "expected {} bytes, got {size}",
            asset.size
        )));
    }
    if !head.starts_with(MZ_MAGIC) {
        return Err(UpdateError::Verify("not a Windows program".into()));
    }
    match &asset.sha256 {
        Some(expected) if expected != sha256 => {
            Err(UpdateError::Verify("the checksum does not match".into()))
        }
        Some(_) => Ok(()),
        None => {
            tracing::warn!("the release has no checksum, checked the size only");
            Ok(())
        }
    }
}

fn file_is_valid(path: &Path, asset: &Asset) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut hasher = Sha256::new();
    let mut head = Vec::new();
    let mut total = 0u64;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(read) => {
                if head.len() < MZ_MAGIC.len() {
                    head.extend_from_slice(&buf[..read.min(MZ_MAGIC.len())]);
                }
                hasher.update(&buf[..read]);
                total += read as u64;
            }
            Err(_) => return false,
        }
    }
    verify(asset, total, &head, &hex(&hasher.finalize())).is_ok()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn request_error(url: &str, err: ureq::Error) -> UpdateError {
    match err {
        ureq::Error::StatusCode(403 | 429) => UpdateError::RateLimited,
        err => UpdateError::Request {
            url: url.to_owned(),
            reason: err.to_string(),
        },
    }
}

#[derive(Debug, Deserialize)]
struct RawRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Debug, Deserialize)]
struct RawAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    digest: Option<String>,
}

impl RawRelease {
    fn into_release(self) -> Option<Release> {
        if self.draft {
            return None;
        }
        let exe = self
            .assets
            .into_iter()
            .find(|asset| asset.name == EXE_ASSET && asset.size > 0)
            .map(|asset| Asset {
                url: asset.browser_download_url,
                size: asset.size,
                sha256: asset.digest.as_deref().and_then(update::parse_digest),
            });
        Some(Release {
            tag: self.tag_name,
            prerelease: self.prerelease,
            published: self.published_at,
            exe,
        })
    }
}
