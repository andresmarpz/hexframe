use greyscreen_core::health::{AudioHealth, EncodeTiming, RecordingHealth};
use rune::Runtime;

use super::{review_runtime, tap, visible_texts};
use crate::ui::Ui;

fn health(dropped_pct: f64, drift_ms: f64) -> RecordingHealth {
    RecordingHealth {
        target_fps: 60,
        duration_secs: 100.0,
        frames_encoded: 5_980,
        frames_expected: 6_000,
        frames_dropped: 20,
        dropped_pct,
        avg_bitrate_kbps: 18_000,
        longest_video_gap_ms: 33.0,
        encode_ms: Some(EncodeTiming {
            avg: 3.0,
            p95: 6.0,
            max: 9.0,
        }),
        audio: Some(AudioHealth {
            drift_ms,
            ..AudioHealth::default()
        }),
        ..RecordingHealth::default()
    }
}

fn set_health(rt: &mut Runtime<Ui>, health: Option<RecordingHealth>) {
    rt.update(move |ui, _| {
        if let Some(review) = &mut ui.review {
            review.entry.meta.health = health;
        }
    });
    rt.frame(1.0 / 60.0);
}

fn has_text(rt: &Runtime<Ui>, wanted: &str) -> bool {
    visible_texts(rt).iter().any(|t| t == wanted)
}

#[test]
fn the_info_button_opens_the_summary_and_toggles_it_shut() {
    let mut rt = review_runtime();
    set_health(&mut rt, Some(health(0.3, 4.0)));
    assert!(rt.find("health-summary").is_none());
    tap(&mut rt, "review-health");
    assert!(has_text(
        &rt,
        "59.8 fps · 0.3% dropped · audio OK · 18 Mbps"
    ));
    tap(&mut rt, "review-health");
    assert!(rt.find("health-summary").is_none());
}

#[test]
fn pressing_outside_closes_it() {
    let mut rt = review_runtime();
    set_health(&mut rt, Some(health(0.3, 4.0)));
    tap(&mut rt, "review-health");
    assert!(rt.find("health-summary").is_some());
    rt.pointer_down(300.0, 400.0);
    rt.pointer_up(300.0, 400.0);
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    assert!(rt.find("health-summary").is_none());
    assert!(!rt.state.review.as_ref().is_some_and(|r| r.health_open));
}

#[test]
fn warnings_and_bad_areas_get_their_own_tone_rows() {
    let mut rt = review_runtime();
    set_health(&mut rt, Some(health(1.2, -400.0)));
    tap(&mut rt, "review-health");
    for level in ["WARNING", "BAD", "OK"] {
        assert!(has_text(&rt, level), "no {level} row");
    }
    assert!(has_text(
        &rt,
        "59.8 fps · 1.2% dropped · audio bad · 18 Mbps"
    ));
}

#[test]
fn a_recording_without_health_says_so() {
    let mut rt = review_runtime();
    set_health(&mut rt, None);
    tap(&mut rt, "review-health");
    assert!(has_text(
        &rt,
        "No health data (recorded before 0.1.11-preview.14)"
    ));
    assert!(rt.find("health-summary").is_none());
}
