# Thumbnails

Goal: a small picture for every game and clip in the library, from MP4s that `crates/mp4` wrote (H.264 High, AAC), on Windows and macOS, with no ffmpeg or other bundled decoder.

## Options

| Option | Result |
|---|---|
| Rune `Player` / `Video` | Rejected. It shows a frame through a zero-copy native surface (an `IOSurface` on macOS, a shared D3D12 texture on Windows). There is no call that returns pixels, so it cannot fill an `Image`. Rune's own headless `Still` and the macOS poster code decode on the CPU, but they are private. One live player per thumbnail would also be heavy for a list. |
| Grab a frame at record time | Rejected as the main path. The NV12 frame is on the GPU, so it needs a staging texture, a read-back and a colour conversion inside `capture`, which a Mac cannot run or test. Clips are cut from encoded packets in the ring, so no frame exists for them. It also cannot cover games recorded before the feature. |
| Pure-Rust or bundled H.264 decoder (openh264, ffmpeg) | Rejected. Adds a C or C++ build, a licence question (openh264) or a large binary, for something the OS already does. |
| Media Foundation `IMFSourceReader` on Windows | Chosen for Windows. The H.264 decoder ships with Windows. The reader seeks to the time, decodes NV12, and Rust converts the one frame to RGB (BT.709). About 150 lines. |
| `AVAssetImageGenerator` on macOS | Chosen for macOS. Rune already links `objc2-av-foundation` with this feature, so the build gains no new crates. About 70 lines. Zero seek tolerance gives the exact frame. |

## Design

- New crate `greyscreen-thumbs`. `grab(video, at_secs, max_width) -> Frame` (RGBA) has one backend per OS. Other targets return `Unsupported`. `write_thumbnail` crops to 16:9, resizes to 400x225 with a triangle filter, encodes JPEG quality 80 (about 14 KB) and renames a `.part` file into place.
- `unsafe` lives in this crate for the two OS bindings (each block has a `SAFETY:` line). The rule in `AGENTS.md` now lists it next to `capture`.
- Files: `thumb.jpg` in the game folder and `clips/<clip name>.jpg` next to each clip. `greyscreen_core::library::thumb` holds the names and the frame to pick: a clip at 70 % of its length (the highlight sits about 75 % in with the default 15 s / 5 s), a game at the moment of its first clip or at 40 % of its length.
- Engine: `thumbs::wrap` wraps the UI sink. It watches `Library`, `RecordingSaved` and `ClipSaved`, queues the missing files on one background thread (newest game first) and sends `UiMsg::Thumbnail(path)` after each one. A path that failed once is not retried in the same run. Nothing blocks the control thread.
- UI: `view::thumb` keeps a small memo of rune `Image`s by path (cleared at 160 entries). A `Thumbnail` message drops that path from the memo, so the file loads on the next frame. A missing file shows a flat placeholder.
- `greyscreen-cli thumb <video> --at <secs> --out <jpg>` runs the decoder alone, to check it on a machine.

## Checked

- macOS: an engine test cuts a clip from the fake media source (real H.264 test pattern), runs the worker and checks a 400x225 JPEG with many distinct colours. Running the real app on a seeded library made 22 of 22 thumbnails in the background.
- A grab of `review.mp4` at 3.5 s shows the burned-in timecode `00:00:03.500`, so seeking is frame exact.
- Windows: compiled and linted through `cargo xwin clippy` only. Not run. The unverified parts are the seek and read loop, the 2D buffer stride handling and the NV12 crop for 1080p streams that report 1088 coded rows. The pure NV12 to RGB code is shared and unit tested on the Mac.

## Limits

- A game's thumbnail is one frame, not the first clip's thumbnail (they differ when the game has clips).
- The engine does not rebuild a thumbnail when the clip is trimmed later (no such feature yet).
