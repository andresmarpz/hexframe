# Adopting Hex practices

`docs/research/hex-practices.md` records how anomalyco/hex is set up. This page decides which of those practices greyscreen takes, and turns each one into a ticket a worker can run without design calls.

Our situation shapes every decision:

- One developer, many AI workers pushing straight to `main`. The gate must be one command with a clear failure code, because workers cannot ask which check failed.
- Development on macOS, cross-compiled to Windows. Windows behaviour is only testable on the owner's PC, so Windows-only changes must be small and logged.
- Rune is pinned to `eb9f1104` on purpose (live resize lag after `1dc3b333`). Nothing here may move the pin.
- CI minutes on a private repo are paid, and the local gate (`just ci`) already runs the Windows clippy.

Already in place, so not listed below: `[patch.crates-io]` for Rune's forks at the same rev, static CRT, `windows_subsystem`, `release_gpu_when_closed`, a named-event single instance, a SEH crash filter, a panic hook that logs, workspace lints with `unsafe_op_in_unsafe_fn = deny`, dev profile `debug = "line-tables-only"`, `Cargo.lock` has one `windows` version (0.62.2), `ci-release` profile, typos.

Findings from reading our code that matter for the tickets:

- Rune ends a quitting app with `_exit` unless `rune::orderly_exit()` is called. `main` keeps `_engine` and `_tray` alive after `app.run()`, so their `Drop` (engine shutdown, which finalizes a running recording) never runs on a normal quit. The recorder recovers `.part` files, but a clean quit should not rely on recovery.
- Rune already ships `--snap out.png --scene NAME --theme dark|light --size WxH --list-scenes` through `App::run()`, and our `AppSpec.scenes` already feeds it. We need only a test that calls it and compares PNGs. Rune has no golden comparer to reuse.
- The 500-line rule is prose only. The largest non-test file is `crates/mp4/src/recover.rs` at 521 lines, including its inline tests, so the true count has to come from a script.

## Decisions

| Practice | Decision | Why | Effort | Files touched |
|---|---|---|---|---|
| One `scripts/check` gate, per-stage exit codes | Adapt | We have `just ci`, but it hides which stage failed and `just` is a second entry point. Keep `just ci` as a thin call to the script. Stages: fmt 10, clippy 11, windows clippy 12, tests 13, code-shape 14, typos 15 | S | `scripts/check`, `justfile` (after the other justfile worker), `AGENTS.md` |
| nextest config that kills and names hung tests | Adapt | Agents hang on the engine and fake-league tests. Per-test timeout, no groups needed. Script falls back to `cargo test` | S | `.config/nextest.toml`, `scripts/check` |
| Code-shape script with ratcheting exceptions | Adapt | Our 500-line rule is unenforced. A 60-line Python script, files only (no folder or prefix rules), exceptions file that only goes down | S | `scripts/code-shape.py`, `scripts/file-size-exceptions.txt`, `scripts/check` |
| `build.rs` guard against split `windows` crate versions | Adopt | A Rune bump with `cargo update -p rune` broke Hex twice. Today we have one version, so the guard is free insurance. Put it in `crates/app/build.rs` so every host build runs it | S | `crates/app/build.rs` |
| Headless golden snapshot tests | Adapt | Best protection against UI regressions made by workers. Use Rune's own `--snap`, macOS only, dark theme plus light for 3 scenes, tolerance compare. No Linux or CI job: private-repo minutes and Windows renders differ | M | `crates/app/tests/golden.rs`, `crates/app/tests/golden/*.png`, `crates/app/Cargo.toml` (dev-dependency `png` is already a dependency), `AGENTS.md` |
| `AGENTS.md` structure | Adapt | Add invariants as testable sentences, a Current Work section, "update the doc in the same change". Keep it under about 120 lines. Move the Rune notes into Invariants | S | `AGENTS.md` |
| Rune bump discipline | Adopt | Our pin is fragile and a bump needs a Windows resize test. Write the recipe into `AGENTS.md`. No code | S | `AGENTS.md` |
| `rune::orderly_exit`, `try_run` | Adopt | Fixes the unfinalized-recording-on-quit gap above. `try_run` lets `main` log a startup error instead of a silent panic. Windows resize must be re-checked by the owner, because exit path touches window teardown | S | `crates/app/src/main.rs` |
| `restrict_dll_search` first in `main` | Adopt | One-call hardening (`SetDefaultDllDirectories`), no behaviour risk, matters because the app loads Media Foundation and D3D DLLs | S | `crates/capture/src/dll.rs`, `crates/capture/src/lib.rs`, `crates/app/src/main.rs`, `crates/cli/src/main.rs` |
| GPU-fallback crash marker | Adapt | Our users have laptops with an iGPU and a dGPU, and Rune picks the low-power one. Hex's marker plus `WGPU_POWER_PREF=high` is a cheap crash-loop guard. Keep it to about 60 lines, no version-change logic beyond the build version | M | `crates/capture/src/gpu_fallback.rs`, `crates/app/src/main.rs` |
| Panic hook writing a backtrace | Adapt | Our hook logs the message only. Add a forced backtrace to the same log. A GUI app has no console | S | `crates/app/src/logging.rs` |
| CI: path filters, concurrency, drafts | Adapt | We already cancel stale runs. Add `paths-ignore` for `docs/**` and `**/*.md`, since workers push docs often. Skip drafts and SHA pinning (no PR flow, trusted first-party actions only) | S | `.github/workflows/ci.yml` |
| CI: Windows GUI / no VC runtime check | Adopt | Catches a build that silently needs the VC++ redistributable or opens a console window. Runs in the existing `release` job on the built exe | S | `.github/workflows/ci.yml` |
| Release hygiene: version stamping | Adapt | Show the git commit and `-dirty` in `--version` and the log header for local and CI builds, so a tester's log names the build. No `0.0.0-dev-N` scheme: we tag releases by hand | S | `crates/app/build.rs`, `crates/cli/build.rs`, `crates/app/src/main.rs`, `crates/cli/src/main.rs` |
| Release hygiene: release notes from PR titles | Skip | We have no PRs, commits go straight to `main`. Commit subjects follow `<crate>: <what changed>` already and a release can `git log` them | - | - |
| Issue templates | Adapt | One `bug_report.yml` asks for version, GPU, Windows build, capture mode, and warns not to post private video. Skip feature request template and config | S | `.github/ISSUE_TEMPLATE/bug_report.yml` |
| `clippy.toml` bans on `env::var` and `SystemTime::now` | Skip | We have no config module to funnel through, and `mono_now` already owns time. Revisit if a worker introduces ad hoc time | - | - |
| Clippy ratchet-as-comments in `Cargo.toml` | Skip | Our lints are already at their final level and clippy is clean | - | - |
| Agent-policy gate (`.github/agent-policy`) | Skip | It protects a many-person repo from hostile PRs. We have no PRs and one owner | - | - |
| Reusable per-OS check workflows, Nix, release service, signing | Skip | Too large for one developer, and we do not ship a signed installer | - | - |
| Motion gate and accessibility-tree tests | Skip | Our UI is small. Existing `Runtime` tests already cover behaviour. Revisit after goldens exist | - | - |
| Labs / "new features off by default" | Skip | We are pre-1.0 with one user. A setting can be added when a feature needs a switch | - | - |
| `CODEOWNERS` and rulesets on `.github/` | Skip | One owner and direct pushes to `main` are the workflow | - | - |

## Order by value

1. Gate script plus nextest plus code-shape plus the `build.rs` guard (ticket 1). Every later ticket runs through it.
2. `AGENTS.md` restructure and Rune bump recipe (ticket 1, so workers see the new rules at once).
3. Windows skeleton: `orderly_exit`, `try_run`, DLL search, panic backtrace, GPU fallback (ticket 3). This fixes a real quit-time gap.
4. Golden snapshots (ticket 2).
5. CI additions, version stamp, issue template (ticket 4).

## Ticket order and conflicts

The justfile, `crates/app/src/main.rs` and the settings screen are being edited by other workers right now. Start tickets 1 and 3 only after those land, and rebase first. Ticket 2 adds a new file and touches no screen code, but its goldens must be regenerated after the settings-screen work lands. Ticket 4 is independent.

---

## Ticket 1: Gate, code shape and agent doc (about 250 lines)

Start after the justfile worker has pushed.

1. Create `scripts/check` (bash, `set -uo pipefail`, `cd` to repo root). Print `==> <stage>` before each stage. Stages and exit codes: `cargo fmt --all --check` 10; `cargo clippy --workspace --all-targets --locked -- -D warnings` 11; the Windows clippy command copied from the `check-win` recipe 12 (skip on non-Darwin hosts that lack `cargo-xwin`, print a notice); tests 13; `python3 scripts/code-shape.py --check` 14; `typos` 15. With one argument it runs only fmt plus the tests matching that filter.
2. Tests run with `cargo nextest run --locked --workspace --no-tests=warn "$@"` when `cargo-nextest` exists, else `cargo test --workspace --locked -- "$@"` with a notice.
3. Create `.config/nextest.toml`: `[profile.default] slow-timeout = { period = "20s", terminate-after = 3 }`, `fail-fast = false`, `failure-output = "final"`. Add `[profile.ci]` with `slow-timeout = { period = "60s", terminate-after = 3 }`. The engine tests that run a fake match get the CI period by an override on `package(greyscreen-engine)`.
4. Create `scripts/code-shape.py`: lists files with `git ls-files '*.rs'`. For each, count lines that are non-blank and outside a `#[cfg(test)]` item or file (path matches `tests.rs`, `*/tests/*`, `tests/`). Fail when above 500 unless the path is in `scripts/file-size-exceptions.txt` (`<path> <ceiling>`, `#` comments). A listed file fails when it exceeds its ceiling. A listed file now under 500 fails with "remove from exceptions". `--update-exceptions` rewrites ceilings down only. Seed the exceptions file from the script's own output on the current tree.
5. Replace the `just ci` body with `scripts/check`, keeping `just lint`, `just test` and `just check-win` as they are.
6. `crates/app/build.rs`: add `check_windows_split()` copied in shape from Hex `build.rs` (reads `Cargo.lock`, finds the `windows` dependency line inside the `gpu-allocator` and `wgpu-hal` package blocks, asserts they are equal, `cargo:rerun-if-changed` on the lock path from `CARGO_MANIFEST_DIR/../../Cargo.lock`). Missing lock file or missing package: return without failing. Run it on every target OS, before the Windows resource step.
7. Rewrite `AGENTS.md` in this order, under 120 lines: Worker protocol (keep, with step 5 now `scripts/check`), Purpose (two lines), Invariants, Commands, Rune pin, Current Work. Invariants are one sentence each that a test or script can check, for example "No `unwrap` outside tests: clippy `unwrap_used`", "A file over 500 non-test lines is listed in `scripts/file-size-exceptions.txt`", "Any change to what a screen draws updates `crates/app/tests/golden/` in the same commit", "Docs change in the same commit as the behaviour they describe". Rune pin section: the current text, plus the bump recipe (change `rev` on `rune`, `rune-kit` and all six `[patch.crates-io]` entries together; run `cargo update -p rune -p rune-kit`; confirm one `windows` version in `Cargo.lock`; one commit titled `app: bump Rune to <sha>, <what it fixes>`; the owner resizes a release build on Windows before the commit lands). Current Work: the Windows first-run status from `docs/roadmap.md`, and a note that Rune is frozen at `eb9f1104`.
8. Acceptance: `scripts/check` exits 0 on the tree. Edit a file to 501 lines: exit 14. Insert a `clippy` error: exit 11. Run `cargo check -p greyscreen` after changing the lock's `gpu-allocator` `windows` line in a temp copy: the build fails with the guard message. `just ci` still works.

## Ticket 2: Golden snapshot tests (about 150 lines of Rust, plus PNGs)

Start after the settings-screen worker has pushed, so the first goldens match the final settings screen.

1. Create `crates/app/tests/golden.rs`, an integration test that runs the real binary through `env!("CARGO_BIN_EXE_greyscreen")` with `--snap <tmp>/<name>-<theme>.png --scene <name> --theme <theme> --size 960x640 --scale 1`. Gate the whole file with `#![cfg(target_os = "macos")]`: the renderer differs per OS and our goldens come from the owner's Mac.
2. Scenes and themes: every name in `SCENES` (`idle`, `waiting`, `recording`, `finalizing`, `error`, `recordings`, `recordings-empty`, `settings`, `review`) in dark; `recording`, `settings`, `recordings` also in light. Goldens live in `crates/app/tests/golden/<name>-<theme>.png`.
3. Compare with the `png` crate (add as a `dev-dependency` of `greyscreen` only if not already available to tests). Fail when dimensions differ, or when more than 0.1 percent of pixels differ by more than 4 in any channel. On failure write `<name>-<theme>.actual.png` next to the temp output and print its path in the panic message.
4. `GREYSCREEN_UPDATE_GOLDENS=1 cargo test -p greyscreen --test golden` overwrites the golden files and passes. A missing golden fails with the update command in the message.
5. The `review` scene shows a video frame. Before committing, run the update twice and compare the bytes. If the two runs differ, drop that scene from the list and say so in the report.
6. Add `crates/app/tests/golden/*.actual.png` to `.gitignore`.
7. Add the invariant to `AGENTS.md` (from ticket 1) and the update command to Commands. Do not add a CI job.
8. Acceptance: tests pass on the Mac twice in a row with no update. Change a colour in `crates/app/src/view` temporarily: the test fails, names the scene, and writes the `.actual.png`. `scripts/check` passes on a non-macOS host because the file compiles to nothing. Final report lists the PNGs added and their total size (keep it under 2 MB).

## Ticket 3: Windows app skeleton hardening (about 200 lines)

Start after the `main.rs` worker has pushed. All of it compiles on the Mac through `just check-win`, but only the owner's PC can run it. Put each item in its own commit so a bad one is easy to revert, and list in the report that nothing ran on Windows.

1. `restrict_dll_search`: create `crates/capture/src/dll.rs` with `pub fn restrict_search()` calling `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_APPLICATION_DIR)`, with a `// SAFETY:` line, ignoring the error with a `tracing::warn`. Export it from the capture crate (Windows only). Call it as the first statement of `main` in `crates/app/src/main.rs` and `crates/cli/src/main.rs`, before the single-instance claim.
2. Panic backtrace: in `crates/app/src/logging.rs` `install_panic_hook`, log `std::backtrace::Backtrace::force_capture()` with the message and the thread name, in one `tracing::error!` call. Add a unit test that sets the hook, catches a panic with `std::panic::catch_unwind`, and checks nothing aborts.
3. `try_run` and `orderly_exit`: in `main.rs`, call `rune::orderly_exit()` before building the app, replace `app.run()` with `app.try_run()`, log an `Err` with `tracing::error!` and exit with code 1. Keep `_engine` and `_tray` in scope so they drop in order after the run returns. The owner must resize the release build on Windows and quit with a recording running, then confirm `game.mp4` is a finished file and not `game.mp4.part`. If resize lags, revert only this item and record it in `docs/roadmap.md` Status.
4. GPU fallback: create `crates/capture/src/gpu_fallback.rs` (Windows only). `choose(dir: &Path, build: &str)` reads `gpu-draw.pending` in the logs folder. If it holds the same `build` string, set `WGPU_POWER_PREF=high` and log a warning. Otherwise delete the marker. Then write the marker with the build string. `clear(dir)` deletes it. Call `choose` in `main` after `logging::init`, and `clear` from the first poll after the window drew (use the first `Ui::poll` call that sees a frame, or the clean quit path). Unit-test the marker logic with a temp dir.
5. Build string: `env!("CARGO_PKG_VERSION")` plus the commit from ticket 4 if present, else the version alone.
6. Acceptance: `just ci` passes. The unit tests for the marker and the panic hook pass on the Mac (marker logic uses no Windows API, so keep it in a `cfg`-free module and only the `choose` call site under `cfg(windows)`). Report which items the owner still has to run on the PC: items 1, 3, 4.

## Ticket 4: CI shape, version stamp, issue template (about 120 lines)

Independent of the other tickets.

1. `.github/workflows/ci.yml`: add `paths-ignore: ['docs/**', '**/*.md', '.github/ISSUE_TEMPLATE/**']` to both `push` and `pull_request`. Keep the tag trigger without a filter (a `paths-ignore` on a tag push is ignored by GitHub, but keep tags in their own `push` entry for clarity).
2. In the `release` job after the build, add a step (PowerShell) that finds `dumpbin` through `vswhere`, runs `dumpbin /dependents` on `greyscreen.exe` and `greyscreen-cli.exe`, and fails when the output contains `VCRUNTIME`, `MSVCP` or `api-ms-win-crt`. Then `dumpbin /headers greyscreen.exe` must contain `subsystem (Windows GUI)`, and `greyscreen-cli.exe` must contain `subsystem (Windows CUI)`.
3. `crates/app/build.rs` and `crates/cli/build.rs`: add `print_build_version()`. It runs `git rev-parse --short=8 HEAD` and `git status --porcelain` (ignore failure) and prints `cargo:rustc-env=GREYSCREEN_BUILD=<version>+<sha>[-dirty]`, with `cargo:rerun-if-changed=../../.git/HEAD`. Fall back to the crate version when git is missing. Use `env!("GREYSCREEN_BUILD")` in the clap `version` of the CLI and in the first `tracing::info!` line in `main.rs` (`greyscreen <build> starting`).
4. Create `.github/ISSUE_TEMPLATE/bug_report.yml` with fields: what happened (required), what you expected, greyscreen build (placeholder: "from the first line of the log"), Windows version, GPU and driver, game display mode (dropdown: Borderless, Windowed, Fullscreen), steps. Add the warning "Do not post recordings with voice chat or full logs; trim them first". Add `config.yml` with `blank_issues_enabled: true` only if the file would otherwise be absent; skip it otherwise.
5. Acceptance: a docs-only commit pushed to a branch does not start CI (check the Actions tab). A forced `workflow_dispatch` run passes the dumpbin step. `greyscreen-cli --version` prints the commit and `-dirty` after an edit. `just ci` passes.
