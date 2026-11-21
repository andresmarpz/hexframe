use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use greyscreen_core::update::{
    Asset, Release, UpdateInfo, UpdateState, Version, download_file_name, is_stale_download,
    newest_update,
};
use greyscreen_league::{GithubReleases, UpdateError};

use crate::api::UiMsg;

const FIRST_CHECK: Duration = Duration::from_secs(10);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const BUSY_RETRY: Duration = Duration::from_secs(60);
const INSTALL_GRACE: Duration = Duration::from_secs(30);
const DOWNLOAD_DIR: &str = "Greyscreen";

pub trait UpdateSource: Send + Sync {
    fn releases(&self) -> Result<Vec<Release>, UpdateError>;
    fn download(&self, asset: &Asset, dest: &Path) -> Result<(), UpdateError>;
}

impl UpdateSource for GithubReleases {
    fn releases(&self) -> Result<Vec<Release>, UpdateError> {
        GithubReleases::releases(self)
    }

    fn download(&self, asset: &Asset, dest: &Path) -> Result<(), UpdateError> {
        GithubReleases::download(self, asset, dest)
    }
}

pub type Launcher = Box<dyn Fn(&Path) -> io::Result<()> + Send>;

pub struct UpdateConfig {
    pub source: Arc<dyn UpdateSource>,
    pub running: String,
    pub can_install: bool,
    pub dir: PathBuf,
    pub launch: Launcher,
    pub first_check: Duration,
    pub interval: Duration,
    pub busy_retry: Duration,
}

impl std::fmt::Debug for UpdateConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateConfig")
            .field("running", &self.running)
            .field("can_install", &self.can_install)
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

impl UpdateConfig {
    pub fn github(running: &str, can_install: bool) -> Self {
        Self {
            source: Arc::new(GithubReleases::new(running)),
            running: running.to_owned(),
            can_install,
            dir: std::env::temp_dir().join(DOWNLOAD_DIR),
            launch: Box::new(|path| std::process::Command::new(path).spawn().map(drop)),
            first_check: FIRST_CHECK,
            interval: CHECK_INTERVAL,
            busy_retry: BUSY_RETRY,
        }
    }
}

#[derive(Debug)]
pub(crate) enum UpdateCmd {
    Check,
    Install,
    Auto(bool),
    Busy(bool),
}

#[derive(Debug)]
pub(crate) struct UpdateHandle {
    tx: Sender<UpdateCmd>,
}

impl UpdateHandle {
    pub(crate) fn send(&self, cmd: UpdateCmd) {
        if self.tx.send(cmd).is_err() {
            tracing::debug!("updater already stopped");
        }
    }
}

pub(crate) fn spawn(
    config: UpdateConfig,
    auto: bool,
    sink: Arc<dyn Fn(UiMsg) + Send + Sync>,
) -> Option<UpdateHandle> {
    let (tx, rx) = unbounded();
    let worker = Worker::new(config, auto, sink);
    let spawned = thread::Builder::new()
        .name("engine-updates".into())
        .spawn(move || worker.run(&rx));
    match spawned {
        Ok(_) => Some(UpdateHandle { tx }),
        Err(err) => {
            tracing::warn!(%err, "cannot start the update thread");
            None
        }
    }
}

struct Candidate {
    info: UpdateInfo,
    asset: Asset,
    path: PathBuf,
}

struct Worker {
    config: UpdateConfig,
    running: Option<Version>,
    sink: Arc<dyn Fn(UiMsg) + Send + Sync>,
    auto: bool,
    busy: bool,
    next_check: Option<Instant>,
    pending: Option<Candidate>,
    ready: Option<Candidate>,
    installing_until: Option<Instant>,
    rate_limit_logged: bool,
}

impl Worker {
    fn new(config: UpdateConfig, auto: bool, sink: Arc<dyn Fn(UiMsg) + Send + Sync>) -> Self {
        let running = Version::parse(&config.running);
        Self {
            running,
            sink,
            auto,
            busy: false,
            next_check: None,
            pending: None,
            ready: None,
            installing_until: None,
            rate_limit_logged: false,
            config,
        }
    }

    fn scheduled(&self) -> bool {
        self.auto && self.config.can_install
    }

    fn run(mut self, rx: &Receiver<UpdateCmd>) {
        if self.config.can_install {
            self.remove_stale_downloads();
        }
        self.reschedule(Instant::now() + self.config.first_check);
        loop {
            let wake = [self.next_check, self.installing_until]
                .into_iter()
                .flatten()
                .min();
            let received = match wake {
                Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match received {
                Ok(cmd) => self.on_cmd(cmd),
                Err(RecvTimeoutError::Timeout) => self.on_wake(),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn reschedule(&mut self, at: Instant) {
        self.next_check = self.scheduled().then_some(at);
    }

    fn on_cmd(&mut self, cmd: UpdateCmd) {
        match cmd {
            UpdateCmd::Check => self.check(true),
            UpdateCmd::Install => self.install(),
            UpdateCmd::Auto(auto) => {
                self.auto = auto;
                match (self.scheduled(), self.next_check) {
                    (true, None) => self.reschedule(Instant::now() + self.config.first_check),
                    (false, _) => self.next_check = None,
                    _ => {}
                }
            }
            UpdateCmd::Busy(busy) => {
                self.busy = busy;
                if !busy {
                    self.download_pending();
                }
            }
        }
    }

    fn on_wake(&mut self) {
        let now = Instant::now();
        if self.installing_until.is_some_and(|until| now >= until) {
            self.installing_until = None;
            if let Some(ready) = &self.ready {
                self.send(UpdateState::Ready(ready.info.clone()));
            }
        }
        if self.next_check.is_some_and(|at| now >= at) {
            if self.busy {
                self.reschedule(now + self.config.busy_retry);
            } else {
                self.reschedule(now + self.config.interval);
                self.check(false);
            }
        }
    }

    fn send(&self, state: UpdateState) {
        (self.sink)(UiMsg::Update(state));
    }

    fn check(&mut self, manual: bool) {
        let Some(running) = self.running.clone() else {
            tracing::warn!(version = %self.config.running, "the running version has no valid number");
            if manual {
                self.send(UpdateState::Failed(
                    "This build cannot check for updates.".into(),
                ));
            }
            return;
        };
        if manual {
            self.send(UpdateState::Checking);
        }
        let releases = match self.config.source.releases() {
            Ok(releases) => {
                self.rate_limit_logged = false;
                releases
            }
            Err(err) => return self.check_failed(&err, manual),
        };
        let Some(release) = newest_update(&running, &releases) else {
            if manual {
                self.send(UpdateState::UpToDate);
            }
            return;
        };
        let Some(asset) = release.exe.clone() else {
            return;
        };
        let info = UpdateInfo::from_release(release);
        if !self.config.can_install {
            self.send(UpdateState::AvailableManual(info));
            return;
        }
        if let Some(ready) = &self.ready
            && ready.info.tag == info.tag
        {
            self.send(UpdateState::Ready(info));
            return;
        }
        let path = self.config.dir.join(download_file_name(&info.tag));
        self.pending = Some(Candidate { info, asset, path });
        if self.busy {
            if let Some(pending) = &self.pending {
                self.send(UpdateState::Available(pending.info.clone()));
            }
        } else {
            self.download_pending();
        }
    }

    fn check_failed(&mut self, err: &UpdateError, manual: bool) {
        match err {
            UpdateError::RateLimited => {
                if !self.rate_limit_logged {
                    tracing::warn!("update check refused: GitHub is limiting requests");
                    self.rate_limit_logged = true;
                }
            }
            err => tracing::warn!(%err, "update check failed"),
        }
        if manual {
            self.send(UpdateState::Failed(match err {
                UpdateError::RateLimited => {
                    "GitHub is limiting checks right now. Try again later.".into()
                }
                _ => "Couldn't check for updates. Try again later.".into(),
            }));
        }
    }

    fn download_pending(&mut self) {
        if self.busy {
            return;
        }
        let Some(candidate) = self.pending.take() else {
            return;
        };
        self.send(UpdateState::Downloading(candidate.info.clone()));
        match self
            .config
            .source
            .download(&candidate.asset, &candidate.path)
        {
            Ok(()) => {
                tracing::info!(tag = %candidate.info.tag, "update downloaded and checked");
                self.send(UpdateState::Ready(candidate.info.clone()));
                self.ready = Some(candidate);
            }
            Err(err) => {
                tracing::warn!(%err, tag = %candidate.info.tag, "update download failed");
                self.send(UpdateState::Failed(match err {
                    UpdateError::Verify(_) => {
                        "The downloaded update failed its check and was discarded.".into()
                    }
                    _ => "Couldn't download the update. Try again later.".into(),
                }));
                self.pending = Some(candidate);
            }
        }
    }

    fn install(&mut self) {
        let Some(ready) = &self.ready else {
            return;
        };
        if !self.config.can_install || self.busy {
            self.send(UpdateState::Ready(ready.info.clone()));
            return;
        }
        match (self.config.launch)(&ready.path) {
            Ok(()) => {
                tracing::info!(tag = %ready.info.tag, "launched the downloaded update");
                self.send(UpdateState::Installing(ready.info.clone()));
                self.installing_until = Some(Instant::now() + INSTALL_GRACE);
            }
            Err(err) => {
                tracing::warn!(%err, "cannot launch the downloaded update");
                self.send(UpdateState::Failed("Couldn't start the update.".into()));
            }
        }
    }

    fn remove_stale_downloads(&self) {
        let (Some(running), Ok(entries)) = (&self.running, fs::read_dir(&self.config.dir)) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if is_stale_download(&name.to_string_lossy(), running) {
                fs::remove_file(entry.path()).ok();
            }
        }
    }
}

#[cfg(test)]
mod tests;
