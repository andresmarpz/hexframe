mod api;
mod clipboard;
mod clipjob;
mod control;
mod delete;
mod diagnostics;
mod disk;
mod encoder_check;
mod export;
pub(crate) mod fake;
pub(crate) mod feedback;
mod hotkey;
mod library;
mod recovery;
mod ring;
mod settings_save;
mod storage;
mod storyboard;
mod thumbs;
pub mod updater;
mod writer;

pub use api::{
    ClipEntry, Engine, EngineCmd, EngineConfig, EngineHandle, EngineStatus, EngineTiming, FeedItem,
    LeagueFactory, LeagueGuard, MANUAL_CLIP_LABEL, NoticeAction, NoticeLevel, RecordingEntry,
    UiMsg, UiSink, league_factory_default, reveal,
};
pub use clipboard::copy_file;
pub use disk::{FreeSpace, LOW_DISK_URL};
pub use encoder_check::{
    ChildRunner, EncoderCheckSpec, InProcessRunner, ProbeResult, ProbeRunner,
    abandon_encoder_check, run_encoder_check, run_isolated_encoder_check, run_probe_child,
    spawn_encoder_check, spawn_isolated_encoder_check,
};
pub use export::REENCODE_AVAILABLE;
pub use fake::{FailAt, FakeMediaSource, FakeProbe, Faults};
pub use feedback::{FeedbackConfig, FeedbackTransport};
pub use hotkey::HotkeyBackend;
pub use library::entries as library_entries;
pub use storyboard::build_storyboard;
pub use writer::set_write_limit;
