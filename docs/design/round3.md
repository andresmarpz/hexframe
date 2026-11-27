# Round 3: Last game screen and Home

The round-2 review split Home into two screens:

- **Last game:** a screen about one game, the one just played. It is based on variation A (Cinema). It opens on its own after a game ends, or from Home.
- **Home:** the real home page. It is based on variation E (Calm), and it stays minimal and simple for now.

## Andres's decisions (2026-11-27)

- He likes A's hero. It needs a gradient overlay that is dark at the bottom and fades to clear, so the **game's preview frame** shows through.
- Remove "This week" from the game screen. It is an unrelated comparison. It lives on Home.
- The brand icon in the titlebar takes you to Home.
- B (Bento) and C (Story) are dropped.
- Timeline: F's boxes look better than a straight line with icons. Squares suit key events. Boxes are **green, red and grey**.
- D's detailed lanes are interesting as an optional **Compact / Detailed timeline** setting.
- E (Calm) becomes the real Home. It is minimal, and we keep it simple to begin with.
- The Last game screen is A plus highlight clips in a bar at the bottom, with no This week, the box timeline and the optional detailed timeline.

## Shared rules

- Read `home-variations.md` for the sample data, the Polish layer and the rules. All of them still apply.
- Every artboard uses the **Polish** parts:
  - the kit Mini switch;
  - the kit Contrast button at Large size for the primary action;
  - Hex key caps;
  - raised cards.
- The Paper file is Greyscreen App. Build on page **Round 3** (`p-4-0`). Each artboard is 1280 × 760 and goes at the x and y given below.
- Reuse what exists. Read these with get_jsx and copy them:

  | Part | Node |
  |---|---|
  | A Cinema | `5S-0` (page `p-2-0`) |
  | E Calm | `5T-0` |
  | F Editorial, for the minute boxes | `5R-0` |
  | D Session, for the lanes and the left rail | `7S-0` |

### Titlebar (all six)

- Height 44, `var(--color-surface)`, with a hairline at the bottom.
- **Left:** the brand group is the Home button: the hex mark plus "Greyscreen". On Last game screens it sits inside a kit Ghost button with a tooltip "Home".
- **Centre:** the segmented navigation: Home, Games, Clips.
  - On **Home**, the "Home" segment is selected.
  - On **Last game**, the "Games" segment is selected, because a game page belongs to Games. Under the titlebar, a 32 px breadcrumb row reads "Games › Ahri · Victory · 2 hours ago" (13 px; the last part muted).
- **Right:** one kit Neutral 28 px settings icon button, then the caption buttons.

### Preview frame and scrim

- The hero shows the game's preview frame. Here that is a placeholder: a `var(--color-layer-2)` fill with the 4 corner brackets from Neon's designs (12 px L-shapes, 1 px `var(--color-white-12)`, inset 16).
- **Scrim:** a vertical gradient over the bottom 65% of the hero, from `var(--color-page)` at the bottom to `color-mix(in srgb, var(--color-page) 0%, transparent)` at the top.
  - In Paper, set `backgroundImage` on its own layer; do not use the `background` shorthand.
  - The top of the frame stays clear.
- Text and actions sit on the dark part.

### Box timeline (compact)

One box per minute of the game, from 0 to 32, so 33 boxes. They fill the given width with a 2 px gap. Each box is 20 px tall with radius 2.

| Box state | Fill | Top border |
|---|---|---|
| Empty minute | `var(--color-layer-2)` | none |
| Kill minute (one of mine) | `var(--color-success-bg)` | 2 px `var(--color-success-fg)` |
| Death minute (one of mine) | `var(--color-danger-bg)` | 2 px `var(--color-danger-fg)` |
| Objective minute, mine or the enemy's (grey) | `var(--color-layer-4)` | 2 px `var(--color-text-muted)` |

- Priority when a minute holds more than one event: death, then kill, then objective.
- A minute with 2 or more of my kills shows "2" (9 px, weight 600, success-fg) centred in the box.
- Under every 5th box sits a minute label: 10 px mono, `var(--color-text-faint)`.

Minutes for the sample game:

| Event | Minutes |
|---|---|
| Kills | 4, 9, 14 (×2), 18, 22, 27, 30 |
| Deaths | 7, 16, 24 |
| Objectives | 6, 8, 13, 15, 19, 25 |

Minute 22 also has an enemy dragon, but the kill wins.

### Box timeline (detailed)

Three lanes, each a row of 33 boxes 16 px tall, with a 72 px label column on the left (12 px muted).

- **You:** kills are green and deaths are red. A box with 2 kills shows "2".
- **Objectives:** my team's objectives are grey with an info-fg 2 px top border. Enemy objectives are grey with a text-faint top border. A 9 px letter sits in each box: D, G, H or B.
- **Clips:** the minutes a clip covers are `var(--color-info-bg)` with no border. A clip runs from 15 s before its event to 5 s after, so it covers 1 or 2 boxes.
- Minute labels sit under the last lane.

### Timeline header

- "Timeline" (13 px medium).
- Inline legend items: 8 px swatches with the labels "Kill", "Death" and "Objective" (12 px muted).
- On the right, a kit segmented control "Compact | Detailed" at its smallest size. Its selection follows the variant.

### Highlights bar

A strip at the bottom of the Last game screen: "Highlights · 4 clips" on the left and a kit Ghost "All clips" on the right, then the 4 clips. The style differs per variant.

### Sample games for Home lists

| Game | Result | KDA | Length | When |
|---|---|---|---|---|
| Ahri | Victory | 8 / 3 / 11 | 32:14 | 2 h ago |
| Ahri | Victory | 11 / 2 / 6 | 25:12 | 3 h ago |
| Jinx | Defeat | 4 / 7 / 5 | 28:40 | 4 h ago |
| Lux | Victory | 9 / 5 / 21 | 19:03 | yesterday, ARAM |
| Jinx | Defeat | 2 / 6 / 4 | 30:55 | yesterday |

- Rows other than the first need small box strips. Spread their kills and deaths evenly across the game's minutes. Add objectives at minutes 6, 13 and 20, but only in games that are 20 minutes or longer.

## Last game screens (row 1, y 0)

### L1 · Cinema (x 0)

A in its purest form.

- Titlebar, then the breadcrumb row.
- **Hero:** inset 24 on the sides and 8 on top. It is 1232 × 440, radius 12, with the preview frame and the scrim.
  - Play circle: 56 px, centred in the clear top area.
  - Bottom-left, inset 28:
    - "Last game · 2 hours ago", 12 px muted;
    - "Victory" at 56 px, weight 650, success, letter-spacing −0.03em;
    - avatar plus "Ahri" plus "Summoner's Rift · Classic · 32:14";
    - "8 / 3 / 11 KDA".
  - Bottom-right, inset 28, bottom-aligned with the KDA line: Contrast Large "Watch game" (play icon and a Space key cap), Neutral Large "Review deaths" (D key cap), and Neutral icon `folder`.
  - The **compact box timeline** spans the hero's full inner width, 20 px above its bottom edge, with no header. The minute labels sit inside the scrim.
- **Highlights bar**, 16 px below the hero: a row of 4 clip cards, each with a 16:9 thumbnail 128 × 72, a title and "at 14:32 · 0:20" to its right. The cards share the width equally and sit on a raised card that spans the full width.

### L2 · Cinema + Detailed (x 1400)

The setting's other state.

- Titlebar, then the breadcrumb row.
- **Hero:** 1232 × 360, with the preview frame, the scrim, and the same text and actions as L1. There is no timeline inside the hero.
- **Timeline section**, 20 px below: the timeline header with "Detailed" selected, then the **detailed lanes** across the full width.
- **Highlights bar** pinned at the bottom as a filmstrip, 64 px tall, on a `var(--color-surface)` band with a top hairline, padding 0 24. It holds "Highlights" and 4 compact clip chips: a 64 × 36 thumbnail, a title, then "14:32" in muted text. "All clips" sits on the right.

### L3 · Poster + Deaths (x 2800)

The hero sits next to the game's deaths: the related next step, with no comparisons.

- Titlebar, then the breadcrumb row.
- **Row**, gap 16:
  - **Hero** (flex 1, 440 tall): the preview frame, the scrim, and the L1 text and actions.
  - **Deaths card** (320 wide, 440 tall, raised card):
    - Header: "Deaths", with "0 of 3 reviewed" muted on the right.
    - 3 rows of 64 px. Each has a red number badge, the time in mono, and "Killed by Lee Sin" with "+ Zed" muted on the next line.
    - At the bottom, a Neutral Large full-width "Review deaths" with a D key cap.
    - In this variant the hero's own actions are "Watch game" and the folder only.
- The **compact timeline** with its header ("Compact" selected) runs across the full width below, 20 px down.
- The **Highlights bar** uses the L2 filmstrip style, at the bottom.

## Home screens (row 2, y 900)

### H1 · Calm (x 0)

E, refined, as the real home page.

- Titlebar (Home selected). Content is a centred column 880 wide, padding-top 28.
- **Recorder feature card**: as in E, with the 3D prism, "Greyscreen is ready", the F9 key cap line, the Mini switch and the storage hint.
- **"Last game"** grouped panel, hint "2 hours ago". There are 2 rows:
  1. A 192 × 108 thumbnail with a mini scrim; "Victory" at 20 px success; "Ahri · 32:14 · 8 / 3 / 11"; and on the right Contrast "Watch game", Neutral "Open" (goes to the Last game screen) and the icon `folder`.
  2. The compact box timeline across the full inner width, 14 px tall.
- **Two panels** side by side:
  - "This week" (hint "12 games"): "58% win rate", the strip, and "7 wins · 5 losses · Most played: Ahri".
  - "Recent clips" (hint "4 new"): 2 rows of clips.

### H2 · Calm list (x 1400)

Home as a calm log of recent games, where every game shows its shape.

- Titlebar (Home selected). Content is a centred column 880 wide.
- **Recorder card**, the same as H1 but more compact: a 40 px prism, one line of text, and the switch.
- **"Recent games"** grouped panel, hint "This week: 7–5 · 58%". There are 5 rows, 60 px each. Each row holds, in fixed slots:
  - a 28 px avatar;
  - the champion and result (13 px medium; the result in success or danger);
  - "8 / 3 / 11 · 32:14" muted;
  - a **mini box strip** of 240 px with 10 px boxes, 1 px gaps and no labels;
  - the time ago, right-aligned;
  - a chevron.
  - The first row also has a Contrast "Watch game" that shows on hover. Draw it visible.
- A footer link row: "All games →".

### H3 · Calm two columns (x 2800)

- Titlebar (Home selected). Content padding 28, in two columns with a 24 px gap.
- **Left (flex 1):**
  - A "Last game" card: a raised card holding a 16:9 hero 360 px tall with the preview frame, the scrim, "Victory" at 32 px, "Ahri · 8 / 3 / 11 · 32:14", and Contrast "Watch game", Neutral "Open" and the folder icon. The compact box timeline sits under the hero, inside the card.
  - "Earlier today": 2 slim rows with mini strips (the H2 row style).
- **Right (340):**
  - A **Recorder** raised card, tall: the 3D prism at 72 px centred; "Ready" (17 px medium); "Your next game records itself."; an "F9 · Save last 30 s" key cap row; a hairline; the "Record games" Mini switch row; and "112 of 250 GB" with a 4 px bar.
  - A **This week** raised card: "58%", the strip and "7 wins · 5 losses".

## Done means

- 6 artboards on page `p-4-0`, named:
  - `Last game — L1 Cinema`
  - `Last game — L2 Detailed`
  - `Last game — L3 Poster`
  - `Home — H1 Calm`
  - `Home — H2 List`
  - `Home — H3 Two columns`
- Nothing is clipped, and every colour is a token. Box minutes match the table above.
- The scrim fades from dark at the bottom to clear at the top.
- finish_working_on_nodes is called with each worker's own artboard id.
