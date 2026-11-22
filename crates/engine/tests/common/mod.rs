#![allow(dead_code, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use greyscreen_core::game::{GameEvent, GameInfo, GameResult, Player, Team};
use greyscreen_core::library::{self, GameRecord, META_VERSION, RecordingMeta, VideoInfo};
use greyscreen_core::media::{
    AudioFormat, CaptureConfig, CaptureTarget, MediaEvent, MediaSource, Packet, Track, VideoFormat,
};
use greyscreen_core::settings::Settings;
use greyscreen_core::time::Hns;
use greyscreen_engine::{
    Engine, EngineCmd, EngineConfig, EngineHandle, EngineStatus, EngineTiming, FakeMediaSource,
    FreeSpace, LeagueFactory, RecordingEntry, UiMsg,
};
use greyscreen_league::LeagueMsg;
use greyscreen_mp4::fragmented::FragmentedWriter;
use jiff::Zoned;

pub(crate) const WAIT: Duration = Duration::from_secs(20);

pub(crate) struct Harness {
    pub engine: Option<EngineHandle>,
    pub ui: Receiver<UiMsg>,
    pub dir: PathBuf,
    pub settings_path: PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.engine.take();
        std::fs::remove_dir_all(&self.dir).ok();
        std::fs::remove_file(&self.settings_path).ok();
    }
}

pub(crate) fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!("greyscreen-e2-{tag}-{}-{nanos}", std::process::id()))
}

pub(crate) fn player(name: &str, champion: &str, team: Team) -> Player {
    Player {
        name: name.to_owned(),
        champion: champion.to_owned(),
        team,
        is_bot: false,
    }
}

pub(crate) fn info() -> GameInfo {
    GameInfo {
        mode: "CLASSIC".to_owned(),
        map: Some(11),
        me: player("Hero", "Ahri", Team::Order),
        players: vec![
            player("Hero", "Ahri", Team::Order),
            player("Villain", "Zed", Team::Chaos),
        ],
    }
}

pub(crate) fn kill(killer: &str, victim: &str) -> GameEvent {
    GameEvent::ChampionKill {
        killer: killer.to_owned(),
        victim: victim.to_owned(),
        assisters: Vec::new(),
    }
}

pub(crate) fn no_league() -> LeagueFactory {
    Box::new(|_out: Sender<LeagueMsg>| Box::new(()))
}

pub(crate) type Tap = Box<dyn Fn(&UiMsg) + Send + Sync>;

pub(crate) fn start_engine(
    settings: Settings,
    media: Arc<dyn MediaSource>,
    league: LeagueFactory,
    tap: Option<Tap>,
) -> Harness {
    start_engine_with_space(settings, media, league, tap, None)
}

pub(crate) fn start_engine_with_space(
    settings: Settings,
    media: Arc<dyn MediaSource>,
    league: LeagueFactory,
    tap: Option<Tap>,
    free_space: Option<Arc<dyn FreeSpace>>,
) -> Harness {
    start_engine_full(settings, media, league, tap, free_space, None)
}

pub(crate) fn start_engine_timed(
    settings: Settings,
    media: Arc<dyn MediaSource>,
    league: LeagueFactory,
    timing: EngineTiming,
) -> Harness {
    start_engine_full(settings, media, league, None, None, Some(timing))
}

fn start_engine_full(
    settings: Settings,
    media: Arc<dyn MediaSource>,
    league: LeagueFactory,
    tap: Option<Tap>,
    free_space: Option<Arc<dyn FreeSpace>>,
    timing: Option<EngineTiming>,
) -> Harness {
    let dir = settings.output_dir.clone();
    let settings_path = dir.with_extension("settings.json");
    let (tx, rx) = unbounded();
    let sink = Box::new(move |msg: UiMsg| {
        if let Some(tap) = &tap {
            tap(&msg);
        }
        tx.send(msg).ok();
    });
    let engine = Engine::start_with(EngineConfig {
        settings,
        media,
        league,
        sink,
        settings_path: Some(settings_path.clone()),
        updates: None,
        feedback: None,
        hotkeys: None,
        free_space,
        timing,
    });
    Harness {
        engine: Some(engine),
        ui: rx,
        dir,
        settings_path,
    }
}

pub(crate) fn start_idle(settings: Settings, media: Arc<dyn MediaSource>) -> Harness {
    start_engine(settings, media, no_league(), None)
}

pub(crate) fn wait_for<T>(
    ui: &Receiver<UiMsg>,
    mut pick: impl FnMut(&UiMsg) -> Option<T>,
) -> Option<T> {
    while let Ok(msg) = ui.recv_timeout(WAIT) {
        if let Some(found) = pick(&msg) {
            return Some(found);
        }
    }
    None
}

pub(crate) fn recording_since(ui: &Receiver<UiMsg>) -> Hns {
    wait_for(ui, |msg| match msg {
        UiMsg::Status(EngineStatus::Recording { since, .. }) => Some(*since),
        _ => None,
    })
    .expect("recording status")
}

pub(crate) fn ffprobe(path: &Path, args: &[&str]) -> Option<String> {
    let Ok(output) = Command::new("ffprobe")
        .args(["-v", "error"])
        .args(args)
        .arg(path)
        .output()
    else {
        eprintln!("ffprobe not found, skipping");
        return None;
    };
    assert!(
        output.status.success(),
        "ffprobe failed on {}: {output:?}",
        path.display()
    );
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn assert_playable_from_key_frame(path: &Path) {
    if let Some(streams) = ffprobe(path, &["-show_entries", "stream=codec_name"]) {
        assert!(streams.contains("h264"), "no h264 stream: {streams}");
        assert!(streams.contains("aac"), "no aac stream: {streams}");
    }
    let first = ffprobe(
        path,
        &[
            "-select_streams",
            "v",
            "-show_frames",
            "-read_intervals",
            "%+#1",
            "-show_entries",
            "frame=key_frame",
            "-of",
            "csv=p=0",
        ],
    );
    if let Some(first) = first {
        assert_eq!(first.trim(), "1", "first frame is not a key frame: {first}");
    }
}

pub(crate) struct Media {
    pub video: VideoFormat,
    pub audio: AudioFormat,
    pub packets: Vec<Packet>,
}

pub(crate) fn capture_config() -> CaptureConfig {
    Settings::default().capture_config(1080)
}

pub(crate) fn media(video_frames: usize) -> Media {
    let (tx, rx) = bounded(4096);
    let session = FakeMediaSource::fast()
        .start(CaptureTarget::PrimaryMonitor, capture_config(), tx)
        .expect("fake media");
    let mut formats = None;
    let mut packets = Vec::new();
    let mut video_seen = 0;
    while video_seen < video_frames {
        match rx.recv().expect("media event") {
            MediaEvent::Started { video, audio, .. } => formats = Some((video, audio)),
            MediaEvent::Packet(packet) => {
                video_seen += usize::from(packet.track == Track::Video);
                packets.push(packet);
            }
            _ => {}
        }
    }
    drop(rx);
    session.stop();
    let (video, audio) = formats.expect("started event");
    Media {
        video,
        audio: audio.expect("audio format"),
        packets,
    }
}

pub(crate) fn leftover_part(output: &Path, name: &str, media: &Media, finish: bool) -> PathBuf {
    let folder = output.join(name);
    fs::create_dir_all(&folder).expect("folder");
    let part = folder.join("game.mp4.part");
    let mut writer =
        FragmentedWriter::create(&part, &media.video, Some(&media.audio)).expect("writer");
    for packet in &media.packets {
        writer.push(packet).expect("push");
    }
    if finish {
        writer.finish().expect("finish");
    } else {
        drop(writer);
    }
    let meta = RecordingMeta {
        version: META_VERSION,
        id: name.to_owned(),
        started_at: Zoned::now(),
        duration_secs: 0.0,
        video: "game.mp4".to_owned(),
        game: GameRecord {
            info: info(),
            result: GameResult::Unknown,
        },
        events: Vec::new(),
        clips: Vec::new(),
        encoder: "fake".to_owned(),
        video_format: VideoInfo {
            width: media.video.width,
            height: media.video.height,
            fps: media.video.fps,
        },
        review: Default::default(),
        health: None,
        recovered: false,
    };
    library::write_meta(&folder, &meta).expect("sidecar");
    folder
}

pub(crate) fn library_message(harness: &Harness) -> Vec<RecordingEntry> {
    wait_for(&harness.ui, |msg| match msg {
        UiMsg::Library(entries) => Some(entries.clone()),
        _ => None,
    })
    .expect("library message")
}

pub(crate) fn send(harness: &Harness, cmd: EngineCmd) {
    harness
        .engine
        .as_ref()
        .expect("engine running")
        .cmd
        .send(cmd)
        .expect("engine listening");
}

pub(crate) fn wait_status(harness: &Harness, want: fn(&EngineStatus) -> bool) {
    wait_for(&harness.ui, |msg| match msg {
        UiMsg::Status(status) => want(status).then_some(()),
        _ => None,
    })
    .expect("status reached");
}

pub(crate) fn wait_saved(harness: &Harness) -> RecordingEntry {
    wait_for(&harness.ui, |msg| match msg {
        UiMsg::RecordingSaved(entry) => Some(entry.clone()),
        _ => None,
    })
    .expect("recording saved")
}

pub(crate) fn inspect_video(entry: &RecordingEntry) -> greyscreen_mp4::inspect::Inspected {
    greyscreen_mp4::inspect::inspect(&entry.folder.join(&entry.meta.video)).expect("inspect")
}

pub(crate) struct BlockingFreeSpace {
    armed: AtomicBool,
    stall: Duration,
}

impl BlockingFreeSpace {
    pub(crate) fn new(stall: Duration) -> Arc<Self> {
        Arc::new(Self {
            armed: AtomicBool::new(false),
            stall,
        })
    }

    pub(crate) fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }
}

impl FreeSpace for BlockingFreeSpace {
    fn free_bytes(&self, _dir: &Path) -> Option<u64> {
        if self.armed.swap(false, Ordering::SeqCst) {
            std::thread::sleep(self.stall);
        }
        Some(u64::MAX / 2)
    }

    fn check_every(&self) -> Duration {
        Duration::from_millis(50)
    }
}
