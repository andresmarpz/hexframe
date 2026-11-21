use std::collections::HashSet;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use greyscreen_core::game::{GameEvent, GameInfo, GameResult, TimedEvent};
use greyscreen_core::time::{Hns, mono_now, secs_to_hns};
use serde_json::Value;

use crate::client::{ClientError, LiveClient};
use crate::locator::GameLocator;
use crate::parse::{event_id, event_secs, to_game_event, to_game_info};

pub(crate) const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAX_REFUSED_POLLS: u32 = 3;
const MAX_IDENTIFY_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, PartialEq)]
pub enum LeagueMsg {
    WindowFound { hwnd: isize, pid: u32 },
    Ready(GameInfo),
    Event(TimedEvent),
    Ended(GameResult),
    Gone,
    Skipped(String),
}

#[derive(Debug)]
pub struct Poller;

impl Poller {
    pub fn spawn(
        client: LiveClient,
        locator: Box<dyn GameLocator>,
        out: Sender<LeagueMsg>,
    ) -> PollerHandle {
        Self::spawn_with_interval(client, locator, out, DEFAULT_POLL_INTERVAL)
    }

    pub fn spawn_with_interval(
        client: LiveClient,
        locator: Box<dyn GameLocator>,
        out: Sender<LeagueMsg>,
        interval: Duration,
    ) -> PollerHandle {
        let (stop, stop_rx) = crossbeam_channel::bounded(0);
        let worker = Worker {
            client,
            locator,
            out,
            state: State::Idle,
        };
        let thread = std::thread::Builder::new()
            .name("league-poller".into())
            .spawn(move || worker.run(&stop_rx, interval))
            .ok();
        PollerHandle {
            stop: Some(stop),
            thread,
        }
    }
}

#[derive(Debug)]
pub struct PollerHandle {
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for PollerHandle {
    fn drop(&mut self) {
        self.stop.take();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("league poller thread panicked");
        }
    }
}

struct Closed;

enum State {
    Idle,
    Waiting { attempts: u32 },
    Recording(Recording),
    Cooldown { announce_gone: bool },
}

struct Recording {
    ready_at: Hns,
    seen: HashSet<u32>,
    offset: Option<Hns>,
    last_at: Hns,
    refused: u32,
}

struct Worker {
    client: LiveClient,
    locator: Box<dyn GameLocator>,
    out: Sender<LeagueMsg>,
    state: State,
}

impl Worker {
    fn run(mut self, stop: &Receiver<()>, interval: Duration) {
        loop {
            if self.tick().is_err() {
                return;
            }
            if !matches!(stop.recv_timeout(interval), Err(RecvTimeoutError::Timeout)) {
                return;
            }
        }
    }

    fn send(&self, msg: LeagueMsg) -> Result<(), Closed> {
        self.out.send(msg).map_err(|_| Closed)
    }

    fn tick(&mut self) -> Result<(), Closed> {
        match std::mem::replace(&mut self.state, State::Idle) {
            State::Idle => self.tick_idle(),
            State::Waiting { attempts } => self.tick_waiting(attempts),
            State::Recording(recording) => self.tick_recording(recording),
            State::Cooldown { announce_gone } => self.tick_cooldown(announce_gone),
        }
    }

    fn tick_idle(&mut self) -> Result<(), Closed> {
        if let Some((hwnd, pid)) = self.locator.find() {
            self.send(LeagueMsg::WindowFound { hwnd, pid })?;
            self.state = State::Waiting { attempts: 0 };
        }
        Ok(())
    }

    fn skip(&mut self, reason: &str) -> Result<(), Closed> {
        self.state = State::Cooldown {
            announce_gone: true,
        };
        self.send(LeagueMsg::Skipped(reason.to_owned()))
    }

    fn tick_waiting(&mut self, attempts: u32) -> Result<(), Closed> {
        self.state = State::Waiting { attempts };
        if self.locator.find().is_none() {
            self.state = State::Idle;
            return self.send(LeagueMsg::Gone);
        }
        let Ok(stats) = self.client.gamestats() else {
            return Ok(());
        };
        if stats.game_time.is_none() {
            return Ok(());
        }
        if stats.game_mode.eq_ignore_ascii_case("TFT") {
            return self.skip("tft");
        }
        if self.client.replay_playback_ok() {
            return self.skip("replay");
        }
        let identified = self
            .client
            .activeplayername()
            .ok()
            .zip(self.client.playerlist().ok())
            .and_then(|(name, players)| to_game_info(&name, &players, &stats));
        let Some(info) = identified else {
            let attempts = attempts + 1;
            if attempts >= MAX_IDENTIFY_ATTEMPTS {
                return self.skip("no active player");
            }
            self.state = State::Waiting { attempts };
            return Ok(());
        };
        self.state = State::Recording(Recording {
            ready_at: mono_now(),
            seen: HashSet::new(),
            offset: None,
            last_at: Hns::MIN,
            refused: 0,
        });
        self.send(LeagueMsg::Ready(info))
    }

    fn tick_cooldown(&mut self, announce_gone: bool) -> Result<(), Closed> {
        if self.locator.find().is_some() {
            self.state = State::Cooldown { announce_gone };
            return Ok(());
        }
        if announce_gone {
            self.send(LeagueMsg::Gone)?;
        }
        Ok(())
    }

    fn tick_recording(&mut self, mut rec: Recording) -> Result<(), Closed> {
        if self.locator.find().is_none() {
            return self.send(LeagueMsg::Gone);
        }
        let events = self.client.eventdata();
        let before_stats = mono_now();
        let stats = self.client.gamestats();
        let after_stats = mono_now();

        if matches!(events, Err(ClientError::Refused)) {
            rec.refused += 1;
            if rec.refused >= MAX_REFUSED_POLLS {
                self.state = State::Cooldown {
                    announce_gone: false,
                };
                return self.send(LeagueMsg::Gone);
            }
            self.state = State::Recording(rec);
            return Ok(());
        }
        rec.refused = 0;

        if let Some(game_time) = stats.ok().and_then(|s| s.game_time) {
            let mid = before_stats + (after_stats - before_stats) / 2;
            rec.offset = Some(mid - secs_to_hns(game_time));
        }

        let outcome = match (events, rec.offset) {
            (Ok(data), Some(offset)) => self.emit_new_events(&mut rec, data.events, offset),
            _ => Ok(None),
        };
        match outcome {
            Ok(Some(result)) => {
                self.state = State::Cooldown {
                    announce_gone: true,
                };
                self.send(LeagueMsg::Ended(result))
            }
            Ok(None) => {
                self.state = State::Recording(rec);
                Ok(())
            }
            Err(closed) => Err(closed),
        }
    }

    fn emit_new_events(
        &self,
        rec: &mut Recording,
        mut events: Vec<Value>,
        offset: Hns,
    ) -> Result<Option<GameResult>, Closed> {
        events.sort_by_key(|e| event_id(e).unwrap_or(u32::MAX));
        for raw in &events {
            let (Some(id), Some(game_secs)) = (event_id(raw), event_secs(raw)) else {
                continue;
            };
            if !rec.seen.insert(id) {
                continue;
            }
            let at = offset + secs_to_hns(game_secs);
            if at < rec.ready_at {
                continue;
            }
            rec.last_at = rec.last_at.max(at);
            let kind = to_game_event(raw);
            let ended = match &kind {
                GameEvent::GameEnd { result } => Some(*result),
                _ => None,
            };
            self.send(LeagueMsg::Event(TimedEvent {
                id,
                game_secs,
                at: rec.last_at,
                kind,
            }))?;
            if ended.is_some() {
                return Ok(ended);
            }
        }
        Ok(None)
    }
}
