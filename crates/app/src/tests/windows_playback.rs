use rune::{Key, PlayerState, Runtime};

use super::{press, press_char, rect, runtime_sized};
use crate::ui::fake_player::sim::{self, Media};
use crate::ui::{ReviewState, Ui, review_entry};

const TICK: f64 = 1.0 / 30.0;

struct Session {
    rt: Runtime<Ui>,
}

fn windows(media: Media) -> Session {
    sim::simulate(media);
    let mut ui = Ui::preview();
    ui.player_factory = sim::factory;
    let mut rt = runtime_sized(ui, (1400.0, 680.0));
    rt.on_key = Some(Ui::on_key);
    rt.update(|ui, _| ui.open_review_entry(review_entry()));
    rt.frame(1.0 / 60.0);
    let mut s = Session { rt };
    s.run(TICK);
    s
}

fn layered(duration: f64) -> Media {
    Media {
        seek_latency: 0.2,
        layered: true,
        ..Media::new(duration)
    }
}

impl Session {
    fn run(&mut self, secs: f64) {
        for _ in 0..(secs / TICK).round() as usize {
            sim::advance(TICK);
            self.rt.update(|ui, tx| {
                ui.poll(0.0, tx);
            });
            self.rt.frame(TICK);
        }
    }

    fn review(&self) -> &ReviewState {
        self.rt.state.review.as_ref().expect("review")
    }

    fn time(&self) -> f64 {
        self.review().time()
    }

    fn playing(&self) -> bool {
        self.review().is_playing()
    }

    fn space(&mut self) {
        press(&mut self.rt, Key::Text(" ".into()));
    }

    fn drag_to(&mut self, fraction: f32) {
        let [x, y, w, h] = rect(&self.rt, "timeline");
        let (px, py) = (x + w * fraction, y + h / 2.0);
        self.rt.pointer_down(px, py);
        self.rt.pointer_up(px, py);
        self.rt.frame(TICK);
    }

    fn heard_over(&mut self, secs: f64) -> f64 {
        sim::reset_heard();
        self.run(secs);
        sim::heard()
    }
}

fn assert_heard(heard: f64, secs: f64, what: &str) {
    assert!(
        heard >= secs - 0.5,
        "{what}: heard {heard:.2} s of {secs} s"
    );
}

#[test]
fn the_video_plays_its_own_sound_on_the_layer() {
    let mut s = windows(layered(60.0));
    s.run(0.5);
    let heard = s.heard_over(4.0);
    assert_heard(heard, 4.0, "layered playback");
}

#[test]
fn sound_comes_back_after_a_pause() {
    let mut s = windows(layered(60.0));
    s.run(2.0);
    s.space();
    s.run(1.0);
    s.space();
    let heard = s.heard_over(3.0);
    assert_heard(heard, 3.0, "after pause and resume");
}

#[test]
fn sound_comes_back_after_a_seek() {
    let mut s = windows(layered(60.0));
    s.run(1.0);
    s.drag_to(0.5);
    let heard = s.heard_over(3.0);
    assert_heard(heard, 3.0, "after a seek");
}

#[test]
fn sound_comes_back_after_mute_and_unmute() {
    let mut s = windows(layered(60.0));
    s.run(1.0);
    press_char(&mut s.rt, "m");
    s.run(0.5);
    assert!(s.heard_over(1.0) < 0.1, "muted but heard sound");
    press_char(&mut s.rt, "m");
    let heard = s.heard_over(3.0);
    assert_heard(heard, 3.0, "after unmute");
}

#[test]
fn no_sound_while_paused_or_at_other_speeds() {
    let mut s = windows(layered(60.0));
    s.run(1.0);
    s.space();
    assert!(s.heard_over(1.0) < 0.1, "heard sound while paused");
    s.space();
    s.rt.update(|ui, _| ui.review_set_speed(1));
    assert!(s.heard_over(1.0) < 0.1, "heard sound at half speed");
}

#[test]
fn a_seek_past_the_last_picture_shows_it_and_ends() {
    let mut s = windows(Media {
        pictures_end: 59.5,
        ..layered(60.0)
    });
    s.drag_to(0.999);
    s.run(0.5);
    assert!(sim::landed(), "the seek to the tail never landed");
    assert_eq!(sim::picture(), 59.5);
    s.run(1.0);
    let state = s.review().player.as_ref().map(|p| p.state());
    assert_eq!(state, Some(PlayerState::Ended), "at {:.2}", s.time());
}

#[test]
fn pausing_after_the_last_picture_does_not_wedge_the_player() {
    let mut s = windows(Media {
        pictures_end: 59.5,
        ..layered(60.0)
    });
    s.drag_to(0.97);
    s.run(1.0);
    assert!(sim::landed(), "the seek to the tail never landed");
    while s.time() < 59.6 {
        s.run(TICK);
    }
    for _ in 0..4 {
        s.space();
        s.run(0.1);
        assert!(sim::landed(), "a pause at {:.2} never settled", s.time());
    }
    s.run(1.0);
    let state = s.review().player.as_ref().map(|p| p.state());
    assert_eq!(state, Some(PlayerState::Ended), "at {:.2}", s.time());
    s.space();
    s.run(1.0);
    assert!(s.playing() && s.time() < 2.0, "restarted at {}", s.time());
}

#[test]
fn play_on_the_last_picture_restarts_the_clip_at_once() {
    let mut s = windows(layered(60.0));
    s.run(0.5);
    s.space();
    s.rt.update(|ui, _| ui.review_seek(60.0));
    s.run(0.5);
    assert!(sim::landed(), "the seek to the end never landed");
    assert!(!s.playing());
    assert_eq!(
        s.review().player.as_ref().map(|p| p.state()),
        Some(PlayerState::Ready)
    );
    s.space();
    s.run(1.0);
    assert!(s.playing(), "one press did not keep the clip playing");
    assert!(s.time() < 3.0, "restarted at {:.2}", s.time());
}
