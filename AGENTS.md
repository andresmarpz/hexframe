# Greyscreen

League of Legends gameplay recorder for Windows. Rust workspace with a Rune GPU UI, a website in `apps/web` and a Cloudflare backend in `apps/api`. Read `docs/architecture.md` first: it fixes the shared types, units and rules. `docs/roadmap.md` has what is done, GitHub issues have what is next, `docs/research/` has the API details.

Crates: `core` (pure logic), `mp4`, `league` (Live Client, updates and feedback clients), `capture` (Windows only), `engine` (threads and I/O), `thumbs` (OS thumbnails), `app` (the Rune UI and the setup window) and `cli`. `docs/architecture.md` has the dependency direction.

## Worker protocol

1. Work in the worktree you were given. Run `git fetch origin && git rebase origin/main` before starting.
2. Read `AGENTS.md`, `docs/architecture.md`, the ticket and the research sections it names. A change that makes a doc untrue fixes the doc in the same commit.
3. Change only the files the ticket lists. If something outside them must change, make the smallest change and say so in your final report.
4. Do not take design decisions. If the ticket is ambiguous or wrong, pick the option closest to `docs/architecture.md`, and list it under "Deviations" in your report.
5. `just check` must pass: fmt and host clippy, no tests. When you change recording, playback or a bug that has a test, run that crate's tests with `just test <filter>`. Windows-only code (`capture`, `thumbs`, `cfg(windows)`) is checked on the Windows PC with `just win`.
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
- Tests only guard recording, playback and bugs we fixed. A bug fix adds one test that fails without the fix. Do not add tests for layout, copy, settings plumbing or UI wiring.

## Keybinds

- Every key action is one line in `actions!` in `crates/core/src/keybinds/action.rs`: id, label, group, scope (Global works in game through `RegisterHotKey`, App only in the window) and default keys. `Mod` means Ctrl on Windows and Cmd on macOS.
- Handle the new action in `Ui::run_action` (`crates/app/src/ui/keys.rs`) for App scope, or in the engine's `global_action` for Global scope. The Keybinds page, conflict checks and settings storage need no change.
- Never match raw keys in app code. Users' overrides live in `settings.keybinds`, only the changed ones.

## Commands

- `just check`: the everyday gate on the Mac: fmt check and host clippy. Under a minute warm.
- `just test <filter>`: the tests matching a filter, on the Mac. `cargo install cargo-nextest --locked`.
- `just win`: fmt, clippy and every test on the Windows PC (`windows.yml` on the self-hosted runner, labels `greyscreen` and `windows`). Setup: `scripts/setup-windows-runner.ps1 <token>` on the PC.
- QA never uses OS-level input, never activates or shows a window, never plays sound out loud and never changes system settings. Never QA by driving the real app with UI Automation, SendInput or a visible window.
- CI: pushes run only the spell check (`ci.yml`). The Windows PC runs `windows.yml` (manual, and before each release) and `release.yml`. Neither runs on pull requests.
- `just release <tag>`: checks the tree, the version and the changelog, then starts `release.yml` on the Windows PC and waits. The PC runs fmt, clippy and the tests, builds both exes, checks them, publishes the GitHub release and redeploys the website. The release assets are `GreyscreenSetup.exe` (the website's download), `greyscreen.exe` (the same binary, which the in-app updater downloads), `greyscreen-cli.exe`, `README.txt` and `LICENSE.txt`. Write `apps/web/src/content/changelog/<version>.md` first: its title and bullets become the release title and notes. The tree must be clean and `HEAD` already on `origin/main`; the release tags that commit. A tag with a dash is a prerelease.
- `just release-sync`: rewrite every GitHub release title and notes from the changelog.
- `just mac-app`: build an ad-hoc signed `dist/mac/Greyscreen.app` (fake capture media, for UI testing only) and install it to `~/Applications/Greyscreen.app`.
- `just brand`: regenerate the icons and web images from the SVGs in `brand/`.
- `just fmt`: format the workspace.
- `just web-dev`: run the website (`apps/web`) dev server. Run `bun install` first.
- `just web-build`: build the website into `apps/web/dist`.
- `just web-check`: `astro check` and a website build.
- Pushes to `main` that change `apps/web/`, `brand/`, `package.json` or `bun.lock` deploy the website (https://greyscreen-lake.vercel.app): the GitHub workflow `web.yml` triggers a Vercel deploy hook. Vercel's Git deployments are off, so other pushes create no Vercel deployment.
- `just web-deploy`: manual fallback, a local authenticated build published to Vercel production, then a URL check.
- `just api-dev`: run the backend (`apps/api`, Cloudflare Worker on Effect and Alchemy) locally in workerd with emulated R2 and D1. No Cloudflare account needed.
- `just api-test`, `just api-check`: backend tests, and typecheck plus tests. See `apps/api/README.md` for deploys.
- `just web-tokens`: regenerate the website's theme tokens from Rune's neutral kit.
- `just web-screens`: render the app scenes into `apps/web/public/screens`.

The Mac cannot run Windows code: check it on the PC with `just win`.

## Rune on Windows

- Rune is pinned to a `dev` commit in the root `Cargo.toml`. Before you bump the pin, resize a release build on Windows and compare it against the current one: an older Rune (`1dc3b333` to `2c2ff033`) made live resize lag about a second.
- The app uses `greyscreen_capture::instance` (a named event that brings the running window forward and waits out an old copy on relaunch) instead of `rune::single_instance`, which only reports whether the app is the first. Keep it.
