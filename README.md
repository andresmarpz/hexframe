<p align="center"><img src="assets/icon.svg" width="72" alt=""></p>
<h1 align="center">greyscreen</h1>
<p align="center">Automatic League of Legends recorder for Windows.</p>

Free, open-source, and written in Rust.

## Features

- **GPU hardware encoding:** Keeps frames on the GPU with near-zero FPS cost.
- **Vanguard-safe game capture:** Records only League video and audio without touching the game process.
- **Automatic event timeline:** Starts and stops with each match, plotting kills and objectives.
- **Highlight clips:** Cuts clips around chosen moments without re-encoding.
- **Crash-safe MP4 files:** Saves fragmented recordings and repairs unfinished files after a crash.
- **One small exe:** Native tray app with no installer or extra runtime.

## Install

Download `greyscreen-windows-x64.zip` from GitHub Releases, unzip it anywhere, and run `greyscreen.exe`.

**Requirements:**

- Windows 10 2004 or newer (Windows 11 recommended).
- A GPU with a hardware H.264 encoder (any NVIDIA, AMD, or Intel GPU from the last several years).

Set League of Legends to **Borderless** or **Windowed** mode, because exclusive fullscreen cannot be captured. On Windows 10 a thin yellow border may appear around the game while recording, which is a Windows limitation.

## Usage

Start greyscreen and play. Recording begins when the game finishes loading and stops a few seconds after the Nexus falls. TFT, replays, and spectator games are not recorded.

Each match is saved to `Videos\Greyscreen\<date>_<champion>_<mode>\`:

- `game.mp4`: the full match recording.
- `game.json`: the match event timeline.
- `clips\`: automatic highlight clips.

Open **Recordings** to review a match and click any event marker to jump to that moment. To verify capture before a match, use **Test recording (10 s)** on the **Status** screen to record your primary monitor.

**Settings** cover quality (`Balanced`, `High`, `Max`), resolution (`Native`, `1440p`, `1080p`, `720p`), frame rate (`30` or `60` fps), audio, clip options, and the output folder.

- Settings file: `%APPDATA%\Greyscreen\settings.json`
- Logs folder: `%LOCALAPPDATA%\Greyscreen\logs`

## Troubleshooting

Use `greyscreen-cli.exe` from a terminal to diagnose capture or encoding issues:

```powershell
# Print GPU info, available hardware encoders, and capture/encoder test results
greyscreen-cli.exe doctor

# Record 10 seconds of the primary monitor
greyscreen-cli.exe record --seconds 10

# Record 10 seconds of a specific window
greyscreen-cli.exe record --seconds 10 --window "<title>"

# Repair an unfinalized recording after a crash
greyscreen-cli.exe recover <file.mp4.part>
```

Black video usually means the game is in exclusive fullscreen, or you are on a laptop with two GPUs. Switch League to Borderless or Windowed mode, and set greyscreen to **High performance** in Windows Graphics settings.

When reporting a problem, attach the contents of `%LOCALAPPDATA%\Greyscreen\logs`.

## How it works

Video frames stay on the GPU from capture to encode, audio is isolated to the League process, and match state comes from Riot's local HTTP API. The fragmented MP4 writer is pure Rust and written specifically for greyscreen.

```text
Game window -> Windows.Graphics.Capture (D3D11)
                 -> ID3D11VideoProcessor (BGRA -> NV12, BT.709)
                   -> Hardware H.264 MFT -> Packet ring --+
                                                          +-> Fragmented MP4 writer
Game audio  -> WASAPI process loopback -> AAC MFT --------+

Live Client Data API (127.0.0.1:2999)
  -> Session state machine -> Start/stop, event timeline, highlight clips
```

## Building from source

greyscreen is developed on macOS and cross-compiled to Windows with `cargo-xwin`. It also builds natively on Windows with the MSVC toolchain.

**Requirements:**

- Rust (toolchain version is pinned in `rust-toolchain.toml`)
- `just`
- On macOS: `cargo-xwin` and Homebrew LLVM

```bash
# macOS setup
brew install just llvm typos-cli
cargo install --locked cargo-xwin
```

**Commands:**

```bash
# Format check, clippy on macOS and Windows, and tests
just ci

# Build Windows release executables
just build-win

# Build and package dist/greyscreen-windows-x64.zip
just dist

# Simulate a League match, then run the UI against it with fake capture
cargo run -p greyscreen-cli -- fake-league --port 2999 --speed 4
GREYSCREEN_LIVE_URL=http://127.0.0.1:2999 cargo run -p greyscreen
```

## Project layout

| Path | Description |
| --- | --- |
| `crates/app` | The Rune UI binary. |
| `crates/cli` | The `doctor`, `record`, `recover`, and `fake-league` CLI tool. |
| `crates/core` | Settings, events, the session state machine, and clip logic. |
| `crates/engine` | The recording engine. |
| `crates/capture` | Windows capture and hardware encoding. |
| `crates/mp4` | Pure Rust fragmented MP4 writer. |
| `crates/league` | Live Client Data API client. |

Design notes live in [docs/architecture.md](docs/architecture.md), [docs/plan.md](docs/plan.md), [docs/roadmap.md](docs/roadmap.md), and [docs/research/](docs/research/).

## Status

greyscreen is an early preview. It is built and tested on macOS and in CI, and real-hardware testing on Windows is under way. Bug reports are welcome.

## License

MIT. See [LICENSE](LICENSE).

## Legal

Greyscreen isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.
