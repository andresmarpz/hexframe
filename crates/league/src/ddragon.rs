use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE: &str = "https://ddragon.leagueoflegends.com";
const INDEX_FILE: &str = "index.json";
const RECHECK_SECS: u64 = 24 * 60 * 60;
const WORKERS: usize = 4;
const MAX_FAILURES: usize = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const GLOBAL_TIMEOUT: Duration = Duration::from_secs(20);
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

#[derive(Debug, thiserror::Error)]
pub enum DdragonError {
    #[error("request to {url} failed: {reason}")]
    Request { url: String, reason: String },
    #[error("bad response from {url}: {reason}")]
    Bad { url: String, reason: String },
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampionIndex {
    pub version: String,
    pub checked_at: u64,
    pub names: BTreeMap<String, String>,
}

impl ChampionIndex {
    pub fn load(dir: &Path) -> Option<Self> {
        let raw = fs::read_to_string(dir.join(INDEX_FILE)).ok()?;
        serde_json::from_str(&raw).ok()
    }

    pub fn id_for(&self, champion: &str) -> Option<&str> {
        self.names.get(&normalize(champion)).map(String::as_str)
    }

    fn ids(&self) -> BTreeSet<&str> {
        self.names.values().map(String::as_str).collect()
    }

    fn save(&self, dir: &Path) -> Result<(), DdragonError> {
        let body = serde_json::to_vec(self).unwrap_or_default();
        write_atomic(&dir.join(INDEX_FILE), &body)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChampionIcons {
    dir: PathBuf,
    index: ChampionIndex,
}

impl ChampionIcons {
    pub fn open(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
            index: ChampionIndex::load(dir).unwrap_or_default(),
        }
    }

    pub fn icon_path(&self, champion: &str) -> Option<PathBuf> {
        let id = self.index.id_for(champion)?;
        let path = self.dir.join(format!("{id}.png"));
        path.is_file().then_some(path)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub version: String,
    pub checked_versions: bool,
    pub downloaded: usize,
    pub failed: usize,
}

pub fn sync(base: &str, dir: &Path, now_unix: u64) -> Result<SyncReport, DdragonError> {
    let client = Client::new(base);
    fs::create_dir_all(dir).map_err(|source| DdragonError::Write {
        path: dir.to_owned(),
        source,
    })?;
    let mut index = ChampionIndex::load(dir).filter(|index| !index.names.is_empty());
    let fresh = index
        .as_ref()
        .is_some_and(|index| now_unix.saturating_sub(index.checked_at) < RECHECK_SECS);
    let mut checked_versions = false;
    if !fresh {
        let latest = client.latest_version()?;
        checked_versions = true;
        index = Some(match index {
            Some(known) if known.version == latest => ChampionIndex {
                checked_at: now_unix,
                ..known
            },
            _ => ChampionIndex {
                names: client.champion_names(&latest)?,
                version: latest,
                checked_at: now_unix,
            },
        });
        if let Some(index) = &index {
            index.save(dir)?;
        }
    }
    let Some(index) = index else {
        return Ok(SyncReport::default());
    };
    let missing: VecDeque<String> = index
        .ids()
        .into_iter()
        .filter(|id| !dir.join(format!("{id}.png")).is_file())
        .map(str::to_owned)
        .collect();
    let (downloaded, failed) = client.download_icons(dir, &index.version, missing);
    Ok(SyncReport {
        version: index.version,
        checked_versions,
        downloaded,
        failed,
    })
}

struct Client {
    agent: ureq::Agent,
    base: String,
}

impl Client {
    fn new(base: &str) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(GLOBAL_TIMEOUT))
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .build();
        Self {
            agent: config.into(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    fn get(&self, path: &str) -> Result<Vec<u8>, DdragonError> {
        let url = format!("{}{path}", self.base);
        let mut response = self
            .agent
            .get(&url)
            .call()
            .map_err(|err| DdragonError::Request {
                url: url.clone(),
                reason: err.to_string(),
            })?;
        response
            .body_mut()
            .read_to_vec()
            .map_err(|err| DdragonError::Request {
                url,
                reason: err.to_string(),
            })
    }

    fn latest_version(&self) -> Result<String, DdragonError> {
        let path = "/api/versions.json";
        let versions: Vec<String> = parse(path, &self.get(path)?)?;
        versions
            .into_iter()
            .next()
            .filter(|version| {
                !version.is_empty()
                    && version
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == '_')
            })
            .ok_or_else(|| DdragonError::Bad {
                url: format!("{}{path}", self.base),
                reason: "no usable version".into(),
            })
    }

    fn champion_names(&self, version: &str) -> Result<BTreeMap<String, String>, DdragonError> {
        #[derive(Deserialize)]
        struct Champions {
            data: BTreeMap<String, Champion>,
        }
        #[derive(Deserialize)]
        struct Champion {
            id: String,
            name: String,
        }
        let path = format!("/cdn/{version}/data/en_US/champion.json");
        let champions: Champions = parse(&path, &self.get(&path)?)?;
        let mut names = BTreeMap::new();
        for champion in champions.data.into_values() {
            if champion.id.is_empty() || !champion.id.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }
            names.insert(normalize(&champion.name), champion.id.clone());
            names.insert(normalize(&champion.id), champion.id);
        }
        if names.is_empty() {
            return Err(DdragonError::Bad {
                url: format!("{}{path}", self.base),
                reason: "no champions".into(),
            });
        }
        Ok(names)
    }

    fn download_icons(&self, dir: &Path, version: &str, ids: VecDeque<String>) -> (usize, usize) {
        let queue = Mutex::new(ids);
        let downloaded = AtomicUsize::new(0);
        let failed = AtomicUsize::new(0);
        thread::scope(|scope| {
            for _ in 0..WORKERS {
                scope.spawn(|| {
                    while failed.load(Ordering::Relaxed) < MAX_FAILURES {
                        let next = queue
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .pop_front();
                        let Some(id) = next else { break };
                        match self.download_icon(dir, version, &id) {
                            Ok(()) => downloaded.fetch_add(1, Ordering::Relaxed),
                            Err(err) => {
                                tracing::warn!(%err, "cannot download champion icon");
                                failed.fetch_add(1, Ordering::Relaxed)
                            }
                        };
                    }
                });
            }
        });
        (
            downloaded.load(Ordering::Relaxed),
            failed.load(Ordering::Relaxed),
        )
    }

    fn download_icon(&self, dir: &Path, version: &str, id: &str) -> Result<(), DdragonError> {
        let path = format!("/cdn/{version}/img/champion/{id}.png");
        let bytes = self.get(&path)?;
        if !bytes.starts_with(PNG_MAGIC) {
            return Err(DdragonError::Bad {
                url: format!("{}{path}", self.base),
                reason: "not a PNG".into(),
            });
        }
        write_atomic(&dir.join(format!("{id}.png")), &bytes)
    }
}

fn parse<T: serde::de::DeserializeOwned>(path: &str, body: &[u8]) -> Result<T, DdragonError> {
    serde_json::from_slice(body).map_err(|err| DdragonError::Bad {
        url: path.to_owned(),
        reason: err.to_string(),
    })
}

fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), DdragonError> {
    let partial = path.with_extension("part");
    fs::write(&partial, bytes)
        .and_then(|()| fs::rename(&partial, path))
        .map_err(|source| DdragonError::Write {
            path: path.to_owned(),
            source,
        })
}
