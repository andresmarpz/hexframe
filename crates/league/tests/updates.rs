#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use greyscreen_core::update::{Asset, Version, newest_update};
use greyscreen_league::{GithubReleases, UpdateError};
use serde_json::json;
use sha2::{Digest, Sha256};
use tiny_http::{Response, Server};

type Handler = Box<dyn Fn(&str) -> (u16, Vec<u8>) + Send>;
type Seen = Arc<Mutex<Vec<(String, Option<String>)>>>;

struct FakeGithub {
    base: String,
    seen: Seen,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl FakeGithub {
    fn start(handler: Handler) -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let thread = {
            let (server, seen) = (Arc::clone(&server), Arc::clone(&seen));
            std::thread::spawn(move || {
                for request in server.incoming_requests() {
                    let agent = request
                        .headers()
                        .iter()
                        .find(|h| h.field.equiv("User-Agent"))
                        .map(|h| h.value.to_string());
                    let url = request.url().to_owned();
                    seen.lock().unwrap().push((url.clone(), agent));
                    let (status, body) = handler(&url);
                    drop(request.respond(Response::from_data(body).with_status_code(status)));
                }
            })
        };
        Self {
            base,
            seen,
            server,
            thread: Some(thread),
        }
    }

    fn client(&self) -> GithubReleases {
        GithubReleases::with_bases(
            &self.base,
            &format!("{}/download/", self.base),
            "0.1.6-preview.8",
        )
    }

    fn hits(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
}

impl Drop for FakeGithub {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn exe_bytes() -> Vec<u8> {
    let mut bytes = b"MZ".to_vec();
    bytes.extend((0..5000u32).map(|i| (i % 251) as u8));
    bytes
}

fn listing(base_hint: &str) -> Vec<u8> {
    let exe = exe_bytes();
    serde_json::to_vec(&json!([
        {
            "tag_name": "v0.2.0",
            "draft": false,
            "prerelease": false,
            "published_at": "2026-12-01T10:00:00Z",
            "assets": [
                { "name": "README.txt", "browser_download_url": format!("{base_hint}/download/readme"), "size": 10 },
                { "name": "greyscreen-cli.exe", "browser_download_url": format!("{base_hint}/download/cli"), "size": 99 },
                { "name": "greyscreen.exe", "browser_download_url": format!("{base_hint}/download/exe"), "size": exe.len(), "digest": format!("sha256:{}", sha(&exe)) }
            ]
        },
        {
            "tag_name": "v0.1.7-preview.9",
            "draft": false,
            "prerelease": true,
            "published_at": "2026-11-28T10:00:00Z",
            "assets": [
                { "name": "greyscreen.exe", "browser_download_url": format!("{base_hint}/download/old"), "size": 7 }
            ]
        },
        { "tag_name": "v0.9.9", "draft": true, "prerelease": false, "assets": [
            { "name": "greyscreen.exe", "browser_download_url": "x", "size": 1 }
        ] },
        { "tag_name": "v0.1.0-preview.2", "prerelease": true, "published_at": null, "assets": [] }
    ]))
    .unwrap()
}

fn asset(base: &str, size: usize, digest: Option<String>) -> Asset {
    Asset {
        url: format!("{base}/download/exe"),
        size: size as u64,
        sha256: digest,
    }
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn serving_exe() -> FakeGithub {
    FakeGithub::start(Box::new(|url| match url {
        "/download/exe" => (200, exe_bytes()),
        _ => (404, b"no".to_vec()),
    }))
}

#[test]
fn lists_releases_with_the_exe_asset_and_digest() {
    let github = FakeGithub::start(Box::new(|_| (200, listing("http://127.0.0.1:1"))));
    let releases = github.client().releases().unwrap();
    let tags: Vec<_> = releases.iter().map(|r| r.tag.as_str()).collect();
    assert_eq!(tags, ["v0.2.0", "v0.1.7-preview.9", "v0.1.0-preview.2"]);
    let newest = &releases[0];
    assert!(!newest.prerelease);
    assert_eq!(newest.published.as_deref(), Some("2026-12-01T10:00:00Z"));
    let exe = newest.exe.as_ref().unwrap();
    assert_eq!(exe.size, exe_bytes().len() as u64);
    assert_eq!(exe.sha256.as_deref(), Some(sha(&exe_bytes()).as_str()));
    assert!(exe.url.ends_with("/download/exe"));
    assert!(releases[1].exe.as_ref().unwrap().sha256.is_none());
    assert!(releases[2].exe.is_none());
}

#[test]
fn the_request_names_the_repo_and_the_app_version() {
    let github = FakeGithub::start(Box::new(|_| (200, b"[]".to_vec())));
    assert!(github.client().releases().unwrap().is_empty());
    let seen = github.seen.lock().unwrap();
    assert_eq!(
        seen[0].0,
        "/repos/andresmarpz/greyscreen/releases?per_page=20"
    );
    assert_eq!(seen[0].1.as_deref(), Some("greyscreen/0.1.6-preview.8"));
}

#[test]
fn a_listing_feeds_the_channel_rule() {
    let github = FakeGithub::start(Box::new(|_| (200, listing("http://x"))));
    let releases = github.client().releases().unwrap();
    let running = Version::parse("0.1.6-preview.8").unwrap();
    assert_eq!(newest_update(&running, &releases).unwrap().tag, "v0.2.0");
    let stable = Version::parse("0.2.0").unwrap();
    assert!(newest_update(&stable, &releases).is_none());
}

#[test]
fn rate_limits_are_their_own_error() {
    for status in [403, 429] {
        let github = FakeGithub::start(Box::new(move |_| (status, b"{}".to_vec())));
        assert!(matches!(
            github.client().releases(),
            Err(UpdateError::RateLimited)
        ));
    }
}

#[test]
fn server_errors_and_junk_are_errors() {
    let github = FakeGithub::start(Box::new(|_| (500, b"boom".to_vec())));
    assert!(matches!(
        github.client().releases(),
        Err(UpdateError::Request { .. })
    ));
    let github = FakeGithub::start(Box::new(|_| (200, b"<html>".to_vec())));
    assert!(matches!(
        github.client().releases(),
        Err(UpdateError::Bad { .. })
    ));
}

#[test]
fn downloads_and_verifies_the_exe() {
    let github = serving_exe();
    let dir = scratch();
    let dest = dir.path().join("sub").join("greyscreen-v0.2.0.exe");
    let exe = exe_bytes();
    github
        .client()
        .download(&asset(&github.base, exe.len(), Some(sha(&exe))), &dest)
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), exe);
    assert!(!dest.with_extension("part").exists());
}

#[test]
fn a_missing_digest_still_checks_the_size() {
    let github = serving_exe();
    let dir = scratch();
    let exe = exe_bytes();
    let dest = dir.path().join("a.exe");
    github
        .client()
        .download(&asset(&github.base, exe.len(), None), &dest)
        .unwrap();
    assert!(dest.exists());
    let wrong = dir.path().join("b.exe");
    let err = github
        .client()
        .download(&asset(&github.base, exe.len() - 1, None), &wrong)
        .unwrap_err();
    assert!(matches!(err, UpdateError::Verify(_)));
    assert!(!wrong.exists());
}

#[test]
fn a_wrong_digest_leaves_nothing_behind() {
    let github = serving_exe();
    let dir = scratch();
    let dest = dir.path().join("bad.exe");
    let err = github
        .client()
        .download(
            &asset(&github.base, exe_bytes().len(), Some("0".repeat(64))),
            &dest,
        )
        .unwrap_err();
    assert!(matches!(err, UpdateError::Verify(_)));
    assert!(!dest.exists());
    assert!(!dest.with_extension("part").exists());
}

#[test]
fn a_larger_body_than_announced_is_rejected() {
    let github = FakeGithub::start(Box::new(|_| (200, vec![b'M'; 10_000])));
    let dir = scratch();
    let dest = dir.path().join("big.exe");
    let err = github
        .client()
        .download(&asset(&github.base, 100, None), &dest)
        .unwrap_err();
    assert!(matches!(err, UpdateError::Verify(_)));
    assert!(!dest.exists());
}

#[test]
fn something_that_is_not_an_exe_is_rejected() {
    let body = vec![b'x'; 300];
    let digest = sha(&body);
    let github = FakeGithub::start(Box::new(move |_| (200, body.clone())));
    let dir = scratch();
    let dest = dir.path().join("html.exe");
    let err = github
        .client()
        .download(&asset(&github.base, 300, Some(digest)), &dest)
        .unwrap_err();
    assert!(matches!(err, UpdateError::Verify(_)));
    assert!(!dest.exists());
}

#[test]
fn a_failed_request_leaves_nothing_behind() {
    let github = FakeGithub::start(Box::new(|_| (404, b"no".to_vec())));
    let dir = scratch();
    let dest = dir.path().join("gone.exe");
    let err = github
        .client()
        .download(&asset(&github.base, 10, None), &dest)
        .unwrap_err();
    assert!(matches!(err, UpdateError::Request { .. }));
    assert!(!dest.exists());
    assert!(!dest.with_extension("part").exists());
}

#[test]
fn a_valid_file_on_disk_is_reused_without_a_request() {
    let github = serving_exe();
    let dir = scratch();
    let dest = dir.path().join("greyscreen-v0.2.0.exe");
    let exe = exe_bytes();
    let asset = asset(&github.base, exe.len(), Some(sha(&exe)));
    github.client().download(&asset, &dest).unwrap();
    let hits = github.hits();
    github.client().download(&asset, &dest).unwrap();
    assert_eq!(github.hits(), hits);
}

#[test]
fn a_corrupt_file_on_disk_is_downloaded_again() {
    let github = serving_exe();
    let dir = scratch();
    let dest = dir.path().join("greyscreen-v0.2.0.exe");
    let exe = exe_bytes();
    std::fs::write(&dest, b"MZ half a file").unwrap();
    github
        .client()
        .download(&asset(&github.base, exe.len(), Some(sha(&exe))), &dest)
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), exe);
}

#[test]
fn addresses_outside_the_release_downloads_are_refused() {
    let github = serving_exe();
    let dir = scratch();
    let dest = dir.path().join("x.exe");
    let outside = Asset {
        url: format!("{}/elsewhere/exe", github.base),
        size: 10,
        sha256: None,
    };
    let err = github.client().download(&outside, &dest).unwrap_err();
    assert!(matches!(err, UpdateError::Bad { .. }));
    assert_eq!(github.hits(), 0);
}
