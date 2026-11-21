use super::window::{self, Host};
use super::*;
use crate::video::windows::audio::fake;

/// How far the clock may be from what is heard: under lip-sync's 45 ms.
const SYNC: f64 = 0.04;
const SIZE: (u32, u32) = (320, 180);
/// The fixture's last picture: 6 s at 60 fps.
const LAST: u32 = 359;

fn secs(d: f64) -> Duration {
    Duration::from_secs_f64(d)
}

/// Every drawn picture is the one `time` names, and they move on.
fn assert_pictures_follow(shots: &[Shot], what: &str) {
    let drawn: Vec<(f64, u32)> = shots.iter().filter_map(|s| s.picture.map(|p| (s.time, p))).collect();
    for (time, picture) in &drawn {
        assert_eq!(*picture, frame_of(*time), "{what}: drew picture {picture} at time {time:.4}");
    }
    let mut distinct: Vec<u32> = drawn.iter().map(|(_, p)| *p).collect();
    distinct.dedup();
    assert!(distinct.len() * 2 >= drawn.len(), "{what}: only {} pictures in {} frames", distinct.len(), drawn.len());
}

fn assert_in_sync(rig: &Rig, heard: &[fake::Heard], after: Instant, what: &str) {
    let offsets = rig.offsets(heard, after);
    eprintln!("{what}: the clock ran {offsets:.4?} s past each beat heard");
    assert!(offsets.len() >= 2, "{what}: heard {} beats", offsets.len());
    for o in &offsets {
        assert!(o.abs() < SYNC, "{what}: the sound was {:.1} ms off the clock", o * 1000.0);
    }
}

fn open_playing(sound: bool) -> Rig {
    let mut rig = Rig::open(&fixture(), sound);
    rig.player.play();
    rig.until(5.0, "the first picture", |_, s| s.picture.is_some());
    let decoding = rig.decoding();
    eprintln!("pictures on the GPU, hardware decoder: {decoding:?}");
    assert_eq!(decoding.map(|(gpu, _)| gpu), Some(true), "the pictures didn't reach Rune on the GPU");
    rig
}

#[test]
fn pictures_follow_the_clock_and_name_the_time() {
    let mut rig = open_playing(false);
    let shots = rig.run(2.0);
    assert_eq!(rig.player.state(), PlayerState::Playing);
    assert_pictures_follow(&shots, "playing");
    let (first, last) = (&shots[0], &shots[shots.len() - 1]);
    let played = last.time - first.time;
    let wall = last.at.duration_since(first.at).as_secs_f64();
    assert!((played - wall).abs() < 0.05, "played {played:.3} s in {wall:.3} s");
}

#[test]
fn sound_is_heard_in_sync_with_the_clock() {
    let mut rig = open_playing(true);
    let start = Instant::now();
    let shots = rig.run(4.0);
    let heard = fake::take();
    assert_pictures_follow(&shots, "playing with sound");
    let level = rms(&heard, start + secs(0.5), Instant::now());
    assert!(level > 0.05, "the sound was silent: rms {level:.4}");
    assert_in_sync(&rig, &heard, start + secs(1.0), "playing");
}

#[test]
fn a_pause_holds_its_picture_and_silences_the_sound() {
    let mut rig = open_playing(true);
    rig.run(1.5);
    rig.player.pause();
    let paused = Instant::now();
    let shots = rig.run(0.8);
    let time = rig.player.time();
    assert!(shots.iter().all(|s| (s.time - time).abs() < 1e-9), "the time moved while paused");
    assert_eq!(shots.last().and_then(|s| s.picture), Some(frame_of(time)), "the paused picture isn't the paused time");
    let heard = fake::take();
    assert!(heard.iter().all(|h| h.at < paused + secs(0.1)), "sound played on after the pause");

    rig.player.play();
    let resumed = Instant::now();
    let shots = rig.run(2.5);
    assert_pictures_follow(&shots, "resumed");
    let heard = fake::take();
    let back = first_sound(&heard, resumed).map(|t| t.duration_since(resumed).as_secs_f64());
    assert!(back.is_some_and(|b| b < 0.15), "the sound came back after {back:?} s");
    assert_in_sync(&rig, &heard, resumed + secs(0.5), "resumed");
}

#[test]
fn a_seek_lands_on_its_picture_paused_and_playing() {
    let mut rig = open_playing(true);
    rig.run(0.5);
    rig.player.pause();
    rig.player.seek(3.0);
    let shot = rig.until(2.0, "the paused seek", |_, s| s.picture == Some(frame_of(3.0)));
    assert!((shot.time - 3.0).abs() < 0.5 / FPS, "the paused seek stands at {:.4}", shot.time);

    rig.player.play();
    rig.run(0.5);
    fake::take();
    rig.player.seek(1.0);
    let sought = Instant::now();
    let shots = rig.run(2.5);
    assert_pictures_follow(&shots, "after a seek while playing");
    let time = rig.player.time();
    assert!((time - 3.5).abs() < 0.25, "2.5 s after a seek to 1 s the time is {time:.3}");
    assert_in_sync(&rig, &fake::take(), sought + secs(0.5), "after a seek");
}

#[test]
fn a_seek_past_the_end_shows_the_last_picture() {
    let mut rig = open_playing(false);
    rig.player.pause();
    rig.player.seek(10.0);
    rig.until(2.0, "the last picture after a paused seek past the end", |_, s| s.picture == Some(LAST));
    rig.player.play();
    rig.player.seek(10.0);
    let shot = rig.until(2.0, "the end after a seek past it while playing", |_, s| s.state == PlayerState::Ended);
    assert_eq!(shot.picture, Some(LAST));
}

#[test]
fn other_speeds_keep_the_pictures_and_silence_the_sound() {
    let mut rig = open_playing(true);
    rig.run(1.0);
    rig.player.set_rate(2.0);
    let fast = Instant::now();
    let before = rig.player.time();
    let shots = rig.run(1.0);
    assert_pictures_follow(&shots, "at 2x");
    let played = rig.player.time() - before;
    assert!((played - 2.0).abs() < 0.15, "played {played:.3} s in 1 s at 2x");
    let heard = fake::take();
    assert!(heard.iter().all(|h| h.at < fast + secs(0.1)), "sound played at 2x");

    rig.player.set_rate(1.0);
    let normal = Instant::now();
    rig.run(2.5);
    assert_in_sync(&rig, &fake::take(), normal + secs(0.5), "back at 1x");
}

#[test]
fn the_layer_plays_with_sound_and_a_pause_there_holds_its_picture() {
    let mut rig = open_playing(true);
    let mut host = Host::open(window::mode()).expect("the layer's window");
    rig.run(0.5);
    let mut committed = None;
    rig.until(3.0, "the layer showing", |rig, _| {
        let status = rig.layer(Some(SIZE));
        assert!(!status.refused, "the decoder refused the layer");
        if let (Some(host), Some(handle)) = (host.as_mut(), status.handle) {
            committed = Some(host.show(handle).expect("the layer on the window"));
        }
        status.showing
    });
    eprintln!("window {:?}, commit taken: {committed:?}", window::mode());
    fake::take();
    let layered = Instant::now();
    let presented = rig.presented();
    rig.run_with(2.0, |rig, _| {
        assert!(rig.layer(Some(SIZE)).showing, "the layer stopped showing");
    });
    let heard = fake::take();
    let moved = rig.presented().zip(presented).map(|(now, then)| now - then);
    assert!(moved.is_some_and(|m| m > 1.7), "the layer presented {moved:?} s of pictures in 2 s with window {:?}; with none, nothing takes the layer's frames", window::mode());
    let level = rms(&heard, layered + secs(0.2), Instant::now());
    assert!(level > 0.05, "the sound was silent on the layer: rms {level:.4}");
    assert_in_sync(&rig, &heard, layered + secs(0.5), "on the layer");

    rig.player.pause();
    let time = rig.player.time();
    let on_layer = rig.presented().unwrap_or(f64::NAN);
    assert!((time - on_layer).abs() < 1.5 / FPS, "paused at {time:.4} while the layer showed {on_layer:.4}");
    rig.layer(None);
    rig.unlayer();
    let first = rig.frame();
    eprintln!("first frame after the layer: picture {:?} at time {:.4}", first.picture, first.time);
    // The held picture may be gone from the queue: then the paused clip seeks to it.
    let shot = rig.until(1.0, "the paused picture after the layer", |_, s| s.picture == Some(frame_of(s.time)));
    assert!((shot.time - time).abs() < 2.0 / FPS, "the pause moved from {time:.4} to {:.4}", shot.time);
    let held = rig.run(0.5);
    assert!(held.iter().all(|s| s.picture == shot.picture && s.time == shot.time), "the paused picture moved");
}
