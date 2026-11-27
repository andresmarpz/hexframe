use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, Server};

const BUILTIN: &str = include_str!("fake_match.json");
const POLL_WAIT: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Script {
    pub(crate) loading_secs: f64,
    pub(crate) tail_secs: f64,
    pub(crate) duration_secs: f64,
    pub(crate) active_player: String,
    pub(crate) game_mode: String,
    pub(crate) players: Vec<Value>,
    pub(crate) events: Vec<Value>,
}

impl Script {
    pub(crate) fn builtin() -> Result<Self> {
        serde_json::from_str(BUILTIN).context("built-in match script")
    }

    pub(crate) fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading script {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing script {}", path.display()))
    }
}

#[derive(Debug)]
pub(crate) struct FakeLeague {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeLeague {
    pub(crate) fn start(port: u16, speed: f64, script: Script) -> Result<Self> {
        if !speed.is_finite() || speed <= 0.0 {
            bail!("speed must be a positive number");
        }
        let server = Server::http(("127.0.0.1", port))
            .map_err(|e| anyhow!("cannot listen on 127.0.0.1:{port}: {e}"))?;
        let port = server
            .server_addr()
            .to_ip()
            .map_or(port, |addr| addr.port());
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::Builder::new()
                .name("fake-league".into())
                .spawn(move || serve(&server, &script, speed, &stop))
                .context("spawning fake league thread")?
        };
        Ok(Self {
            port,
            stop,
            thread: Some(thread),
        })
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    pub(crate) fn wait(mut self) {
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("fake league thread panicked");
        }
    }
}

impl Drop for FakeLeague {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!("fake league thread panicked");
        }
    }
}

pub(crate) fn run(port: u16, speed: f64, script: Script) -> Result<()> {
    let league = FakeLeague::start(port, speed, script)?;
    println!("fake league client on http://127.0.0.1:{}", league.port());
    league.wait();
    println!("match over, server closed");
    Ok(())
}

fn serve(server: &Server, script: &Script, speed: f64, stop: &AtomicBool) {
    let started = Instant::now();
    let loading = script.loading_secs / speed;
    let end_wall = loading + script.duration_secs / speed + script.tail_secs;
    while !stop.load(Ordering::Relaxed) {
        let wall = started.elapsed().as_secs_f64();
        if wall >= end_wall {
            return;
        }
        match server.recv_timeout(POLL_WAIT) {
            Ok(Some(request)) => {
                let game_time = (wall >= loading).then_some((wall - loading) * speed);
                respond(request, script, game_time);
            }
            Ok(None) => {}
            Err(_) => return,
        }
    }
}

fn respond(request: Request, script: &Script, game_time: Option<f64>) {
    let path = request
        .url()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_owned();
    let body = if *request.method() == Method::Get {
        game_time.and_then(|t| answer(&path, script, t))
    } else {
        None
    };
    let result = match body {
        Some(value) => {
            let mut response = Response::from_string(value.to_string());
            if let Ok(header) = Header::from_bytes("Content-Type", "application/json") {
                response.add_header(header);
            }
            request.respond(response)
        }
        None => request.respond(Response::empty(404)),
    };
    if let Err(e) = result {
        tracing::debug!("fake league response failed: {e}");
    }
}

fn answer(path: &str, script: &Script, game_time: f64) -> Option<Value> {
    match path {
        "/liveclientdata/gamestats" => Some(json!({
            "gameMode": script.game_mode,
            "gameTime": game_time,
            "mapName": "Map11",
            "mapNumber": 11,
            "mapTerrain": "Default",
        })),
        "/liveclientdata/eventdata" => Some(json!({ "Events": visible_events(script, game_time) })),
        "/liveclientdata/activeplayername" => Some(json!(script.active_player)),
        "/liveclientdata/playerlist" => Some(Value::Array(script.players.clone())),
        _ => None,
    }
}

fn visible_events(script: &Script, game_time: f64) -> Vec<Value> {
    script
        .events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            event
                .get("EventTime")
                .and_then(Value::as_f64)
                .is_some_and(|at| at <= game_time)
        })
        .map(|(id, event)| {
            let mut event = event.clone();
            if let Some(map) = event.as_object_mut() {
                map.insert("EventID".to_owned(), json!(id));
            }
            event
        })
        .collect()
}
