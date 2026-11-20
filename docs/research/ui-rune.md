# greyscreen UI with Rune: research

Researched 2026-11-20 against `anomalyco/rune` branch `dev`, HEAD `7a707454fbfb8d3fd200723c65778fb7b98fd31e` (clone and `git ls-remote` agree). Rune paths below are relative to that repo.

## Summary

1. Rune is a good fit: `fn view(&State, &mut Cx<State>) -> El`, a `poll` hook, and a thread-safe `AppHandle::update(..)` for the recorder and event-watcher threads. It also ships a Windows tray (`rune::Tray`), `rune::single_instance`, `rune::set_app_icon`, a folder picker, and Media Foundation H.264 video with seeking.
2. Pin Rune by git `rev` and copy its six-entry `[patch.crates-io]` table (winit, sctk-adwaita, parley_engine, wgpu-hal, accesskit_windows, accesskit_macos), all pointed at the same rev. Verified: git form resolves, and `cargo xwin build --release --target x86_64-pc-windows-msvc` produced a 25 MB GUI-subsystem Windows exe in 60 s on this Mac.
3. The same code compiles and runs on macOS (Metal) and renders headless to PNG, element-tree JSON and scripted input. Verified here: `--snap`, `--tree`, `--script` with taps all work. Develop the UI on the Mac with a fake recorder backend behind a trait.
4. Rune is pre-release, with no Windows CI of its own (only macOS, docs and file-size workflows). I could not run anything on Windows: the D3D12 path, tray, single-instance and Media Foundation playback are compile-checked only.
5. Gaps to plan around: no window-icon API beyond `set_app_icon`, `Tx::open_url` refuses `file:` (open-folder needs `explorer.exe`), no video scrubber or slider in rune-kit (build it), default CRT is dynamic (add `+crt-static`).

---

## 1. Depending on Rune

Source: `README.md` ("Add it"), `crates/rune/README.md` ("Depending on Rune"), root `Cargo.toml`.

Rune says: "Pin a rev of `dev`; `dev` is never rewritten" (`AGENTS.md`: `dev` is fast-forward-only). Cargo does not inherit a dependency's `[patch]`, so the app workspace must repeat the patches at the same rev.

App workspace-root `Cargo.toml` (this exact form was resolved with `cargo fetch --target x86_64-pc-windows-msvc`; `cargo tree -i` showed winit, wgpu-hal, parley_engine and accesskit_windows all coming from the Rune git rev):

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.dependencies]
rune     = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e", features = ["native-file-dialog"] }
rune-kit = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e", default-features = false }

[patch.crates-io]
winit             = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e" }
sctk-adwaita      = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e" }
parley_engine     = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e" }
wgpu-hal          = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e" }
accesskit_windows = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e" }
accesskit_macos   = { git = "https://github.com/anomalyco/rune", rev = "7a707454fbfb8d3fd200723c65778fb7b98fd31e" }
```

Rules from the Rune docs:

- Patch winit and sctk-adwaita from the same rev: Rune's winit calls `AdwaitaFrame::content_corner_radius`, which only the vendored frame has. A winit-only patch does not compile.
- When bumping Rune, re-read Rune's root `Cargo.toml` `[patch.crates-io]` at the new rev and update every line plus the `rune` and `rune-kit` revs together.
- `vendor/tiny-skia-path` is a plain path dependency inside the Rune repo, not a patch. Nothing to copy.
- What each patch does for Windows: `winit` skips a TaskbarList COM call per window and loads system DLLs from System32 only; `wgpu-hal` adds `expose_adapter`, lazy FXC loading, partial presents, System32-only DLLs; `parley_engine` fixes bidi trailing whitespace and shaping cost; `accesskit_windows` adds headings and landmarks to UI Automation. `sctk-adwaita` and `accesskit_macos` only affect Linux and macOS, but copy them: the Mac dev build uses `accesskit_macos`.
- Seed the app's `Cargo.lock` from Rune's at the same rev (`curl https://raw.githubusercontent.com/anomalyco/rune/<rev>/Cargo.lock`). Rune's lock is not used by dependents, so this keeps the transitive versions the Rune maintainers built with. I built all probes this way.

Rust version and edition:

- `rust-toolchain.toml`: `channel = "1.99.1"`, components clippy and rustfmt. Rune declares no `rust-version`. 1.99.1 is what I used for every build here (`rustc 1.99.1 (58cf8f9c3 2026-10-13)`).
- Edition: `[workspace.package] edition = "2024"`, resolver 3. Use edition 2024 in the app.

Features (`crates/rune/Cargo.toml`):

| Feature | Default | Effect on a Windows app | Decision |
|---|---|---|---|
| `accessibility` | yes | UI Automation via accesskit_windows, nothing built until a screen reader asks | keep |
| `dbus` | yes | Linux only | no effect |
| `embedded-fonts` | yes | bundles Inter (about 1.25 MB) | keep |
| `metal-backend` | yes | Apple only | no effect |
| `ogg-audio` | yes | macOS only | no effect |
| `svg` | yes | resvg, rasterizes the SVG icon bodies every `Icon` uses | keep (icons need it) |
| `native-file-dialog` | no | rfd folder and file pickers on Windows | add (output-folder picker) |
| `tooling` | no | `--snap`, `--tree`, `--script` in release builds (+3.4 MB). Debug builds always have it | off for the shipped exe |
| `keys-overlay` | no | keycaps and click ripples in videos (+1.46 MB) | off |
| `segmenter-dictionaries` | no | links CJK word dictionaries (+1.9 MB) instead of asking Windows ICU | off |
| `avif` | no | needs NASM on x86 | off |
| `hot` | no | live Rust editing | off |

So on Windows the default features are already lean: the Mac and Linux features do not compile there. `rune-kit` default feature `syntax` pulls `rune-highlights` (a wasm-to-Rust codegen at build time, 80 lexers). Use `default-features = false` on `rune-kit` (verified to build for both targets). Do not enable `markdown` or `mermaid`.

Release exe sizes with no profile tuning: 22.2 MB (hello plus rune-kit) and 25.3 MB (the probe app with `native-file-dialog`, `tooling`, tray, folder picker).

---

## 2. App model (exact signatures)

### AppSpec and run

`crates/rune/src/tooling.rs`:

```rust
pub struct AppSpec<S: 'static> {
    pub title: &'static str,
    pub size: (f32, f32),
    /// The smallest window the user can resize to, in points (None: 420×400).
    pub min_size: Option<(f32, f32)>,
    pub init: fn() -> S,
    pub view: fn(&S, &mut Cx<S>) -> El,
    pub scenes: &'static [(&'static str, fn(&mut S))],
    pub fonts: &'static [&'static [u8]],
    pub families: (&'static str, &'static str),
    /// Called once with a waker other threads use when new data arrives.
    pub setup: Option<fn(&mut S, Waker)>,
    /// Called at the start of every frame with seconds since start (virtual when headless);
    /// true when state changed.
    pub poll: Option<fn(&mut S, f64, &mut Tx) -> bool>,
    pub on_key: Option<fn(&mut S, &KeyEvent, &mut Tx) -> bool>,
}
impl<S: 'static> AppSpec<S> {
    pub fn new(title: &'static str, size: (f32, f32), init: fn() -> S, view: fn(&S, &mut Cx<S>) -> El) -> Self;
}

pub fn run<S: 'static>(spec: AppSpec<S>);   // one window, quits when it closes
```

- Apps build `AppSpec` with `..AppSpec::new(..)` (AGENTS.md: adding a pub field breaks literals, so never list every field).
- `init`, `setup`, `poll`, `on_key` are plain `fn` pointers, not closures. Anything captured (a channel, a host) goes in through `App::init(closure)` (below), not through `AppSpec::init`.
- `Waker = std::sync::Arc<dyn Fn() + Send + Sync>` (`crates/rune-core/src/lib.rs`).
- `setup` runs once with a `Waker`; calling the waker makes Rune run `poll` on the next frame.
- `poll` should return true only when visible state changed; "an idle window draws nothing".

`RunOptions` (`crates/rune/src/window/spec.rs`):

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct RunOptions {
    /// macOS: no Dock icon/default menu and no activation at launch. Ignored elsewhere.
    pub accessory: bool,
}
```

It is only used by `run_windows_with(options, factory)` and the `*_with_args` variants. A single-window app does not need it.

### App (the lifetime wrapper greyscreen needs)

`crates/rune/src/app.rs`. `rune::run(spec)` is `App::new(spec).run()`. A recorder must outlive its window (closing to tray), so use `App`:

```rust
App::new(spec: AppSpec<S>) -> App<S>
  .init(f: impl FnOnce() -> S + 'static)             // first state from a closure; scenes still apply
  .keep_running()                                    // closing the window hides it; process, state, loop stay
  .windowless()                                      // start hidden (implies keep_running)
  .release_gpu_when_closed()                         // closing also drops the GPU device and window
  .on_close(f: fn(&mut S, &mut Tx))                  // runs before every close; tx.quit() quits instead
  .on_ready(f: impl FnMut(&WindowHandle) + 'static)  // first frame done, once per native window creation
  .on_window(f: impl FnMut(&WindowHandle) + 'static) // after each native window creation
  .window_options(WindowOptions { min_size, max_size, resizable, .. })
  .titlebar(Titlebar::new(height))                   // optional custom titlebar (Windows: Rune draws caption buttons)
  .handle() -> AppHandle<S>                          // Clone + Send + Sync, usable before run
  .run()                                             // exits the process with code 1 on startup failure
  .try_run() -> Result<(), String>
```

- `on_ready` is for native window tweaks after the first frame. It is not called in headless runs. Greyscreen does not need it in v1.
- `Closing hides the window on macOS, Windows and X11` (doc, "App lifetime"). `release_gpu_when_closed()` is worth using: while the user plays League with the window closed to tray, Rune holds no GPU device. Reopening costs a device creation.

### Background threads to UI (exact API)

Three mechanisms, all in the source. Recommended: the first.

```rust
// crates/rune/src/app.rs. AppHandle<S>: Clone, Send, Sync; obtain with App::handle() before run().
pub fn update(&self, f: impl FnOnce(&mut S, &mut Tx) + Send + 'static)
pub fn update_if(&self, f: impl FnOnce(&mut S, &mut Tx) -> bool + Send + 'static)  // false: no view re-run, no redraw
pub fn wake(&self)                  // run AppSpec::poll, also while the window is hidden
pub fn open(&self); pub fn show(&self); pub fn hide(&self); pub fn focus(&self);
pub fn close(&self); pub fn minimize(&self); pub fn quit(&self);
pub fn is_visible(&self) -> bool; pub fn window_error(&self) -> Option<String>;
```

Doc: "`f` runs on the UI thread, in call order, even while no window shows, and the next frame commits it". So a closed-to-tray app keeps its state current. `Tx` is the handler context (`wake_after`, `open_url`, `open_folder`, `quit`, `set_title`, `instant`).

Pattern A (recommended): the backend threads hold `AppHandle<Ui>` and a `Msg` enum:

```rust
enum Msg { GameStarted(GameInfo), GameEnded, Event(LiveEvent), Recorder(RecorderState), ClipSaved(PathBuf), Error(String) }
impl Ui { fn apply(&mut self, m: Msg) { /* pure state change, unit-testable */ } }

// recorder thread / event watcher:
let h = handle.clone();
sink(move |m| h.update(move |ui: &mut Ui, _tx: &mut Tx| ui.apply(m)));
```

Pattern B (channel + waker; `examples/live-values`, `examples/video-player` use the waker): store `mpsc::Receiver<Msg>` in `Ui`, drain it in `poll`, and call `handle.wake()` (or the `setup` `Waker`) after each send.

```rust
fn poll(s: &mut Ui, _clock: f64, _tx: &mut Tx) -> bool {
    let mut changed = false;
    while let Ok(m) = s.rx.try_recv() { s.apply(m); changed = true; }
    changed
}
```

Pattern A needs no state-side receiver and no `setup`. Use B only if you want to batch many events per frame. For a high-rate value (an audio level meter) use `Live<f32>`, not `update`. `rune::Latest<T>` exists for "newest answer wins" worker requests (search box); greyscreen does not need it.

Timers: `tx.wake_after(Duration)` schedules a `poll` with no timer thread (use for the elapsed-time label while recording: ask again from `poll` while `recording`). `Tx::clock()` is the logical clock; `Tx::wake_at(due)`.

### Window size, title, icon, theme

- Title, size, min size: `AppSpec::new("Greyscreen", (900.0, 640.0), ..)` plus `min_size: Some((640.0, 480.0))`. The default minimum is 420 x 400 points. `Tx::set_title(..)` changes it at runtime. `window_options(WindowOptions { resizable, max_size, .. })` for limits.
- Icon (`crates/rune/src/platform/app_icon.rs`):
  `pub fn set_app_icon(bytes: impl Into<Vec<u8>>) -> Result<(), String>`: on Windows, bytes are PNG or JPEG, best square and 256 px or more; each window opened after the call shows it in the title bar, taskbar and Alt+Tab. Call before `run`. `include_bytes!("../assets/icon.png")` works.
  `#[cfg(windows)] pub fn set_app_icon_resource(id: u16)` uses the exe's own icon resource instead (needs an .rc or `winresource` build step; bitmap images up to 128 px). v1: use `set_app_icon`. Explorer's file icon of the exe still needs a resource; that is separate from the window icon.
- Dark theme: there is no global switch. The view chooses tokens: `tok::theme(true)` and `Kit::neutral(true)` for always-dark, or `cx.system_dark()` (`Cx::system_dark(&self) -> bool`) to follow the OS. The native title bar follows the OS theme (winit `window.theme()`), so an always-dark UI under a light OS gets a light title bar. For a fully dark window use `App::titlebar(Titlebar::new(h))`: on Windows Rune removes the native bar and draws Min/Max/Close, keeping resize edges, Aero Snap and the system menu (`crates/rune/README.md` "Window chrome"). Not run on Windows by me. v1 decision: follow the system theme with `cx.system_dark()` and the native frame.
- GUI subsystem: add `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`. Verified: with it the release exe is `PE32+ executable (GUI)`; without it, a console window opens. (Release then has no stderr; log to a file.)

### System tray

Rune has one: `crates/rune/src/tray.rs` ("`Shell_NotifyIcon` on its own message-only window on Windows", own thread, works while the window is closed; macOS unsupported). Exported as `rune::{Tray, TraySpec, TrayIcon, TrayItem, TrayEvent, TrayError}`:

```rust
TrayIcon::rgba(width: u32, height: u32, pixels: Vec<u8>) -> TrayIcon         // straight RGBA, w*h*4 bytes
TraySpec::new(id: impl Into<String>, title: impl Into<String>, icon: TrayIcon)
    .tooltip(..).menu(Vec<TrayItem>)
TrayItem::action(id, label) / TrayItem::Separator / .enabled(bool)
Tray::new(spec: TraySpec, on_event: impl Fn(TrayEvent) + Send + Sync + 'static) -> Result<Tray, TrayError>
tray.set_menu(Vec<TrayItem>); tray.set_tooltip(..)
enum TrayEvent { Activate { token: Option<String> }, Menu(String) }   // #[non_exhaustive]
```

Dropping the `Tray` removes the icon, so keep it alive for the whole run. The callback runs on the tray thread: hand work to the app through `AppHandle`. Verified: compiles for macOS (returns `Unsupported` there) and for `x86_64-pc-windows-msvc`. Not run on Windows.

Recommendation: include it in v1 (about 15 lines, `Tray::new` returns a `Result`, treat an error as "no tray, log and continue"). Decode the icon from the same PNG as the window icon with the `image` crate Rune already pulls in, or ship a raw 32x32 RGBA blob.

### Single instance

```rust
// crates/rune/src/open_url/instance.rs, re-exported as rune::single_instance
pub fn single_instance(id: &str) -> bool
```

Call first thing in `main`. On Windows it claims a per-user named pipe. Returns false when another process already runs: this launch handed its arguments over, the running process brings its front window forward (restored), and `main` must return. Returns true if first (also if the first process hangs). macOS and others: always true. A launch without links triggers `App::on_reopen` and then the default window-raise. Use id `dev.greyscreen.app`. Skip it when `rune::is_headless()`.

---

## 3. rune-kit components

Every constructor chain below was compiled in a probe app for macOS (`cargo check`) and the Button, Switch, SegmentedControl, Select, TextInput, Card, Badge, Progress, Toast, ToastRegion, virtual_rows, folder picker and tray ones also for `x86_64-pc-windows-msvc`. Dialog, List, Table, IconButton, Field, Spinner and `icon()` were compiled for macOS and Windows as well; none were run on Windows.

Setup: `rune_kit::Kit::neutral(dark) -> Kit` (Rune's neutral tokens). Builders end in `.build(&k)`; overlays end in `.build(&k, &mut layer)`; mount with `rune_kit::root(base, layer)` as the view's root:

```rust
fn view(s: &Ui, cx: &mut Cx<Ui>) -> El {
    let k = Kit::neutral(cx.system_dark());
    let mut layer: Vec<El> = Vec::new();
    // ... build body, pushing overlay elements into `layer` ...
    rune_kit::root(body, layer)
}
```

| Need | Constructor and usage |
|---|---|
| Button | `Button::new("Start").variant(ButtonVariant::Submit).control_size(ControlSize::Regular).icon(&ICON).disabled(bool).on_press_tx(\|s: &mut Ui, tx: &mut Tx\| ..).build(&k)`. Variants: `Neutral, Danger, Warning, Outline, Contrast, Submit, Ghost, GhostMuted, GhostFaint, Loading`. Also `.on_press(\|s: &mut Ui\| ..)`, `.help("tooltip")` |
| Icon button | `IconButton::new(&ICON).label("Open folder").variant(IconButtonVariant::Ghost).on_press(..).build(&k)`. Always give `.label` |
| Toggle | `Switch::new(s.auto_start).label("Start with game").description("..").on_toggle(\|s: &mut Ui\| s.auto_start = !s.auto_start).build(&k)` |
| Segmented | `SegmentedControl::new(&["Low", "Medium", "High"], Some(s.quality)).on_pick(\|s: &mut Ui, i: usize, _: &mut Tx\| s.quality = i).build(&k)` |
| Select | `Select::new("quality", &["Low", "Medium"]).current(Some(&s.quality_id)).open(s.open == Some("quality")).width(220.0).label("Quality").on_toggle(..).on_close(..).on_choose(\|s: &mut Ui, v: &str, _: &mut Tx\| ..).build(&k, &mut layer)`. Needs `.label` (debug builds warn without one) and `.width(..)` (default trigger is full width; observed in the probe). Variants: `Select::grouped`, `.details(..)` |
| Text input | `TextInput::new(&s.clip_secs).label("Clip length (s)").placeholder("30").on_edit(\|s: &mut Ui\| &mut s.clip_secs).on_submit(\|s, tx\| ..).invalid(bool).build(&k)` with `clip_secs: rune::TextEdit` in state (`TextEdit::new("30")`; the current string is the pub field `clip_secs.text`). Also `TextArea`, `Field::new().label(..).control(el).build(&k)` |
| Card | `Card::new().title("Recording", None).description("..").action(badge_el).child(el).accent(color).build(&k)` |
| Badge | `Badge::new("REC").variant(BadgeVariant::Accent \| Neutral \| Subtle).build(&k)`; `Divider::horizontal().build(&k)` |
| Settings rows | No dedicated component. Compose `row()` with a label `col()` and a `Switch` / `SegmentedControl` / `Select`, one `Card` per group. `Field` gives label plus control |
| Lists, long | `col().key("rows").w_full().growth(GrowthSpec::default()).virtual_rows(count, est_row_h, \|s: &Ui, i\| key_i, \|s: &Ui, i\| row_el)` inside a `.scroll(ScrollSpec { stick_bottom: true })` parent. `El::virtual_rows<S>(self, count: usize, estimated_row_height: f32, key: impl for<'a> Fn(&'a S, usize) -> Cow<'a, str>, build: impl Fn(&S, usize) -> El)`. Use stable ids as keys (not indexes) |
| Lists, selectable | `List::new().item(ListItem::new("x").selected(b).on_select(..)).search(&edit, "Search")..build(&k)` (small lists) |
| Table | `Table::<Ui>::new(\|s\| &s.table, \|s\| &mut s.table, \|s\| s.recs.len(), \|s, i\| s.recs[i].id.as_str().into()).column(TableColumn::new("name", "Name").sortable()).cell(\|s, i, col\| Cell::text(..)).on_open(\|s, id, tx\| ..).zebra(true).build(s, &k)` with `table: TableState` in state. Virtualized, keyboard driven, resizable columns |
| Toast | `ToastRegion::new().toast(Toast::new("saved", "Clip saved").description(path).on_close(\|s: &mut Ui\| s.toast = None).build(&k)).build(&k)`, pushed into `layer`. The app owns dismissal: store `toast_until: f64` and clear in `poll` using the clock (`tx.wake_at(until)`). `ToastStack` (state machine with timers, `rune_kit::toast_stack`) exists for stacking |
| Progress, spinner | `Progress::new(Some(0.4)).label(..).show_value().build(&k)` (`None` = indeterminate, static), `Spinner::new().size(14.0).build(&k)`, `ProgressCircle::new(0.4)` |
| Dialog | `Dialog::new("confirm", "Delete clip?").open(bool).child(el).on_close(..).build(&k, &mut layer)` (returns `()`; pushes into `layer`) |
| Icons | `rune::Icon { name, view, body }` is an SVG body, drawn with `rune::icon(&ICON, 16.0)` (an `El`; `.color(c)` tints). Free sets: `rune_kit::glyphs::*` (28 shapes: check, chevrons, close, ...) and the `rune-theme-opencode` crate's `icons::*` (172: `FOLDER`, `STOP`, `SETTINGS_GEAR`, `TRASH`, `CIRCLE_CHECK`, `CLOSE`, ...; depends on that crate, only needed if you want them). Otherwise define `pub static RECORD: Icon = Icon { name: "record", view: 16.0, body: r#"<circle cx="8" cy="8" r="5" fill="currentColor"/>"# };` |
| Primitives | `col()`, `row()`, `el()`, `text(s).font(type_::BODY).color(c)`, `.key(..)`, `.role(..)`, `.label(..)`, `.px/.py/.gap/.grow/.w_full/.h_full/.min_h(0.0)/.center()/.fill/.radius/.border`, `.on_press(..)`, `.on_drag(\|s, d: &mut Drag, tx\| ..)`, `.absolute(x, y)`, `.help("tooltip")`; tokens `rune::tok::{space::{S1,S2,S3,S6}, radius::{SM,MD,LG}, type_::{BODY,LABEL,SECTION,TITLE}}` |

Gotchas confirmed in Rune's README: a grown `.scroll(..)` viewport in an auto-height column collapses to 0 pt, so bound an ancestor (`.h_full()`), and put `.min_h(0.0)` on every flex column between it and the viewport. `.virtualize`/`.virtual_rows` need `.growth(GrowthSpec::default())` on the same element and must be the viewport's first child. The small apps `examples/hello`, `settings` and `background` use only `rune` plus `rune::tok` with no kit, so the kit is optional.

---

## 4. Video playback (rune-media, for the later review screen)

Docs: `crates/rune/README.md` ("Playing video and audio"), code: `crates/rune-media/src/video/windows/`.

Windows path: Media Foundation source reader on a decoder thread per clip, GPU decode (DXVA via a D3D11 device on Rune's adapter) with software fallback, each picture copied on the GPU into shared NV12 textures sampled in place by Rune's D3D12 device. No CPU copy. Audio through WASAPI following the playback clock. Media Foundation, D3D11 and WASAPI are loaded on the first clip through `GetProcAddress`. Formats: "mp4/mov/m4v (H.264; HEVC, VP9 and AV1 with their system extensions), WebM, mp3, m4a/AAC, wav, flac". A Windows N edition without the Media Feature Pack fails every clip with a message. So an H.264 MP4 file plays, with audio.

API (`crates/rune-media/src/video/mod.rs`; re-exported as `rune::Player`, alias `Video`, plus `rune::{PlayerState, SurfaceFit}`):

```rust
Player::file(path: impl Into<PathBuf>) -> Player         // muted video; also Player::url(..), Player::new(MediaSource)
fn play(&self); fn pause(&self);
fn seek(&self, seconds: f64);                            // clamped; keeps playing if it was playing
fn set_rate(&self, f32); fn set_looping(&self, bool); fn set_muted(&self, bool); fn set_volume(&self, f32);
fn state(&self) -> PlayerState;   // Loading | Ready | Playing | Ended | Failed(String)
fn time(&self) -> f64; fn duration(&self) -> Option<f64>; fn natural_size(&self) -> Option<(u32, u32)>; fn has_sound(&self) -> Option<bool>;
fn set_waker(&self, waker: Waker);                       // called on load, end, fail, loop; not on progress
fn preload(&self);
// in a view:
el().key("video").w(w).h(w * h as f32 / w0 as f32).video(&player, SurfaceFit::Fit)   // or SurfaceFit::Fill
```

Seeking (read in `mf.rs` and `decoder.rs`): the reader lands on the key frame at or before the target, then pictures before the target are decoded and skipped (`skip_before`), so a seek shows the requested frame, not the key frame. Cost grows with the distance to the previous key frame: have the encoder emit a key frame at least every 1 to 2 seconds so marker clicks and scrubbing stay fast. I did not measure seek latency.

Restrictions (HEAD commit `fix(media): restrict local-file video opening`): local files must be a plain local disk path with a media extension (`validate_local_file`). UNC shares (`\\server\share`), `\\?\` device paths, alternate data streams and shortcut or playlist extensions are refused with `Failed(..)`. A recordings folder on a mapped network share still passes only if the path is a drive letter. Request headers are not supported on Windows.

Timeline with event markers: rune-kit has no slider or video controls. Build it: a `row()` track, marker children placed with `.absolute(x, y)` at `t / duration * track_w`, `on_press` on a marker calls `player.seek(t)`, `.on_drag(..)` on the track maps `d.x / d.w` to a time (`Drag { phase, x, y, w, h, dx, dy, .. }`, `examples/settings` slider is the reference). To show progress, ask `tx.wake_after(Duration::from_millis(33))` from `poll` while `state() == Playing` and return true (as `examples/video-player` does for audio). Seek while paused: not verified.

On macOS the same `Player` plays H.264 through AVFoundation (`examples/video-player`, fixture `crates/rune/tests/fixtures/rgbw.mp4`), so the review screen can be built and screenshotted on the Mac. Headless runs decode the frame at the virtual time with no sound. Other platforms report `Failed("video playback isn't supported ...")`.

---

## 5. Testing and developing on the Mac

Verified on this Mac (probe app with the structure below, `cargo build`, debug):

- Render to PNG: `./app --snap out.png --size 900x640 --scale 2 [--scene NAME] [--at SECONDS] [--theme dark|light]`. Took 2.2 s including GPU init (Metal).
- Dump the element tree: `./app --tree tree.json` (key, role, label, text, presented rect, fill and text colour, hover, press, plus an `accessibility` tree).
- Drive input with a script and snapshot: `echo '{"events":[{"t":0.1,"type":"tap","key":"segment2"},{"t":0.3,"type":"tap","key":"folder-trigger"}]}' > tap.json; ./app --script tap.json --snap after.png --at 1.0`. The PNG showed the segment changed and the dropdown open.
- Other flags: `--list-scenes`, `--frames N --out dir/`, `--video out.mp4` (needs `ffmpeg`), `--trace`, `--script` events `tap`, `down/up`, `drag`, `type`, `key`, `scroll`, `wheel`, `mark`, `file-dialog`, `menu-pick`.
- Debug builds always have these flags; release builds need Rune's `tooling` feature. Rune prints a11y warnings in debug (for example a `Select` without `.label`).
- Unknown CLI args are left to the app: `rune::extra_args() -> &'static [String]`; `rune::is_headless() -> bool` tells whether this is a headless run (skip the tray, single-instance and backend threads then).
- Unit tests with no GPU (pattern from `examples/hello`):

```rust
let env = Env { system_dark: true, scale: 2.0 };          // or Env::new(true, 1.0)
let mut rt = Runtime::new(Ui::fake(), view, (900.0, 640.0), env);
rt.frame(0.0);
let (x, y) = rt.find("start").unwrap();                   // keyed element centre
rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(1.0 / 60.0);
assert!(rt.state.recording);
```

`Runtime::elements()` lists elements; `Runtime::accessibility_tree()` gives the AccessKit tree. `rune::motion_gate` can fail a test on visible animation glitches.

Does the app run on macOS? Yes: Rune builds there with its Metal backend and the probe rendered headless through it (the same renderer as a window). I did not open an interactive window. `Tray::new` returns `TrayError::Unsupported` on macOS, `single_instance` is always true, `set_app_icon` takes any NSImage format, and `windows_subsystem` is ignored.

### Recommended structure

```
crates/
  greyscreen-core/   // no UI, no OS: Backend trait, Command, UiMsg/Event types, GameInfo, Recording, Settings (serde)
  greyscreen-ui/     // depends on rune, rune-kit, greyscreen-core: Ui state, view, apply(Msg), screens, scenes
  greyscreen-win/    // #[cfg(windows)] real backend: capture, encode, LCU/live-client watcher, shell open
  greyscreen-fake/   // FakeBackend: a thread replays a scripted match (game start, kills, clip saved), writes real tiny mp4s from fixtures
  greyscreen/        // bin: picks the backend by cfg, wires AppHandle, tray, single-instance, icon
```

```rust
// greyscreen-core
pub trait Backend: Send + 'static {
    fn start(self: Box<Self>, sink: Sink) -> Commands;   // spawns its own threads; pushes Msg through sink
}
pub type Sink = Box<dyn Fn(Msg) + Send + Sync>;           // in the bin: move |m| handle.update(move |ui, _| ui.apply(m))
pub struct Commands(pub std::sync::mpsc::Sender<Command>); // UI to backend: StartRecording, StopRecording, SaveClip, ApplySettings(Settings)
pub trait Shell { fn reveal(&self, path: &std::path::Path); }  // explorer.exe on Windows, `open` on macOS; fake logs
```

The bin (`crates/greyscreen/src/main.rs`):

```rust
fn main() {
    let headless = rune::is_headless();
    if !headless && !rune::single_instance("dev.greyscreen.app") { return; }
    let _ = rune::set_app_icon(include_bytes!("../assets/icon.png").as_slice());
    let app = App::new(AppSpec { poll: None, min_size: Some((720.0, 520.0)), ..AppSpec::new("Greyscreen", (960.0, 640.0), Ui::scene_default, ui::view) })
        .keep_running().release_gpu_when_closed()
        .init(move || Ui::new(settings, cmds));          // real state: replaces AppSpec::init
    let handle = app.handle();
    if !headless {
        let sink: Sink = Box::new({
            let handle = handle.clone();
            move |m| { let h = handle.clone(); h.update(move |ui: &mut Ui, tx: &mut Tx| ui.apply(m, tx)); }
        });
        let _cmds = backend().start(sink);               // cfg(windows) real backend, otherwise Fake
        let _tray = make_tray(handle.clone());           // Tray::new(..); log on Err; keep alive
    }
    app.run();
}
```

`Ui::apply(&mut self, Msg, &mut Tx)` is a pure state transition, so unit tests feed `Msg` values directly without threads. For snapshots use `AppSpec::scenes` (`("recording", |s| s.apply(Msg::...))`, run with `--scene recording`), not live threads: the output is deterministic. The backend threads never touch rune types besides `AppHandle`.

---

## 6. Cross-compiling from macOS

Verified on this Mac (`cargo-xwin` on PATH, rustc 1.99.1, `x86_64-pc-windows-msvc` target installed, cargo-xwin cache `~/Library/Caches/cargo-xwin/xwin` = 1.1 GB):

- `cargo xwin check --target x86_64-pc-windows-msvc`: OK, 26 s fresh.
- `cargo xwin build --release --target x86_64-pc-windows-msvc`: OK, 60 s wall (hello + rune-kit), 26 s incremental. Output `PE32+ executable (GUI) x86-64`.
- `.cargo/config.toml` with `[target.x86_64-pc-windows-msvc] linker = "rust-lld"` (Rune's own file has the same) worked with cargo-xwin. No `link.exe` or MSVC install needed.
- Needs `clang-cl` on PATH: `blake3` (a dependency of Rune) has a `build.rs` that compiles C and MSVC `.asm` through `cc`, which uses `clang-cl` for this target. It is not on the default PATH here; `/opt/homebrew/opt/llvm/bin` has it. Run `PATH=/opt/homebrew/opt/llvm/bin:$PATH cargo xwin build ...`. I did not test the failure mode without it.
- No FXC or DXC at build time: the uber shader DXBC is checked in (`crates/rune/shaders/dxbc/*.dxbc`, pinned by hash against the WGSL by `build.rs`). At run time the patched wgpu-hal loads `d3d12.dll`, `dxgi.dll`, `dcomp.dll`, `d3dcompiler_47.dll` from System32 lazily. The exe's import table shows `dxgi.dll`, `dwrite.dll`, `uiautomationcore.dll`, `dwmapi.dll`, `comctl32.dll`, `shell32.dll`, `ole32.dll`, etc., and no `d3d12.dll` or Media Foundation import (loaded on demand).
- Other build scripts in the graph (`wgpu-hal`, `winit`: `cfg_aliases` only; `rune-highlights` only if the kit `syntax` feature is on; `rune-luau` not in the app graph) ran fine.
- Dynamic CRT: the default exe imports `VCRUNTIME140.dll` and `api-ms-win-crt-*`, so a PC without the VC++ Redistributable fails to start it. Adding `rustflags = ["-C", "target-feature=+crt-static"]` to the `[target.x86_64-pc-windows-msvc]` table removed all of those imports (re-verified with `llvm-readobj --coff-imports`, build still OK, +0.2 MB).
- GPU: Rune uses D3D12 only (`wgpu` feature `dx12`). A machine with no D3D12 hardware adapter draws on WARP. Vulkan is not compiled for Windows.

Not verified: running the exe (no Windows machine here). Rune has no Windows CI job; its Windows behaviour is covered by its maintainers' own testing. The first run on the PC is the real test of the D3D12 path.

---

## 7. Repo conventions worth copying

- `clippy.toml`:
  ```toml
  allow-unwrap-in-tests = true
  allow-expect-in-tests = true
  allow-panic-in-tests = true
  allow-dbg-in-tests = true
  disallowed-methods = [
    { path = "std::env::var", reason = "use rune_core::env::var (or rune::env::var)" },
    { path = "std::time::SystemTime::now", reason = "use rune_core::time::system_now (or rune::time::system_now)" },
    { path = "std::time::Instant::now", reason = "use rune_core::time::now (or rune::time::now) for wasm/headless clocks" },
  ]
  ```
  Copy the unwrap and expect allowances. The three `disallowed-methods` are only needed if greyscreen wants virtual-clock determinism in headless runs; at minimum use `rune::time::now()` for timestamps the UI shows.
- Root `Cargo.toml` lints (each crate has `[lints] workspace = true`):
  ```toml
  [workspace.lints.rust]
  unsafe_op_in_unsafe_fn = "deny"
  unused_must_use = "deny"
  unsafe_code = "deny"            # leaf crates that need FFI opt in locally; pure crates use #![forbid(unsafe_code)]
  [workspace.lints.clippy]
  await_holding_lock = "deny"
  await_holding_refcell_ref = "deny"
  dbg_macro = "deny"
  disallowed_methods = "deny"
  disallowed_types = "deny"
  expect_used = "warn"
  let_underscore_must_use = "deny"
  todo = "warn"
  undocumented_unsafe_blocks = "deny"
  unwrap_used = "warn"
  ```
- Profiles: `[profile.dev] debug = "line-tables-only"` and `[profile.dev.package."*"] debug = false` (Rune hit 18 to 23 GB RAM per build with full debug info). `[profile.release] codegen-units = 16, incremental = true`.
- rustfmt: no `rustfmt.toml` and the code is not rustfmt-formatted (long lines). Do not copy a rustfmt config from Rune; use greyscreen's own.
- Examples use `#![forbid(unsafe_code)]`. Keep Rust files under 800 non-test lines; tests in a sibling `tests.rs`; `lib.rs` lists one `mod`/`pub use` per line, sorted.
- Rune's state rule (AGENTS.md "State"): modal behaviour is one `enum` state plus `fn step(&mut State, Event) -> Effects<Effect>` (`rune::machine`), not clusters of booleans. Worth following for the recording-state lifecycle (Idle, Armed, Recording, Saving, Error).
- Performance rules to follow in the UI: no file I/O or process spawning in `view`, `poll` or handlers during frames (list recordings from the backend thread and push the result with `update`); `poll` returns true only when visible state changed; build only mounted rows (`virtual_rows`).

---

## 8. Recommendation for v1

Decisions, so the implementer makes none:

- Use `App` (not `rune::run`) with `.keep_running().release_gpu_when_closed()`; closing the window hides to tray, tray Quit calls `AppHandle::quit()`.
- Rune with defaults plus `native-file-dialog`; rune-kit with `default-features = false`; `Kit::neutral(cx.system_dark())`; native window frame; `set_app_icon(include_bytes!(..png))`; `windows_subsystem = "windows"` in release; `+crt-static`.
- Backend to UI: Pattern A (`AppHandle::update` with a `Msg` enum and `Ui::apply`). UI to backend: `mpsc::Sender<Command>` held in `Ui`.
- State machine for the recorder (`rune::machine`) lives in the backend or core crate, not the view.
- Min window 720 x 520, default 960 x 640, one window, three screens in a left `SegmentedControl`-style nav or a plain `row()` of `Button`s with `ButtonVariant::Ghost` (selected one `Neutral`). No `shell::SplitView` in v1 (it is built around macOS traffic lights).

### Screen 1: Status

| Element | Rune API |
|---|---|
| Recording state card: "Recording" / "Idle" / "Waiting for game", elapsed time | `Card::new().title(..).description(..).action(Badge::new("REC").variant(BadgeVariant::Accent).build(&k))`. Elapsed: `tx.wake_after(Duration::from_millis(250))` from `poll` while recording, return true when the second changed |
| Start / Stop button | `Button::new(..).variant(ButtonVariant::Submit \| Danger).on_press_tx(send_command)`; `.disabled(true)` while Saving |
| Current game: champion, mode, game time, "No game running" | `text(..)` rows inside a `Card`; set by `Msg::GameStarted(GameInfo)`; cleared by `Msg::GameEnded` |
| Live event feed (kills, deaths, objectives, with game timestamp) | scroll viewport `col().w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: true })` containing `col().growth(GrowthSpec::default()).virtual_rows(feed.len(), 28.0, key = event id, build = row with Badge::new(kind) + text)`. Cap the feed at 500 entries in `apply` |
| Error banner | `Card` with `.accent(red)`, or a `Toast` pushed into `layer` |
| Clip saved notice | `Toast::new("clip", "Clip saved").description(path).on_close(..)` plus `Button` "Show" calling `Shell::reveal` |

### Screen 2: Recordings

| Element | Rune API |
|---|---|
| List of recordings (name, date, duration, size, event count) | `Table::<Ui>::new(..).column(TableColumn::new("date","Date").sortable()).column(..).zebra(true).on_open(\|s, id, tx\| s.open_recording(id)).on_sort(..).build(s, &k)`; `TableState` in `Ui`; rows come from `Msg::Recordings(Vec<Recording>)` (scan the folder on the backend thread) |
| Open folder | `Button::new("Open folder").icon(&FOLDER).on_press(\|s: &mut Ui\| s.shell.reveal(&s.settings.output_dir))`. `Tx::open_url` refuses `file:`, so use the `Shell` trait: Windows `std::process::Command::new("explorer.exe").arg(path).spawn()` (from a thread; explorer returns exit code 1 on success), macOS `open` |
| Reveal one file | `explorer.exe /select,<path>` through the same trait |
| Empty state | `text("No recordings yet")` centred |
| Delete (if wanted) | `Dialog::new("del", "Delete recording?")` + Danger `Button`; not in v1 unless asked |
| Later: review screen | `Player::file(path)` + `.video(&player, SurfaceFit::Fit)` + custom timeline with markers (section 4) |

### Screen 3: Settings

| Setting | Rune API |
|---|---|
| Output folder | read-only `text(path)` + `Button::new("Change...").on_press_tx(\|_, tx\| tx.open_folder(FileDialogOptions { title: "Output folder".into(), ..Default::default() }, \|s: &mut Ui, r, _\| if let Ok(Some(p)) = r { s.set_output_dir(p) }))` (needs `native-file-dialog`; verified compile and signature) |
| Quality preset | `SegmentedControl::new(&["Low", "Medium", "High"], Some(i)).on_pick(\|s, i, _\| s.set_quality(i)).build(&k)` (3 to 4 presets fit; use `Select` if more) |
| Clip length (seconds) | `TextInput::new(&s.clip_secs).label("Clip length (s)").on_edit(..).on_change(\|s, tx\| s.validate_clip_len()).invalid(s.clip_len_invalid).build(&k)`. Parse to `u32` with a 5 to 300 range on change; apply on valid |
| Start recording with game | `Switch::new(b).label("Start with game").on_toggle(..)` (optional) |
| Persistence | Backend or core serializes `Settings` (serde, `%APPDATA%\Greyscreen\settings.json`) from a worker thread after each change (debounced via `tx.wake_after`), never in a handler or frame |

Each setting row: `row().w_full().items centered`: label `col()` on the left (`text(label).font(type_::LABEL)`, optional description in `tok::theme(dark).text_secondary`), control on the right; one `Card` per group. Settings changes call `Command::ApplySettings(Settings)` so the backend sees them.

Nav: a `row()` of `Button`s at the top with `.key("nav-status")` etc. Keys on every actionable element so `--script` taps and `rt.find("key")` work.

Test plan for the implementer: (1) unit tests on `Ui::apply` and the recorder state machine; (2) `Runtime` tests that tap keyed buttons and assert state; (3) a `--scene` per screen state (`idle`, `recording`, `feed-long`, `recordings-empty`, `recordings-many`, `settings-invalid-clip`) rendered with `--snap` at 960x640 scale 2 in dark and light and read as images; (4) `cargo xwin build --release` in CI-equivalent locally; (5) the first Windows run: window opens, tray icon, second launch raises the first window, close hides to tray, tray Quit exits, Task Manager shows near-zero GPU use while closed, a recorded H.264 MP4 plays in `Player`.

---

## 9. Pitfalls

1. `AppSpec::init`, `setup`, `poll`, `on_key` are `fn` pointers. Closures that capture the backend must go through `App::init(closure)` and `AppHandle`; do not reach for globals.
2. `App::init(..)` replaces `AppSpec::init` even in headless runs. Keep `AppSpec::init` returning a fake-backed `Ui` so `--snap` works, and skip `single_instance`, the tray and backend threads when `rune::is_headless()`.
3. Closing the window with `keep_running` hides it; the view and layout stop running while hidden, but `update`, `poll` and timers still run. State must stay correct without a frame: apply messages to state in `apply`, never rely on the view to consume events.
4. `AppHandle::update_if` returning false after a state change the view reads leaves the screen stale. Use `update` unless a message truly changes nothing.
5. `Tray` must be kept alive (drop removes the icon). Its callback runs on the tray thread: only use `AppHandle` there.
6. `Tx::open_url` refuses `file:` (only `http`, `https`, `mailto`). Open-folder is an app-side `explorer.exe` call, not a Rune API. `Tx::open_folder` is the folder picker, not "reveal in Explorer", and needs `native-file-dialog`; only one dialog at a time.
7. No file I/O, process spawn or `stat` on the UI thread during frames (Rune AGENTS.md: antivirus filter callbacks on Windows can stall). Scan recordings and write settings on worker threads.
8. Default CRT is dynamic: add `+crt-static` or users without the VC++ Redistributable cannot start the exe. Without `windows_subsystem = "windows"` a console window opens.
9. `clang-cl` must be on PATH for the cross-build (blake3 `build.rs`). `PATH=/opt/homebrew/opt/llvm/bin:$PATH`.
10. Window icon: `set_app_icon` must run before `run` and the PNG should be square and at least 256 px. The exe's Explorer icon is separate and needs a resource.
11. A `Select` trigger fills its parent's width; give it `.width(..)` and `.label(..)`. A `TextInput` with no `.label` and no placeholder has no accessible name (debug builds warn).
12. A grown scroll viewport with no bounded ancestor collapses to 0 pt (debug builds warn once per call site); `.virtualize` without `.growth` panics in debug.
13. `virtual_rows` keys must be stable ids, not indexes, or handlers act on the wrong row when rows are inserted above.
14. Rune has no Windows CI and is pre-release: pin the rev, commit `Cargo.lock`, bump deliberately, and re-check the patch table each bump. A rewritten `dev` is promised not to happen.
15. Rune's `dev` rule: apps build `AppSpec`, `RunOptions`, `Env` and similar as literals, so always end with `..AppSpec::new(..)`.
16. Video: seeks decode forward from the previous key frame, so long GOPs make marker clicks slow; local files must be plain drive-letter paths (UNC refused); an N edition of Windows without the Media Feature Pack cannot play video at all; HEVC, VP9 and AV1 need system codec extensions.
17. Always-dark UI plus a light OS gives a light native title bar. Either follow `cx.system_dark()` (v1 choice) or use `App::titlebar`.
18. The fake backend must produce real small MP4s (from a fixture) so the Mac can exercise the review screen; `Player` on macOS plays H.264 through AVFoundation.
19. Not verified at all: any run on Windows (D3D12 path, tray, single instance, window icon, folder picker, Media Foundation playback and seeking), seek while paused, `Titlebar` on Windows, release-build binary size tuning (`strip`, LTO).
