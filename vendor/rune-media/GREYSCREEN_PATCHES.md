# Greyscreen's patches to rune-media

Base: Rune `eb9f11047bbe2b3927ad4b65ffcf7bfef4d3deb8`, `crates/rune-media`. Every change is in the Windows video path (`src/video/windows`). Each change is a commit titled `vendor: rune-media ...` (a patch, then its follow-up fixes), so each can go upstream as its own pull request.

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

Upstream: no issue filed yet. The write-up is "Windows: a video's own sound is silent at 60 fps and on the layer".

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

## Known gaps

- A clip with sound that nobody draws (window minimised, scrolled away) keeps its sound and clock running while its decoder waits with a full picture queue. Drawn again, the pictures stall and the clock goes back to the newest picture, so some sound plays twice. Upstream behaves the same; seeking the decoder to the clock would fix it.
- Audio clips (`Player::audio`) still lose the sound a stop drops: nothing reads it again.

## Not verified on the Mac

The Mac only compile-checks the Windows path (`cargo xwin check --tests`). The Windows tests (`machine/tests.rs`, `audio/tests.rs`, `ahead.rs`) and the behaviour itself need a Windows run.
