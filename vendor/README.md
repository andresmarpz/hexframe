# Vendored crates

## rune-media

`rune-media` from Rune at rev `eb9f11047bbe2b3927ad4b65ffcf7bfef4d3deb8`, the same rev as the rest of Rune in the root `Cargo.toml`. MIT, see `rune-media/LICENSE`.

Why: Rune's Windows video path has bugs that Greyscreen's review player hits on every clip: a pause on the compositor layer shows a stale picture (anomalyco/rune#1320), a seek past the last picture never completes (anomalyco/rune#1321), and the video's own sound goes silent at 60 fps. Fixing them in Rune is simpler and more correct than working around them in the app. `rune-media/GREYSCREEN_PATCHES.md` lists each change.

How it is wired:

- The root `Cargo.toml` replaces Rune's `rune-media` with this copy through `[patch."https://github.com/anomalyco/rune"]`.
- `vendor` is excluded from the workspace, so Greyscreen's lints, `cargo fmt` and clippy skip it.
- Its `Cargo.toml` is Rune's with the `workspace = true` fields made concrete; the other Rune crates come from git at the same rev.

Test it on its own from the repo root: `cargo test --manifest-path vendor/rune-media/Cargo.toml --target-dir target/vendor --lib`. Its `Cargo.toml` patches `wgpu-hal` for that run only, as Rune's workspace does. On a Mac, six tests fail at this rev because they read fixtures from Rune's other crates; the Windows player's tests are in `src/video/windows` and run on Windows only.

How to drop it: when Rune has the fixes upstream, bump the Rune rev in the root `Cargo.toml`, remove the `[patch."https://github.com/anomalyco/rune"]` section and the `exclude`, delete `vendor/rune-media` and run `cargo update -p rune-media`. When you bump Rune without upstream fixes, copy the new `crates/rune-media` here and replay the patches in `GREYSCREEN_PATCHES.md`.
