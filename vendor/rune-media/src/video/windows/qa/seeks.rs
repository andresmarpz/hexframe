//! Seeks forward while a clip shaped like a Greyscreen recording plays, with the window drawing the
//! pictures and with the layer showing them: pictures of the new position must show within a
//! second and keep moving with the clock. A frozen picture with the sound and the time running
//! on fails them.

use super::super::layer::Layer;
use super::window::{self, Host};
use super::*;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::Win32::Foundation::{CompareObjectHandles, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE};
use windows::Win32::System::Threading::GetCurrentProcess;

const SIZE: (u32, u32) = (320, 180);
/// How long a seek may take to show pictures of its new position.
const LAND: f64 = 1.0;
/// How long the pictures must keep moving after it.
const WATCH: f64 = 10.0;
/// The longest one picture may stay while the clock runs.
const STILL: f64 = 0.3;
/// How far the layer's picture may trail the clock.
const BEHIND: f64 = 0.15;

fn open_recording() -> Option<Rig> {
    if !ffmpeg_available() {
        eprintln!("skipped: no ffmpeg (GREYSCREEN_QA_FFMPEG or PATH), and the recording fixture needs it");
        return None;
    }
    let mut rig = Rig::open(&recording_fixture(), true);
    rig.player.play();
    rig.until(5.0, "the first picture", |_, s| s.picture.is_some());
    Some(rig)
}

#[test]
fn a_seek_forward_in_a_recording_shows_the_new_pictures_and_plays_on() {
    let Some(mut rig) = open_recording() else { return };
    rig.run(5.0);
    rig.player.seek(150.0);
    drawn_plays_on_from(&mut rig, 150.0, "a seek 2.5 min forward");
    rig.player.seek(60.0);
    rig.run(0.05);
    rig.player.seek(200.0);
    drawn_plays_on_from(&mut rig, 200.0, "two seeks forward 50 ms apart");
}

/// After a seek to `to` with the window drawing the pictures: within `LAND` it draws the
/// picture the time names, at `to` or past it, and for `WATCH` every drawn picture names the
/// time, no picture stays longer than `STILL`, and the time keeps pace with the wall clock.
fn drawn_plays_on_from(rig: &mut Rig, to: f64, what: &str) {
    let sought = Instant::now();
    let landed = rig.until(LAND, what, |_, s| s.state == PlayerState::Playing && s.time >= to - 0.5 / FPS && s.picture == Some(frame_of(s.time)));
    eprintln!("{what}: the new pictures showed after {:.1} ms", landed.at.duration_since(sought).as_secs_f64() * 1000.0);
    let shots = rig.run(WATCH);
    let mut since = landed.clone();
    for s in &shots {
        assert_eq!(s.picture, Some(frame_of(s.time)), "{what}: drew picture {:?} at time {:.4}", s.picture, s.time);
        if s.picture != since.picture {
            since = s.clone();
        }
        let still = s.at.duration_since(since.at).as_secs_f64();
        assert!(still < STILL, "{what}: picture {:?} stayed {still:.2} s while playing; time {:.3}", s.picture, s.time);
    }
    let last = shots.last().expect("frames after the seek");
    let (played, wall) = (last.time - landed.time, last.at.duration_since(landed.at).as_secs_f64());
    assert!((played - wall).abs() < 0.25, "{what}: played {played:.3} s in {wall:.3} s");
}

#[test]
fn a_seek_forward_on_the_layer_shows_the_new_pictures_and_plays_on() {
    let Some(mut rig) = open_recording() else { return };
    let mut window = LayerWindow::open();
    rig.run_with(5.0, |rig, _| {
        window.step(rig);
    });
    assert_eq!(window.stage, Stage::Layered, "the layer never showed: {:?}", rig.layer(Some(SIZE)));
    rig.player.seek(150.0);
    layer_plays_on_from(&mut rig, &mut window, 150.0, "a seek 2.5 min forward on the layer");
    rig.player.seek(60.0);
    rig.run_with(0.05, |rig, _| {
        window.step(rig);
    });
    rig.player.seek(200.0);
    layer_plays_on_from(&mut rig, &mut window, 200.0, "two seeks forward 50 ms apart on the layer");
}

/// After a seek to `to` while the window shows the layer: within `LAND` the layer shows again
/// with a picture at `to` or past it, and for `WATCH` it keeps showing, the window's visual is
/// the surface the decoder presents to, and the picture there follows the clock.
fn layer_plays_on_from(rig: &mut Rig, window: &mut LayerWindow, to: f64, what: &str) {
    let sought = Instant::now();
    loop {
        rig.frame();
        if let Some(handle) = window.step(rig) {
            assert!(window.sees(handle), "{what}: the window shows a layer surface the decoder no longer presents to, so the picture freezes");
            if rig.presented().is_some_and(|p| p >= to - 1.0 / FPS) {
                break;
            }
        }
        let took = sought.elapsed().as_secs_f64();
        assert!(took < LAND, "{what}: no picture of the new position on the layer after {took:.2} s; window {:?}, presented {:?}", window.stage, rig.presented());
        std::thread::sleep(REFRESH);
    }
    eprintln!("{what}: the new pictures showed on the layer after {:.1} ms", sought.elapsed().as_secs_f64() * 1000.0);
    let (mut last, mut since) = (rig.presented(), Instant::now());
    rig.run_with(WATCH, |rig, shot| {
        let handle = window.step(rig);
        let handle = handle.unwrap_or_else(|| panic!("{what}: the layer stopped showing; window {:?}, time {:.3}", window.stage, shot.time));
        assert!(window.sees(handle), "{what}: the window shows a layer surface the decoder no longer presents to, so the picture freezes");
        let now = rig.presented();
        if now != last {
            (last, since) = (now, Instant::now());
        }
        let still = since.elapsed().as_secs_f64();
        assert!(still < STILL, "{what}: the layer's picture {last:?} stayed {still:.2} s while playing; time {:.3}", shot.time);
        let clock = rig.clock().expect("a running clock").position(Instant::now());
        let behind = clock - now.unwrap_or(f64::NAN);
        assert!(behind.abs() < BEHIND, "{what}: the layer shows {now:?} while the clock stands at {clock:.3}");
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Drawn,
    Placing,
    Layered,
}

/// The window's side of the layer as Rune's `video_layer` runs it for a clip whose place holds
/// still: it asks for the layer while the clip plays, shows the layer's surface once a picture
/// is there, takes the pictures back when the clip stops playing, and keeps its visual while the
/// layer's surface handle has the same value.
struct LayerWindow {
    stage: Stage,
    /// The handle value the visual was made from, and a handle of the test's own to its surface.
    visual: Option<(usize, OwnedHandle)>,
    host: Option<Host>,
}

impl LayerWindow {
    fn open() -> Self {
        Self { stage: Stage::Drawn, visual: None, host: Host::open(window::mode()).expect("the layer's window") }
    }

    /// One frame's step, after the frame pulled the clip: the layer's surface handle while the
    /// layer shows the pictures.
    fn step(&mut self, rig: &Rig) -> Option<usize> {
        let ask = (self.stage != Stage::Drawn).then_some(SIZE);
        let status = rig.layer(ask);
        let ready = status.running && !status.refused;
        match self.stage {
            Stage::Drawn if ready => {
                self.stage = Stage::Placing;
                rig.layer(Some(SIZE));
                None
            }
            Stage::Drawn | Stage::Placing if !ready => {
                self.stage = Stage::Drawn;
                rig.layer(None);
                None
            }
            Stage::Placing if status.showing => {
                let handle = status.handle.expect("a layer that shows has a surface");
                self.show(handle);
                self.stage = Stage::Layered;
                Some(handle)
            }
            Stage::Drawn | Stage::Placing => None,
            Stage::Layered if ready && status.showing => status.handle,
            Stage::Layered => {
                self.stage = Stage::Drawn;
                rig.layer(None);
                rig.unlayer();
                None
            }
        }
    }

    /// Makes the visual from `handle` unless it was made from a handle of the same value.
    fn show(&mut self, handle: usize) {
        if self.visual.as_ref().is_some_and(|(value, _)| *value == handle) {
            return;
        }
        self.visual = Some((handle, duplicate(handle)));
        if let Some(host) = &mut self.host {
            host.show(handle).expect("the layer on the window");
        }
    }

    /// Whether the visual shows the surface `handle` names.
    fn sees(&self, handle: usize) -> bool {
        // SAFETY: two open handles of this process.
        self.visual.as_ref().is_some_and(|(_, own)| unsafe { CompareObjectHandles(HANDLE(own.as_raw_handle()), HANDLE(handle as *mut _)) }.as_bool())
    }
}

/// A handle of the test's own to the object `handle` names, open after the decoder closes its own.
fn duplicate(handle: usize) -> OwnedHandle {
    let mut own = HANDLE::default();
    // SAFETY: duplicates an open handle of this process into this process; `own` is ours to close.
    unsafe {
        let process = GetCurrentProcess();
        DuplicateHandle(process, HANDLE(handle as *mut _), process, &mut own, 0, false, DUPLICATE_SAME_ACCESS).expect("a handle to the layer's surface");
        OwnedHandle::from_raw_handle(own.0)
    }
}

#[test]
fn a_new_layer_never_takes_the_surface_handle_of_one_that_closed() {
    let mut rig = Rig::open(&fixture(), false);
    rig.player.play();
    rig.until(5.0, "the first picture", |_, s| s.picture.is_some());
    let picture = rig.player.shared.native.lock().unwrap().facts.and_then(|f| f.picture).expect("the clip's picture");
    let device = super::super::gpu::device().expect("the decoders' device");
    let mut closed: Vec<usize> = Vec::new();
    for i in 0..20 {
        let layer = Layer::new(device.clone(), picture, SIZE).expect("a layer");
        let handle = layer.handle();
        let recent = &closed[closed.len().saturating_sub(8)..];
        assert!(!recent.contains(&handle), "layer {i} took surface handle {handle:#x} of a layer that closed: a window that keys its visual on the value keeps showing the closed one");
        closed.push(handle);
        drop(layer);
    }
}
