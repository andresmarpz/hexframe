# Home, first design

The Home screen in the state most users see: League is closed, Greyscreen is ready, and a game just ended. Paper file: Greyscreen › **Greyscreen App**, page **Home**. Read `product.md` first.

## Idea

Home is the post-game desk. The last game is the hero, with its result, its shape as a timeline and the one thing to do next (Review deaths). Its clips and the week sit below it. Recording state and storage move out of Home into a thin status bar at the bottom of every screen, so Home never spends its best space on "Waiting for a game".

New app shell (applies to every screen):

- **Titlebar (44 px):** Greyscreen mark on the left, navigation as a segmented control in the centre (Home, Games, Clips), Send feedback and Settings on the right, then the Windows caption buttons. There is no sidebar, so review screens get the full width.
- **Status bar (28 px):** recording state on the left, then encoder and quality, storage use, and the Record games switch on the right. It replaces the old sidebar status block, storage ring and dock pill.

## Artboard

- Name `Home — Ready`, 1280 × 760 (the default window), on page `Home`, at x 0, y 0.
- Background `var(--color-page)`. A vertical flex column: Titlebar, Content (flex 1), Status bar.
- Font Inter (`var(--font-sans)`). Use `font-variant-numeric: tabular-nums` on every number, time and KDA.
- Colours come only from tokens (`var(--color-*)`). There are no hex literals anywhere.

## Kit sources (Rune Kit file)

Copy the markup of these nodes with `get_jsx`, keep their structure and styles, and change only the text, the icon and the width.

| Need | Rune Kit page › node |
|---|---|
| Segmented control, with a trailing badge | Segmented Control › `AE-0`, section "Rich", the first row (Comments 3) |
| Buttons: Submit, Neutral, Ghost | Button › `54-0` (Variants) and `5A-0` (Icon) |
| Icon button, ghost | Icon Button page |
| Key caps | Keybind page |
| Switch, small | Switch › `MR-0` (Sizes) |
| Badge, compact | Badge › `1N-0` |
| Progress bar | Progress & Loaders page |
| Icons (16 px) | Icons › `43M-0` (Gallery and Catalog) |
| Champion placeholder | Tab Strip › `2J3-0`, the letter avatars ("A", "B", "C") |

## Layout, top to bottom

### Titlebar

- Height 44. Background `var(--color-surface)`. Bottom border 1 px `var(--color-border)`. Row, align centre, padding-left 16.
- **Left:** the Greyscreen mark is a 16 px hexagon outline (SVG, stroke `var(--color-text)`, 1.5 px). Put "Greyscreen" next to it, 13 px, weight 500, `var(--color-text)`, gap 8. The group is 338 px wide (200 plus the 138 px caption buttons), so the navigation sits on the true window centre.
- **Centre** (flex 1, justify centre): the kit segmented control with segments "Home" (selected), "Games" and "Clips". "Clips" carries the kit's trailing count badge "4". Use labels only, with no icons.
- **Right:** a 200 px group, justify end, gap 4, padding-right 8:
  - Ghost icon button with the `comment` icon (Send feedback).
  - Ghost icon button with the `settings-gear` icon.
- **Far right:** the Windows caption buttons, 138 px wide (3 × 46 px, height 44). Draw the minimize, maximize and close glyphs (10 px, 1 px stroke, `var(--color-text-muted)`). There is no hover state.

### Content

Padding 32 left and right, 28 top. A column with gap 40.

#### Last game (hero)

Row, gap 32, align start.

- **Thumbnail**, 576 × 324, radius `var(--radius-lg)`, background `var(--color-layer-2)`, border 1 px `var(--color-border)`.
  - Centre: a 48 px circle, background `var(--color-scrim)`, with a white play triangle (`var(--color-white)`, 16 px).
  - Bottom-left, inset 12: a compact kit badge "32:14".
- **Info column** (flex 1, gap 0, column). Name the layer `Last game / Info`.
  1. **Overline:** "Last game · 2 hours ago". 12 px, `var(--color-text-muted)`, weight 400.
  2. 8 px below: **Result** "Victory". 44 px, weight 600, letter-spacing -0.02em, line-height 48 px, `var(--color-success-fg)`.
  3. 12 px below: **Champion row**, gap 10, align centre:
     - Champion avatar 28 px: the kit letter avatar in purple, `var(--color-avatar-purple-bg)` with border `var(--color-avatar-purple-border)`, letter "A".
     - "Ahri", 15 px, weight 500, `var(--color-text)`.
     - "Summoner's Rift · Classic", 13 px, `var(--color-text-muted)`.
  4. 24 px below: **Stats row**, gap 32. Each stat has a value (22 px, weight 500, `var(--color-text)`) over a label (12 px, `var(--color-text-muted)`, 2 px gap):
     - "8 / 3 / 11" labelled "KDA".
     - "32:14" labelled "Length".
     - "4" labelled "Clips".
     - "0 of 3" labelled "Deaths reviewed".
  5. 24 px below: **Game ribbon** (`Last game / Ribbon`). It is the full width of the column, 40 px tall, and is the signature element (see "Game ribbon" below).
  6. 24 px below: **Actions row**, gap 8:
     - Kit Submit button "Review deaths". Its key (D) goes in the tooltip: a key cap beside the button read as a stray chip.
     - Kit Neutral button "Watch game".
     - Kit Ghost button with the `folder` icon, "Show in folder".

#### Below the hero

Row, gap 40, align start.

**Clips from this game** (flex 1):

- Header row: "Clips from this game", 13 px, weight 500, `var(--color-text)`. Then "4", 13 px, `var(--color-text-muted)`, gap 8. On the right (margin-left auto) a kit Ghost button "All clips".
- 12 px below: a row of 4 clip cards, gap 16, each flex 1. Each card is a column with gap 8:
  - Thumbnail, 16:9 (aspect-ratio 16 / 9, width 100%), radius `var(--radius-md)`, background `var(--color-layer-2)`, border 1 px `var(--color-border)`. A compact kit badge with the length sits bottom-right, inset 8.
  - Title: 13 px, weight 500, `var(--color-text)`.
  - Meta: 12 px, `var(--color-text-muted)`.
- The 4 clips:

  | Title | Meta | Length |
  |---|---|---|
  | Kill | at 4:12 | 0:20 |
  | Double kill | at 14:32 | 0:29 |
  | Baron | at 25:40 | 0:20 |
  | Kill | at 30:02 | 0:20 |

**This week** (fixed width 336, flex-shrink 0):

- Header: "This week", 13 px, weight 500.
- 12 px below: "58%" (28 px, weight 600, `var(--color-text)`) followed by "win rate" (13 px, muted), baseline aligned, gap 8.
- 4 px below: "7 wins · 5 losses · 6 h 40 m played", 12 px, muted.
- 12 px below: the **results strip**, 12 squares of 12 × 12, radius `var(--radius-xs)`, gap 4, oldest first. A win is `var(--color-success-fg)`, a loss is `var(--color-danger-fg)`. The order is W L W W L W L L W L W W. The last square is the hero game.
- 20 px below: the **Recent games** list, 3 rows of 36 px, each with a top border 1 px `var(--color-border)`. Columns:
  - 24 px letter avatar, a fixed slot.
  - Champion, 13 px, weight 500, 72 px slot.
  - Result: "Victory" in `var(--color-success-fg)` or "Defeat" in `var(--color-danger-fg)`, 12 px, 56 px slot.
  - KDA, 12 px, muted, flex 1.
  - Length, 12 px, muted, right aligned, 40 px slot.
- The 3 rows:

  | Avatar | Champion | Result | KDA | Length |
  |---|---|---|---|---|
  | purple "A" | Ahri | Victory | 11 / 2 / 6 | 25:12 |
  | green "J" | Jinx | Defeat | 4 / 7 / 5 | 28:40 |
  | blue "L" | Lux | Victory | 9 / 5 / 21 | 19:03 |

### Status bar

- Height 28. Background `var(--color-surface)`. Top border 1 px `var(--color-border)`. Padding 0 12. Row, align centre, gap 16. Text 12 px.
- **Left:**
  - 8 px dot `var(--color-success-fg)`.
  - "Ready" (weight 500, `var(--color-text)`).
  - "Records your next game automatically. F9 saves the last 30 seconds." in `var(--color-text-muted)`.
- **Right** (margin-left auto, gap 16, separated by 1 × 12 px dividers in `var(--color-border)`):
  - "NVIDIA NVENC · 1080p 60 fps · High", muted.
  - The storage group: "112 of 250 GB", muted, next to a kit progress bar 64 px wide at 45%.
  - "Record games" (muted) with a small kit switch, on.

## Game ribbon

A one-line picture of the whole game drawn from its events. It reads left to right like the seek bar in the review screen, so the user learns it once.

- Width is the column width. Height 40. Time runs from 0:00 to 32:14 across the width.
- **Track:** a 2 px line at y 20, `var(--color-layer-3)`, full width, radius 1.
- **My kills:** 2 × 14 px ticks centred on the track, `var(--color-success-fg)`, at 4:12, 9:40, 14:32, 14:41, 18:05, 22:50, 27:10 and 30:02 (8 kills).
- **My deaths:** 2 × 14 px ticks, `var(--color-danger-fg)`, at 7:55, 16:20 and 24:38 (3 deaths). These match "0 of 3 deaths reviewed".
- **Objectives** (my team): 6 px dots at y 6, above the track, `var(--color-info-fg)`, at 6:30 (dragon), 8:00 (void grubs), 13:10 (dragon), 19:45 (dragon) and 25:40 (baron).
- **Time labels:** "0:00", "10:00", "20:00" and "30:00" at y 34, 11 px, `var(--color-text-faint)`, at their positions. Left-align the first label.
- Position = time ÷ 1934 s × width. Compute the exact left offsets. Do not eyeball them.

## Naming

Name layers `Region / Part`: `Titlebar`, `Titlebar / Nav`, `Last game / Thumbnail`, `Last game / Info`, `Last game / Ribbon`, `Clips / Card 1`, `Week / Results`, `Week / Recent`, `Status bar`.

## Done means

- One artboard, 1280 × 760, with nothing clipped. The content fits above the status bar, and the status bar sits at the very bottom.
- Every colour is a token. Kit parts match the Rune Kit file exactly.
- The ribbon ticks sit at the computed positions.
- The screenshot passes the Paper review checklist: spacing, type, contrast, alignment and fit.

## Later variants (not in this pass)

- `Home — Recording`: the hero becomes the live game (champion, clock and the live feed growing along the ribbon). The status bar shows a red pulse dot, "Recording · 12:34", and Save clip F9.
- `Home — First run`: the hero becomes "Set up Greyscreen" (folder, quality, test recording).
- `Home — Problem`: a kit danger Card above the hero, with one action and Learn more.
- `Home — Empty`: no games yet.
