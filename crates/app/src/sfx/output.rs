use std::fmt::Debug;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use greyscreen_core::paths;
use rune::{Player, PlayerState};

use super::Cue;

const KEEP_ALIVE: Duration = Duration::from_secs(3);

pub(super) trait Output: Debug + Send {
    fn play(&mut self, cue: Cue, volume: f32);
}

#[derive(Debug)]
pub(super) struct Silent;

impl Output for Silent {
    fn play(&mut self, _: Cue, _: f32) {}
}

#[derive(Debug)]
struct Files {
    dir: PathBuf,
    ready: OnceLock<bool>,
}

impl Files {
    fn ensure(&self) -> bool {
        *self.ready.get_or_init(|| match install(&self.dir) {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!(%err, dir = %self.dir.display(), "cannot write the sound files");
                false
            }
        })
    }

    fn path(&self, cue: Cue) -> PathBuf {
        self.dir.join(cue.file_name())
    }
}

#[derive(Debug)]
struct Live {
    started: Instant,
    player: Player,
}

#[derive(Debug)]
pub(super) struct RuneOutput {
    files: Arc<Files>,
    live: Vec<Live>,
}

impl RuneOutput {
    pub(super) fn start() -> RuneOutput {
        let output = RuneOutput::in_dir(paths::sounds_dir());
        let files = output.files.clone();
        let spawned = std::thread::Builder::new()
            .name("sfx-install".into())
            .spawn(move || {
                files.ensure();
            });
        if let Err(err) = spawned {
            tracing::warn!(%err, "cannot start the sound installer");
        }
        output
    }

    pub(super) fn in_dir(dir: PathBuf) -> RuneOutput {
        RuneOutput {
            files: Arc::new(Files {
                dir,
                ready: OnceLock::new(),
            }),
            live: Vec::new(),
        }
    }

    pub(super) fn reap(&mut self) -> Vec<String> {
        let mut failures = Vec::new();
        self.live.retain(|live| match live.player.state() {
            PlayerState::Failed(reason) => {
                tracing::warn!(%reason, "sound effect cannot play");
                failures.push(reason);
                false
            }
            PlayerState::Ended => false,
            _ => live.started.elapsed() < KEEP_ALIVE,
        });
        failures
    }

    #[cfg(test)]
    pub(super) fn playing(&self) -> usize {
        self.live.len()
    }
}

impl Output for RuneOutput {
    fn play(&mut self, cue: Cue, volume: f32) {
        self.reap();
        if !self.files.ensure() {
            return;
        }
        let path = self.files.path(cue);
        if !path.is_file() {
            tracing::warn!(path = %path.display(), "sound file is missing");
            return;
        }
        let player = Player::audio(path);
        player.set_volume(volume);
        player.play();
        self.live.push(Live {
            started: Instant::now(),
            player,
        });
    }
}

pub(super) fn install(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    for cue in Cue::ALL {
        let path = dir.join(cue.file_name());
        if fs::read(&path).is_ok_and(|have| have == cue.bytes()) {
            continue;
        }
        let mut tmp = path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, cue.bytes())?;
        fs::rename(&tmp, &path)?;
    }
    Ok(())
}
