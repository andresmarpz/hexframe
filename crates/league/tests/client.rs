mod common;

use common::FakeServer;
use greyscreen_league::{ClientError, LiveClient};
use serde_json::json;

#[test]
fn not_ready_is_reported_with_status() {
    let server = FakeServer::start();
    let client = LiveClient::new(&server.base);
    assert!(matches!(
        client.gamestats(),
        Err(ClientError::NotReady(404))
    ));
    assert!(!client.replay_playback_ok());
}

#[test]
fn reads_all_endpoints() {
    let server = FakeServer::start();
    server.with(|s| {
        s.ready = true;
        s.push_event_now(json!({"EventName":"GameStart"}));
    });
    let client = LiveClient::new(&format!("{}/", server.base));

    let stats = client.gamestats().unwrap();
    assert_eq!(stats.game_mode, "CLASSIC");
    assert_eq!(stats.map_number, Some(11));
    assert!(stats.game_time.unwrap() >= 100.0);
    assert_eq!(client.eventdata().unwrap().events.len(), 1);
    assert_eq!(client.activeplayername().unwrap(), "Faker#KR1");
    assert_eq!(client.playerlist().unwrap().len(), 2);

    server.with(|s| s.replay = true);
    assert!(client.replay_playback_ok());
}

#[test]
fn closed_port_is_refused() {
    let mut last = None;
    for _ in 0..10 {
        let server = FakeServer::start();
        let base = server.base.clone();
        server.shutdown();
        let result = LiveClient::new(&base).gamestats();
        if matches!(result, Err(ClientError::Refused)) {
            return;
        }
        last = Some(result);
    }
    panic!("never refused, last result: {last:?}");
}

#[test]
fn non_json_body_is_bad() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    let thread = std::thread::spawn(move || {
        if let Ok(request) = server.recv() {
            drop(request.respond(tiny_http::Response::from_string("<html>")));
        }
    });
    assert!(matches!(
        LiveClient::new(&base).gamestats(),
        Err(ClientError::Bad(_))
    ));
    thread.join().unwrap();
}

#[test]
fn stalled_server_times_out_as_timeout_not_refused() {
    let server = FakeServer::start();
    server.with(|s| {
        s.ready = true;
        s.stall = std::time::Duration::from_millis(400);
    });
    let client = LiveClient::with_timeouts(
        &server.base,
        std::time::Duration::from_millis(100),
        std::time::Duration::from_millis(100),
    );
    assert!(matches!(client.gamestats(), Err(ClientError::Timeout)));
}
