use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use greyscreen_core::feedback::{
    FeedbackDraft, FeedbackReport, FeedbackResult, ReportContext, build_report, home_variants,
    read_newest_log,
};
use greyscreen_core::paths;
use greyscreen_league::{FeedbackClient, FeedbackError};

use crate::api::UiMsg;

pub trait FeedbackTransport: Send + Sync {
    fn send(&self, report: &FeedbackReport) -> Result<(), FeedbackError>;
}

impl FeedbackTransport for FeedbackClient {
    fn send(&self, report: &FeedbackReport) -> Result<(), FeedbackError> {
        self.submit(report).map(drop)
    }
}

#[derive(Clone)]
pub struct FeedbackConfig {
    pub transport: Arc<dyn FeedbackTransport>,
    pub app_version: String,
    pub os: Option<String>,
    pub logs_dir: PathBuf,
    pub home_dirs: Vec<String>,
}

impl std::fmt::Debug for FeedbackConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeedbackConfig")
            .field("app_version", &self.app_version)
            .field("logs_dir", &self.logs_dir)
            .finish_non_exhaustive()
    }
}

impl FeedbackConfig {
    pub fn api(app_version: &str) -> Self {
        let client = FeedbackClient::new(&greyscreen_core::api::base_url(), app_version);
        Self {
            transport: Arc::new(client),
            app_version: app_version.to_owned(),
            os: None,
            logs_dir: paths::logs_dir(),
            home_dirs: paths::home_dir()
                .map(|home| home_variants(&home))
                .unwrap_or_default(),
        }
    }
}

pub(crate) fn spawn_send(
    config: Option<FeedbackConfig>,
    draft: FeedbackDraft,
    sink: Arc<dyn Fn(UiMsg) + Send + Sync>,
) {
    let worker_sink = sink.clone();
    let spawned = thread::Builder::new()
        .name("engine-feedback".into())
        .spawn(move || worker_sink(UiMsg::Feedback(deliver(config.as_ref(), &draft))));
    if let Err(err) = spawned {
        tracing::warn!(%err, "cannot start the feedback thread");
        sink(UiMsg::Feedback(FeedbackResult::Failed {
            error: "Couldn't start sending. Try again.".into(),
            report: String::new(),
        }));
    }
}

fn deliver(config: Option<&FeedbackConfig>, draft: &FeedbackDraft) -> FeedbackResult {
    let Some(config) = config else {
        return FeedbackResult::Failed {
            error: "Feedback is not available in this build.".into(),
            report: String::new(),
        };
    };
    let os = config.os.clone().unwrap_or_else(system_os);
    let report = build_report(
        draft,
        ReportContext {
            app_version: &config.app_version,
            os: &os,
            home_dirs: &config.home_dirs,
        },
        || read_newest_log(&config.logs_dir),
    );
    match config.transport.send(&report) {
        Ok(()) => {
            tracing::info!("feedback sent");
            FeedbackResult::Sent
        }
        Err(err) => {
            tracing::warn!(%err, "feedback was not sent");
            FeedbackResult::Failed {
                error: describe(&err),
                report: report.to_json(),
            }
        }
    }
}

fn describe(err: &FeedbackError) -> String {
    match err {
        FeedbackError::Request { .. } => {
            "Couldn't reach Greyscreen. Check your connection and try again.".into()
        }
        FeedbackError::RateLimited => {
            "You've sent a lot of feedback already. Try again in a little while.".into()
        }
        FeedbackError::Rejected(reason) => format!("Greyscreen couldn't take that: {reason}"),
        FeedbackError::Status { status } => {
            format!("Greyscreen's server had a problem (error {status}). Try again later.")
        }
        FeedbackError::Encode(_) => "Couldn't prepare the report.".into(),
    }
}

#[cfg(target_os = "macos")]
fn system_os() -> String {
    command_output("sw_vers", &["-productVersion"])
        .map_or_else(|| "macOS".into(), |version| format!("macOS {version}"))
}

#[cfg(windows)]
fn system_os() -> String {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned());
    output
        .as_deref()
        .and_then(windows_build)
        .map_or_else(|| "Windows".into(), |build| format!("Windows {build}"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn system_os() -> String {
    std::env::consts::OS.to_owned()
}

#[cfg(target_os = "macos")]
fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

#[cfg(any(windows, test))]
fn windows_build(ver_output: &str) -> Option<String> {
    let inside = ver_output.split_once('[')?.1.split_once(']')?.0;
    let build = inside.split_whitespace().next_back()?;
    Some(build.to_owned())
}

#[cfg(test)]
mod tests;
