# Greyscreen

League of Legends gameplay recorder for Windows. Rust workspace with a Rune GPU UI. Read `docs/architecture.md` first: it fixes the shared types, units and rules. `docs/roadmap.md` has the tickets, `docs/research/` has the API details.

## Worker protocol

1. Work in the worktree you were given. Run `git fetch origin && git rebase origin/main` before starting.
2. Read `AGENTS.md`, `docs/architecture.md`, the ticket and the research sections it names.
3. Change only the files the ticket lists. If something outside them must change, make the smallest change and say so in your final report.
4. Do not take design decisions. If the ticket is ambiguous or wrong, pick the option closest to `docs/architecture.md`, and list it under "Deviations" in your report.
5. `just ci` must pass: fmt, clippy on the host, clippy for Windows through cargo-xwin, and the tests.
6. Commit with the message `<crate>: <what changed>` (for example `mp4: add fragmented writer`). Then `git fetch origin && git rebase origin/main && just ci && git push origin HEAD:main`. On a push race, repeat the rebase.
7. Final report, under 15 lines: commit sha, what was built, the tests added, deviations, and anything left unverified.

## Rules for code

- No comments or doc comments, except one line in plain English when a "why" is not obvious. Names carry the meaning. Never mention tickets or files in code.
- Files stay under 500 lines, excluding tests. Tests go in a sibling `tests.rs` (`#[cfg(test)] mod tests;`) or in `tests/` for integration tests.
- Use rune-kit components with their own look. Never restyle or rebuild a kit component (buttons, segmented controls, selects, switches, badges) with custom colours or a local copy. If the kit can't do something, keep the kit's look and say so in the report.
- `unsafe` only in `capture` and the Windows locator, and each block carries a `// SAFETY:` line (clippy `undocumented_unsafe_blocks`).
- No `unwrap`/`expect` outside tests, except on a mutex lock.
- Libraries return `thiserror` errors. Binaries use `anyhow`.
- `just ci` passes before every push. It runs fmt, clippy on macOS and on Windows through cargo-xwin, and the tests.

## Commands

- `just ci`: fmt check, host clippy, typos, Windows clippy through cargo-xwin, tests.
- `just build-win`: release build of `greyscreen.exe` and `greyscreen-cli.exe` for Windows.
- `just release <tag> "<notes>"`: runs `just ci`, builds and publishes a GitHub release from the Mac. GitHub Actions has no minutes to spare, so releases never go through CI. A tag with a dash is a prerelease.
- `just dist`: build both exes and copy them with `README.txt` into `dist/` as separate files.
- `just fmt`: format the workspace.
- `just web-dev`: run the website (`apps/web`) dev server. Run `bun install` first.
- `just web-build`: build the website into `apps/web/dist`.
- `just web-check`: `astro check` and a website build.
- `just web-tokens`: regenerate the website's theme tokens from Rune's neutral kit.
- `just web-screens`: render the app scenes into `apps/web/public/screens`.

The Mac cannot run Windows code: compile-check it with `just check-win`.

## Rune on Windows

- Rune is pinned to `eb9f1104`. Rune `1dc3b333` (#1222) and everything after it make live window resize lag about a second on Windows. Before you bump the pin, resize a release build on Windows and compare it against this one.
- The app uses `greyscreen_capture::instance` (a named event that brings the running window forward) instead of `rune::single_instance`. On Rune `e1d5cd48`, `single_instance` together with the app's other startup work made live resize lag. A minimal repro on `eb9f1104` did not lag, so the exact trigger is unknown. Keep the named event.
