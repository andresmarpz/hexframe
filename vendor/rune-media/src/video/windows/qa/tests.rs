use super::window::{self, Host};
use super::*;
use crate::video::windows::audio::fake;

/// How far the clock may be from what is heard: under lip-sync's 45 ms.
const SYNC: f64 = 0.04;
const SIZE: (u32, u32) = (320, 180);
/// The fixture has a keyframe at every multiple of this picture number (x264 adds some on its own).
const KEYFRAME_EVERY: u32 = 32;
/// The fixture's last picture: 6 s at 60 fps.
const LAST: u32 = 359;
/// Beats come once a second, so a window of 2.2 s always holds two.
const LAYER_RUN: f64 = 2.7;
/// How long a seek may show the picture from before it.
const SEEK_MS: f64 = 250.0;

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
    // Until the seek's first picture is decoded, the surface keeps the one from before it.
    let landed = rig.until(1.0, "the seek while playing", |_, s| s.picture == Some(frame_of(s.time)));
    let took = landed.at.duration_since(sought).as_secs_f64() * 1000.0;
    eprintln!("the seek while playing showed its picture after {took:.1} ms");
    assert!(took < SEEK_MS, "the seek while playing showed its picture after {took:.1} ms");
    assert!((landed.time - 1.0).abs() < 0.5 / FPS, "the seek while playing landed at {:.4}", landed.time);
    let shots = rig.run(2.5);
    assert_pictures_follow(&shots, "after a seek while playing");
    let time = rig.player.time();
    let expected = landed.time + landed.at.elapsed().as_secs_f64();
    assert!((time - expected).abs() < 0.25, "2.5 s after the seek landed the time is {time:.3}, not {expected:.3}");
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
    rig.run_with(LAYER_RUN, |rig, _| {
        assert!(rig.layer(Some(SIZE)).showing, "the layer stopped showing");
    });
    let heard = fake::take();
    let moved = rig.presented().zip(presented).map(|(now, then)| now - then);
    assert!(moved.is_some_and(|m| m > LAYER_RUN - 0.3), "the layer presented {moved:?} s of pictures in {LAYER_RUN} s with window {:?}; with none, nothing takes the layer's frames", window::mode());
    let mut lateness = rig.take_present_lateness_ms();
    lateness.sort_by(f64::total_cmp);
    if !lateness.is_empty() {
        let p50 = lateness[lateness.len() / 2];
        let p95 = lateness[(lateness.len() * 95 / 100).min(lateness.len() - 1)];
        let max = *lateness.last().unwrap();
        eprintln!("layer present lateness ({} presents): p50={p50:.2} ms, p95={p95:.2} ms, max={max:.2} ms", lateness.len());
        assert!(p95 < 5.0, "layer present p95 lateness too high: {p95:.2} ms");
    }
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

/// The most pictures shown in any `window` of the times they were shown.
fn busiest(shown: &[Instant], window: Duration) -> usize {
    (0..shown.len()).map(|i| shown[i..].iter().take_while(|t| t.saturating_duration_since(shown[i]) < window).count()).max().unwrap_or(0)
}

#[test]
fn a_decoder_that_fell_behind_on_the_layer_jumps_to_the_clock_instead_of_playing_fast() {
    let mut rig = open_playing(true);
    let mut host = Host::open(window::mode()).expect("the layer's window");
    rig.run(0.5);
    rig.until(3.0, "the layer showing", |rig, _| {
        let status = rig.layer(Some(SIZE));
        if let (Some(host), Some(handle)) = (host.as_mut(), status.handle) {
            host.show(handle).expect("the layer on the window");
        }
        status.showing
    });
    let pipe = rig.player.shared.native.lock().unwrap().pipe.clone().expect("the decoder's pipe");
    let held = pipe.lock();
    std::thread::sleep(secs(1.2));
    drop(held);
    let (mut shown, mut last) = (Vec::new(), None);
    let end = Instant::now() + secs(0.8);
    while Instant::now() < end {
        let now = rig.presented();
        if now != last {
            shown.push(Instant::now());
            last = now;
        }
        std::thread::sleep(Duration::from_micros(100));
    }
    let most = busiest(&shown, secs(0.1));
    eprintln!("{} pictures shown in 0.8 s after a 1.2 s stall, at most {most} in 100 ms", shown.len());
    assert!(most <= 14, "{most} pictures shown within 100 ms after the stall: the decoder replayed what it missed at {:.1}x", most as f64 / 6.0);
    assert!(rig.layer(Some(SIZE)).showing, "the layer stopped showing");
    let clock = rig.clock().expect("a running clock");
    let on_layer = rig.presented().expect("a picture on the layer");
    let behind = (clock.position(Instant::now()) - on_layer).rem_euclid(6.0);
    assert!(behind < 0.1 || behind > 5.9, "the layer shows {on_layer:.3} s while the clock stands at {:.3} s", clock.position(Instant::now()));
}

/// `count` seeks spread over a second, as a drag on a seek bar sends them: `to` names each. The
/// window draws a frame at every refresh in between, so a picture that lands is drawn before the
/// next seek makes it stale.
fn storm(rig: &mut Rig, count: u32, fast: bool, to: impl Fn(u32) -> f64) -> Vec<Shot> {
    let mut shots = Vec::new();
    let step = Duration::from_secs(1) / count;
    let mut next = Instant::now();
    for i in 0..count {
        if fast { rig.player.seek_fast(to(i)) } else { rig.player.seek(to(i)) }
        next += step;
        loop {
            shots.push(rig.frame());
            let left = next.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            std::thread::sleep(REFRESH.min(left));
        }
    }
    shots
}

/// After a last seek to `to` while playing: pictures that name the time, moving on from `to`,
/// within `SEEK_MS`, and the sound back in sync.
fn assert_plays_on_from(rig: &mut Rig, to: f64, what: &str) {
    let sought = Instant::now();
    let landed = rig.until(2.0, what, |_, s| s.state == PlayerState::Playing && s.picture == Some(frame_of(s.time)) && s.time >= to);
    let took = landed.at.duration_since(sought).as_secs_f64() * 1000.0;
    eprintln!("{what}: playing again after {took:.1} ms");
    assert!(took < SEEK_MS, "{what}: playing again after {took:.1} ms");
    assert!(landed.time - to < 2.0 / FPS, "{what}: landed at {:.4}, not {to}", landed.time);
    fake::take();
    let shots = rig.run(2.0);
    assert_pictures_follow(&shots, what);
    let heard = fake::take();
    let back = first_sound(&heard, landed.at).map(|t| t.duration_since(landed.at).as_secs_f64());
    assert!(back.is_some_and(|b| b < 0.15), "{what}: the sound came back after {back:?} s");
    assert_in_sync(rig, &heard, landed.at + secs(0.5), what);
}

#[test]
fn a_storm_of_seeks_while_playing_plays_on_from_the_last() {
    let mut rig = open_playing(true);
    rig.run(0.5);
    storm(&mut rig, 60, false, |i| 0.5 + f64::from(i) * 0.06);
    rig.player.seek(1.5);
    assert_plays_on_from(&mut rig, 1.5, "after 60 seeks forward in a second");
    storm(&mut rig, 60, false, |i| if i % 2 == 0 { 4.5 } else { 0.8 });
    rig.player.seek(2.0);
    assert_plays_on_from(&mut rig, 2.0, "after 60 seeks back and forth in a second");
}

#[test]
fn a_scrub_shows_keyframes_then_plays_on_from_where_it_stops() {
    let mut rig = open_playing(true);
    rig.run(0.5);
    rig.player.pause();
    let shots = storm(&mut rig, 30, true, |i| 0.6 + f64::from(i) * 0.15);
    let keyframes: Vec<u32> = shots.iter().filter_map(|s| s.picture).filter(|p| *p > 0 && p % KEYFRAME_EVERY == 0).collect();
    eprintln!("the scrub showed keyframes {keyframes:?} among pictures {:?}", shots.iter().map(|s| s.picture).collect::<Vec<_>>());
    assert!(keyframes.len() >= 3, "the scrub showed keyframes {keyframes:?} among pictures {:?}", shots.iter().map(|s| s.picture).collect::<Vec<_>>());
    assert!(!rig.player.is_playing());
    rig.player.seek(3.3);
    rig.player.play();
    assert_plays_on_from(&mut rig, 3.3, "after a scrub");
}

#[test]
fn a_long_seek_lands_as_soon_as_a_short_one() {
    if !ffmpeg_available() {
        eprintln!("skipped: ffmpeg is not on PATH, and the long fixture needs it");
        return;
    }
    let mut rig = Rig::open(&long_fixture(), true);
    rig.player.play();
    rig.until(5.0, "the first picture", |_, s| s.picture.is_some());
    rig.run(0.5);
    rig.player.seek(500.0);
    assert_plays_on_from(&mut rig, 500.0, "10 min of 10 forward");
    rig.player.seek(20.0);
    assert_plays_on_from(&mut rig, 20.0, "8 min back");
    storm(&mut rig, 60, false, |i| f64::from(i) * 9.5);
    rig.player.seek(300.0);
    assert_plays_on_from(&mut rig, 300.0, "after 60 seeks across the clip in a second");
}
