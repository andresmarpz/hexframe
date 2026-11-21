# How anomalyco/hex is configured

Source: clone at `(scratch clone)`, `dev` at `80b84f7b`-era (2026-11-20/21), 2,030 commits, version 3.4.0. Paths below are relative to that repo. Everything here was read from the files and `git log`, not from memory.

## 1. Early setup

- The repo did not start empty. The first commit (`1ede22ec`, 2026-07-14, Kit Langton, "feat: add native macOS voice control") is a 26-file, 7,931-line drop. It was a project called "voice-control" (the AGENTS.md title was "Voice Control Agent Guide") that later became Hex.
- That first commit already contained, all at once:
  - `AGENTS.md` (96 lines: purpose, architecture module list, invariants, dev commands)
  - `Cargo.toml` and `Cargo.lock`, `.gitignore`, `README.md`, `ROADMAP.md`
  - `scripts/setup.sh` and `scripts/setup-parakeet.sh`
  - `src/*.rs`, one flat module per concern (audio, commands, config, dictation, events, feedback, keyboard, main, moonshine, parakeet, paste, recognition, suppression), plus `resources/audio/*.mp3`
- So the agent guide, the roadmap and the scripts folder came with commit 1. No CI, no justfile, no toolchain pin, no rustfmt/clippy config existed then.
- Order of structure over the next ~30 commits (all 07-14 to 07-21):
  - 07-15 `64a340a8` "ship HEX v2 desktop app" adds the app bundle layer (`app/Info.plist`, packaging).
  - 07-17 `b1671672` "ship HEX 2.0.9" adds `release-notes/` (2.0.0 to 2.0.9 are the only notes ever; release notes later moved to the release service and PR-title dev notes).
  - 07-18 first PR (#1, `feat/linux-x11-dictation-pr`), Linux install scripts.
  - 07-19 signed Linux auto-update, README trimmed to "installation and use", `site/` marketing site, iOS keyboard.
  - 07-21 local transcription service.
- Early commits were direct pushes to the default branch by one person. Discipline (PRs, checks, rulesets) came later, around late October when Dax joined (see section 5).
- Subject style from day one: Conventional-Commit-ish `feat:` / `fix:` / `refactor:` / `docs:`. By November that changed (section 5).

## 2. Agent configuration

What exists:
- `AGENTS.md` only, at the root (135 KB, 1,979 lines). No `CLAUDE.md`, no `.claude/`, no `.cursor/`, no `.codex/`. There is one skill: `skills/hex-personal-commands/SKILL.md`, which is a product skill for Hex users (how to write `hex.config.ts`), not a dev-agent rule.
- `.hivemind.toml` configures their own land/verify tooling (verify runs `cargo test --locked --workspace --lib` and a local app build, then installs `/Applications/Hex.app`).
- `.github/agent-policy/check.ts` plus `.github/workflows/agent-policy.yml` is a deterministic gate for PRs opened by their automated issue pipeline (head branch `agent/*`). See below.

Structure of `AGENTS.md`: `Private Kit Tool` (workflow), `Purpose`, `Feature Map`, `Architecture` (about 600 lines, one bullet per module), `Invariants` (about 900 lines of behavior rules), `Development`, `Diagnostics`, `Future Direction`, `Current Work`. It is a spec, not a style guide. It is far too long to copy as is; the useful part is the shape.

Most useful rules, quoted:
- Workflow (top of file): "`dev` and `production` take changes only through pull requests ... Branch from `dev`, open a PR to `dev`, and merge it yourself once it compiles (don't wait on CI); merge any open PR without asking."
- Golden images: "Any change to what the window draws regenerates the goldens in the same PR: run `HEX_UPDATE_GOLDENS=1 cargo test --locked --bin hex rune_ui` on Linux ... look at every changed image, and commit them with the change."
- New features go behind a flag: "**new or user-requested features land in Labs, off by default, not on by default.** ... Never add a master switch or ship such a feature on by default."
- Docs stay current: "Update the affected map in the same change when behavior, entry points, defaults, prerequisites, platform availability, recovery, or verification coverage changes."
- Honest verification: "A test's existence is not a passing run, and a preview is not native end-to-end proof. Record missing coverage and known defects explicitly; never rewrite an expected outcome merely to hide a regression."
- Local loop: "Use `scripts/check [filter]` for the local loop, not raw `cargo test` ... Read its output whole rather than piping it through `grep | head`."
- No speculative abstraction: "Do not add hypothetical seams for roadmap items. Introduce a seam once there are two real adapters or a current test requires substitution." and "Keep modules deep. Callers should not coordinate ... handles, formats, ... details."
- Release safety: "Never overwrite a published artifact with different bytes." "Do not reuse a released version for changed code."
- File size is enforced by script, not prose: `scripts/code-shape.py` (see section 3).
- Not present: no tone/length rules for agent replies, no commit message rules, no "write tests for everything" rule. Test duty is expressed as golden images plus the check script.

Agent policy gate (`.github/agent-policy/check.ts`, "No AI"): a Bun script that reads the PR diff and returns clean / flagged / dax.
- Dax-only paths: `.github/`, CODEOWNERS, release/signing scripts, updater code, entitlements.
- Flagged paths need a strict reviewer label (`agent-strict-review`): `build.rs`, `.cargo/`, any `Cargo.toml`/`Cargo.lock`, `packaging/`, `AGENTS.md`/`CLAUDE.md`, `.config/`, JS deps.
- Line rules flag new URLs, process spawning, network APIs, `unsafe`, encoded blobs, bidi/zero-width characters, secret env reads. Limits: 600 changed lines, 25 files.
- It runs from the base branch's copy, so a patch cannot edit its own policy.

History of `AGENTS.md` (`git log --follow`): 343 commits touching it, since 2026-07-14.
- 96 lines at birth, about 1,979 now (about 20x). It was touched about 20-27 times a day at the busiest (10-27, 10-30, 10-31).
- It is edited inside feature PRs, not in separate docs commits: the rule "update the doc in the same change" is followed in practice. Early edits were "docs: clarify command ownership", "docs: remove captain's log guidance"; later ones are feature PRs that carry the AGENTS.md hunk.
- Risk to note: at 135 KB it is bigger than most agent context budgets. They get away with it because agents read targeted sections and the Feature Map index.

## 3. Tooling

Rust toolchain
- No `rust-toolchain.toml`. CI uses `dtolnay/rust-toolchain` with `toolchain: stable` and components `rustfmt, clippy`. Edition 2024. Docs tell Windows users to use `stable-x86_64-pc-windows-msvc`.
- No `rustfmt.toml`, no `deny.toml`, no `typos`, no pre-commit hooks, no husky/lefthook. None of these exist (`git ls-files` checked).
- Formatting: `cargo fmt --all --check` in `scripts/check` and in Linux/Windows CI.
- `clippy.toml` (marked "shared-lints", copied from a family of repos): `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`, `allow-dbg-in-tests`; `disallowed-methods` bans `std::env::var` ("read env in config.rs at startup") and `std::time::SystemTime::now` ("use config::now() or an injected clock").
- Workspace lints in `Cargo.toml`: rust `unsafe_op_in_unsafe_fn = deny`, `unused_must_use = deny`; clippy `await_holding_lock`, `await_holding_refcell_ref`, `dbg_macro`, `disallowed_types` = deny, `todo = warn`. The stricter shared ones (`unsafe_code`, `disallowed_methods`, `let_underscore_must_use`, `undocumented_unsafe_blocks`, `expect_used`, `unwrap_used`) are set to `allow` with comments counting the errors left ("346 errors left"). That is a ratchet written as comments. Crates opt in with `[lints] workspace = true`.
- CI runs clippy with `-D warnings`.
- Profiles: `[profile.dev] debug = "line-tables-only"` and `[profile.dev.package."*"] debug = false` (comment: full debug info was about 19 GB per worktree). `[profile.release] strip = true`. No LTO, no codegen-units, no opt-level tweaks.

`.cargo/config.toml` (only 2 things): static CRT on both Windows targets (`-C target-feature=+crt-static`, so no VC++ redistributable), and an `[env] CMAKE_ARGS` for the native speech dependency (MSVC runtime match, Vulkan shader subset).

Scripts (no justfile, no Makefile)
- `scripts/check` is the one local gate. Runs fmt, `code-shape.py`, the Nix git-pin check, clippy (`--all-targets -D warnings`), then tests via cargo-nextest. Exit codes name the failing stage (10 fmt, 11 clippy, 12 tests, 13 code-shape, 14 nix pins). `scripts/check <filter>` skips clippy and runs matching tests only. Falls back to `cargo test` if nextest is missing.
- `.config/nextest.toml`: each test is its own process, `slow-timeout` 15s with `terminate-after = 2`, so a hung test is killed and named. GPU tests share a `gpu` group limited to 2 threads. A `ci` profile has longer timeouts and writes JUnit.
- `scripts/code-shape.py`: files warn above 800 non-test lines and fail above 1,500; test files warn at 1,000; folders warn above 40 source files and fail above 60; prefix families (`x.rs` beside `x_*.rs`) fail and must become `x/`. Existing offenders live in `scripts/file-size-exceptions.txt` (17 entries) and `folder-shape-exceptions.txt` with ceilings that only go down. A Rust test (`src/code_shape_tests.rs`) also runs it.
- `scripts/check-nix-git-pins.py` fails if Nix's recorded git revs differ from `Cargo.lock`.
- Many `build-*`, `release-*`, `smoke-*`, `install-*` scripts; release scripts live in `.github/release/`.

CI (`.github/workflows/`, 15 files)
- Philosophy, from the comments and AGENTS.md: "CI is minimal (Dax)". A PR runs only `hex-agent-policy` and `compile.yml`. A push to `dev` runs no checks except Release/Test builds. The full suites run only inside Release's `check` and `stable` channels or by hand.
- `compile.yml`: triggers on PRs touching code paths (`Cargo.*`, `src/**`, `crates/**`, `native/**`, ...), non-draft only. Three jobs: Linux (fmt + `cargo clippy --locked --bin hex -D warnings`), macOS (clippy), Windows x64 (clippy, plus `hex-console` clippy and tests). Concurrency group cancels stale runs. `permissions: contents: read`, `persist-credentials: false`.
- `check-linux.yml`, `check-macos.yml`, `check-windows.yml`: reusable (`workflow_call` with a `sha` input) and manual. Windows covers x64 (Vulkan) and ARM64 and runs `cargo nextest run --release --profile ci`, then a dumpbin check: no `VCRUNTIME`/`MSVCP` imports, `vulkan-1.dll` delay-loaded not statically imported, and subsystem is "Windows GUI".
- Caching: `Swatinem/rust-cache` with `cache-on-failure: true`; on Windows `env-vars: INCLUDE LIB ImageVersion` so cache keys follow the MSVC toolset (a cached object from a newer toolset fails to link); Vulkan SDK install with `cache: true`; Nix uses magic-nix-cache.
- All third-party actions are pinned by commit SHA with a version comment.
- `test-builds.yml`: on push to `dev`, unsigned downloadable builds for teammates (macOS test app, Windows x64/ARM64 exe + installer, Linux deb/rpm). `docs/test-builds.md`.
- `release.yml` (push to `dev` cuts a dev build; manual `channel` = check | dev | stable): plan, build (no secrets), sign (in the `release` environment, sparse checkout of `.github/release/` only), verify, installers, full suites + onboarding goldens for stable, publish via OIDC to their release service, tag, GitHub mirror, AUR, Homebrew, dev notes. See `docs/releasing.md` for the full tree. Paths-ignore `docs/**`, `site/**`, `**/*.md` so docs-only changes cut no build.
- Others: `deploy.yml` (site to Cloudflare), `release-ios.yml`, `release-typescript.yml` (npm trusted publishing, Changesets), `aur-check.yml`, `windows-installer(s).yml`, `windows-setup-engine.yml`.
- `.github/CODEOWNERS`: `/.github/ @thdxr`. Ruleset "release boundary (.github needs Dax)". Branch rules: `dev` and `production` are PR-only; required status check is `hex-agent-policy` only; no `main`.

Versioning and changelog
- Version in `Cargo.toml` (3.4.0), tags `vX.Y.Z` (older ones `app-vX.Y.Z`). Dev builds are stamped `0.0.0-dev-<run number>` by `scripts/set-crate-version.sh` in the build only, never committed. Prereleases are `X.Y.Z-beta.N`. `build.rs` reads it into `HEX_BUILD_VERSION`; a local build reports `0.0.0-dev-local-<commit>[-dirty]`.
- No CHANGELOG file. `release-notes/` froze at 2.0.9. Now: PR titles are the notes ("dev-notes-stage": PR titles since the last dev build go to `dev-notes.json` on a `dev-notes` branch). So PR titles are written for users.
- The TypeScript SDK uses Changesets (`sdk/.changeset/`).

## 4. Using Rune in an app

Dependency wiring (root `Cargo.toml`)
- Rune is a plain public git dependency, pinned by full rev, on three crates plus two utilities:
  `rune`, `rune-kit`, `rune-theme-opencode`, `rune-motion`, `procwatch`, all `{ git = "https://github.com/anomalyco/rune.git", rev = "<sha>" }`. Not crates.io. `rune` features: `native-file-dialog`, `macos-background-cursor`.
- **Current Rune rev: `c853811c8e0e60a632eaa2e72992f871225737ef`** ("bump(rune): c853811c, a settle lands before the frame's input", 2026-11-19, hex commit `2747e34e`). `Cargo.lock` agrees (`git+https://github.com/anomalyco/rune.git?rev=c853811c...`).
- `[patch.crates-io]` re-applies Rune's own patches because "a dependency's `[patch]` never reaches the workspace that depends on it". It points these at the same Rune rev: `winit`, `sctk-adwaita`, `parley_engine`, `wgpu-hal`, `accesskit_windows`, `accesskit_macos`. Comment: patch winit and sctk-adwaita together; bump with the rune pin. (Rune's README has a "Depending on Rune" section describing this.) This is the first thing to check against our own repo: without it you get stock winit and stock wgpu-hal, not Rune's forks.
- Other deps: `wgpu = "30"` with `std, parking_lot, metal, wgsl` for the welcome hero's own GPU pass; Windows uses `windows-sys` 0.61 and `windows` 0.62.2.
- `build.rs` has `check_windows_split()`: fails every OS's build if `Cargo.lock` has `gpu-allocator` and `wgpu-hal` on different `windows` crate versions (it broke the Windows build twice after `cargo update -p rune`, #997 and #1085). Pin bump recipe (`docs/plans/rune-dependency.md`): change `rev` on all crates together, run `cargo update -p rune -p rune-kit -p rune-theme-opencode`, keep `windows` 0.62.2 for gpu-allocator.
- Bump cadence: the Rune rev line changed on 12 distinct days between 11-03 and 11-19 (up to 9 times on 2026-11-04). It is bumped per fix, in its own small PR titled "bump rune: <what it fixes>".
- Both repos by the same team; Rune has a single published root commit (`7eea3a3`), so old revs vanish: always pin a rev that exists on Rune's branches.

Windows app setup (all in `src/main.rs`, `src/windows.rs`, `src/windows/*.rs`, `build.rs`)
- **DPI / manifest**: Hex does nothing itself. There is no `.manifest` file, no `SetProcessDpiAwareness*` call. `build.rs` `embed_windows_resources()` writes an `.rc` with only the icon (`1 ICON`) and a `VERSIONINFO` block, compiled with `embed-resource` and `.manifest_optional()`. DPI is entirely Rune/winit's job. Hex only converts when restoring size: `tx.resize_window((width as f32 / scale, height as f32 / scale))` in `src/windows/rune.rs` (`keep_window_frame`).
- **GUI subsystem**: `#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]` in `src/main.rs`. A tiny `hex-console` crate builds `hex.com`, the console entry point, which waits on `hex.exe` and passes the exit code. `windows::attach_parent_console()` joins the parent terminal for CLI use; `detach_from_console()` before the app runs.
- **`main()`** (Windows variant), in order: `windows::restrict_dll_search()` (DLL hardening, first), `attach_parent_console()`, `windows::vulkan::check()` (delay-loaded `vulkan-1.dll`; sets `GGML_DISABLE_VULKAN` if loader below 1.2), `windows::gpu_fallback::choose()`, CPU floor check (AVX2), then `windows::run(&SHUTDOWN)`. `run` parses the CLI, handles control commands and headless `--snap` renders without touching app data, then `init_process_logging`, then `windows::rune::open(...)`.
- **Single instance**: `windows::app::AppInstance::claim` creates a named auto-reset event. If it already exists, the new launch signals it (after `AllowSetForegroundWindow`) and exits; a `--hidden` login launch only exits. The first process runs a `hex-relaunch` thread that waits on the event and calls `AppHandle::open()`. A separate `desktop::instance` lock file guards the listener.
- **Tray**: `rune::Tray::new(TraySpec::new("hex","Hex",icon))` (Rune's tray, not tray-icon). Creating it waits for Explorer, so it is built on its own thread `hex-tray-setup` while overlays install. If there is no tray, closing the window quits ("Hex must not run on with no way back to it"). Tray events call `AppHandle::open()` or send a `Request`.
- **Window lifecycle**: `app.keep_running().release_gpu_when_closed()` (or `.windowless()` for a login launch). Closing destroys the native window and swapchain so a tray-resident Hex is not mistaken for a game by NVIDIA Instant Replay (#1308). Before closing it records size and position (`WindowHandle::set_position`) so it reopens as the user left it. `rune::orderly_exit()` is called first, because Rune otherwise ends with `_exit` and Hex has teardown to do. `app.try_run()` is used, not `run`, so Hex can tear down after a startup error.
- **Title bar**: `on_window` callback gets the raw `HWND` via `with_raw_handle`; applies icon (`WM_SETICON`) and `DwmSetWindowAttribute` caption colour / immersive dark mode; a `SetWindowSubclass` proc (`keep_caption`) re-applies the caption after `WM_SETTINGCHANGE`, which winit would otherwise overwrite.
- **Logging**: `app_paths::init_process_logging`: `tracing_subscriber::fmt` with `EnvFilter` default `hex=info`, writer = stderr plus a private append file `%LOCALAPPDATA%\hex\logs\process.log`; `log_panics()` panic hook writes thread name plus forced backtrace to the log (a GUI app has no console); `log_trim` keeps the file around 5 MB. `color_eyre` is installed. Machine-local data and logs are in LocalAppData, settings in `%APPDATA%\hex`.
- **GPU fallback** (`src/windows/gpu_fallback.rs`): Rune picks the lowest-power adapter. Before the first GPU device it writes a `gpu-draw.pending` marker (via `rune::on_gpu`), and clears it at the first drawn frame or clean quit. A marker from the same build means the last launch crashed while drawing, so `WGPU_POWER_PREF=high` is set for this and later launches until the version changes. This is a cheap crash-loop guard for flaky iGPU drivers.
- Also: Windows uses the DX12 backend for the window (`windows/vulkan.rs` says "The window draws with DX12"); a lost GPU after standby is recovered (`03e34b75`).
- **Threads and UI messaging**: the main thread runs Rune's event loop and window (`hex-ui`). A named `hex-runtime` thread owns the listener, transcription and updates, and keeps running with the window hidden. The window talks to it through `RuntimeHandle`: an `mpsc::Sender<Request>` for UI to runtime; the runtime publishes an `Arc<Published>` snapshot into `Mutex<Option<Arc<Published>>>`; the UI registers wakers (`subscribe(Arc<dyn Fn()>)`) that Rune uses to re-run the view; and the runtime reaches the UI through a cloneable `rune::AppHandle<Ui>` (`open()`, `quit()`, `is_visible()`). Control calls use a bounded `mpsc::sync_channel(8)`. There is no polling of UI state: it is snapshot plus wake. A `RUNTIME: OnceLock<RuntimeHandle>` is `None` in headless snapshots.
- **Resize / render performance on Windows: nothing special for resize.** There is no `WM_SIZE`/`WM_ENTERSIZEMOVE` handling, no present-mode setting, no `DwmFlush`, no live-resize hack in Hex. `rg -i 'resiz|present_mode|DwmFlush|PerMonitor|SetProcessDpi'` only hits the size-restore code above. Whatever resize behaviour Hex has is whatever Rune c853811 provides (Rune's patched `winit`, `wgpu-hal`). The render-performance work that exists in Hex is about overlays and the GPU, all upstreamed by bumping Rune:
  - `31feb6f0` (#1828) "pace the dictation HUD at about 60 Hz": on Windows an overlay with no swap-chain wait presented at CPU speed, each in-flight frame held staging buffers and D3D12 allocators, memory climbed and hiding waited on a backed-up present queue. Fix: the HUD reports "not display-paced" and asks for frames via an overlay poll at about 60 Hz, stepping its model by wall time.
  - `687ea32d` (#1832) "Bump Rune to 1899e64: overlays pace frames with the swap chain on Windows".
  - `2747e34e` bump to the current rev: "a settle lands before the frame's input".
  - `341d4409` (#1622) release window and swapchain on close; `eec77f6a` (#1487) hidden start makes no D3D device.
  - `6cfefd3c` (#1373) "open with no console window or white flash".
- Takeaway for our resize lag: Hex has no extra code to copy. Check, in this order: (1) our `[patch.crates-io]` carries Rune's winit and wgpu-hal at the same rev as `rune`; (2) our Rune rev is at least `1899e64` (swap-chain pacing) and ideally `c853811c`; (3) `Cargo.lock` has gpu-allocator and wgpu-hal on one `windows` crate version; (4) whether our overlays or other windows are polling without swap-chain waits, as Hex's HUD was.

## 5. Shipping velocity

Numbers (from `git log`, 2026-11-06 to 11-20, 14 active days):
- 434 commits in 15 days, about 31 per day on average (27 per day over the whole project); peak 81 on 2026-11-20, 46 on 11-12, 45 on 11-16.
- Top authors in that span: Dax (about 235, two identities), Kit Langton 117, Ryan Vogel 81, Aiden Cline 6, Luke Parker 5.
- 388 of 424 non-merge commits end with a `(#NNNN)` PR number, so about 91% land through PRs (squash-merged: only 10 merge commits). PR numbers reached #2127.
- Direct-to-dev is rare now, but was the norm in July/August. A ruleset makes `dev` PR-only with one required check (`hex-agent-policy`), which passes instantly for human PRs. The rule is "merge as soon as it compiles locally, don't wait on CI; the CI failure pipeline fixes breakage on `dev` afterwards."
- Outside contributors are not accepted: `CONTRIBUTING.md` says "We don't take pull requests. Open an issue ... our agents usually turn it around fast." An automated issue pipeline opens `agent/*` branches, gated by the policy check and the `agent-strict-review` label.

Commit and PR message style
- Subject: plain sentence, lower-case start or a short area prefix, user-facing, with the PR number. Examples: `windows: closing the main window releases its window and swapchain (#1622)`, `fix(windows): pace the dictation HUD at about 60 Hz (#1828)`, `iOS: swipe typing for English and Spanish (Experiment, off by default) (#2106)`, `bump(rune): c853811c, a settle lands before the frame's input (#1994) (#2002)`. Area prefixes in use: `windows:`, `linux:`, `macOS:`, `iOS:`, `labs:`, `meetings:`, `site:`, `perf:`, `fix(scope):`. Early July used `feat:`/`fix:`.
- Subjects describe the user-visible result, because they become the dev release notes.
- Bodies for fixes are long and evidence-based: symptom, cause, "What changed", "Verified (device, OS, steps, before/after)", "Not verified". Example `#2071` states build number, simulator, before/after pid and states, and test counts. Bug-fix commit bodies name the issue number and say what was not verified.

How they keep quality while shipping fast
- Local gate `scripts/check` (fmt, code-shape, nix pins, clippy `-D warnings`, nextest). About 4,184 `#[test]` functions.
- Headless snapshot/golden tests: `hex app -- --snap out.png --scene <name> [--theme light|dark] [--size WxH]` draws a fixture scene from `SCENES` in `src/rune_ui/scenes.rs` without the runtime or data folder, on every OS (lavapipe software Vulkan when there is no GPU). `cargo test rune_ui` compares light and dark goldens in `tests/golden/` (PNG pairs). Goldens are Linux-only; from a Mac you dispatch `check-linux.yml` and download its `golden-actual` artifact. Update with `HEX_UPDATE_GOLDENS=1`.
- `scripts/check-onboarding.sh` renders every onboarding scene against goldens and walks a clean first run; it gates every stable release.
- Motion gate test (`motion_gate::the_first_run_moves_cleanly`) records 30 headless windows through `rune-motion`.
- Performance: `strobe.json` and `perf/*.bench.ts` (launch, idle, listening, interaction, dictation) drive the app through its accessibility tree (`docs/perf.md`).
- Accessibility tree tests (`src/rune_ui/accessibility_tests.rs`), and the app is driven through assistive APIs in benchmarks.
- Release gates: the `check` channel builds and verifies without publishing; stable publishes only after the four suites pass; artifacts are checksummed and re-verified between jobs; the Windows exe is dumpbin-checked.
- Windows cross-compile check on a Mac without a PC: `cargo xwin clippy --target x86_64-pc-windows-msvc` (documented in `docs/windows.md`). Honest limits are written into the roadmap: "Windows has only been cross-compiled", and a person walks a checklist before stable.
- Feature flags: everything new is a Lab, off by default, gated at the start of the work.

## 6. Repo hygiene

- `README.md` (111 lines): How it works, What it does, Speech models, Downloads, Command line, Contributing. User-facing only, trimmed on 07-19 to "installation and use". The architecture lives in AGENTS.md.
- `docs/`: `README.md` index ("Start with the document for your task. Plans and archived prototypes do not override shipped behavior."), per-OS guides (`windows.md`, `linux.md`, `nix.md`), `releasing.md`, `perf.md`, `design-system.md`, `cli.md`, `features/` (one map per capability, indexed), `plans/` (design decisions with dates and owners, e.g. `rune-dependency.md`, `rune-ui.md`, `release-service.md`), `specs/`, `research/` (benchmarks and measurements).
- Other root files: `ROADMAP.md` (the authoritative work list), `SECURITY.md` (GitHub private vulnerability reporting), `CONTRIBUTING.md` (issues only), `THIRD_PARTY_NOTICES.md`, `LICENSE` (MIT), `flake.nix`.
- `.github/ISSUE_TEMPLATE/`: `bug_report.yml` (version, platform dropdown, system details, behavior, with a warning not to post private audio or full logs), `feature_request.yml`, and `config.yml` (blank issues allowed; links to install help, Linux requirements and SECURITY.md).
- No PR template. No `dependabot.yml`/`renovate`.
- Labels: GitHub defaults (bug, documentation, duplicate, enhancement, good first issue, help wanted, invalid, question, wontfix) plus `agent-strict-review`, `needs-design`, `needs-repro`.
- `CODEOWNERS` only for `.github/`.
- Docs hold decisions with owner and date ("Decision (Dax, 2026-10-22): ..."). Freeze notices are written into AGENTS.md "Current Work".

## What greyscreen should copy

1. Check our Rune wiring against Hex's: pin `rune`, `rune-kit` etc. by full `rev`, and copy the `[patch.crates-io]` block for `winit`, `sctk-adwaita`, `parley_engine`, `wgpu-hal`, `accesskit_windows`, `accesskit_macos` at the same rev. Bump them together. Highest value for the resize lag, because stock winit/wgpu-hal means none of Rune's Windows fixes apply.
2. Move our Rune pin to at least `c853811c8e0e60a632eaa2e72992f871225737ef` (Hex's current) and, if we are older than `1899e64`, take swap-chain pacing for overlays. Do one small PR per bump, titled with what it fixes.
3. Add Hex's `build.rs` guard (`check_windows_split`) so `gpu-allocator` and `wgpu-hal` cannot land on different `windows` crate versions after `cargo update -p rune`.
4. Windows app skeleton from `src/windows/rune.rs`: `windows_subsystem = "windows"`, `rune::orderly_exit()`, `try_run()`, `keep_running().release_gpu_when_closed()`, runtime thread plus `RuntimeHandle` (mpsc requests in, `Arc<Published>` snapshot plus wakers out, `AppHandle` for open/quit), tray built on its own thread. Replace nothing in the resize path: Hex has no custom resize code, so a lag in ours is likely in our code or pin, not a missing Hex trick.
5. Windows robustness bits: named-event single instance (`windows::app::AppInstance`), `gpu_fallback` crash-marker that switches to `WGPU_POWER_PREF=high`, `restrict_dll_search` first in `main`, panic hook that writes a backtrace to `process.log`, static CRT in `.cargo/config.toml`.
6. One local gate, `scripts/check`, with distinct exit codes per stage (fmt, clippy `-D warnings`, nextest), plus `.config/nextest.toml` that kills and names hung tests. Simple to copy, high payoff for agents.
7. Headless snapshot tests: a `--snap <png> --scene <name> --theme --size` CLI over a `SCENES` list, with light/dark goldens in `tests/golden/` and `HEX_UPDATE_GOLDENS=1` to refresh. Rule: any UI change commits its updated goldens in the same PR.
8. `scripts/code-shape.py` with the 800 warn / 1,500 fail file limit, folder limits, and a ratcheting exceptions file. Enforces file size by script instead of prose.
9. Workspace lints plus a ratchet written as comments (`allow` with the error count left), `clippy.toml` banning `std::env::var` and `SystemTime::now` outside config, `unsafe_op_in_unsafe_fn = deny`, `dev` profile `debug = "line-tables-only"` with `debug = false` for dependencies.
10. AGENTS.md shape (not its length): workflow at the top, Purpose, Invariants as testable sentences, Development commands, Current Work with freezes. Keep the rules "update the doc in the same change" and "new features land behind a flag, off by default". Keep ours a fraction of Hex's 135 KB; split per-feature maps under `docs/features/` with an index.
11. CI: minimal PR job (path-filtered compile plus clippy on all three OSes, concurrency cancel, drafts skipped), SHA-pinned actions, `Swatinem/rust-cache` with `cache-on-failure`, and a Windows cache key that includes the MSVC toolset. A dumpbin check that the exe is a GUI program with no VC++ runtime import. Full suites only on release and by hand.
12. Release hygiene: PR titles written as user-facing release notes (so no CHANGELOG), version stamped into the build only (`set-crate-version.sh`, `0.0.0-dev-<run>`), build jobs without secrets separated from signing jobs, `CODEOWNERS` plus a ruleset on `.github/`, and long evidence-style PR bodies ("What changed / Verified / Not verified"). Skip what does not fit a small team: the agent-policy gate, Nix, and the multi-store release pipeline.
