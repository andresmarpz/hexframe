use std::time::Duration;

use greyscreen_core::feedback::FeedbackReport;
use serde::Deserialize;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SEND_TIMEOUT: Duration = Duration::from_secs(30);
const FEEDBACK_PATH: &str = "/v1/feedback";
const VERSION_HEADER: &str = "x-greyscreen-version";

#[derive(Debug, thiserror::Error)]
pub enum FeedbackError {
    #[error("cannot reach {url}: {reason}")]
    Request { url: String, reason: String },
    #[error("too many reports from this computer")]
    RateLimited,
    #[error("the server refused the report: {0}")]
    Rejected(String),
    #[error("the server answered {status}")]
    Status { status: u16 },
    #[error("cannot encode the report: {0}")]
    Encode(String),
}

#[derive(Debug, Clone)]
pub struct FeedbackClient {
    url: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct Accepted {
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Refusal {
    error: Option<String>,
}

impl FeedbackClient {
    pub fn new(base_url: &str, version: &str) -> Self {
        Self {
            url: format!("{}{FEEDBACK_PATH}", base_url.trim_end_matches('/')),
            version: version.to_owned(),
        }
    }

    pub fn submit(&self, report: &FeedbackReport) -> Result<Option<String>, FeedbackError> {
        let body =
            serde_json::to_vec(report).map_err(|err| FeedbackError::Encode(err.to_string()))?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(format!("greyscreen/{}", self.version))
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(SEND_TIMEOUT))
            .http_status_as_error(false)
            .max_idle_connections(0)
            .max_idle_connections_per_host(0)
            .build()
            .into();
        let mut response = agent
            .post(&self.url)
            .header("content-type", "application/json")
            .header(VERSION_HEADER, &self.version)
            .send(&body[..])
            .map_err(|err| FeedbackError::Request {
                url: self.url.clone(),
                reason: err.to_string(),
            })?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap_or_default();
        match status {
            200..=299 => Ok(serde_json::from_str::<Accepted>(&text)
                .ok()
                .and_then(|ok| ok.id)),
            429 => Err(FeedbackError::RateLimited),
            400 => Err(refusal(&text)),
            status => Err(FeedbackError::Status { status }),
        }
    }
}

fn refusal(text: &str) -> FeedbackError {
    match serde_json::from_str::<Refusal>(text)
        .ok()
        .and_then(|r| r.error)
    {
        Some(reason) if !reason.trim().is_empty() => FeedbackError::Rejected(reason),
        _ => FeedbackError::Status { status: 400 },
    }
}
