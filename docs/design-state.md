# Greyscreen — Design State & UI Audit

> Baseline: commit `3c0cd3c` (`v0.1.19-preview.21`). Audited via 34 headless Rune renders (`1180×680` default and `1440×900` wide, dark theme) and full inspection of `crates/app/src/view/`, `crates/app/src/ui/`, `crates/app/src/setup/view.rs`, and `rune/src/el/motion.rs`.

## Locked-In Design Decisions (Starting Point)

- **1a — Review Screen & Games List Layout:** Keep the split-pane mental model, add independent toggles to hide/show the right Inspector panel and the Games list inside a normal window, make the right Inspector flush (remove the outer floating card padding), hide the global sidebar's `Recent` games section while on `Games`, decouple **Windowed Theater (`T`)** from **OS Fullscreen (`F`)**, and raise the default window size (`1280×760`) so first launch does not land below `FOLD_BELOW`.
- **2a — Tighter Settings IA:** Reorganize Settings from 6 uneven tabs into 5 balanced tabs (`General`, `Recording`, `Clips & Review`, `Storage`, `Keybinds`) with sub-group cards, move Search into the left settings sidebar, remove the `"Changes are saved automatically."` subtitle, remove the duplicate `Send feedback` row, and simplify Keybinds.
- **3a — Review Toolbar Declutter & Transport Export:** Move `Export this moment` from the top-right toolbar down into the player transport bar; fold `Recording health`, `Open folder`, and `Delete…` into a single `•••` overflow menu (with a warning/danger indicator dot only when health is degraded); tone down `Review deaths (N)` from bright white `Submit` to `Neutral`; cap the top-right toolbar to at most 3 items (`Review deaths`, `•••`, `Inspector toggle`).

---

## Part 1 — What Is Working Well (Preserve These Strengths)

Before cutting or reshaping anything, preserve what already feels genuinely strong:

1. **Calm, neutral dark palette (`Kit::neutral`).** High legibility, restrained accent usage, zero gamer-RGB clutter, and clean tabular figures (`'tnum' 1`) on clocks, KDA, durations, and storage stats.
2. **Death Review is a standout product workflow.** The dedicated 15-second clip loop, amber slow-mo zone on the progress bar, killer portrait, progress pips, instant tag chips + freeform note, and end-of-game summary (`Death review complete` with tag breakdown + clickable death timeline) feel purposeful and genuinely differentiated.
3. **Seek bar storyboard preview & scrubbing physics.** Zero-decode sprite-sheet hover preview (`view/review/seek_preview.rs`), throttled keyframe scrub + exact seek on rest (`ui/review/scrub.rs`), and smart mutual exclusion with event hover cards (`150 ms` hover intent) feel YouTube/Linear-grade.
4. **Plain-language error recovery (`core::problem::Problem`).** Every capture, encoder, folder, and disk failure gives a clear title, one concrete action, and a contextual fix button (`Choose folder…`, `Retry`, `Learn more`).
5. **Direct-manipulation resize grips & keyboard-first ergonomics.** Resizable panes with double-click reset, instant page switching (`screens::settled`), `Up`/`Down` game switching, and Escape hierarchy are strong desktop instincts.
6. **Windows Setup window (`crates/app/src/setup/view.rs`).** Clean `460×420` layout that explicitly reserves `ACTION_HEIGHT = 112.0` so phase transitions never shift vertical layout. This discipline should become the app-wide standard.
7. **Delete confirmation & Row context menu.** Clear, concise, names Recycle Bin/Trash, shows file size, and uses a proper danger CTA.

---

## Part 2 — Holistic Cluster Reviews (Root Structural Problems)

Many visible defects are not isolated micro-bugs—they are symptoms of seven structural clusters that must be addressed holistically.

### Cluster 1 — Review Screen Spatial Architecture & Four-Column Overload

**Root problem:** The Review screen tries to keep four persistent vertical panes alive simultaneously (`Sidebar` + `Games list` + `Video` + `Events/Clips Inspector`) inside a window whose default width is `1180×680`.

- At `1180×680`, chrome consumes `684 px` (`56` rail + `300` games list + `32` padding + `16` gap + `280` panel), starving the 16:9 gameplay video down to `~496×279 px` inside a `~540 px` tall black container—more dead letterbox than gameplay.
- At `1440×900`, all four columns render at once (`220` sidebar + `300` games list + `~590` video + `280` panel), AND the global sidebar's `Recent` section renders the **exact same 5 games** directly adjacent to the Games list. Even in the `56 px` collapsed rail, the 5 champion squares (`A A Y L J`) sit 12 px next to the same 5 rows.
- **Perverse breakpoint inversion (`STACK_BELOW = 800.0` in `view/review.rs:24`):** Making the Games list *wider* drops the review area below `800 px`, stacking the inspector below the player—which ironically makes the video *much bigger* and letterbox-free.
- **Default launch hits the collapsed breakpoint:** Default window size is `1180×680` (`main.rs:51`), which is `20 px` narrower than `FOLD_BELOW = 1200.0` (`view/shell/sidebar.rs:20`), so new users always launch into the collapsed rail despite `sidebar_collapsed = false`.
- **No non-fullscreen focus mode:** `review.theater` (`ui/keys.rs:117-132`) is hard-coupled to `tx.toggle_full_screen()`. Users cannot hide the right panel or Games list inside a regular desktop window (essential for dual-monitor or split-screen workflows).
- **Wasted double-framed padding:** `review-panel` is rendered as a bordered `card(k)` floating inside `PAGE_PADDING = 16.0` on all sides, wasting `32 px` horizontally and visually boxing the panel instead of anchoring it as a flush inspector separated by a 1 px hairline.
- **Clip-playing banner lives in the wrong pane:** `clip-notice` (`Playing clip: ... [Full game]`) sits inside `review-panel` (`view/review/panel.rs:120-156`). Once the inspector can be hidden, the user loses the button to return to the full game unless it moves to the player/toolbar area.

**Holistic target:**
- Give the video canvas first-class spatial priority: independent collapsible right Inspector (`◫`) and Games list (`◧` / toggle), flush inspector divider, hide sidebar `Recent` on `Games`, decouple **Windowed Theater (`T`)** from **OS Fullscreen (`F`)**, and default opening window to `1280×760`.

---

### Cluster 2 — Review Toolbar & Action Topology

**Root problem:** The top bar of Review mixes six different metadata styles on the left and five unrelated actions on the right (`Review deaths`, `Export`, `Health`, `Open folder`, `Delete`) with no hierarchy, grouping, or safety separation.

- **Five right-side controls with mismatched jobs:** Primary workflow (`Review deaths`), playhead creation (`Export`), rare telemetry (`Recording health`), OS file reveal (`Open folder`), and destructive deletion (`Trash`) sit 8 px apart at equal icon weight.
- **Misclick hazard:** `Trash` sits immediately adjacent to `Open folder` with no separator and no danger hover cue.
- **Spatial disconnect:** `Export this moment` cuts around the *current playhead time*, yet sits in the top-right global game header instead of near the timeline/transport controls.
- **Visual distraction:** `Review deaths (N)` uses bright white `ButtonVariant::Submit`, creating a high-luminance rectangle in the corner that pulls the eye away from dark gameplay footage.
- **Left metadata overload:** `[Icon] Ahri [Summoner's Rift] [Victory] Today 13:00 1/1/0 · 00:10` stacks 6 visual treatments in one row and repeats metadata already highlighted in the selected Games list row right below it.
- **Health popover noise:** `view/review/health.rs` shows five `76 px` wide bright green `OK` pills even when everything is healthy, and occupies a permanent toolbar icon slot for a diagnostic checked <1% of the time.
- **Death Review toolbar leak:** Entering Death Review keeps `Export`, `Health`, `Folder`, and `Trash` active in the top bar while adding a *second* header bar below it (`← Finish (Esc)  J  Death 3 of 7...`).

**Holistic target:**
- Max 3 controls on the top right: `Review deaths (N)` (quiet `Neutral` variant), `•••` overflow menu (`Recording health`, `Show in folder`, separator, danger `Delete…`, with an amber/red badge dot only when health is degraded), and the far-right `Inspector` toggle (`◫`).
- Move `Export this moment` into the transport bar next to playback controls.
- Simplify left metadata line; quiet down healthy rows in the Health popover; suppress unrelated toolbar actions during Death Review.

---

### Cluster 3 — Settings Information Architecture & Controls

**Root problem:** Settings grew organically by appending rows to `Recording` and spawning tiny single-purpose tabs (`Review`, `Storage`), creating an unbalanced, hard-to-scan modal.

- **`Recording` is a 12-row junk drawer (`view/settings/catalog.rs:103-116`):** Mixes automatic capture, video encoding, audio/cursor capture, **Navigation style (Sidebar/Dock)**, **Sound effects**, **Sound volume**, and **Detailed recording diagnostics** in one unbroken card with zero subheadings.
- **Extreme tab imbalance:** `Recording` has 12 rows (overflows scroll at default tier), `Review` has 2 rows (14 checkboxes + an entire 52 px row just to hold a `Reset to default` button), `Storage` has 2 rows, `Updates & About` mixes startup (`Start with Windows`), updates, troubleshooting (`Logs`), duplicate `Send feedback`, and Riot legal text.
- **Wasted modal header vertical space (`view/settings.rs:98-136`):** Permanent `"Changes are saved automatically."` subtitle + full-width top search bar burn ~100 px of height above the split body.
- **Keybinds clutter (`view/settings/keybinds.rs`):** 26 identical `"Change"` buttons plus an invisible `28 px` `RESET_SLOT` gutter on every single unchanged row create noisy alignment and unnecessary chrome.
- **Empty storage bar (`view/settings/storage.rs:64`):** When `storage_limit_gb` is `None`, renders an empty 0% `Progress` bar that looks broken/meaningless.

**Holistic target:**
- 5 balanced sections:
  1. **General** — *App* (`Start with Windows`, `Navigation style`), *Sound* (`Sound effects`, `Sound volume`), *Updates* (`Updates`, `Check for updates automatically`), *Diagnostics* (`Detailed recording diagnostics`, `Open logs folder`), quiet footer (`Version` + Riot legal notice). Remove duplicate `Send feedback` row.
  2. **Recording** — *Capture* (`Record games`, `Record the full game`), *Video* (`Quality`, `Resolution`, `Frame rate`, `Encoder`), *Audio & Input* (`Record game audio`, `Show cursor`).
  3. **Clips & Review** — *Highlight clips* (`Save highlight clips`, `Moments`, `Clip window`, `Save clip hotkey`), *Timeline defaults* (`Events shown` with compact inline `Reset to default` button in the section header).
  4. **Storage** — `Recordings folder`, `Storage limit` (hide progress bar when limit is Off).
  5. **Keybinds** — Direct click-to-record on keycap pills (remove 26 `"Change"` buttons and 28 px dead-space gutter; keep fixed-min-width capture pill so rows never shift horizontally or vertically).
- Move `Search settings…` to the top of the left settings nav column and drop the `"Changes are saved automatically."` subtitle.

---

### Cluster 4 — Motion, Layout Stability & Loading State System

**Root problem:** Rune defaults to springing layout changes and cross-fading mounts/unmounts (`200 ms` enter / `150 ms` exit, `rune/src/el/motion.rs:176-215`) on every user commit unless explicitly opted out. Greyscreen only opted out inside `screens::screen(...)`, leaving the shell, dock, theater, modals, popovers, and sliders overly animated and jittery.

- **Dock auto-hide resizes the video player (`view/screens.rs:72-81`):** `dock-reserve` toggles between `52.0` and `0.0` whenever the mouse nears the bottom edge during Review in Dock mode—making the entire video player and timeline physically jump up and down!
- **Un-instantized shell & overlays:** Sidebar collapse/expand, Settings search filtering, Export modal phase transitions, Feedback error insertion, Filter badge pops, and Keybind capture all run unintended layout springs and 150 ms ghost fades.
- **Laggy volume slider (`view/widgets/volume_slider.rs:38-51`):** Unlike `timeline.rs` and `export/range.rs`, the volume slider omits `.instant_changes()`, so the thumb and fill lag behind pointer drags on a spring.
- **Dual out-of-sync pulsing dots (`recording-game.png`):** Sidebar status dot pulses at `1.6 s` (`0.4..1.0`) while the live row in the Games list pulses at `1.4 s` (`0.25..1.0`) right beside it.
- **Unconstrained buttons resizing on state change:**
  - `export-copy` (`view/overlays/export.rs:221-264`): `"Copy file"` → `"Copied"` has no locked width, shrinking the button and pulling `"Show in folder"` rightward.
  - `bind-btn-{id}` (`view/settings/keybinds.rs:115-123`): `"Change"` → `"Press keys…"` expands width while hiding keycaps and growing row height.
  - `event-filter` (`view/review/panel/filter.rs:18-51`): `"Filter"` → `"Filter · 4 hidden"` resizes on every checkbox click while `"Not your default"` pops beside it.
  - `stop-recording` (`view/home/actions.rs:136-153`): Hardcoded `.w(168.0)` makes resting `"Stop"` comically wide.
  - `first-run-test` (`view/home/first_run.rs:78-111`): Button vanishes (`None`) during test recording, collapsing step 3.
  - `export` Dialog (`view/overlays/export.rs:20-57`): Collapses from `~340 px` (`Editing`) to `~140 px` (`Working`), springing vertically on screen.
- **Three conflicting loading patterns:** Dimmed inline spinner inside button (`updates.rs`), left-aligned footer spinner (`feedback.rs`), full-area replacement (`setup/view.rs`).

**Holistic target:**
- **Motion Charter:** Flip default to `.instant_changes()` on the root shell and modal bodies; explicitly opt in with `.animated_changes()` ONLY for overlay-only transitions (floating Dock slide without layout reserve, Theater HUD scrim fade, Toasts, active spinners).
- **Stable-Width Button Pattern:** Create one shared widget helper that stacks the widest label invisibly (`opacity(0.0).a11y_hidden()`) to lock intrinsic width and swaps the active label or centered full-contrast `Spinner` on top.

---

### Cluster 5 — Home Screen Lifecycle & Density

**Root problem:** Home mixes onboarding, live status, troubleshooting, and post-game review without adapting cleanly between first-run and daily use.

- **`Update banner` collides with Home (`view/overlays/update_banner.rs:7-18`):** Pinned at `top: 52px`, it floats directly on top of Home's state card, obscuring `"Idle"` and `"Waiting for a game"`.
- **`Test recording (10 s)` stays a permanent hero button (`view/home/actions.rs:81-91`):** Valuable before game #1, visual noise after a player has 20+ recorded games. Should demote once recordings exist (or live in Settings > Recording).
- **Empty `This week` zeros during first-run (`view/home.rs:40`):** Shows `0 / 0–0 / — / 0m` underneath the onboarding card before any game has ever been played.
- **Duplicate `Record games` switch in Dock mode (`dock.png`):** Home's state card has `Record games [switch]` at top right while the floating dock right below it shows `Rec [switch]`.
- **Redundant idle copy:** `• Idle` pill + `"Waiting for a game"` + `"Greyscreen starts recording when a game starts."` repeats the same calm state three ways.
- **Last Game card click target:** Only the `Open` button opens the game instead of the full card row; `Recent clips` header lacks a `"View all →"` link to `Clips`.

---

### Cluster 6 — Clips & Library Discoverability

**Root problem:** List and gallery items have small information-density and discoverability leaks that compound as libraries grow.

- **Duplicate `00:30` duration on Clips cards (`view/clips.rs:51,87`):** Duration prints inside the thumbnail bottom-right badge AND again directly underneath in the footer left corner.
- **Fixed `256 px` Clips card width (`view/clips.rs:11`):** Produces ragged right margins across window widths instead of a clean responsive column grid.
- **No search or filter on Clips screen (`view/clips.rs`):** Unusable once a user accumulates dozens or hundreds of auto-clips.
- **Games list row truncation (`view/games/rows.rs:120-127`):** At default `300 px` width, `"Summoner's Rift · 13:00 · 2 of 3 reviewed"` always truncates before the review progress (`Summoner's Rift · 13:00...`).
- **Dead `play_overlay` code (`view/widgets/thumb.rs:119-131`):** `game_thumb` and `clip_thumb` embed a hover-revealed play icon using `.group_opacity(0.0, 1.0)`, but `games/rows.rs`, `clips.rs`, and `home/last_game.rs` never call `.hover_group()`, so it only works on Home's Recent Clips!
- **Invisible clip actions (`view/review/panel/clips.rs:26,36`):** Export and Folder buttons sit at `opacity: 0` until pointer hover, hiding functionality at rest and from keyboard navigation.
- **Empty `Games` pick state (`view/games.rs:96-113`):** Navigating to `Games` with recordings available should auto-select the newest game instead of showing a blank `"Pick a game"` screen.

---

### Cluster 7 — Design System, `rune-kit` Discipline & Accessibility

**Root problem:** Local one-off visual helpers drifted away from `rune-kit` stock primitives, and several focus/a11y details slipped through.

- **Four competing pill/badge implementations:** `rune_kit::Badge`, `review::tone_pill` (`22px`, Title Case), `home::pill` (`24px`, dot), and `home/feed::tinted_pill` (`20px`, `11px` ALL-CAPS)—causing `VICTORY` (Home), `Victory` pill (Review toolbar), and green text `Victory` (Games list) to all look different.
- **Custom button restyling violating `AGENTS.md`:** `home/actions.rs:103-118` overrides `.fill(k.alpha(ink, 0.16))` on `ButtonVariant::Neutral`.
- **Inconsistent primary CTA variants:** `Submit` (`Review deaths`, `Death summary Done`, `Setup Install`) vs `Contrast` (`Export`, `Feedback Send`, `Install and restart`) vs custom tinted `Neutral` (`Save clip`, `Retry`).
- **Live accessibility warning:** `review-video` (`view/review.rs:151-157`) is `focusable(true)` without `.label(...)`—Rune logs a debug a11y warning on every Review render.
- **Keyboard focus traps/gaps:** `games/rows.rs:31` sets `.focusable(false)`; `panel/clips.rs` hover-only icons aren't discoverable via keyboard; `settings/updates.rs` dims the active loading spinner to disabled opacity.
- **Inconsistent shortcut display:** `"Exit (Esc)"` (`theater.rs:75`) and `"Finish (Esc)"` (`deaths.rs:74`) bake `(Esc)` into button text, while every other button puts shortcuts in `.shortcut(...)`.

---

## Part 3 — Complete Issue Checklist

### Cluster 1 — Review Layout & Spatial Architecture
- [x] **C1-1 (Critical):** Right Inspector (`review-panel`) cannot be hidden in normal review mode. Add collapsible toggle (`◫`) + keyboard action + persisted setting. (`view/review.rs`, `view/review/toolbar.rs`, `core/settings.rs`, `core/keybinds/action.rs`) — *done in `82ff5d0`*
- [x] **C1-2 (High):** Games list (`games-list`) cannot be hidden while reviewing a game. Add collapsible toggle + keyboard action + persisted setting. (`view/screens.rs`, `view/games.rs`, `core/settings.rs`) — *done in `82ff5d0`*
- [x] **C1-3 (High):** `review.theater` is coupled to OS fullscreen (`tx.toggle_full_screen()`). Decouple **Windowed Theater (`T`)** from **OS Fullscreen (`F`)**. (`ui/keys.rs`, `core/keybinds/action.rs`, `view/review/transport.rs`, `view/review/theater.rs`) — *done in `00d7c24`*
- [x] **C1-4 (High):** Default window size (`1180×680`) is narrower than `FOLD_BELOW` (`1200.0`), causing first launch to auto-collapse the sidebar. Raise default window to `1280×760` (or lower `FOLD_BELOW`). (`main.rs`, `view/shell/sidebar.rs`) — *done in `82ff5d0`*
- [x] **C1-5 (High):** Global sidebar (`nav-recent`, both expanded and collapsed rail) shows the exact same 5 recent games right next to `games-list` on `Screen::Games`. Hide `Recent` when `screen == Screen::Games`. (`view/shell/sidebar.rs`, `view/shell/sidebar/collapsed.rs`) — *done in `82ff5d0`*
- [x] **C1-6 (Medium):** `review-panel` has floating double-card padding (`16 px` around a bordered card). Make inspector flush to the right edge with a single 1 px vertical divider to reclaim horizontal width. (`view/review.rs`, `view/review/panel.rs`) — *done in `82ff5d0`*
- [x] **C1-7 (Medium):** `clip-notice` (`Playing clip: ... [Full game]`) lives inside `review-panel`, so hiding the inspector hides the way back to the full game. Move above the video player (or into toolbar). (`view/review/panel.rs`, `view/review.rs`) — *done in `82ff5d0`*
- [ ] **C1-8 (Medium):** `STACK_BELOW = 800.0` creates a perverse inversion where widening the games list makes the video larger. Re-evaluate threshold once inspector is collapsible and flush. (`view/review.rs`)
- [ ] **C1-9 (Low):** Theater drawer only shows Events, not Clips; allow tab switching or keep it clearly scoped. (`view/review/theater.rs`)
- [x] **C1-10 (Low):** Games list stays open during Death Review, squeezing the death clip. Auto-hide Games list while `review.death.is_some()`. (`view/screens.rs`) — *done in `82ff5d0`*

### Cluster 2 — Review Toolbar & Action Topology
- [x] **C2-1 (Critical):** Top-right toolbar has 5 unrelated items (`Review deaths`, `Export`, `Health`, `Open folder`, `Delete`). Reduce to max 3: `Review deaths`, `•••` overflow menu, and `Inspector` toggle. (`view/review/toolbar.rs`) — *done in `84faaec`, `82ff5d0`*
- [x] **C2-2 (High):** Move `Export this moment` from top-right toolbar into the player transport bar (right next to Speed/Volume/Theater). (`view/review/toolbar.rs`, `view/review/transport.rs`) — *done in `84faaec`*
- [x] **C2-3 (High):** Fold `Recording health`, `Show in folder`, and `Delete…` (separated, danger) into a `•••` overflow menu in the Review toolbar; show subtle amber/red dot on `•••` when health has warnings/errors. (`view/review/toolbar.rs`, `view/review/health.rs`, `view/review/controls.rs`, `ui/review.rs`) — *done in `84faaec`*
- [x] **C2-4 (Medium):** `Review deaths (N)` uses glaring white `ButtonVariant::Submit` next to dark video. Change to `ButtonVariant::Neutral`. (`view/review/toolbar.rs`) — *done in `84faaec`*
- [ ] **C2-5 (Medium):** Left toolbar metadata stacks 6 competing styles and repeats selected Games row info. Tighten to `[Icon] Champion · Result · K/D/A · Mode`. (`view/review/toolbar.rs`)
- [ ] **C2-6 (Low):** `Recording health` popover uses five loud `76 px` green `OK` pills; quiet down `OK` rows and reserve colored pills for `WARNING`/`BAD`. (`view/review/health.rs`)
- [ ] **C2-7 (Low):** Death Review keeps `Export`/`Health`/`Folder`/`Trash` in the top bar while adding a second header bar (`← Finish`). Clean up toolbar in Death Review mode. (`view/review/toolbar.rs`, `view/review/deaths.rs`)

### Cluster 3 — Settings IA & Controls
- [x] **C3-1 (Critical):** `Recording` tab is a 12-row catch-all containing `Navigation style`, `Sound effects`, `Sound volume`, and `Diagnostics`. Split into `General` and `Recording` with sub-group cards. (`ui/settings_sheet.rs`, `view/settings/*`) — *done in `97832b5`*
- [x] **C3-2 (High):** Merge 2-row `Review` tab into `Clips & Review` (5 balanced tabs total: `General`, `Recording`, `Clips & Review`, `Storage`, `Keybinds`), and fold `Updates & About` into `General`. (`ui/settings_sheet.rs`, `view/settings/catalog.rs`, `view/settings/*`) — *done in `97832b5`*
- [x] **C3-3 (High):** Move `Search settings…` to the top of the left settings nav column and drop the redundant `"Changes are saved automatically."` subtitle to reclaim ~72 px vertical space. (`view/settings.rs`) — *done in `97832b5`*
- [ ] **C3-4 (Medium):** Keybinds tab has 26 `"Change"` buttons and a `28 px` dead-space `RESET_SLOT` gutter on every row. Replace with direct click-to-record fixed-min-width keycap pills and inline reset only when modified. (`view/settings/keybinds.rs`)
- [x] **C3-5 (Medium):** Remove duplicate `Send feedback` row from Settings (already permanently in main sidebar/dock). (`view/settings/catalog.rs`, `view/settings/about.rs`) — *done in `97832b5`*
- [x] **C3-6 (Low):** Move `Reset to default` in `Review` events out of a full `52 px` setting row into a compact header action beside `"Events shown"`. (`view/settings/review.rs`) — *done in `97832b5`*
- [x] **C3-7 (Low):** Hide the 0% `Progress` bar on `Storage` when `storage_limit_gb` is `None`. (`view/settings/storage.rs`) — *done in `97832b5`*

### Cluster 4 — Motion, Layout Stability & Loading States
- [ ] **C4-1 (Critical):** Dock auto-hide animates `dock-reserve` (`0` ↔ `52 px`), physically bouncing/resizing the video player when the pointer nears the bottom edge. Remove `dock-reserve`; float the dock as a pure overlay. (`view/screens.rs`, `view/shell/dock.rs`)
- [ ] **C4-2 (High):** Over-animated defaults across shell, sidebar collapse, settings search, filter menu, keybinds, and modals because `.instant_changes()` is only on `screens::screen`. Flip shell/modals to `.instant_changes()` and opt in explicitly for overlay motion only. (`view.rs`, `view/settings.rs`, `view/overlays/*`)
- [ ] **C4-3 (High):** `export-copy` (`"Copy file"` → `"Copied"`) resizes horizontally on click and shifts `"Show in folder"`. Lock width with a shared stable-width button helper. (`view/overlays/export.rs`)
- [ ] **C4-4 (High):** `bind-btn-{id}` (`"Change"` → `"Press keys…"`) resizes horizontally and grows row height vertically on click. Fix via fixed-width keycap capture pill + header hint. (`view/settings/keybinds.rs`)
- [ ] **C4-5 (High):** `export` Dialog collapses ~200 px vertically when switching from `Editing` to `Working`. Keep a stable minimum height or inline footer progress. (`view/overlays/export.rs`)
- [ ] **C4-6 (Medium):** `volume_slider.rs` omits `.instant_changes()` on fill and thumb, causing drag lag. Add `.instant_changes()`. (`view/widgets/volume_slider.rs`)
- [ ] **C4-7 (Medium):** `stop-recording` hardcodes `.w(168.0)` so `"Stop"` looks awkwardly wide. Replace with compact `"Stop"` / `"Confirm?"` (~104 px) or confirmation popover. (`view/home/actions.rs`)
- [ ] **C4-8 (Medium):** `event-filter` button mutates width on every checkbox toggle (`Filter` → `Filter · 4 hidden`) and pops `"Not your default"` badge beside it. Use a stable button + right-aligned `"Reset"` action when customized. (`view/review/panel/filter.rs`)
- [ ] **C4-9 (Medium):** Three inconsistent loading patterns (`updates.rs` dimmed inline spinner, `feedback.rs` left footer spinner, `setup/view.rs` centered replacement) and vanishing `first-run-test` button. Unify on one shared `busy_button` pattern with locked width and full spinner contrast. (`view/widgets/`, `view/settings/updates.rs`, `view/overlays/feedback.rs`, `view/home/first_run.rs`)
- [ ] **C4-10 (Low):** Two out-of-sync pulsing recording dots visible simultaneously (`1.6 s` in sidebar status, `1.4 s` in live game row). Remove continuous pulsing loops (use a crisp steady red dot, or at most one subtle pulse). (`view/shell/sidebar/status.rs`, `view/widgets/dot.rs`)
- [ ] **C4-11 (Low):** Timeline stacks two separate thumbs (`12 px` + `14 px` `group_opacity`). Simplify to a single clean thumb. (`view/review/timeline.rs`)

### Cluster 5 — Home Screen & Global Overlays
- [ ] **C5-1 (High):** `update-banner` floats at `top: 52px` directly over Home's state card. Move into a non-occluding bottom-right toast or sidebar footer pill. (`view/overlays/update_banner.rs`)
- [ ] **C5-2 (Medium):** `Test recording (10 s)` remains a prominent button on Home even after games exist. Keep it in first-run and Settings > Recording (or demote to quiet ghost action once `recordings` is non-empty). (`view/home/actions.rs`)
- [ ] **C5-3 (Medium):** `This week` card shows all zeros during first-run onboarding. Hide `week::section` when `s.recordings.is_empty()`. (`view/home.rs`)
- [ ] **C5-4 (Low):** Dock `"Wait"` label (`view/shell/dock.rs:165`) feels clipped; use `"Loading"` or amber dot + tooltip. Also consider avoiding duplicate `Record games` switch on Home when Dock is active. (`view/shell/dock.rs`, `view/home.rs`)
- [ ] **C5-5 (Low):** Make sidebar `Storage` block clickable to jump to `Settings > Storage`, and keep it quiet unless a limit is set or usage is high. (`view/shell/sidebar/storage.rs`)
- [ ] **C5-6 (Low):** Make `Last game` card clickable across the whole row, and add `"View all →"` on `Recent clips` header. (`view/home/last_game.rs`, `view/home/clips.rs`)

### Cluster 6 — Clips & Library Discoverability
- [ ] **C6-1 (Medium):** Clips screen cards print duration `00:30` twice (thumbnail badge + footer text). Remove footer duration text. (`view/clips.rs`)
- [ ] **C6-2 (Medium):** Games list row metadata (`Summoner's Rift · 13:00 · 2 of 3 reviewed`) truncates at default `300 px` width, hiding death-review progress. Shorten mode label (`Rift` / `SR`) or use a compact `2/3` indicator. (`view/games/rows.rs`, `ui/library/format.rs`)
- [ ] **C6-3 (Medium):** `thumb::play_overlay` is dead code on Games rows, Clips cards, and Last Game because parents omit `.hover_group()`. Either wire `.hover_group()` intentionally or remove the dead overlay. (`view/widgets/thumb.rs`, `view/games/rows.rs`, `view/clips.rs`)
- [ ] **C6-4 (Medium):** Clip row actions (`Export`, `Show in folder`) in Review panel are `opacity: 0` until hover. Keep `Export` subtly visible at rest (`0.55` → `1.0`) or add a context menu. (`view/review/panel/clips.rs`)
- [ ] **C6-5 (Low):** Clips screen uses fixed `256 px` card width (ragged right margin) and has no search/filter. Make grid responsive and add search/filter. (`view/clips.rs`)
- [ ] **C6-6 (Low):** Navigating to `Games` when recordings exist should auto-select the newest game instead of showing `"Pick a game"`. (`ui/shell/screens.rs`, `view/games.rs`)
- [ ] **C6-7 (Low):** Pane resize grips (`grip.rs`) have zero hover visual feedback (`opacity: 0.02`). Add a subtle 1–2 px hover indicator line so discoverability matches Linear/VS Code. (`view/widgets/grip.rs`)
- [ ] **C6-8 (Low):** Add `1`–`7` keyboard shortcuts for Death Review tag chips and tooltips on Death Summary timeline dots. (`view/review/deaths.rs`, `view/review/death_summary.rs`)

### Cluster 7 — Design System, `rune-kit` Discipline & Accessibility
- [x] **C7-1 (High):** `review-video` is `focusable(true)` without `.label(...)`, triggering a live Rune a11y warning on every render. Add `.label("Game video")`. (`view/review.rs`) — *done in `84faaec`*
- [ ] **C7-2 (Medium):** Four competing pill/badge implementations (`rune_kit::Badge`, `review::tone_pill`, `home::pill`, `home/feed::tinted_pill`). Consolidate into `rune_kit::Badge` + one thin tone helper so `Victory` / `Defeat` and event badges are consistent everywhere. (`view/review.rs`, `view/home.rs`, `view/home/feed.rs`, `view/home/last_game.rs`)
- [ ] **C7-3 (Medium):** Custom `.fill(k.alpha(ink, 0.16))` override on `ButtonVariant::Neutral` in `home/actions.rs` violates `AGENTS.md` kit-discipline rule, and primary CTAs split between `Submit` and `Contrast`. Standardize on stock `rune-kit` button variants. (`view/home/actions.rs`, `view/review/toolbar.rs`, `view/overlays/*`)
- [ ] **C7-4 (Medium):** `games/rows.rs` sets `.focusable(false)`, preventing keyboard Tab navigation into the games list. (`view/games/rows.rs`)
- [x] **C7-5 (Low):** `"Exit (Esc)"` (`theater.rs:75`) and `"Finish (Esc)"` (`deaths.rs:74`) hardcode `(Esc)` inside button text instead of using `.shortcut("Esc")`. (`view/review/theater.rs`, `view/review/deaths.rs`) — *done in `00d7c24`*
- [ ] **C7-6 (Low):** Panel tab label `"Events 7 of 10"` is verbose for a 280 px panel; tighten to `"Events"` + compact count (`7/10`). (`view/review/panel.rs`)
