#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use greyscreen_league::ddragon::{ChampionIcons, sync};
use serde_json::json;
use tiny_http::{Response, Server};

const DAY: u64 = 86_400;
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake";

const CHAMPIONS: [(&str, &str); 8] = [
    ("LeeSin", "Lee Sin"),
    ("MonkeyKing", "Wukong"),
    ("Nunu", "Nunu & Willump"),
    ("Kaisa", "Kai'Sa"),
    ("Renata", "Renata Glasc"),
    ("DrMundo", "Dr. Mundo"),
    ("Leblanc", "LeBlanc"),
    ("Ahri", "Ahri"),
];

struct FakeDdragon {
    base: String,
    version: Arc<Mutex<String>>,
    hits: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl FakeDdragon {
    fn start(version: &str) -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let version = Arc::new(Mutex::new(version.to_owned()));
        let hits = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (server, version, hits, stop) = (
                Arc::clone(&server),
                Arc::clone(&version),
                Arc::clone(&hits),
                Arc::clone(&stop),
            );
            std::thread::spawn(move || {
                for request in server.incoming_requests() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let url = request.url().to_owned();
                    hits.lock().unwrap().push(url.clone());
                    let current = version.lock().unwrap().clone();
                    drop(match answer(&url, &current) {
                        Some(body) => request.respond(Response::from_data(body)),
                        None => request.respond(Response::from_string("no").with_status_code(404)),
                    });
                }
            })
        };
        Self {
            base,
            version,
            hits,
            stop,
            server,
            thread: Some(thread),
        }
    }

    fn hits_for(&self, part: &str) -> usize {
        self.hits
            .lock()
            .unwrap()
            .iter()
            .filter(|url| url.contains(part))
            .count()
    }

    fn release(&self, version: &str) {
        *self.version.lock().unwrap() = version.to_owned();
    }
}

impl Drop for FakeDdragon {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

fn answer(url: &str, version: &str) -> Option<Vec<u8>> {
    if url == "/api/versions.json" {
        return Some(json!([version, "0.0.1"]).to_string().into_bytes());
    }
    let prefix = format!("/cdn/{version}/");
    let rest = url.strip_prefix(&prefix)?;
    if rest == "data/en_US/champion.json" {
        let data: BTreeMap<_, _> = CHAMPIONS
            .iter()
            .map(|(id, name)| (*id, json!({"id": id, "name": name})))
            .collect();
        return Some(json!({"data": data}).to_string().into_bytes());
    }
    let id = rest.strip_prefix("img/champion/")?.strip_suffix(".png")?;
    CHAMPIONS
        .iter()
        .any(|(known, _)| *known == id)
        .then(|| PNG.to_vec())
}

#[test]
fn downloads_every_icon_and_maps_display_names() {
    let server = FakeDdragon::start("16.23.1");
    let dir = tempfile::tempdir().unwrap();

    let report = sync(&server.base, dir.path(), 1000).unwrap();

    assert_eq!(report.version, "16.23.1");
    assert_eq!(report.downloaded, CHAMPIONS.len());
    assert_eq!(report.failed, 0);
    let icons = ChampionIcons::open(dir.path());
    for (id, name) in CHAMPIONS {
        let path = icons.icon_path(name).unwrap();
        assert_eq!(path, dir.path().join(format!("{id}.png")));
        assert_eq!(std::fs::read(path).unwrap(), PNG);
    }
}

#[test]
fn matches_names_ignoring_case_and_punctuation() {
    let server = FakeDdragon::start("16.23.1");
    let dir = tempfile::tempdir().unwrap();
    sync(&server.base, dir.path(), 1000).unwrap();
    let icons = ChampionIcons::open(dir.path());
    let file = |name: &str| {
        icons
            .icon_path(name)
            .and_then(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()))
    };

    assert_eq!(file("lee sin").as_deref(), Some("LeeSin.png"));
    assert_eq!(file("LeeSin").as_deref(), Some("LeeSin.png"));
    assert_eq!(file("WUKONG").as_deref(), Some("MonkeyKing.png"));
    assert_eq!(file("MonkeyKing").as_deref(), Some("MonkeyKing.png"));
    assert_eq!(file("Nunu & Willump").as_deref(), Some("Nunu.png"));
    assert_eq!(file("Nunu").as_deref(), Some("Nunu.png"));
    assert_eq!(file("Kai'Sa").as_deref(), Some("Kaisa.png"));
    assert_eq!(file("kaisa").as_deref(), Some("Kaisa.png"));
    assert_eq!(file("Renata Glasc").as_deref(), Some("Renata.png"));
    assert_eq!(file("Dr Mundo").as_deref(), Some("DrMundo.png"));
    assert_eq!(file("Le Blanc").as_deref(), Some("Leblanc.png"));
    assert_eq!(file("Teemo"), None);
}

#[test]
fn skips_icons_that_already_exist() {
    let server = FakeDdragon::start("16.23.1");
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Ahri.png"), b"mine").unwrap();

    let report = sync(&server.base, dir.path(), 1000).unwrap();

    assert_eq!(report.downloaded, CHAMPIONS.len() - 1);
    assert_eq!(server.hits_for("Ahri.png"), 0);
    assert_eq!(std::fs::read(dir.path().join("Ahri.png")).unwrap(), b"mine");
}

#[test]
fn reuses_the_index_without_asking_again_within_a_day() {
    let server = FakeDdragon::start("16.23.1");
    let dir = tempfile::tempdir().unwrap();
    sync(&server.base, dir.path(), 1000).unwrap();
    let requests = server.hits.lock().unwrap().len();

    let again = sync(&server.base, dir.path(), 1000 + DAY - 1).unwrap();

    assert!(!again.checked_versions);
    assert_eq!(again.downloaded, 0);
    assert_eq!(server.hits.lock().unwrap().len(), requests);
}

#[test]
fn works_offline_from_the_cached_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let server = FakeDdragon::start("16.23.1");
        sync(&server.base, dir.path(), 1000).unwrap();
    }

    let icons = ChampionIcons::open(dir.path());
    assert!(icons.icon_path("Lee Sin").is_some());
    assert!(icons.icon_path("Wukong").is_some());
    assert!(sync("http://127.0.0.1:1", dir.path(), 1000 + DAY - 1).is_ok());
    assert!(sync("http://127.0.0.1:1", dir.path(), 1000 + 2 * DAY).is_err());
    assert!(
        ChampionIcons::open(dir.path())
            .icon_path("Lee Sin")
            .is_some()
    );
}

#[test]
fn a_new_version_after_a_day_refreshes_names_and_keeps_icons() {
    let server = FakeDdragon::start("16.23.1");
    let dir = tempfile::tempdir().unwrap();
    sync(&server.base, dir.path(), 1000).unwrap();

    server.release("16.24.1");
    let report = sync(&server.base, dir.path(), 1000 + DAY).unwrap();

    assert!(report.checked_versions);
    assert_eq!(report.version, "16.24.1");
    assert_eq!(report.downloaded, 0);
    assert_eq!(server.hits_for("16.23.1/img"), CHAMPIONS.len());
    assert_eq!(server.hits_for("16.24.1/img"), 0);
}

#[test]
fn downloads_again_the_icons_that_were_deleted() {
    let server = FakeDdragon::start("16.23.1");
    let dir = tempfile::tempdir().unwrap();
    sync(&server.base, dir.path(), 1000).unwrap();
    std::fs::remove_file(dir.path().join("Ahri.png")).unwrap();
    std::fs::remove_file(dir.path().join("Nunu.png")).unwrap();

    let report = sync(&server.base, dir.path(), 1000 + 10).unwrap();

    assert_eq!(report.downloaded, 2);
    assert_eq!(report.failed, 0);
    assert!(dir.path().join("Ahri.png").is_file());
}
