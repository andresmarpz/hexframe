#![allow(unreachable_pub, dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use greyscreen_league::GameLocator;
use serde_json::{Value, json};
use tiny_http::{Header, Response, Server};

pub struct Script {
    pub ready: bool,
    pub mode: String,
    pub replay: bool,
    pub active_player: Option<String>,
    pub events: Vec<Value>,
    pub game_time_base: f64,
    pub stall: Duration,
    frozen: Option<f64>,
    started: Instant,
}

impl Script {
    pub fn game_time(&self) -> f64 {
        self.frozen
            .unwrap_or_else(|| self.game_time_base + self.started.elapsed().as_secs_f64())
    }

    pub fn freeze(&mut self) {
        self.frozen = Some(self.game_time());
    }

    pub fn freeze_at(&mut self, secs: f64) {
        self.frozen = Some(secs);
    }

    pub fn unfreeze(&mut self) {
        if let Some(frozen) = self.frozen.take() {
            self.game_time_base = frozen - self.started.elapsed().as_secs_f64();
        }
    }

    pub fn push_event_now(&mut self, mut event: Value) -> u32 {
        let id = u32::try_from(self.events.len()).unwrap();
        event["EventID"] = json!(id);
        event["EventTime"] = json!(self.game_time());
        self.events.push(event);
        id
    }
}

pub struct FakeServer {
    pub base: String,
    pub script: Arc<Mutex<Script>>,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl FakeServer {
    pub fn start() -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let script = Arc::new(Mutex::new(Script {
            ready: false,
            mode: "CLASSIC".into(),
            replay: false,
            active_player: Some("Faker#KR1".into()),
            events: Vec::new(),
            game_time_base: 100.0,
            stall: Duration::ZERO,
            frozen: None,
            started: Instant::now(),
        }));
        let thread = {
            let server = Arc::clone(&server);
            let script = Arc::clone(&script);
            std::thread::spawn(move || {
                for request in server.incoming_requests() {
                    let (status, body) = answer(&script, request.url());
                    let stall = script.lock().unwrap().stall;
                    std::thread::sleep(stall);
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    drop(
                        request.respond(
                            Response::from_string(body)
                                .with_status_code(status)
                                .with_header(header),
                        ),
                    );
                }
            })
        };
        Self {
            base,
            script,
            server,
            thread: Some(thread),
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut Script) -> R) -> R {
        f(&mut self.script.lock().unwrap())
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn answer(script: &Mutex<Script>, url: &str) -> (u16, String) {
    let script = script.lock().unwrap();
    let path = url.split('?').next().unwrap_or(url);
    match path {
        "/replay/playback" if script.replay => (200, "{}".into()),
        "/liveclientdata/gamestats" if script.ready => (
            200,
            json!({
                "gameMode": script.mode, "gameTime": script.game_time(),
                "mapName": "Map11", "mapNumber": 11, "mapTerrain": "Default"
            })
            .to_string(),
        ),
        "/liveclientdata/activeplayername" if script.ready => match &script.active_player {
            Some(name) => (200, json!(name).to_string()),
            None => (404, "{}".into()),
        },
        "/liveclientdata/playerlist" if script.ready => (
            200,
            json!([
                {"championName":"Ahri","isBot":false,"summonerName":"Faker","riotId":"Faker#KR1",
                 "riotIdGameName":"Faker","riotIdTagLine":"KR1","team":"ORDER"},
                {"championName":"Zed","isBot":false,"summonerName":"Chovy","riotId":"Chovy#KR2",
                 "riotIdGameName":"Chovy","riotIdTagLine":"KR2","team":"CHAOS"}
            ])
            .to_string(),
        ),
        "/liveclientdata/eventdata" if script.ready => {
            (200, json!({"Events": script.events}).to_string())
        }
        _ => (404, "{}".into()),
    }
}

#[derive(Clone)]
pub struct Switch(pub Arc<AtomicBool>);

impl Switch {
    pub fn on() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }

    pub fn set(&self, present: bool) {
        self.0.store(present, Ordering::SeqCst);
    }
}

impl GameLocator for Switch {
    fn find(&mut self) -> Option<(isize, u32)> {
        self.0.load(Ordering::SeqCst).then_some((42, 4242))
    }
}

pub fn recv<T>(rx: &crossbeam_channel::Receiver<T>) -> T {
    rx.recv_timeout(Duration::from_secs(15))
        .expect("expected a message within 15 s")
}

pub fn expect_silence<T: std::fmt::Debug>(rx: &crossbeam_channel::Receiver<T>, ms: u64) {
    if let Ok(msg) = rx.recv_timeout(Duration::from_millis(ms)) {
        panic!("unexpected message: {msg:?}");
    }
}
