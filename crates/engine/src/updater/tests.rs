use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_channel::{Receiver, unbounded};

use super::*;

const WAIT: Duration = Duration::from_secs(5);

struct FakeSource {
    releases: Mutex<Result<Vec<Release>, UpdateErrorKind>>,
    checks: AtomicUsize,
    downloads: AtomicUsize,
    download_fails: Mutex<bool>,
}

#[derive(Clone, Copy)]
enum UpdateErrorKind {
    RateLimited,
    Offline,
}

impl FakeSource {
    fn new(releases: Vec<Release>) -> Arc<Self> {
        Arc::new(Self {
            releases: Mutex::new(Ok(releases)),
            checks: AtomicUsize::new(0),
            downloads: AtomicUsize::new(0),
            download_fails: Mutex::new(false),
        })
    }

    fn set(&self, releases: Result<Vec<Release>, UpdateErrorKind>) {
        *self.releases.lock().unwrap() = releases;
    }

    fn checks(&self) -> usize {
        self.checks.load(Ordering::SeqCst)
    }
}

impl UpdateSource for FakeSource {
    fn releases(&self) -> Result<Vec<Release>, UpdateError> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        match &*self.releases.lock().unwrap() {
            Ok(releases) => Ok(releases.clone()),
            Err(UpdateErrorKind::RateLimited) => Err(UpdateError::RateLimited),
            Err(UpdateErrorKind::Offline) => Err(UpdateError::Request {
                url: "x".into(),
                reason: "offline".into(),
            }),
        }
    }

    fn download(&self, _: &Asset, dest: &Path) -> Result<(), UpdateError> {
        self.downloads.fetch_add(1, Ordering::SeqCst);
        if *self.download_fails.lock().unwrap() {
            return Err(UpdateError::Verify("bad".into()));
        }
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(dest, b"MZ").unwrap();
        Ok(())
    }
}

fn release(tag: &str, prerelease: bool) -> Release {
    Release {
        tag: tag.into(),
        prerelease,
        published: None,
        exe: Some(Asset {
            url: format!("https://example.test/{tag}"),
            size: 2,
            sha256: None,
        }),
    }
}

struct Rig {
    handle: UpdateHandle,
    source: Arc<FakeSource>,
    msgs: Receiver<UiMsg>,
    launched: Receiver<PathBuf>,
    dir: tempfile::TempDir,
}

fn start(running: &str, can_install: bool, auto: bool, releases: Vec<Release>) -> Rig {
    let source = FakeSource::new(releases);
    let dir = tempfile::tempdir().unwrap();
    let (tx, msgs) = unbounded();
    let (launch_tx, launched) = unbounded();
    let config = UpdateConfig {
        source: source.clone(),
        running: running.into(),
        can_install,
        dir: dir.path().join("Greyscreen"),
        launch: Box::new(move |path| {
            launch_tx.send(path.to_owned()).ok();
            Ok(())
        }),
        first_check: Duration::from_millis(40),
        interval: Duration::from_millis(60),
        busy_retry: Duration::from_millis(40),
    };
    let handle = spawn(
        config,
        auto,
        Arc::new(move |msg| {
            tx.send(msg).ok();
        }),
    )
    .unwrap();
    Rig {
        handle,
        source,
        msgs,
        launched,
        dir,
    }
}

fn states(rig: &Rig, until: impl Fn(&UpdateState) -> bool) -> Vec<UpdateState> {
    let mut seen = Vec::new();
    while let Ok(msg) = rig.msgs.recv_timeout(WAIT) {
        if let UiMsg::Update(state) = msg {
            let done = until(&state);
            seen.push(state);
            if done {
                return seen;
            }
        }
    }
    panic!("never reached the state, saw {seen:?}");
}

fn quiet_for(rig: &Rig, wait: Duration) {
    assert!(rig.msgs.recv_timeout(wait).is_err());
}

#[test]
fn a_scheduled_check_downloads_a_newer_release() {
    let rig = start(
        "0.1.6-preview.8",
        true,
        true,
        vec![release("v0.1.7-preview.9", true)],
    );
    let seen = states(&rig, |s| matches!(s, UpdateState::Ready(_)));
    assert!(matches!(seen[0], UpdateState::Downloading(_)));
    let UpdateState::Ready(info) = seen.last().unwrap() else {
        panic!()
    };
    assert_eq!(info.tag, "v0.1.7-preview.9");
    assert!(
        rig.dir
            .path()
            .join("Greyscreen")
            .join("greyscreen-v0.1.7-preview.9.exe")
            .is_file()
    );
}

#[test]
fn scheduled_checks_are_silent_when_up_to_date() {
    let rig = start(
        "0.1.7-preview.9",
        true,
        true,
        vec![release("v0.1.7-preview.9", true)],
    );
    quiet_for(&rig, Duration::from_millis(300));
    assert!(rig.source.checks() >= 2);
}

#[test]
fn scheduled_checks_repeat_on_the_interval() {
    let rig = start("0.1.7-preview.9", true, true, vec![]);
    std::thread::sleep(Duration::from_millis(400));
    assert!(rig.source.checks() >= 3, "{}", rig.source.checks());
}

#[test]
fn nothing_is_checked_while_busy_and_the_check_runs_after() {
    let rig = start("0.1.6", true, true, vec![]);
    rig.handle.send(UpdateCmd::Busy(true));
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(rig.source.checks(), 0);
    rig.handle.send(UpdateCmd::Busy(false));
    std::thread::sleep(Duration::from_millis(300));
    assert!(rig.source.checks() >= 1);
}

#[test]
fn no_scheduled_checks_when_the_setting_is_off_or_the_copy_is_not_installed() {
    let off = start("0.1.6", true, false, vec![]);
    let loose = start("0.1.6", false, true, vec![]);
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(off.source.checks(), 0);
    assert_eq!(loose.source.checks(), 0);
}

#[test]
fn turning_the_setting_on_and_off_starts_and_stops_checks() {
    let rig = start("0.1.6", true, false, vec![]);
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(rig.source.checks(), 0);
    rig.handle.send(UpdateCmd::Auto(true));
    std::thread::sleep(Duration::from_millis(250));
    assert!(rig.source.checks() >= 1);
    rig.handle.send(UpdateCmd::Auto(false));
    std::thread::sleep(Duration::from_millis(100));
    let after_off = rig.source.checks();
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(rig.source.checks(), after_off);
}

#[test]
fn a_manual_check_reports_checking_then_up_to_date() {
    let rig = start("0.1.7", true, false, vec![release("v0.1.6", false)]);
    rig.handle.send(UpdateCmd::Check);
    let seen = states(&rig, |s| matches!(s, UpdateState::UpToDate));
    assert_eq!(seen[0], UpdateState::Checking);
}

#[test]
fn a_manual_check_runs_even_with_the_setting_off() {
    let rig = start("0.1.5", true, false, vec![release("v0.2.0", false)]);
    rig.handle.send(UpdateCmd::Check);
    states(&rig, |s| matches!(s, UpdateState::Ready(_)));
}

#[test]
fn a_loose_copy_only_reports_the_release() {
    let rig = start("0.1.5", false, false, vec![release("v0.2.0", false)]);
    rig.handle.send(UpdateCmd::Check);
    let seen = states(&rig, |s| matches!(s, UpdateState::AvailableManual(_)));
    assert_eq!(seen.len(), 2);
    assert_eq!(rig.source.downloads.load(Ordering::SeqCst), 0);
    rig.handle.send(UpdateCmd::Install);
    quiet_for(&rig, Duration::from_millis(150));
    assert!(rig.launched.try_recv().is_err());
}

#[test]
fn a_stable_build_does_not_offer_previews() {
    let rig = start(
        "0.2.0",
        true,
        false,
        vec![release("v0.3.0-preview.30", true)],
    );
    rig.handle.send(UpdateCmd::Check);
    states(&rig, |s| matches!(s, UpdateState::UpToDate));
}

#[test]
fn rate_limits_stay_quiet_on_schedule_and_speak_when_asked() {
    let rig = start("0.1.6", true, true, vec![]);
    rig.source.set(Err(UpdateErrorKind::RateLimited));
    quiet_for(&rig, Duration::from_millis(300));
    rig.handle.send(UpdateCmd::Check);
    let seen = states(&rig, |s| matches!(s, UpdateState::Failed(_)));
    let UpdateState::Failed(text) = seen.last().unwrap() else {
        panic!()
    };
    assert!(text.contains("limiting"));
}

#[test]
fn offline_gives_a_plain_failure_on_a_manual_check() {
    let rig = start("0.1.6", true, false, vec![]);
    rig.source.set(Err(UpdateErrorKind::Offline));
    rig.handle.send(UpdateCmd::Check);
    states(&rig, |s| matches!(s, UpdateState::Failed(_)));
}

#[test]
fn a_busy_session_defers_the_download_until_idle() {
    let rig = start("0.1.6", true, false, vec![release("v0.2.0", false)]);
    rig.handle.send(UpdateCmd::Busy(true));
    rig.handle.send(UpdateCmd::Check);
    let seen = states(&rig, |s| matches!(s, UpdateState::Available(_)));
    assert_eq!(seen[0], UpdateState::Checking);
    quiet_for(&rig, Duration::from_millis(150));
    assert_eq!(rig.source.downloads.load(Ordering::SeqCst), 0);
    rig.handle.send(UpdateCmd::Busy(false));
    states(&rig, |s| matches!(s, UpdateState::Ready(_)));
}

#[test]
fn a_failed_download_reports_and_a_later_check_retries() {
    let rig = start("0.1.6", true, false, vec![release("v0.2.0", false)]);
    *rig.source.download_fails.lock().unwrap() = true;
    rig.handle.send(UpdateCmd::Check);
    let seen = states(&rig, |s| matches!(s, UpdateState::Failed(_)));
    let UpdateState::Failed(text) = seen.last().unwrap() else {
        panic!()
    };
    assert!(text.contains("failed its check"));
    *rig.source.download_fails.lock().unwrap() = false;
    rig.handle.send(UpdateCmd::Check);
    states(&rig, |s| matches!(s, UpdateState::Ready(_)));
}

#[test]
fn install_launches_the_downloaded_exe() {
    let rig = start("0.1.6", true, false, vec![release("v0.2.0", false)]);
    rig.handle.send(UpdateCmd::Check);
    states(&rig, |s| matches!(s, UpdateState::Ready(_)));
    rig.handle.send(UpdateCmd::Install);
    states(&rig, |s| matches!(s, UpdateState::Installing(_)));
    let path = rig.launched.recv_timeout(WAIT).unwrap();
    assert_eq!(path.file_name().unwrap(), "greyscreen-v0.2.0.exe");
}

#[test]
fn install_waits_while_busy() {
    let rig = start("0.1.6", true, false, vec![release("v0.2.0", false)]);
    rig.handle.send(UpdateCmd::Check);
    states(&rig, |s| matches!(s, UpdateState::Ready(_)));
    rig.handle.send(UpdateCmd::Busy(true));
    rig.handle.send(UpdateCmd::Install);
    states(&rig, |s| matches!(s, UpdateState::Ready(_)));
    assert!(rig.launched.try_recv().is_err());
}

#[test]
fn install_without_a_download_does_nothing() {
    let rig = start("0.1.6", true, false, vec![]);
    rig.handle.send(UpdateCmd::Install);
    quiet_for(&rig, Duration::from_millis(150));
    assert!(rig.launched.try_recv().is_err());
}

#[test]
fn old_downloads_are_removed_at_startup() {
    let dir = tempfile::tempdir().unwrap();
    let downloads = dir.path().join("Greyscreen");
    fs::create_dir_all(&downloads).unwrap();
    for name in [
        "greyscreen-v0.1.5-preview.7.exe",
        "greyscreen-v0.1.6-preview.8.exe",
        "greyscreen-v0.1.7-preview.9.exe",
        "other.txt",
    ] {
        fs::write(downloads.join(name), b"x").unwrap();
    }
    let source = FakeSource::new(vec![]);
    let config = UpdateConfig {
        source,
        running: "0.1.6-preview.8".into(),
        can_install: true,
        dir: downloads.clone(),
        launch: Box::new(|_| Ok(())),
        first_check: Duration::from_secs(60),
        interval: Duration::from_secs(60),
        busy_retry: Duration::from_secs(60),
    };
    let _handle = spawn(config, false, Arc::new(|_| {})).unwrap();
    let deadline = Instant::now() + WAIT;
    while downloads.join("greyscreen-v0.1.5-preview.7.exe").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!downloads.join("greyscreen-v0.1.5-preview.7.exe").exists());
    assert!(!downloads.join("greyscreen-v0.1.6-preview.8.exe").exists());
    assert!(downloads.join("greyscreen-v0.1.7-preview.9.exe").exists());
    assert!(downloads.join("other.txt").exists());
}
