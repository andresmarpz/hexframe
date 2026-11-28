# Roadmap

Status on 2026-11-21. `docs/architecture.md` describes how the app works today. The tickets that built it are in the git history. Open work lives in [GitHub issues](https://github.com/andresmarpz/greyscreen/issues).

## Done

- Capture and encode on Windows: WGC, video processor, hardware H.264 with a software fallback and a start-up encoder check, process loopback audio, AAC, pacer, capture session.
- Pure-Rust MP4: fragmented writer, finalize, clip writer, crash recovery.
- Live Client poller, event classification, session state machine with start retries.
- Engine: game writer, clip ring and planner, clip jobs, library scan, thumbnails, storage limit, delete to the Recycle Bin, global hotkeys, updater, feedback.
- App: Home, Games and Clips screens, Sidebar and Dock navigation, settings sheet, keybinds, death review, update banner, feedback dialog, setup window and self-install on Windows.
- CLI: `doctor`, `record`, `fake-league`, `recover`, `thumb`, `storyboard`, `analyze`, `demo`.
- Recording health: a `health` summary in every `game.json`, the Recording health popover in the review toolbar, the optional `diagnostics.jsonl` and `greyscreen-cli analyze`.
- Backend `apps/api`: feedback and experimental crash upload on Cloudflare.
- Website, brand assets and signed releases built by GitHub Actions on a tag (`just release`, `docs/release-signing.md`). Each release has its changelog in `apps/web/src/content/changelog/`.
- Adversarial review of capture, engine, mp4 and league (`docs/reviews/2026-11-21-adversarial.md`). Every item it found is fixed.
