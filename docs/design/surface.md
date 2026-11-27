# Surface inventory

Everything a redesign must cover: the jobs, the states, the actions, the settings and the data. It is taken from the code as of 0.1.20-preview.28. Read `product.md` first.

## Screens and overlays

- **Home:** what Greyscreen is doing now, plus the last game and its next action.
- **Live:** the event feed of the game being recorded, which fills in as it happens (up to 500 items).
- **Games:** every game, grouped by day, searchable by champion, mode or result. A game's menu has Open, Review deaths, Show in folder and Delete.
- **Game review:** the player with transport, a seek bar with storyboard hover preview, event markers and an events and clips panel. It has a per-game event filter, theater (T) and fullscreen (F), and the Games list (G) and Events panel (I) toggle on their own.
- **Death review:** loops each of my deaths from 12 s before to 3 s after, with optional half speed for the last 4 s. It shows the killer, 7 tags (keys 1 to 7) and a note. It ends with a summary of deaths by tag and a death timeline.
- **Clips:** every clip from every game, with search. A clip plays on click and has Export.
- **Settings sheet** (Mod+,), with search. Sections: General, Recording, Clips & Review, Storage, Keybinds.
- **Dialogs:** Export, Send feedback, Delete game. The Recording health popover.
- **Notices:** one toast slot, plus an update banner ("Greyscreen X is ready", Restart to update, Later).
- **Setup window:** a separate 460×420 installer window with the phases Ready, Closing, Installing, Done and Failed.
- **Tray:** the icon changes while recording, the tooltip reads "Greyscreen — Recording 12:34", and the menu has Open, Pause or Resume recording, Save clip and Quit.

## Recording states

Each state shows a short line on every screen and a fuller one on Home.

| State | Trigger | User needs |
|---|---|---|
| Ready | Record games on, no game | "Waiting for your next game." F9 tip. |
| Off | Record games off | "Recording off." The switch to turn it back on. |
| Game running, not recording | Game in progress, Record games off | "Turn on Record games to record this game." |
| League is loading | Game loading | Calm progress. Test recording is blocked. |
| Recording | Game started | Champion, mode, a running clock, the live feed, Save clip (F9) and Stop (asks to confirm after 60 s). |
| Test recording | Manual 10 s test | A countdown, then "It works" with Watch, or a failure. |
| Saving | Game ended | "Saving…". Test and Retry are blocked. |
| Problem | Start or capture failed | A title, one action, an optional hint and a help link. Retry and Dismiss. |

The problems and their single action:

- League is in Fullscreen: switch to Borderless.
- League window not found.
- Can't save to the recordings folder: Choose folder.
- Not enough disk space: Choose folder.
- No video encoder found: update the driver and restart.
- No graphics card found.
- Graphics card cannot convert video.
- Windows too old.
- Windows blocked screen capture: run as admin if League does.
- Capture failed: the raw text and Open logs folder.

Other system states:

- **Encoder:** hardware, software fallback ("uses more CPU", capped at 1080p30), or none works (a danger card on Home).
- **Disk:**
  - A start is refused when the disk is low.
  - Mid-game, the full video stops at 500 MB free while clips keep working.
  - The storage limit cleans up old videos and shows a notice.
- **Mid-game failures:** "Recording stopped: the video encoder or graphics driver failed. The game is saved up to that point." Clip saved, failed or skipped notices.
- **Recovery:** "Recovered N recordings" or "Couldn't recover…" with Reveal.
- **Update:**
  - Checking, Up to date, Available, Downloading, Ready, Installing or Failed.
  - Installs wait until the game ends.
- **First run:** "Set up Greyscreen" with 3 steps (Recordings folder, Quality, Test recording) and Done.
- **Empty states:** "No games yet", "No games match", "Clips you save show up here", "No clips match".

## Actions and keys

Only Save clip works in game. Every other key works only while the window is focused.

- **Record:** Save clip F9 (global). Record games switch, Stop, Retry, Test recording.
- **Navigation:** Home Mod+1, Games Mod+2, Clips Mod+3, Settings Mod+,, Search games Mod+F, Previous or next game Up/Down, Focus player Enter, Back Esc.
- **Playback:** Play or pause Space. Back or forward 5 s ←/J, →/L. Previous or next event Shift+←/→. Frame step , and . Slower or faster [ and ]. Mute M.
- **Death review:** Review deaths D. Next N, Previous P, Replay R. Tags 1 to 7.
- **Layout:** Events panel I, Games list G, Theater T, Fullscreen F.
- **Library:** Open, Review deaths, Show in folder, Delete (to the Recycle Bin). Export clip, Export this moment, Copy file.

## Settings

- **General:**
  - Start with Windows (Windows only).
  - Sound effects and volume, covering 4 sounds: recording started, saved, clip saved, failed.
  - Check for updates.
  - Open logs folder.
  - Version and the Riot notice.
- **Recording:**
  - Record games.
  - Record full game: off keeps only clips.
  - Quality: Balanced, High or Max, with an estimate in MB per minute.
  - Resolution: Native or 2160 down to 720, default 1080.
  - FPS: 30 or 60.
  - Game audio.
  - Show cursor.
  - Detailed diagnostics.
  - Encoder (read only).
- **Clips & Review:**
  - Clips on or off.
  - Seconds before and after: 15 and 5, from 1 to 120.
  - Moments: kills, deaths, multikills, objectives and aces are on. Assists and structures are off.
  - Events shown on the timeline: 14 categories, with Reset.
- **Storage:** Recordings folder and Storage limit (Off, 50, 100, 250 or 500 GB).
- **Keybinds:** every action can be rebound, conflicts are refused, and there is Reset all.

## Data

Design with these fields only.

**Per game** (`game.json`):

- Always present:
  - Start time and duration.
  - Mode (Classic, ARAM and so on) and map.
  - Me: Riot ID, champion and team (blue or red).
  - All 10 players (name, champion, team, bot).
  - Result: Victory, Defeat or Unknown.
  - Video size: width, height and fps.
  - Encoder label and folder size.
- Optional:
  - Death review: tags and a note per death.
  - Recording health: fps, dropped %, audio, bitrate and warnings.
  - Recovered after a crash.
- Derived:
  - My KDA, from kill events.
  - Deaths reviewed "N of M".
  - Clip count.
  - Objective counts, mine and enemy.
- A manual or test recording has no champion, mode or players, and needs a "not a game" look.

**Events:** game time and video time, plus the kind:

- Kill: killer, victim and assisters. The killer may be a minion, turret or monster.
- Multikill (2 to 5) and First blood.
- Ace and First brick.
- Turret and inhibitor, with names such as "Blue top inner turret".
- Dragon (type and stolen), Herald, Baron and Void grubs.
- Game end.

Each event also gets labels relative to me: MyKill, MyDeath, MyAssist, Multikill2 to 5, Ally or Enemy objective, Victory and Defeat.

**Clips:**

- File, start and end in the game, and a label such as "MyKill+Multikill2" (or "Manual").
- Highlight clips are 15 s before and 5 s after, merged when they overlap, with a cap of 90 s. Manual clips are the last 30 s.
- Each clip has a thumbnail.

**Across the library** (cheap):

- Games, wins and losses, win rate, time played and disk used.
- This week's stats, streaks and the results sequence.
- Per-champion games and win rate. Per-mode counts.
- Clip counts.
- Death tag frequency over time.
- Unhealthy or recovered recordings.

**Live, while recording:** state, clock, champion and mode, an event feed with labels, fps, drops, encoder and disk free.

**Not available. Do not design with these:**

- Rank, LP or queue (ranked or normal).
- Role or lane, and the lane opponent.
- CS, gold, damage, vision, items, runes, spells or level.
- Patch.
- Other players' stats beyond kills and deaths.
- Positions or map.
- Timeline bookmarks: clips are the only user marks.
- Game favourites or notes: notes exist only on deaths.
