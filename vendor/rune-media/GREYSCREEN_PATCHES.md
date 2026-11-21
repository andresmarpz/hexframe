# Greyscreen's patches to rune-media

Base: Rune `eb9f11047bbe2b3927ad4b65ffcf7bfef4d3deb8`, `crates/rune-media`. Every change is in the Windows video path (`src/video/windows`). Each one is its own commit on Greyscreen's `main`, titled `vendor: rune-media ...`, so each can go upstream as its own pull request.

Besides the code, `Cargo.toml` has its `workspace = true` fields made concrete, the other Rune crates come from git at the base rev, and a `[patch.crates-io]` for Rune's `wgpu-hal` lets the crate test on its own. Upstream PRs leave those out.

## 1. Pause on the layer holds the layer's picture and time

Upstream: anomalyco/rune#1320.

- Problem: while a clip plays on its compositor layer, the decoder presents the pictures and the UI's surface keeps the picture from when the layer took over. A pause took its time from that stale picture, and the window, drawing the clip again, showed it. The picture jumped back, then forward on play.
- Change:
  - The decoder records the newest picture it presented on the layer (`Inbox::layer_presented`), and that picture stays at the front of the queue.
  - A pause while layered takes its time from that picture and sets the clip's time to it at once.
  - `Native::behind` marks a surface the layer got ahead of. When the window draws a paused or ended clip again, `show` puts the held picture on the surface. If it already left the queue, the clip seeks to it.
- Files: `decoder.rs`, `mod.rs`.

## 2. A seek past the last picture shows it

Upstream: anomalyco/rune#1321, and its note about parked clips.

- Problem: a seek after the last picture dropped every picture before the target and reached the end with nothing to show. The phase stayed Buffering forever, and play and pause couldn't recover it.
- Change:
  - The decoder remembers the newest picture it dropped for the seek. If the video stream ends with nothing kept, it seeks back to that picture once and shows it.
  - If the decoder still ends with nothing, Buffering ends anyway: a playing clip ends, and a paused one holds at the target. This is a new `machine` arm, `Buffering + Ended`.
  - A seek on a parked clip (no decoder after 3 s undrawn) clears the surface's picture, so the clip opens again and shows the seek's picture instead of waiting for play.
- Files: `decoder.rs`, `machine.rs` (with tests in `machine/tests.rs`), `mod.rs`.

## 3. A video's sound reads ahead and leads the clock

Upstream: no issue filed yet. The write-up is "Windows: a video's own sound is silent at 60 fps and on the layer".

- Problem: the sound output writes about 110 ms ahead of the clock, but the decoder read audio only as far as the video read went: 3 pictures (50 ms at 60 fps), and about one while the layer waits for each picture's time. Every chunk came late and was dropped, so a 60 fps video was silent. Also:
  - With sound on, the end waited for the audio stream's end as well as the video's.
  - An edit before `Opened` deselected the audio stream (`sound` was None while `facts` was unknown).
  - The sound followed the clock and jumped back in line only past 40 ms, so the device clock's drift showed as up to 40 ms of offset and then a skip.
- Change:
  - `ahead.rs`: a video's sound comes from its own source reader on its own thread. It keeps 1 s of sound queued past what is heard (`audio::AHEAD`), whatever the picture queue or the layer does. It seeks with the clip, loops on its own at the duration, and stops with the output. The decoder's reader now reads the video stream only, so the clip ends at the video's end. Audio clips still read their sound on the decoder's reader.
  - `audio.rs`, timing: the time a written sample will be heard comes from the device's position (`IAudioClock::GetPosition`, stamped with the performance counter): what was written but not yet played is still to come. A device without a usable position falls back to the buffer padding plus `GetStreamLatency`.
  - `audio.rs`, sound as master: while the sound plays on without a jump, it reports how far it runs from the clock. When the average passes 8 ms, `Clock::nudge` moves the clock to the sound, and the pictures follow it (`Event::Synced` in `machine`, and the layer's present waits re-read the clock). The 40 ms jump stays for starts, seeks and stalls only.
  - `mod.rs`: until the clip opens, the decoder keeps the sound it was asked for.
- Files: `ahead.rs` (new), `audio.rs` (with tests in `audio/tests.rs`), `decoder.rs`, `machine.rs`, `mod.rs`.

## Not verified on the Mac

The Mac only compile-checks the Windows path (`cargo xwin check --tests`). The Windows tests (`machine/tests.rs`, `audio/tests.rs`, `ahead.rs`) and the behaviour itself need a Windows run.
