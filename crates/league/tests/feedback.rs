#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use greyscreen_core::feedback::{FeedbackKind, FeedbackReport};
use greyscreen_league::{FeedbackClient, FeedbackError};
use tiny_http::{Header, Response, Server};

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    url: String,
    version: Option<String>,
    content_type: Option<String>,
    body: String,
}

struct FakeApi {
    base: String,
    seen: Arc<Mutex<Vec<Seen>>>,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl FakeApi {
    fn start(status: u16, body: &'static str) -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let thread = {
            let (server, seen) = (Arc::clone(&server), Arc::clone(&seen));
            std::thread::spawn(move || {
                for mut request in server.incoming_requests() {
                    let header = |name: &'static str| {
                        request
                            .headers()
                            .iter()
                            .find(|h| h.field.equiv(name))
                            .map(|h| h.value.to_string())
                    };
                    let (version, content_type) =
                        (header("x-greyscreen-version"), header("content-type"));
                    let mut text = String::new();
                    request.as_reader().read_to_string(&mut text).unwrap();
                    seen.lock().unwrap().push(Seen {
                        method: request.method().to_string(),
                        url: request.url().to_owned(),
                        version,
                        content_type,
                        body: text,
                    });
                    let json = Header::from_bytes("content-type", "application/json").unwrap();
                    drop(
                        request.respond(
                            Response::from_string(body)
                                .with_status_code(status)
                                .with_header(json),
                        ),
                    );
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
}

impl Drop for FakeApi {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

fn report() -> FeedbackReport {
    FeedbackReport {
        kind: FeedbackKind::Idea,
        message: "Add a dark mode".into(),
        email: None,
        app_version: "0.1.9".into(),
        os: "macOS 15.4".into(),
        log_tail: Some("line\n".into()),
        client_id: "abc".into(),
    }
}

#[test]
fn a_report_is_posted_as_json_with_the_version_header() {
    let api = FakeApi::start(202, r#"{"id":"fb_1"}"#);
    let id = FeedbackClient::new(&format!("{}/", api.base), "0.1.9")
        .submit(&report())
        .unwrap();
    assert_eq!(id.as_deref(), Some("fb_1"));
    let seen = api.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "POST");
    assert_eq!(seen[0].url, "/v1/feedback");
    assert_eq!(seen[0].version.as_deref(), Some("0.1.9"));
    assert_eq!(seen[0].content_type.as_deref(), Some("application/json"));
    let body: serde_json::Value = serde_json::from_str(&seen[0].body).unwrap();
    assert_eq!(body["kind"], "idea");
    assert_eq!(body["email"], serde_json::Value::Null);
    assert_eq!(body["log_tail"], "line\n");
    assert_eq!(body["client_id"], "abc");
}

#[test]
fn a_400_carries_the_server_reason() {
    let api = FakeApi::start(400, r#"{"error":"message is required"}"#);
    let err = FeedbackClient::new(&api.base, "0.1.9")
        .submit(&report())
        .unwrap_err();
    assert!(matches!(&err, FeedbackError::Rejected(reason) if reason == "message is required"));
}

#[test]
fn a_400_without_a_reason_reports_the_status() {
    let api = FakeApi::start(400, "nope");
    let err = FeedbackClient::new(&api.base, "0.1.9")
        .submit(&report())
        .unwrap_err();
    assert!(matches!(err, FeedbackError::Status { status: 400 }));
}

#[test]
fn a_429_is_rate_limiting() {
    let api = FakeApi::start(429, "{}");
    let err = FeedbackClient::new(&api.base, "0.1.9")
        .submit(&report())
        .unwrap_err();
    assert!(matches!(err, FeedbackError::RateLimited));
}

#[test]
fn a_server_error_reports_the_status() {
    let api = FakeApi::start(503, "down");
    let err = FeedbackClient::new(&api.base, "0.1.9")
        .submit(&report())
        .unwrap_err();
    assert!(matches!(err, FeedbackError::Status { status: 503 }));
}

#[test]
fn an_unreachable_server_is_a_request_error() {
    let err = FeedbackClient::new("http://127.0.0.1:1", "0.1.9")
        .submit(&report())
        .unwrap_err();
    assert!(matches!(err, FeedbackError::Request { .. }));
}
