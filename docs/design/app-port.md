# Porting the Paper design into the app

Andres asked on 2026-11-28 for the Paper design ("Greyscreen App" file, pages Home, Last game, Components and Icons) to be built in the Rune app. This doc is the build plan. `round6.md` holds the design decisions, and the earlier `round*.md` files hold the history.

## Source of truth

`docs/design/paper/` holds exports of the official artboards:

- `*.png`: the picture at 1x. Match it.
- `*.jsx`: Paper's markup with exact sizes, paddings, radii, fonts and colour tokens. Read the values from here, never from the PNG.
- `icons/*.svg`: Riot's objective silhouettes (potrace output; keep the `<g transform>` when you inline them) and the kill swords. The skull (death) and `folder-reveal` paths are in `icons-game.jsx` and `last-game.jsx`.

| File | What |
|---|---|
| `home.*` | Home screen. The storage popover is drawn open only to show it; in the app it opens on click. |
| `last-game.*` | The game page, opened for any game. Breadcrumb "Games › Vladimir · Victory · 2 hours ago". |
| `top-bar.*` | The top bar over the plain page and over Home's ambient frame. |
| `recording-dock.*`, `record-button.*`, `storage-popover.*` | The bottom recording dock and its parts. |
| `last-game-card.*` | Home's last game card. Variant A is the one to build; B and C are not built. |
| `clip-card.*`, `game-actions.*`, `timeline-box.*` | Parts of the game page. |
| `icons-game.*` | The icon sheet. |

## Mapping Paper to Rune

The JSX uses CSS names for kit tokens. In Rune:

| Paper | Rune |
|---|---|
| `--color-page`, `--color-surface` | `k.c.page`, `k.c.bg_base` |
| `--color-layer-2/3/4` | `k.c.layer_02/03/04` |
| `--color-text`, `-muted`, `-faint` | `k.c.text_base`, `k.c.text_muted`, `k.c.text_faint` |
| `--color-border`, `--color-border-muted` | `k.c.border_base`, `k.c.border_muted` |
| `--color-success-bg/fg/border` (same for danger, info) | `k.c.success.bg/.fg/.border` |
| `--color-accent` | `k.c.bg_accent` (fills), `k.c.text_accent` (text) |
| `--color-white-12`, `-20` | `k.light(0.12)`, `k.light(0.20)` |
| `color-mix(X N%, transparent)` | `k.alpha(X, N / 100)` |
| `--color-shadow-soft`, box-shadow stacks | `.box_shadow(Shadow { .. })`, once per layer |
| `linear-gradient` | `.linear_gradient(angle, &[(pos, colour)])` (PI = top to bottom) |
| `filter: blur(72px)` | `.blur(sigma)` (capped at 64), or pre-blur on the CPU |
| image `object-fit: cover` | `.pixels_fit(&pixels, SurfaceFit::Fill)` |
| `position: absolute` | `.pin(..)` or `.absolute(x, y)` |
| kit buttons, segmented control, key cap, popover, progress ring | `Button`, `IconButton`, `SegmentedControl`, `Keybind`, `Popover`, `ProgressCircle`, stock look (AGENTS.md) |

Kit components keep their stock look. If Paper shows something the kit can't do (for example the "4" count inside the Clips segment), leave it out and say so in the report.

## Product decisions for the port (head of design, 2026-11-28)

- **One shell.** The top bar and the bottom recording dock replace the sidebar and the old nav dock. The `nav_style` and `sidebar_collapsed` settings go away. Old settings files must still load.
- **Games and Clips screens** keep their current content inside the new shell. They get redesigned later.
- **The game page** ("Last game" in Paper) is what opens when you open any game, from Home's card or from a Games row:
  - The play button opens the existing review player at 0:00.
  - "Review" (D) opens the existing death review.
  - The folder button reveals the game's folder.
  - "All clips" opens the Clips screen.
  - The breadcrumb's "Games" goes back to the Games list.
- **Recording state of Home** (during a game) is not designed yet. While recording, Home keeps today's live feed under the new shell, and the dock shows the red state with the live clock.
- **Timeline mode** (Compact | Detailed) is a later slice. Build Compact only.
- Settings is still the existing sheet, opened from the top bar's gear.

## Slices

Every slice is one worker and one commit to main. Waves run in order; slices in a wave run side by side and own different files.

### Wave 1

**S1 · Shell and top bar** (`top-bar.*`)
- Windows: `App::titlebar(Titlebar::new(44.0))`, so Rune draws the caption buttons over the trailing edge. macOS keeps the traffic lights; the brand starts after `cx.titlebar_inset()` (or `cx.chrome_insets().leading`).
- Left: the logo mark and "Greyscreen". Centre: the **top dock**, a 36 px tray (`bg_base`, radius 10, padding 4, the dock shadow) holding the stock `SegmentedControl` Home | Games | Clips. Right, before the caption buttons: a Neutral 28 px `IconButton` with the kit `SETTINGS_GEAR` path (copy it from Rune's `themes/opencode/src/icons.rs`) that opens Settings.
- The bar has no fill and no line, and content draws under it. Everything that isn't a control is a `.window_drag()` region.
- Delete the sidebar (`view/shell/sidebar*`), the old nav dock and `nav_style` / `sidebar_collapsed`, plus their scenes and settings rows. Leave a mount point at the bottom of the window for S4's recording dock.

**S2 · Game data** (core, no UI)
- `core::timeline`: one box per game minute (0 to the last minute) built from `EventEntry` and `GameInfo`, with this priority: my death > my kill > objective > empty.
  - Kill boxes carry my kill count in that minute and the highest multikill streak (2 Double, 3 Triple, 4 Quadra, 5 Penta).
  - Objective boxes carry the kind (dragon, elder, baron, herald, grubs, tower, inhibitor) and whose team took it (mine or the enemy's). An Elder is a `DragonKill` whose `dragon_type` is "Elder".
- Kill participation: (my kills + my assists) / my team's kills, `None` when the team has no kills.
- Week strip for Home: the results of this week's games, oldest first, plus the most played champion.
- Storage breakdown for the popover: bytes in game videos, clips, and thumbnails and timelines, plus free space on the drive (the engine already reads it). Extend the engine API only if the numbers are not already there.

**S3 · Icons and hero frame**
- `view/widgets/game_icons.rs`: `rune::Icon` statics for KILL, DEATH, DRAGON, ELDER, BARON, HERALD, GRUBS, TOWER, INHIBITOR, FOLDER_REVEAL and SETTINGS_GEAR, from the files above. Non-square view boxes use `Icon::svg`. If the Duotone folder can't be drawn in one colour, use the Line style.
- **Hero frame:** thumbnails are 400×225, too small for the 1232 px hero. Next to `thumb.jpg` the engine also writes `hero.jpg`, 1600 px wide, from the same moment. Games without one fall back to `thumb.jpg`.

### Wave 2 (after wave 1 lands)

**S4 · Recording dock** (`recording-dock.*`, `record-button.*`, `storage-popover.*`)
- A floating pill at the bottom centre, 16 px above the edge: 48 tall, radius 24, padding 0 10, items 28 tall, gap 12, dividers with 4 px of extra air. It keeps today's rule of hiding while a game plays.
- **Record button** (28 px, a raised disc with a state dot and a 3 px glow) toggles `auto_record`. The text beside it changes with the state:

  | State | Dot | Text |
  |---|---|---|
  | Ready (armed) | success fg | "Ready", over "Next game will be recorded" |
  | Recording | danger fg | the live clock |
  | Off | text faint, hollow | "Recording paused" |

- Then a divider, "Save clip" with the F9 `Keybind` (disabled outside a game, sends `SaveClipNow`), and another divider.
- Then the storage Ghost button (a 16 px `ProgressCircle` and "112 GB"), which opens the storage `Popover` drawn in `storage-popover.*`:
  - "Storage settings…" opens Settings at Storage.
  - "Open recordings folder" reveals the folder.

**S5 · Home** (`home.*`, `last-game-card.*`)
- **Ambient background:** the last game's frame, blurred, at 42% opacity, 504 px tall from the window top, fading into the page.
- Content starts 92 px from the top (44 bar + 48), centred, 880 wide as in the JSX.
- **Last game card, variant A:**
  - Thumbnail with play.
  - "Victory" with "Last game · 2 hours ago" on the right.
  - The champion line.
  - The stats row: KDA, kill participation, clips, and deaths reviewed `n / m`.
  - A chevron, and the compact box timeline.
  - The whole card opens the game page.
- **This week:** win rate, the week strip, and "7 wins · 5 losses · Most played: Vladimir".
- **Recent clips:** as in the JSX.
- The encoder-error and first-run cards stay and sit above the card.

**S6 · Game page** (`last-game.*`, `game-actions.*`, `timeline-box.*`, `clip-card.*`)
- The breadcrumb, then the hero (396 tall). The hero has:
  - `hero.jpg` with cover and a full-height scrim;
  - "Last game · 2 hours ago", the big result word and the champion line;
  - the KDA and the centred play button;
  - Review + D and the folder button at the bottom right;
  - the compact box timeline with minute labels.
- **Timeline boxes:** 20 px tall with a 2 px top border.
  - Kill: success colours with the swords and the multikill badge. Penta uses the inverted box.
  - Death: danger colours with the skull.
  - Objective: grey with the silhouette, `text_base` for mine and `text_muted` for the enemy's.
  - Hover shows the kit tooltip with the event names ("Double Kill", "Dragon", …).
- Highlights: the first 4 clips of the game as overlay cards, with the "All clips 12 →" Neutral button.
- A "D" App action (`actions!` line) opens the death review while the game page is shown.
- The timeline component lives in its own file so S5 can reuse it on Home's card.

### Wave 3

- **Sweep** on the Windows PC: `just check` and the kept tests, then fix and push.
- **Design check:** I build the Mac app, render every scene, compare it with the PNGs and send a polish slice.

## Rules for every slice

- Read AGENTS.md, this doc, `round6.md` and your slice's files in `docs/design/paper/`.
- Keep the kit look. Add no motion beyond hover and press (no page or position transitions).
- Files stay under 500 lines. Add no tests for layout.
- Update or add scenes in `ui/scenes/` for the screens you change, and extend `ui/preview.rs` data when a screen needs new fields. Split `preview.rs` if it would pass 500 lines.
- Check on the Mac with `cargo check -p greyscreen`, then render your scene (`cargo run -p greyscreen --release -- --snap /tmp/x.png --scene <name>`) and compare it with the PNG. Do not run `just check`; the sweep does that.
