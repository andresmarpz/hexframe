# Greyscreen's patches to rune-media

Base: Rune `eb9f11047bbe2b3927ad4b65ffcf7bfef4d3deb8`, `crates/rune-media`. Every change is in the Windows video path (`src/video/windows`), apart from test helpers and the `qa` feature (section 4). Each change is a commit titled `vendor: rune-media ...` (a patch, then its follow-up fixes), so each can go upstream as its own pull request.

Besides the code, `Cargo.toml` has its `workspace = true` fields made concrete, the other Rune crates come from git at the base rev, and a `[patch.crates-io]` for Rune's `wgpu-hal` lets the crate test on its own. Upstream PRs leave those out.

## 1. Pause on the layer holds the layer's picture and time

Upstream: anomalyco/rune#1320.

- Problem: while a clip plays on its compositor layer, the decoder presents the pictures and the UI's surface keeps the picture from when the layer took over. A pause took its time from that stale picture, and the window, drawing the clip again, showed it. The picture jumped back, then forward on play.
- Change:
  - The decoder records the newest picture it presented on the layer and kept in the queue (`Inbox::layer_presented`).
  - `Native::behind` marks a surface the layer got ahead of.
  - A pause while layered takes its time from that picture. A pause after the window let go of the layer without drawing the clip again takes the clock's time. Either way the clip's time is set at once.
  - When the window draws a paused or ended clip again, `hold` puts the picture at the pause's time on the surface. If the pause came while the decoder presented the next picture (which drops the one before), it takes that next picture and moves the pause to it. If the picture is gone, a paused clip seeks to it (in the clip's own time).
  - A new `machine` arm, `Paused + Pause`, moves the hold.
- Files: `decoder.rs`, `mod.rs`, `machine.rs` (tests in `machine/tests.rs`).

## 2. A seek past the last picture shows it

Upstream: anomalyco/rune#1321, and its note about parked clips.

- Problem: a seek after the last picture dropped every picture before the target and reached the end with nothing to show. The phase stayed Buffering forever, and play and pause couldn't recover it.
- Change:
  - The decoder remembers the newest picture it dropped for the seek. If the video stream ends with nothing kept, it seeks back to that picture once and shows it.
  - A seek to the duration or past it goes to half a picture before the end, so it lands on the last picture and the sound reader doesn't read the next loop.
  - If the decoder still ends with nothing, Buffering ends anyway: a playing clip ends, and a paused one holds at the target. This is a new `machine` arm, `Buffering + Ended`.
  - A seek on a parked clip (no decoder after 3 s undrawn) forgets which picture is shown, so the clip opens again and shows the seek's picture instead of waiting for play.
- Files: `decoder.rs`, `machine.rs` (with tests in `machine/tests.rs`), `mod.rs`.

## 3. A video's sound reads ahead and leads the clock

Upstream: anomalyco/rune#1325.

- Problem: the sound output writes about 110 ms ahead of the clock, but the decoder read audio only as far as the video read went: 3 pictures (50 ms at 60 fps), and about one while the layer waits for each picture's time. Every chunk came late and was dropped, so a 60 fps video was silent. Also:
  - With sound on, the end waited for the audio stream's end as well as the video's.
  - An edit before `Opened` deselected the audio stream (`sound` was None while `facts` was unknown).
  - The sound followed the clock and jumped back in line only past 40 ms, so the device clock's drift showed as up to 40 ms of offset and then a skip.
- Change:
  - `ahead.rs`: a video's sound comes from its own source reader on its own thread. It keeps 1 s of sound queued past what is heard (`audio::AHEAD`), whatever the picture queue or the layer does. It seeks with the clip, loops on its own at the duration (only when the loop had sound), and stops with the output. The decoder's reader now reads the video stream only, so the clip ends at the video's end. Audio clips still read their sound on the decoder's reader.
  - `audio.rs`, timing: the time a written sample will be heard comes from the device's position (`IAudioClock::GetPosition`, stamped with the performance counter): what was written but not yet played is still to come. Before a start, and on a device without a usable position, it comes from the buffer padding plus `GetStreamLatency`, corrected by what earlier starts measured (`bias`).
  - `audio.rs`, sound as master: while the sound plays on without a jump, it reports how far it runs from the clock. When the average passes 4 ms (a quarter of a picture at 60 fps), `Clock::nudge` moves the clock to the sound, and the pictures follow it (`Event::Synced` in `machine`, and the layer's present waits re-read the clock). The 40 ms jump stays for starts, seeks and stalls only.
  - `audio.rs`, stops: a pause, stall or rate change stops and empties the device, which drops up to 100 ms of sound already taken from the queue. The reader reads it again from 0.1 s before the clock stopped, unless a seek cleared the queue first, so a resume isn't silent for its first 100 ms.
  - `mod.rs`: until the clip opens, the decoder keeps the sound it was asked for.
- Files: `ahead.rs` (new), `audio.rs` (with tests in `audio/tests.rs`), `decoder.rs`, `machine.rs`, `mod.rs`.

## 4. Real-player tests that nobody notices (`qa` feature)

Upstream: none. Test-only, except for the sound output's split below.

- Problem: the bugs above lived in the real Media Foundation player (the layer, WASAPI timing, seeks), which headless runs never open. Checking it meant driving the app on a person's PC: real input, focus changes and sound out loud.
- Change:
  - `audio.rs`: the sound loop runs against an `Endpoint` (`audio/endpoint.rs`): `Wasapi` is the old stream, with the same calls in the same order (the stop and the event's close moved into its `Drop`). The timing (device position to heard time, nudges, bias, stops and re-reads) is untouched and shared.
  - `audio/fake.rs`, with `qa` only: a test device in place of WASAPI. It takes sound on a 10 ms period from a 100 ms buffer at the clip's rate, keeps what it played and when it was heard, and reports a 10 ms latency while its position lags 30 ms, so the start's estimate is wrong and the nudges must fix it. A `qa` build has no WASAPI output at all: nothing can reach a speaker. It doesn't cover a real device's latency, position reports, period or a device change.
  - `qa.rs`, `qa/tests.rs`: the real player on a headless wgpu D3D12 device (`note_device`), so pictures decode on the GPU into the shared ring. Each frame pulls the clip and draws it with Rune's surface pipeline into an offscreen texture that is read back. The fixture's pictures carry their frame number as ten black or white bars, so each drawn picture is checked against `Player::time`. Sound is checked for level, for the time each beat (a tone at each whole second) is heard against the clock, and for silence when paused or not at 1x. Tests: play, sound in sync, pause and resume, seeks (paused, playing, past the end; a seek's picture must replace the one from before it within 250 ms), 2x, and the layer: it presents with sound in sync, and a pause there shows the paused picture when the window takes the pictures back. Section 5 added a storm of 60 seeks in a second while playing, a scrub of fast seeks, and long seeks on a 10-minute clip that `long_fixture` makes with ffmpeg in the temp folder; each must play on from the last seek within 250 ms with sound in sync.
  - `qa/window.rs`: the layer's window, by `RUNE_MEDIA_QA_WINDOW`. `none` (default): the layer's surface is on no visual. `hidden`: a cloaked 1x1 tool window, never shown, whose DirectComposition target shows the layer. `cloaked`: the same window shown with `SW_SHOWNOACTIVATE` after `DWMWA_CLOAK` reads back, which Microsoft documents as "not visible to the user" and "still composed by DWM". It fails, and shows nothing, if the cloak doesn't take or the foreground window changes.
  - `audio/fake/tests.rs`: the sound loop on the test device alone: what is heard follows the clock once the start settles, and a pause stops it and asks the reader for what the device dropped.
  - `native_surface/tests.rs`: the offscreen GPU helpers are `pub(crate)` for the player tests.
  - `fixtures/qa-bars-60fps.m4v` (73 KB, `.m4v` because the repo ignores `*.mp4`): 6 s, 320x180, 60 fps H.264, 48 kHz stereo AAC, made by `fixtures/make-qa-bars.sh`. Like Greyscreen's own recordings it has no B-frames and no edit lists (no `elst`, no `ctts`), so a player that ignores edit lists sees the same times as one that applies them; the audio starts 1024 samples early to cancel the AAC delay that no edit list hides. A first version had B-frames and an edit list that skipped two frames, and the Windows player showed every picture two frames late.
- Run on Windows, from the repo root: `cargo test --manifest-path vendor/rune-media/Cargo.toml --target-dir target/vendor --features qa --lib video::windows -- --test-threads=1 --nocapture`. Keep the `video::windows` filter: the tests turn on playback for the whole process, which other headless tests in the crate don't expect.
- Files: `Cargo.toml` (`qa` feature), `audio.rs`, `audio/endpoint.rs`, `audio/fake.rs`, `audio/fake/tests.rs`, `qa.rs`, `qa/tests.rs`, `qa/window.rs`, `mod.rs`, `native_surface/mod.rs`, `native_surface/tests.rs`, `fixtures/`.

## 5. Seeks never block the UI, a scrub can ask for keyframes, and a wedged seek recovers

Upstream: anomalyco/rune#1336.

- Problem: a drag on a seek bar sends a seek per pointer move. The decoder thread held the inbox lock while Media Foundation moved its source reader (`SetCurrentPosition`), and the UI thread takes that lock for every seek and every pull. So each seek of the drag stalled the UI thread for a whole source seek, the play and pause keys waited behind the drag, and a 20-minute recording took seconds to play again. A seek that never delivered a picture left the phase in Buffering with nothing to end it.
- Change:
  - `decoder.rs`: the decoder takes the newest seek (`Inbox::seek`, a `Seek` with its time, generation and `fast`), lets go of the inbox, then seeks the source and the sound. It checks the inbox again before it reads, so a newer seek replaces this one and only the newest is decoded.
  - `Player::seek_fast`: a seek that shows the first picture after the source's seek, the keyframe at or before the time, without decoding on to the exact picture. For previews while scrubbing. `Player::time` reports the keyframe's time once it lands. macOS and the other players take it as a plain seek.
  - `Player::seeking`: a seek (or a stall, or an open) still waits for its picture. On Windows it follows the phase (`Control::landing`); on macOS, the player's `Settle`.
  - `machine.rs`: `Event::Seek` and `Effect::Seek` carry `fast`. `overdue`: a Buffering phase that has waited past `STUCK` (2 s) for its picture. `mod.rs` then stops the decoder and opens a new one at the phase's time (`Native::restart`), playing or paused as before. Each seek starts the wait again.
  - The machine's tests also run on hosts other than Windows (`video::machine`): they are plain data.
- Files: `decoder.rs`, `machine.rs` (tests in `machine/tests.rs`), `mod.rs`, `qa.rs`, `qa/tests.rs`, `../mod.rs` and `../phases.rs` (the `Player` API and `Control`).

## 6. The layer drops pictures that are late instead of presenting them fast

Upstream: anomalyco/rune#1335.

- Problem: the decoder presents each picture on the layer as soon as its time has passed, and nothing dropped late ones. A decoder that fell behind the clock (the clock follows the sound) replayed its backlog back to back until it caught up, so the picture ran fast for a moment and then went normal.
- Change: on the layer, a picture more than `LAYER_LATE` (50 ms) past its time on the clock is decoded but not copied or presented, so the layer jumps to the clock. The layer still shows a picture at least every `LAYER_GAP` (250 ms), so a decoder slower than the clip still moves. A fast seek's first picture and a seek's own landing are never dropped (no clock runs for them yet).
- Files: `decoder.rs` (`late_for_layer`), test `a_decoder_that_fell_behind_on_the_layer_jumps_to_the_clock_instead_of_playing_fast` in `qa/tests.rs` (it holds the decoder's lock for 1.2 s, then counts the pictures shown per 100 ms; it needs a Windows run, with and without the change).

## 7. A seek storm no longer starves the decoder of textures

Upstream: anomalyco/rune#1334.

- Problem: a texture of the ring is free again only when Rune's draw of its picture signals the release fence. A picture that is thrown away without a draw never does. During a burst of seeks the decoder is a few seeks behind the window, so the window threw away every picture it decoded as stale. After six of them (the ring's size) no texture was free, no draw came to release one, and the decoder waited for ever for the picture of the last seek until the 2 s restart.
- Change: the window hands the textures of pictures it drops as stale to the decoder (`Inbox::unread`, `Inbox::drop_stale`), which frees them at once, and the decoder does the same for the queue a seek flushes. A slot is freed only if it still holds that picture (`Ring::unread` takes the fence value it was written with). The layer path already freed its dropped pictures this way.
- Files: `decoder.rs`, `gpu.rs`, `mod.rs`. Tests: `a_storm_of_seeks_while_playing_plays_on_from_the_last` and `a_scrub_shows_keyframes_then_plays_on_from_where_it_stops` in `qa/tests.rs` (both failed before, with the log showing `decoder waits: ... texture false` after the first seeks). The scrub test now names the fixture's keyframe spacing (every 32 pictures), and a PC without ffmpeg skips the long-seek test.

## 8. Playback events reach the app's log without `RUNE_VIDEO_LOG`

Upstream: none (Rune logs only to stderr under `RUNE_VIDEO_LOG`).

- Problem: a freeze on a user's PC left no trace: the player's own log is off in shipped builds, and the app's stall report only sees `Player::time`, which keeps moving while the layer's clock runs.
- Change: `watch.rs` and a few calls log through `tracing`, a few lines per seek or pause and nothing per picture: each phase change with its time, each seek (from, to, generation, fast), each wedged-seek restart and decoder failure, a decoder wait over 500 ms for a texture or for room while the clock runs, a source seek or read over 500 ms, and one `picture frozen: clock advanced N s since the last presented picture` warning when the clock runs over 1 s with no new picture on the surface or the layer (and one line when it moves again).
- Files: `Cargo.toml` (`tracing`), `watch.rs` (new), `mod.rs`, `decoder.rs`.

## 9. Layer presents align to the display refresh with a high-resolution timer

Upstream: none yet.

- Problem: `Decoder::present_when_due` waited on `Condvar::wait_timeout` without `timeBeginPeriod`, so waits quantized to Windows' 15.6 ms timer tick, and did not snap layer presents to DWM's refresh grid. On a 144 Hz display a 60 fps clip showed uneven cadence and late presents.
- Change:
  - `vsync.rs`: added `nearest` to snap a picture's clock due time to the closest refresh on DWM's vblank grid (`DwmGetCompositionTimingInfo`).
  - `decoder.rs`: replaced the layer present `Condvar::wait_timeout` with a `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` waitable timer plus a manual-reset Win32 wake event on `Pipe` so seeks, pauses, stops and unlayering still wake immediately, followed by a short sub-millisecond yield spin.
  - `watch.rs`: added `PresentLateness` to record wake lateness per present and log one summary line (count, p50, p95, max) per play on pause, seek, unlayer, end or close.
  - `qa.rs`, `qa/tests.rs`: exposed `Rig::present_lateness_ms` under `qa` and added a 60 fps layer present lateness check.
- Files: `vsync.rs`, `watch.rs`, `decoder.rs`, `qa.rs`, `qa/tests.rs`.

## 9. A layer made again never takes the closed layer's surface handle

Upstream: anomalyco/rune#1333. Rune's `window/win32/video_layer.rs` could also key its visual on more than the handle's value.

- Problem: a seek (or a pause) while the layer showed froze the picture for good while the sound and the time ran on. The window lets go of the layer when the clock stops, and the decoder closes the layer and its DirectComposition surface handle. When the seek lands and the window asks again, the decoder makes a new layer, and Windows often gives its handle the value of the one just closed. Rune's window makes a new visual only when the handle's value changes (`video_layer::show`), so it kept the old visual, with the closed surface and its last picture from before the seek, and punched the UI over it. The decoder presented every new picture to a surface nothing showed. `Player::time` follows the layer's clock, so the time moved and the app's stall report saw nothing.
- Change: `layer.rs` remembers the values of the last 16 closed layer handles. A new handle with one of those values stays open while the next is made, then closes, so a new layer always has a value the window hasn't shown. Each new layer logs its handle to the app's log, and a skipped value is logged too.
- Files: `layer.rs`. Tests in `qa/seeks.rs`: `a_new_layer_never_takes_the_surface_handle_of_one_that_closed` makes 20 layers in turn and checks each handle against the 8 closed before it, and `a_seek_forward_on_the_layer_shows_the_new_pictures_and_plays_on` runs the window's side of the layer as Rune does through seeks on a 4-minute 1440p recording and checks that its visual shows the surface the decoder presents to (`CompareObjectHandles`). `a_seek_forward_in_a_recording_shows_the_new_pictures_and_plays_on` does the same seeks with the window drawing the pictures. The recording is made with ffmpeg (`GREYSCREEN_QA_FFMPEG` or PATH); without it those two skip.

## Known gaps

- A clip with sound that nobody draws (window minimised, scrolled away) keeps its sound and clock running while its decoder waits with a full picture queue. Drawn again, the pictures stall and the clock goes back to the newest picture, so some sound plays twice. Upstream behaves the same; seeking the decoder to the clock would fix it.
- Audio clips (`Player::audio`) still lose the sound a stop drops: nothing reads it again.

## Not verified on the Mac

The Mac only compile-checks the Windows path (`cargo xwin check --tests`, and with `--features qa`), and runs the machine's tests. The Windows tests (`machine/tests.rs`, `audio/tests.rs`, `ahead.rs`, and with `qa` the player tests in `qa/tests.rs` and `audio/fake/tests.rs`) and the behaviour itself need a Windows run. For the layer, that run must also show whether a composition swap chain keeps presenting with no window (`none`), under a never-shown window (`hidden`) and under a cloaked one (`cloaked`), and that the cloaked window never shows, never takes focus and has no taskbar button.
