# Roadmap

Status on 2026-11-21. `docs/architecture.md` describes how the app works today. The tickets that built it are in the git history.

## Done

- Capture and encode on Windows: WGC, video processor, hardware H.264 with a software fallback and a start-up encoder check, process loopback audio, AAC, pacer, capture session.
- Pure-Rust MP4: fragmented writer, finalize, clip writer, crash recovery.
- Live Client poller, event classification, session state machine with start retries.
- Engine: game writer, clip ring and planner, clip jobs, library scan, thumbnails, storage limit, delete to the Recycle Bin, global hotkeys, updater, feedback.
- App: Home, Games and Clips screens, Sidebar and Dock navigation, settings sheet, keybinds, death review, update banner, feedback dialog, setup window and self-install on Windows.
- CLI: `doctor`, `record`, `fake-league`, `recover`, `thumb`, `analyze`, `demo`.
- Recording health: a `health` summary in every `game.json`, the Recording health popover in the review toolbar, the optional `diagnostics.jsonl` and `greyscreen-cli analyze`.
- Backend `apps/api`: feedback and experimental crash upload on Cloudflare.
- Website, brand assets and releases from the Mac (`just release`). The latest preview is `0.1.9-preview.11`; each release has its changelog in `apps/web/src/content/changelog/`.
- Adversarial review of capture, engine, mp4 and league (`docs/reviews/2026-11-21-adversarial.md`). Every item it found is fixed.

## Next

The first full run on Windows hardware: `greyscreen-cli doctor`, `greyscreen-cli record`, and a real game. Fix what the logs show.

## Later

- Send crash dumps from the app to `POST /v1/crash` (the backend side exists, the app does not call it).
- Backend auth and telemetry ingest.
- A direct NVENC or AMF encoder behind `MediaSource`, if a vendor MFT misbehaves.
- A setting to capture system audio when process loopback misses the game.
- A native-tls fallback for the Live Client connection, if rustls fails on some machines.
- Headless golden snapshot tests for the app scenes (macOS only).
- A code-shape check for the 500-line rule in `just ci`.
- `rune::orderly_exit` and `try_run` in `main`, so a normal quit finalizes a running recording instead of relying on recovery. Resize a release build on Windows before landing it.
- Restrict the DLL search path first thing in `main`.
- A backtrace in the panic hook's log line.
