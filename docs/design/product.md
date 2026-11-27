# Product brief

The ground truth for the from-scratch redesign. It describes what the app must do, not how today's UI looks. Read `surface.md` for the full list of states and actions, and `home.md` for the first screen.

## What Greyscreen is

- An automatic League of Legends recorder and death review tool for Windows. It is free, open source and needs no account.
- It sits in the tray and records every game. Recording starts when the game loads and stops a few seconds after the Nexus falls. The player never presses record.
- Riot's Live Client API (127.0.0.1:2999) turns each game into an event timeline: kills, deaths, assists, multikills, objectives and structures.
- After the game, the player jumps to any event, steps through each of their deaths (tag and note), or takes the auto-saved highlight clips to Discord.

## Who it is for

- Main user: a solo League player who wants to get better and reviews their own games, mostly their deaths.
- Secondary user: a player who saves and shares plays with friends (Discord).
- Not for: streamers who need scenes, people who want cloud upload, social feeds or AI coaching.

## Why it wins

- Local first: plain MP4 plus a JSON sidecar on the user's disk. No account, no upload, no ads, no tracking.
- Native and light: a small Rust exe with a GPU UI. There is no Overwolf, Electron or libobs. It self-installs without admin rights.
- Safe: Windows Graphics Capture and per-process audio only. There is no memory reading or injection (Vanguard-safe).
- Crash-safe recording. Clips are cut without re-encoding.
- Death review is the signature workflow. Competitors (Outplayed, Medal, Insights Capture, Ascent) lead with social, cloud or AI instead.

## Jobs, in order of how often they happen

1. **After a game:** "How did that go? Show me my deaths. Grab that play." This is the most common reason to open the window. It happens right after a game, often for less than two minutes.
2. **Trust:** "Is it recording? Will my next game be saved? Is my disk OK?" The user should see this at a glance on every screen.
3. **Find:** "Where is that game from Tuesday or that clip?" The user browses games and clips.
4. **Share:** "Send this clip to Discord." The user exports a clip (Original quality, or For Discord under 10 MB) and copies the file.
5. **Fix:** "It didn't record. Why?" The user reads a plain-language problem with one action and a help link.
6. **Set up and tune:** first run, quality, folder, storage limit, clips and keybinds. This is rare.

## Design principles

- **Post-game first.** Home leads with the last game and its next action ("Review deaths"). Everything else is secondary.
- **Status everywhere, quietly.** Recording state and storage live in one place on every screen. They never take the hero spot unless something is wrong.
- **The video is the hero.** Review screens give the player as much width as possible. Chrome is thin.
- **Colour means something.** The UI stays neutral grey. Green is Victory and my kills. Red is Defeat and my deaths. Blue is objectives and info. Amber is warnings. There is no decorative colour.
- **Real data only.** Design only with fields that exist (see `surface.md`, "Data"). There is no rank, LP, CS, gold, items, runes, role or patch. Never mock those.
- **Keyboard first.** Primary actions show their key (D Review deaths, F9 Save clip, Space Play).
- **Stock Rune kit.** Use kit components with their own look: no restyled buttons, switches, badges or segmented controls. If the kit cannot do something, keep the kit look and note the gap.
- **Calm motion.** Use no page, tab or position transitions. Keep hover and press feedback and the recording pulse.
- **Never block the game.** There is no in-game overlay. The only in-game touches are the F9 hotkey, short sounds and the tray.

## Constraints

- Windows 10 2004+ and Windows 11, 64-bit. League must run Borderless or Windowed.
- The window opens at 1280×760, with a minimum of 1000×600. Closing the window keeps the app in the tray.
- Recording at the defaults (1080p60 High) uses about 55 MB per minute, about 2 GB per game. An optional storage limit (50, 100, 250 or 500 GB) deletes the oldest videos first and keeps timelines and clips.
- Assets: champion icons come from Data Dragon (square, may be missing offline, so use a text fallback). Game and clip thumbnails are 16:9 frames that may arrive a few seconds late. A seek-bar storyboard holds 160×90 tiles. There is no splash art and no item, rune or objective art.
- The Riot legal notice must appear in About.

## Vocabulary

Use these words in the UI and in design files.

- **Screens:** Home, Games, Clips. Settings is a sheet, not a screen.
- **Game:** one recorded match. It has a **Result** of Victory, Defeat or Unknown.
- **Clip:** a short MP4 cut from a game. A **highlight clip** is automatic, and a **manual clip** comes from F9 or "Save clip" (the last 30 s).
- **Event:** a kill, death, assist, multikill, first blood, ace, objective (dragon, herald, baron, void grubs), structure (turret, inhibitor), game flow or game end.
- **Marker:** an event drawn on the seek bar.
- **Death review:** "Review deaths", "Death 3 of 7", "Death review complete". There are 7 **tags** (Out of position, Overextended, No vision, Bad trade, Ganked, Teamfight, Unavoidable) and a free note.
- **Export:** presets "Original quality" and "For Discord (under 10 MB)". "Export this moment" exports 15 s each side of the playhead.
- **Record games:** the on/off switch for automatic recording. **Test recording (10 s)**.
- **Recording health:** OK, Warning or Bad, for example "59.8 fps · 0.3% dropped · audio OK · 18 Mbps".
- **Storage limit**, **Recordings folder**.
