# Greyscreen

League of Legends gameplay recorder for Windows. Rust workspace with a Rune GPU UI, a website in `apps/web` and a Cloudflare backend in `apps/api`. Read `docs/architecture.md` first: it fixes the shared types, units and rules. `docs/roadmap.md` has what is done, GitHub issues have what is next, `docs/research/` has the API details.

Crates: `core` (pure logic), `mp4`, `league` (Live Client, updates and feedback clients), `capture` (Windows only), `engine` (threads and I/O), `thumbs` (OS thumbnails), `app` (the Rune UI and the setup window), `cli` and `qa` (the headless QA runner, `docs/qa.md`). `docs/architecture.md` has the dependency direction.

## Worker protocol

1. Work in the worktree you were given. Run `git fetch origin && git rebase origin/main` before starting.
2. Read `AGENTS.md`, `docs/architecture.md`, the ticket and the research sections it names. A change that makes a doc untrue fixes the doc in the same commit.
3. Change only the files the ticket lists. If something outside them must change, make the smallest change and say so in your final report.
4. Do not take design decisions. If the ticket is ambiguous or wrong, pick the option closest to `docs/architecture.md`, and list it under "Deviations" in your report.
5. `just check` must pass: fmt, clippy on the host and the tests. Run `just ci` (adds typos and Windows clippy through cargo-xwin) only when you touch Windows-only code (`capture`, `thumbs`, `cfg(windows)`) or before a release. Real Windows builds and tests are run by the orchestrator on the Windows PC.
6. Commit with the message `<crate>: <what changed>` (for example `mp4: add fragmented writer`). Then `git fetch origin && git rebase origin/main && just check && git push origin HEAD:main`. On a push race, repeat the rebase.
7. Final report, under 15 lines: commit sha, what was built, the tests added, deviations, and anything left unverified.

## Issues

- Open work is a GitHub issue on `andresmarpz/greyscreen`. Run `gh` with `GH_TOKEN=$(gh auth token --user andresmarpz)`.
- Labels: `bug` or `enhancement`, an area (`capture`, `engine`, `app`, `backend`, `web`), `needs-windows` (needs a run on the Windows PC), `needs-andres` (waits on Andres).
- Put `Fixes #N` in the commit body that completes an issue. File a new issue for work you find and do not do.

## Rules for code

- No comments or doc comments, except one line in plain English when a "why" is not obvious. Names carry the meaning. Never mention tickets or files in code.
- Files stay under 500 lines, excluding tests. Tests go in a sibling `tests.rs` (`#[cfg(test)] mod tests;`) or in `tests/` for integration tests.
- Use rune-kit components with their own look. Never restyle or rebuild a kit component (buttons, segmented controls, selects, switches, badges) with custom colours or a local copy. If the kit can't do something, keep the kit's look and say so in the report.
- `unsafe` only in `capture`, `thumbs` and the Windows locator, and each block carries a `// SAFETY:` line (clippy `undocumented_unsafe_blocks`).
- No `unwrap`/`expect` outside tests, except on a mutex lock.
- Libraries return `thiserror` errors. Binaries use `anyhow`.
- `just check` passes before every push.

## Keybinds

- Every key action is one line in `actions!` in `crates/core/src/keybinds/action.rs`: id, label, group, scope (Global works in game through `RegisterHotKey`, App only in the window) and default keys. `Mod` means Ctrl on Windows and Cmd on macOS.
- Handle the new action in `Ui::run_action` (`crates/app/src/ui/keys.rs`) for App scope, or in the engine's `global_action` for Global scope. The Keybinds page, conflict checks and settings storage need no change.
- Never match raw keys in app code. Users' overrides live in `settings.keybinds`, only the changed ones.

## Commands

- `just qa`: headless UI QA (`docs/qa.md`): runs `qa/cases` through the real app with no window, checks the element tree and diffs snapshots against `qa/goldens/<os>`; the report is `target/qa/report.html`. `just qa --update` rewrites the goldens.
- QA never uses OS-level input, never activates or shows a window, never plays sound out loud and never changes system settings. Never QA by driving the real app with UI Automation, SendInput or a visible window.
- `just check`: the everyday gate: fmt check, host clippy, tests (through cargo-nextest when installed: `cargo install cargo-nextest --locked`).
- `just ci`: the full gate, `just check` plus typos and Windows clippy through cargo-xwin. `just release` runs it.
- CI: pushes run only the spell check (`ci.yml`). `Tests` (fmt, clippy and nextest on macOS), `Windows` (clippy, tests and exe build) and `macOS` (app bundle, `just mac-bundle`) run once per published release and on manual runs, not per push. The README badges show the latest release run.
- `just build-win`: release build of `greyscreen.exe` and `greyscreen-cli.exe` for Windows.
- `just release <tag>`: runs `just ci`, builds and publishes a GitHub release from the Mac, then redeploys the website. The release assets are `GreyscreenSetup.exe` (the website's download), `greyscreen.exe` (the same binary, which the in-app updater downloads), `greyscreen-cli.exe`, `README.txt` and `LICENSE.txt`. Write `apps/web/src/content/changelog/<version>.md` first: its title and bullets become the release title and notes. Releases are built and published from the Mac, never through CI. The tree must be clean and `HEAD` already on `origin/main`; the release tags that commit, so pushes to `main` during the build do not stop it. A tag with a dash is a prerelease.
- `just release-sync`: rewrite every GitHub release title and notes from the changelog.
- `just dist`: build both exes and copy them with `README.txt` and `LICENSE.txt` into `dist/` as separate files, plus a `GreyscreenSetup.exe` copy of `greyscreen.exe`.
- `just mac-app`: build an ad-hoc signed `dist/mac/Greyscreen.app` (fake capture media, for UI testing only) and install it to `~/Applications/Greyscreen.app`.
- `just brand`: regenerate the icons and web images from the SVGs in `brand/`.
- `just fmt`: format the workspace.
- `just web-dev`: run the website (`apps/web`) dev server. Run `bun install` first.
- `just web-build`: build the website into `apps/web/dist`.
- `just web-check`: `astro check` and a website build.
- Pushes to `main` that change `apps/web/`, `brand/`, `package.json` or `bun.lock` deploy the website by themselves through Vercel's Git integration (https://greyscreen-lake.vercel.app); other pushes are skipped.
- `just web-deploy`: manual fallback, a local authenticated build published to Vercel production, then a URL check. `just release` runs it.
- `just api-dev`: run the backend (`apps/api`, Cloudflare Worker on Effect and Alchemy) locally in workerd with emulated R2 and D1. No Cloudflare account needed.
- `just api-test`, `just api-check`: backend tests, and typecheck plus tests. They are not part of `just ci`. See `apps/api/README.md` for deploys.
- `just web-tokens`: regenerate the website's theme tokens from Rune's neutral kit.
- `just web-screens`: render the app scenes into `apps/web/public/screens`.

The Mac cannot run Windows code: compile-check it with `just check-win`.

## Rune on Windows

- Rune is pinned to `eb9f1104`. Rune `1dc3b333` (#1222) and everything after it make live window resize lag about a second on Windows. Before you bump the pin, resize a release build on Windows and compare it against this one.
- The app uses `greyscreen_capture::instance` (a named event that brings the running window forward) instead of `rune::single_instance`. On Rune `e1d5cd48`, `single_instance` together with the app's other startup work made live resize lag. A minimal repro on `eb9f1104` did not lag, so the exact trigger is unknown. Keep the named event.
