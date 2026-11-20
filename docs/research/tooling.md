# greyscreen: repo tooling and layout (research, 2026-11-20)

## Summary

- Lean set: rustc 1.99.1 pinned in `rust-toolchain.toml`, `rustfmt` (`style_edition = "2024"`, stable), `clippy` with a curated `[workspace.lints]` (clippy `all` plus a small deny list; no `pedantic`), `typos`, and `just` as the single task runner. No xtask, no lefthook, no nextest, no TOML formatter in phase zero.
- Windows from macOS works today: `cargo xwin clippy|check|build --target x86_64-pc-windows-msvc` ran clean on this machine (1.99.1, cargo-xwin 0.23.1). No env var is needed for the SDK license (using the tool is the consent); needs `clang` and `llvm` from `brew install llvm`.
- Layout: five crates under `crates/*` (`app`, `core`, `capture`, `events`, `mp4`). Every crate compiles on every host. The Windows-only crate is `#![cfg(windows)]` and is a `[target.'cfg(windows)'.dependencies]` entry of the app, so `cargo check --workspace` on macOS passes (verified).
- Release: `lto = "fat"`, `codegen-units = 1`, `strip = "symbols"`, `panic` left at unwind, `+crt-static`, `windows_subsystem = "windows"` in release only, manifest and icon via `winresource` (cross-compiles from macOS, uses `llvm-rc` that cargo-xwin puts on PATH; verified, `PerMonitorV2` ends up in the exe).
- Logging: `tracing` + `tracing-subscriber` + `tracing-appender` to `%LOCALAPPDATA%\greyscreen\logs`. Errors: `thiserror` in library crates, `anyhow` in `app` only.

## Findings

Versions checked against crates.io and GitHub on 2026-11-20.

| Tool | Version | Verdict |
|---|---|---|
| Rust | 1.99.1 (2026-10-13) | pin exactly, like Rune does |
| just | 1.59.0 | use |
| cargo-xwin | 0.23.1 | use (local only) |
| typos-cli | 1.52.2 | use (local + CI) |
| cargo-nextest | 0.9.152 | skip for now |
| cargo-deny | 0.22.0 | add when the dependency tree exists (CI only) |
| cargo-shear | 1.14.1 | optional later; preferred over cargo-machete 0.10.1 (shear understands cfg and workspace deps) |
| taplo-cli | 0.10.0 (2025-05-23) | skip; slow release cadence |
| tombi | 1.11.3 (2026-11-16, brew/pip/npm) | skip in phase zero; if a TOML formatter is wanted later, pick tombi. `tombi-cli` on crates.io is stale (0.0.1), install via brew |
| winresource | 0.1.34 | use |
| embed-resource | 3.0.12 | alternative, not needed |
| tracing / tracing-subscriber / tracing-appender | 0.1.44 / 0.3.23 / 0.2.5 | use |
| anyhow / thiserror | 1.0.104 / 2.0.21 | use |
| windows (windows-rs) | 0.62.2 | use in `capture` |
| actions/checkout | v7 (7.0.1) | use |
| actions/upload-artifact | v7 (7.0.2) | use |
| Swatinem/rust-cache | v2 (2.9.3) | use |
| crate-ci/typos | v1 (1.52.2) | use |
| taiki-e/install-action | v2 | only if nextest or deny are added |

### Formatter

- `style_edition = "2024"` is accepted on stable 1.99.1 with no warning (tested). With `edition = "2024"` in Cargo.toml, `cargo fmt` already uses the 2024 style, but a standalone `rustfmt` run by an editor does not see the edition, so one explicit line is worth keeping.
- Do not use unstable options (`imports_granularity`, `group_imports`, `wrap_comments`): they need nightly, and the repo pins stable.

### Linter

- Rune's approach (read from its root Cargo.toml and clippy.toml): no `pedantic` group. A curated list: `deny` for `dbg_macro`, `await_holding_lock`, `await_holding_refcell_ref`, `disallowed_methods`, `disallowed_types`, `let_underscore_must_use`, `undocumented_unsafe_blocks`; `warn` for `unwrap_used`, `expect_used`, `todo`. Rust lints: `unsafe_op_in_unsafe_fn` and `unused_must_use` deny. `clippy.toml` allows unwrap/expect/panic/dbg in tests. Rune also uses `disallowed-methods` to force its own clock and env wrappers.
- greyscreen copy: the same shape. `pedantic` is noisy (doc_markdown, must_use_candidate, cast lints on every pixel and timestamp conversion in a video app); it adds friction without catching real bugs.
- CI runs clippy with `-D warnings`, so `warn` in the table is a hard gate in CI and a nudge in the editor.
- `unsafe_code` is not denied workspace-wide because `capture` needs `unsafe` for Win32 and COM. Put `#![forbid(unsafe_code)]` at the top of `core`, `events`, `mp4`, and keep `undocumented_unsafe_blocks = "deny"` so every unsafe block has a `// SAFETY:` line.

### Compiler check on Windows from macOS (verified)

- Setup: `rustup target add x86_64-pc-windows-msvc` (already in the toolchain file), `brew install llvm`, `cargo install --locked cargo-xwin`.
- First use downloads the MSVC CRT and Windows SDK into `~/Library/Caches/cargo-xwin` (already cached here). The README says that using the software accepts Microsoft's license; the binary has no `XWIN_ACCEPT_LICENSE` variable (checked by grepping the binary). Only the `XWIN_*` options in its README exist (`XWIN_CACHE_DIR`, `XWIN_ARCH`, ...). In CI on Windows none of this is needed.
- `cargo xwin clippy` works (it is a passthrough of any cargo subcommand: `check`, `clippy`, `build`, `test`). Ran `cargo xwin clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` on the 5-crate skeleton: clean in 3 s warm.
- `cargo xwin test` needs wine to run Windows test binaries; skip it. Windows tests run on the Windows PC and in CI.
- `cargo xwin` sets `PATH` to include `/opt/homebrew/opt/llvm/bin`, `AR=llvm-lib`, `RCFLAGS` with the SDK include dirs. That is why `llvm-rc` is found by `winresource`.
- `cargo xwin build --release` produced a PE32+ GUI-subsystem exe, imports only KERNEL32, ntdll and api-ms-win-core-synch (static CRT, no vcruntime140.dll).

### Test runner

- `cargo test` for now. The workspace is small and mostly pure logic. nextest (0.9.152, `brew install cargo-nextest`) gives per-test process isolation and faster runs, but it does not run doctests and adds an install step to CI. Revisit past a few hundred tests or if a test needs process isolation (global state in the capture crate).

### Task runner

- `just` (justfile, one file, recipes can chain). Cargo aliases cannot chain commands or pass the long `-- -D warnings` tail cleanly, and the Windows-via-xwin command is not a plain cargo alias. xtask is a Rust crate for a problem this repo does not have. `just` is already installed on this Mac. CI calls `cargo` directly (no `just` install needed on runners).
- The recipes below ran on the skeleton (`fmt`, `lint`, `check-win`, `build-win`, `test`), except `typos`, which is not installed on this Mac yet.

### Git hooks

- None. A pre-commit hook that runs clippy twice (host and Windows) is slow and gets bypassed. CI is the gate, `just lint` is the habit. If one is wanted later: a plain `.git/hooks`-free `lefthook` with `cargo fmt --check` only.

### CI

- `windows-latest`: fmt, clippy `-D warnings`, test, release build, upload `greyscreen.exe`. `macos-latest`: clippy and test (the platform-agnostic crates run there; the capture crate is an empty crate). No `dtolnay/rust-toolchain`: with rustup 1.29.3, `rustup toolchain install` with no arguments installs the version in `rust-toolchain.toml` (tested locally), so the pin has one source of truth. If you prefer the action, `dtolnay/rust-toolchain` only has branch refs (`@master`, `@stable`, `@1.99.1`), no release tags.
- Not run in a real GitHub run (no remote yet). Check the first run.

### Release profile, subsystem, manifest, icon

- `lto = "fat"` + `codegen-units = 1` + `opt-level = 3` for peak speed of the muxer and color conversion; build time is higher. `opt-level = "s"` is not worth it for a desktop recorder. `strip = "symbols"` on release. Keep `debug = false`; if crash reports with symbols are needed later, use `split-debuginfo = "packed"` (produces a .pdb on MSVC) rather than not stripping.
- Panic: keep the default (unwind). Reasons: (1) a panic in a capture or encode thread can be caught (`catch_unwind` or a thread join) so the mp4 gets finalized instead of losing the clip; (2) panics that reach an `extern "system"` COM or Win32 callback boundary abort the process regardless of the `panic` setting (since Rust 1.81, unwinding out of a non-`-unwind` ABI aborts), so `abort` buys nothing for safety and costs recoverability. Size gain from `abort` is small. Still: never let code in a callback panic; catch and log inside the callback. Install a panic hook that writes to the log file, since a GUI-subsystem exe has no console to print to.
- `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` at the top of `crates/app/src/main.rs`: release has no console, debug keeps one for `println!`. Verified `Subsystem 2 (Windows GUI)` in the cross-built exe.
- Static CRT: `-C target-feature=+crt-static` for the MSVC target, so the user needs no VC++ redistributable.
- Manifest: embed with `winresource` (`set_manifest_file`, `set_icon`). On a non-Windows host with the MSVC target it runs `llvm-rc` from PATH (or `RC_PATH`); with `cargo xwin` this is automatic. A bare `cargo check --target x86_64-pc-windows-msvc` without xwin would fail in build.rs unless llvm-rc is on PATH, so always use `cargo xwin` for the Windows target on macOS. Gate on `CARGO_CFG_TARGET_OS` at run time, not `cfg!(windows)` (build.rs runs on the host).
- Manifest content: `PerMonitorV2` dpiAwareness (plus `dpiAware` true/pm for older fallbacks), `supportedOS` Windows 10/11 GUID, `longPathAware`, `activeCodePage` UTF-8. Common Controls v6 (`Microsoft.Windows.Common-Controls` dependency) is only needed if the app uses themed native controls (TaskDialog, MessageBox visual styles, file dialogs with modern look). A GPU-drawn Rune UI does not need it; add the dependency block only when a native dialog is introduced. Whether Rune sets DPI awareness itself at runtime was not checked; the manifest is the reliable place regardless.
- Icon: `.ico` with 16, 32, 48, 256 px frames at `crates/app/assets/greyscreen.ico`.

### Workspace layout

- Flat `crates/*`, package names prefixed `greyscreen-`, the binary just `greyscreen`. `members = ["crates/*"]`, so adding a crate needs no root edit. `workspace.dependencies` holds all shared versions; crates inherit with `foo.workspace = true` and `[lints] workspace = true`.
- Windows-only gating: the crate itself always exists and always parses. `capture/src/lib.rs` starts with `#![cfg(windows)]` (empty crate elsewhere), its `windows` dependency sits in `[target.'cfg(windows)'.dependencies]`, and `app` depends on it in `[target.'cfg(windows)'.dependencies]` and calls it under `#[cfg(windows)]`. Alternative (`[target.'cfg(windows)'.dependencies]` only, no crate-level cfg) fails on macOS, because the crate's source would still reference `windows::`. The events client uses the LoL Live Client Data API (HTTPS on localhost with a self-signed cert), which is platform-agnostic, so it stays testable on macOS.
- `core` has no I/O or OS code: state machines, event model, config, clip logic. Everything in it is tested on macOS with `cargo test`.

```
greyscreen/
  Cargo.toml  Cargo.lock  rust-toolchain.toml  rustfmt.toml  clippy.toml  justfile
  .cargo/config.toml  .editorconfig  .gitignore  .gitattributes  _typos.toml
  .github/workflows/ci.yml
  AGENTS.md  CLAUDE.md (symlink to AGENTS.md)  README.md  LICENSE
  docs/
  crates/
    app/       package "greyscreen"          UI binary (Rune), build.rs, app.manifest, assets/greyscreen.ico
    core/      package "greyscreen-core"     state machines, event model, config, clip logic
    capture/   package "greyscreen-capture"  Windows-only capture + encode (cfg(windows))
    events/    package "greyscreen-events"   LoL Live Client Data API client
    mp4/       package "greyscreen-mp4"      mp4 muxer
```

### Logging and errors

- `tracing` + `tracing-subscriber` (features `env-filter`, `fmt`) + `tracing-appender` (daily rolling file, non-blocking writer). The capture and encode paths are multi-threaded with spans per clip; structured spans are worth it over `log` + `env_logger`, and a file appender exists only on the tracing side (env_logger writes to stderr, which a GUI exe does not have). Keep the appender `WorkerGuard` alive for the whole of `main`. Log dir: `%LOCALAPPDATA%\greyscreen\logs` (use the `dirs` crate or `SHGetKnownFolderPath`). Set a panic hook that calls `tracing::error!`.
- Errors: `thiserror` in `core`, `events`, `mp4` (callers match on variants); `anyhow` in `app` and at the top of the capture pipeline, where errors are only logged. Both is the standard split, not duplication.

### Files a polished repo has

- `README.md` (what it is, build on macOS, build on Windows, run), `LICENSE` (MIT; Rune is MIT too, so no conflict), `rust-toolchain.toml`, `.editorconfig`, `.gitignore`, `.gitattributes`, `AGENTS.md` with `CLAUDE.md` as a symlink to it (Rune keeps both files).
- Skip `CONTRIBUTING.md` (single maintainer) and `CHANGELOG.md` until the first release.
- Commit `Cargo.lock` (it is an application).

## Recommendation

Tools to install on the Mac:

```sh
brew install just typos-cli llvm
cargo install --locked cargo-xwin        # 0.23.1
# later, only when needed:
# brew install cargo-nextest cargo-deny tombi
# cargo install --locked cargo-shear
```

### rust-toolchain.toml

```toml
[toolchain]
channel = "1.99.1"
profile = "minimal"
components = ["clippy", "rustfmt"]
targets = ["x86_64-pc-windows-msvc"]
```

### rustfmt.toml

```toml
style_edition = "2024"
```

### clippy.toml

```toml
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-dbg-in-tests = true
```

### Cargo.toml (root)

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.99"
license = "MIT"
publish = false

[workspace.dependencies]
greyscreen-core = { path = "crates/core" }
greyscreen-capture = { path = "crates/capture" }
greyscreen-events = { path = "crates/events" }
greyscreen-mp4 = { path = "crates/mp4" }
windows = "0.62"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
anyhow = "1"
thiserror = "2"

[workspace.lints.rust]
unsafe_op_in_unsafe_fn = "deny"
unused_must_use = "deny"
unreachable_pub = "warn"
missing_debug_implementations = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
await_holding_lock = "deny"
dbg_macro = "deny"
let_underscore_must_use = "deny"
undocumented_unsafe_blocks = "deny"
expect_used = "warn"
todo = "warn"
unimplemented = "warn"
unwrap_used = "warn"

[profile.dev]
debug = "line-tables-only"

[profile.dev.package."*"]
opt-level = 2

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = "symbols"
```

Each crate's Cargo.toml: `version.workspace = true`, `edition.workspace = true`, `rust-version.workspace = true`, `license.workspace = true`, `publish.workspace = true`, and `[lints] workspace = true`. The `app` and `capture` manifests:

```toml
# crates/app/Cargo.toml
[dependencies]
greyscreen-core.workspace = true
greyscreen-events.workspace = true
greyscreen-mp4.workspace = true

[target.'cfg(windows)'.dependencies]
greyscreen-capture.workspace = true

[build-dependencies]
winresource = "0.1"

# crates/capture/Cargo.toml
[dependencies]
greyscreen-core.workspace = true

[target.'cfg(windows)'.dependencies]
windows = { workspace = true, features = ["Win32_Foundation"] }
```

```rust
// crates/capture/src/lib.rs
#![cfg(windows)]
```

```rust
// crates/app/build.rs
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_manifest_file("app.manifest");
        res.set_icon("assets/greyscreen.ico");
        res.compile().expect("compile windows resources");
    }
}
```

```xml
<!-- crates/app/app.manifest -->
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
      <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>
```

(The skeleton test embedded only the `dpiAwareness` element; the full file above is untested beyond XML shape. Run the first build and check with `strings`.)

### .cargo/config.toml

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

### justfile

```make
set shell := ["bash", "-euo", "pipefail", "-c"]

win := "x86_64-pc-windows-msvc"

default:
    @just --list

fmt:
    cargo fmt --all

lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    typos

check-win:
    cargo xwin clippy --workspace --all-targets --target {{win}} --locked -- -D warnings

test:
    cargo test --workspace --locked

build-win:
    cargo xwin build --release --locked --target {{win}} -p greyscreen
    @ls -lh target/{{win}}/release/greyscreen.exe

ci: lint check-win test
```

Plain `cargo check --workspace` on macOS is the host compile check. `cargo xwin check --target x86_64-pc-windows-msvc --workspace` is the Windows one; `check-win` uses clippy, which includes the check.

### .github/workflows/ci.yml

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: 0

jobs:
  windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v7
      - name: Install toolchain
        run: rustup toolchain install
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --locked -- -D warnings
      - run: cargo test --workspace --locked
      - run: cargo build --release --locked -p greyscreen
      - uses: actions/upload-artifact@v7
        with:
          name: greyscreen-windows-x64
          path: target/release/greyscreen.exe
          if-no-files-found: error

  macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v7
      - name: Install toolchain
        run: rustup toolchain install
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets --locked -- -D warnings
      - run: cargo test --workspace --locked

  typos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: crate-ci/typos@v1
```

The windows job builds with the native MSVC target (the runner has Visual Studio Build Tools and `rc.exe`); `rust-toolchain.toml` lists the MSVC target, which is the host there, so it is a no-op. `+crt-static` from `.cargo/config.toml` applies. `cargo fmt --check` only on Windows avoids duplicate work; the `.gitattributes` `eol=lf` rule stops CRLF from failing rustfmt on the Windows checkout.

### .editorconfig

```ini
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true
indent_style = space
indent_size = 4

[*.{toml,yml,yaml,json,md,manifest,xml}]
indent_size = 2

[justfile]
indent_size = 4

[*.md]
trim_trailing_whitespace = false
```

### .gitignore

```gitignore
/target
**/*.rs.bk
.DS_Store
Thumbs.db
*.pdb
*.mp4
/.claude/settings.local.json
.env
```

(`*.mp4` ignores test recordings; use `!docs/**/*.mp4` if an mp4 must be committed.)

### .gitattributes

```gitattributes
* text=auto eol=lf
*.rs text eol=lf
*.toml text eol=lf
*.md text eol=lf
*.yml text eol=lf
justfile text eol=lf
*.manifest text eol=lf
*.rc text eol=lf
*.ico binary
*.png binary
*.mp4 binary
```

### _typos.toml (only when typos flags a false positive)

```toml
[files]
extend-exclude = ["Cargo.lock"]
```

## Pitfalls

- A bare `cargo check --target x86_64-pc-windows-msvc` on macOS fails as soon as a crate needs the Windows SDK headers or `llvm-rc` (build scripts, `cc`). Always go through `cargo xwin`.
- `cargo xwin` needs Homebrew LLVM. `brew install llvm` is keg-only; xwin finds `/opt/homebrew/opt/llvm/bin` itself, but your shell PATH does not have it, so `llvm-rc` is not found outside xwin.
- Do not put Windows-only code behind only a `[target.'cfg(windows)'.dependencies]` table: the crate still compiles on macOS and its `use windows::...` fails. Use `#![cfg(windows)]` in the crate root, or `#[cfg(windows)]` on every Windows module.
- `cfg!(windows)` in `build.rs` tests the host, not the target. Read `CARGO_CFG_TARGET_OS`.
- `windows_subsystem = "windows"` removes stdout, stderr and panic output. Without a log file and a panic hook, crashes are invisible. Gate it with `not(debug_assertions)` so dev builds keep a console.
- `tracing-appender`'s non-blocking writer drops its buffer if the `WorkerGuard` is dropped early. Bind it in `main` and keep it alive to the end.
- Panics never unwind through `extern "system"` callbacks (abort). Catch inside the callback. `panic = "abort"` would also kill the finalize-on-panic path for the mp4.
- `fat` LTO with `codegen-units = 1` makes release builds slow, and the Rune dependency tree is large. Fine for release artifacts; use `cargo build` (dev, deps at opt-level 2) day to day.
- `Swatinem/rust-cache` keys on the lockfile and rustc version; the `rust-toolchain.toml` bump invalidates it, as intended. It does not cache workspace crates by default.
- rustup 1.28 and later no longer installs the toolchain on first `cargo` call; the CI step `rustup toolchain install` is required.
- `dtolnay/rust-toolchain` has no `v1` style release tags usable for pinning to a Rust version; use the branch ref if the action is wanted.
- `cargo-shear`/`cargo-machete` flag deps used only under `cfg(windows)` unless they understand cfg; shear does. Run it from the Windows build if it complains.
- Applies only on first use: the SDK download for cargo-xwin is several hundred MB, in `~/Library/Caches/cargo-xwin`. Not tested here: a cold download (the cache already existed).
- Not run: the CI YAML (no remote), `typos` (not installed on this Mac), `cargo-deny`, `cargo-shear`, nextest, the full manifest, the icon embedding (no .ico in the probe), running the exe on Windows.

## Sources

- Rune repo, shallow clone: rust-toolchain.toml, clippy.toml, Cargo.toml (`[workspace.lints]`, `[profile.*]`), .cargo/config.toml, .gitattributes, .github/workflows/macos-tests.yml
- cargo-xwin README: https://github.com/rust-cross/cargo-xwin (v0.23.1; license consent text, `XWIN_*` options, llvm prerequisite)
- winresource source (0.1.34, `lib.rs`: `llvm-rc` on non-Windows MSVC targets, `RC_PATH`): https://github.com/mullvad/winresource
- crates.io API for current versions of just, cargo-nextest, cargo-deny, cargo-machete, cargo-shear, typos-cli, taplo-cli, tombi-cli, cargo-xwin, winresource, embed-resource, tracing*, anyhow, thiserror, windows
- GitHub releases: actions/checkout, actions/upload-artifact, Swatinem/rust-cache, crate-ci/typos, taiki-e/install-action, casey/just, tombi-toml/tombi, nextest-rs/nextest, EmbarkStudios/cargo-deny-action
- Tombi: https://github.com/tombi-toml/tombi; Taplo: https://github.com/tamasfe/taplo
- Local experiments (scratchpad `probe/` and `ws/`): `cargo xwin clippy` and `build --release`, rustfmt `style_edition = "2024"`, `rustup toolchain install` reading the toolchain file, `objdump` of the built exe
