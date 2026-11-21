use std::io::ErrorKind;
use std::time::Duration;

use serde::de::DeserializeOwned;
use ureq::Error as UreqError;
use ureq::tls::TlsConfig;

use crate::dto::{EventData, GameStats, PlayerEntry};

const DEFAULT_BASE: &str = "https://127.0.0.1:2999";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const GLOBAL_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("connection refused")]
    Refused,
    #[error("api not ready (status {0})")]
    NotReady(u16),
    #[error("request timed out")]
    Timeout,
    #[error("bad response: {0}")]
    Bad(String),
}

#[derive(Debug, Clone)]
pub struct LiveClient {
    agent: ureq::Agent,
    base: String,
}

impl Default for LiveClient {
    fn default() -> Self {
        Self::new(DEFAULT_BASE)
    }
}

impl LiveClient {
    pub fn new(base: &str) -> Self {
        Self::with_timeouts(base, CONNECT_TIMEOUT, GLOBAL_TIMEOUT)
    }

    pub fn with_timeouts(base: &str, connect: Duration, global: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .tls_config(TlsConfig::builder().disable_verification(true).build())
            .timeout_connect(Some(connect))
            .timeout_global(Some(global))
            .http_status_as_error(false)
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .build();
        Self {
            agent: config.into(),
            base: base.trim_end_matches('/').to_owned(),
        }
    }

    pub fn gamestats(&self) -> Result<GameStats, ClientError> {
        self.get_json("/liveclientdata/gamestats")
    }

    pub fn eventdata(&self) -> Result<EventData, ClientError> {
        self.get_json("/liveclientdata/eventdata")
    }

    pub fn activeplayername(&self) -> Result<String, ClientError> {
        self.get_json("/liveclientdata/activeplayername")
    }

    pub fn playerlist(&self) -> Result<Vec<PlayerEntry>, ClientError> {
        self.get_json("/liveclientdata/playerlist")
    }

    pub fn replay_playback_ok(&self) -> bool {
        self.get("/replay/playback").is_ok()
    }

    fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let body = self.get(path)?;
        serde_json::from_str(&body).map_err(|e| ClientError::Bad(e.to_string()))
    }

    fn get(&self, path: &str) -> Result<String, ClientError> {
        let url = format!("{}{}", self.base, path);
        let mut response = self.agent.get(&url).call().map_err(map_error)?;
        let status = response.status().as_u16();
        if status != 200 {
            return Err(ClientError::NotReady(status));
        }
        response.body_mut().read_to_string().map_err(map_error)
    }
}

fn map_error(error: UreqError) -> ClientError {
    match error {
        UreqError::ConnectionFailed => ClientError::Refused,
        UreqError::Io(e) if e.kind() == ErrorKind::ConnectionRefused => ClientError::Refused,
        UreqError::Timeout(_) => ClientError::Timeout,
        UreqError::Io(e) if e.kind() == ErrorKind::TimedOut => ClientError::Timeout,
        other => ClientError::Bad(other.to_string()),
    }
}
