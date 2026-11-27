# Home variations, round 2

Six different takes on Home, built side by side so Andres can judge them. The facts come from `product.md` and `surface.md`. The first pass (`home.md`, artboard `Home — Ready`) is the baseline, and these variations replace it.

## What Andres said about the first pass

1. The titlebar icons (feedback, settings) look disconnected from the rest of the app.
2. The stats look thrown in. No section feels separate from the next, and everything has the same importance. There is no hierarchy.
3. "This week" should clearly be its own section.
4. **Watch game is the main action.** Review deaths comes second. "Show in folder" should be just a folder icon. The timeline needs a different design.
5. The status bar is decent, but the switch is too big for it and the "Record games" label reads oddly. Show recording on or off with an icon or a richer graphic (isometric or 3D is welcome). Do not stop at small icons.
6. Reference: his Hex app. Its key caps (raised and soft), its compact switches, its feature card with a large 3D icon, and its gradient primary button are nicer than ours. See "Polish layer".

## Rules for every variation

- Artboard 1280 × 760, background `var(--color-page)`. Put it on page **Home explorations** (`p-2-0`) of the Greyscreen App file, at the x and y given for each variation (set `left` and `top` with update_styles after creating it).
- Colours come only from `var(--color-*)` tokens. Inter, with tabular figures for every number.
- One focal point per screen. Each section has a clear edge: a surface panel, a hairline rule plus a section label, or strong spacing. Pick the one the variation names.
- Actions on the last game:
  - **Watch game** is the primary button.
  - **Review deaths** is secondary.
  - Show in folder is an icon-only button with the `folder` icon.
- The navigation is Home, Games and Clips. Settings is always one button. Send feedback is never a loose titlebar icon: it moves into the place each variation names.
- Use only real data (the "Sample data" section below). Never show rank, LP, CS, items, role or patch.
- Thumbnails and champion icons are placeholders. A thumbnail is a `var(--color-layer-2)` fill. A champion is a letter avatar.
- Reuse parts already built in this file: the `Home — Ready` artboard on page `Home` has the segmented control, buttons, badges, letter avatars, caption buttons and the status bar. Clone or copy them, and read the Rune Kit (`01M8F9Y9HZ7M8V40VA1JZFFG54`, read only) only for parts that are not there.
- Variations marked **Polish** use the Polish layer (Hex-style key caps, switch, primary button and feature card). Variations marked **Stock** use only stock Rune kit looks. This lets Andres compare the two. In Polish variations, every panel, tile and card is the Polish **raised card** (no border, shadow stack) instead of a bordered surface.

## Sample data (all variations)

Last game, 2 hours ago:

- Result and length: **Victory**, 32:14.
- Me: **Ahri** (purple avatar "A"), Summoner's Rift · Classic.
- KDA **8 / 3 / 11**. 4 clips. Deaths reviewed: 0 of 3.
- Teams:
  - Mine (blue): Ahri, Garen, Vi, Kai'Sa, Nautilus.
  - Enemy (red): Zed, Lee Sin, Ezreal, Thresh, Darius.
- Team kills: **27 – 16**.

Events:

- My kills: 4:12, 9:40, 14:32, 14:41 (Double kill), 18:05, 22:50, 27:10, 30:02.
- My deaths:
  - 7:55, killed by Lee Sin, assist Zed.
  - 16:20, killed by Zed.
  - 24:38, killed by Darius, assists Zed and Thresh.
- My team's objectives: 6:30 Dragon (Infernal), 8:00 Void grubs, 13:10 Dragon (Mountain), 19:45 Dragon (Infernal), 25:40 Baron.
- Enemy objectives: 15:30 Herald, 22:10 Dragon (Ocean).

Kill lead over time (my team minus enemy). Draw the chart through these points:

| Time | 0:00 | 4:00 | 6:00 | 8:00 | 10:00 | 12:00 | 14:00 | 15:00 | 16:30 | 18:00 | 20:00 | 22:00 | 24:00 | 26:00 | 28:00 | 30:00 | 32:14 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Lead | 0 | +1 | +1 | −1 | 0 | +2 | +3 | +5 | +3 | +5 | +6 | +5 | +4 | +8 | +9 | +11 | +11 |

Clips:

| Clip | At | Length |
|---|---|---|
| Kill | 4:12 | 0:20 |
| Double kill | 14:32 | 0:29 |
| Baron | 25:40 | 0:20 |
| Kill | 30:02 | 0:20 |

This week:

- 12 games: 7 wins, 5 losses, 58% win rate, 6 h 40 m played.
- Results, oldest first: W L W W L W L L W L W W.
- Most played: Ahri, 5 games, 4 wins.

Today:

| Champion | Result | KDA | Length | When |
|---|---|---|---|---|
| Ahri (last game) | Victory | 8 / 3 / 11 | 32:14 | 2 h ago |
| Ahri | Victory | 11 / 2 / 6 | 25:12 | 3 h ago |
| Jinx (green "J") | Defeat | 4 / 7 / 5 | 28:40 | 4 h ago |

Earlier this week: Lux (blue "L"), Victory, 9 / 5 / 21, 19:03, ARAM, yesterday.

Recorder:

- Ready (armed) and Record games on.
- NVIDIA NVENC, 1080p, 60 fps, High.
- Storage: 112 of 250 GB.
- F9 saves the last 30 seconds.

Time position on any timeline: `x = seconds ÷ 1934 × width`. Compute it. Do not eyeball it.

## Polish layer (Hex-style)

Hex is built on Rune too (opencode theme), so most of its polish is the stock kit used at better sizes. Only the key cap and the raised card are Hex's own parts. The values below come from `~/dev/third/hex/src/rune_ui/` (`controls.rs`, `output.rs`, `spark.rs`), mapped to our neutral dark tokens. Where Hex uses a black or white alpha, use `color-mix(in srgb, var(--color-…) N%, transparent)`. In Paper, set `backgroundColor` and `backgroundImage` separately: the shorthand drops tokens.

- **Switch**: the kit Switch at **Mini** size, a 36 × 16 track with a 21 × 14 knob inset 1 (Switch › `MR-0` Sizes, the smallest). Hex never uses a bigger one in rows. On is `var(--color-accent)`, off is `var(--color-layer-4)`. Use it as is.
- **Primary button**: the kit **Contrast** variant at **Large** size: 28 px tall, padding 0 11, radius 6, 13 px at weight 530. The soft "gradient" Andres saw is the kit's own top sheen (white 20% → 0) and its edge shadow, which are already part of Contrast. Copy it from Button › `54-0`. A play icon can lead.
- **Secondary text button** (Hex's "Open file"): the kit **Ghost** at Large size: no fill, `var(--color-text)`.
- **Key cap** (Hex "small key"): a low-profile key sitting in a recessed tray.
  - Tray: 28 px tall, padding 3, radius 7.5.
    - Background: a vertical gradient from `var(--color-page)` to `var(--color-surface)`.
    - Border: 0.5 px `color-mix(in srgb, var(--color-white) 7%, transparent)`.
    - Inset shadow `inset 0 1px 2.5px var(--color-shadow)`.
  - Cap: 22 px tall, min width 22, radius 4.5.
    - Background: a vertical gradient from `var(--color-layer-4)` to `var(--color-page)`.
    - Padding: 0.5 px on the left, right and top, and 1.5 px at the bottom (the skirt).
    - Shadows: `0.5px 0.5px 0 var(--color-shadow), 0 1.5px 3px var(--color-shadow-soft)`.
  - Face: 20 px tall, radius 4, `var(--color-surface)`, padding 0 6.
  - Legend: 11 px, weight 400, `var(--color-text)`.
  - A chord is two caps 3 px apart in one tray.
- **Raised card** (Hex feature card and grouped panels):
  - Background `var(--color-surface)`, radius 8, **no border**.
  - Shadow: `0 2px 4px var(--color-shadow-soft), 0 1px 2px var(--color-shadow-soft), 0 0 0 0.5px color-mix(in srgb, var(--color-white) 16%, transparent), inset 0 0.5px 0 color-mix(in srgb, var(--color-white) 6%, transparent)`.
  - Feature card layout: padding 14 16, a mark (30 to 56 px) with a 12 gap, then the copy (title 13/530, line 13/400 muted), and the actions on the right with a 4 px gap.
- **Grouped rows** (Hex settings language):
  - Rows are 48 px, or 60 px with a description, with a 16 px side inset.
  - Separators are a 1 px `var(--color-border-muted)` line inset 16 on both sides.
  - Section header: 13 px at weight 530, `var(--color-text-muted)`, indented 16, 8 px above the card.
  - Right hint: 11 px at weight 440, letter-spacing 0.05em, `var(--color-text-faint)`.
  - Sections are 24 px apart.
- **3D mark**: Hex's sparkle is a lit 3D star mesh that flips about the Y axis every 3.6 s. Ours is a **lit hexagonal prism**, drawn as SVG faces:
  - Top face `var(--color-layer-4)`, left face `var(--color-layer-3)`, right face `var(--color-surface)`.
  - Edges 0.5 px `var(--color-white-12)`.
  - A small bright glint on the top-left edge, `var(--color-white-20)`.
  - State dot on the top face: `var(--color-success-fg)` when armed, `var(--color-danger-fg)` when recording, none at 50% opacity when off.
  - `stop-color` cannot take `var()`, so use flat faces. The flip animation is for the app, not for Paper.

## The six variations

### A · Cinema (Polish), x 0, y 0

The last game is a film poster.

- **Titlebar**, 44 px, `var(--color-surface)`, hairline bottom.
  - Left: the hex mark and "Greyscreen", then a **recorder capsule**: a pill 24 px tall (radius 12, `var(--color-layer-2)`, 1 px `var(--color-border)`) holding an 8 px `var(--color-success-fg)` dot, the word "Ready", a 1 px divider and the kit Mini switch. The capsule is the record toggle.
  - Centre: the navigation segmented control.
  - Right: one kit **Neutral** icon button, 28 px, with `settings-gear`, so it matches the segmented control's filled track. Then the caption buttons.
  - Feedback lives inside Settings, so it is not drawn here.
- **Hero**, full content width 1216 × 400, radius 12, `var(--color-layer-2)`. Content padding is 32 on the sides and 24 on top.
  - Scrim: the bottom 60% fades from transparent to `var(--color-page)`. Use a gradient with tokens. If tokens drop, use a solid `var(--color-scrim)` band.
  - Bottom-left, inset 32: "Last game · 2 hours ago" (12 px, muted). Below it, "Victory" at 56 px, weight 650, `var(--color-success-fg)`, letter-spacing −0.03em. Below that, the champion row (avatar 28, "Ahri" 16 px medium, "Summoner's Rift · Classic · 32:14" muted), then "8 / 3 / 11" at 15 px with "KDA" muted.
  - Bottom-right, inset 32 and aligned to the bottom of the KDA line: the Polish primary "Watch game" with a play icon, a kit Neutral "Review deaths" (Large) with the Polish key cap "D", and a Neutral icon button `folder`. All are 28 px tall.
  - **Scrubber timeline** along the hero's bottom edge, inset 32 left and right, 16 above the bottom: a 4 px track `var(--color-white-12)`. My kills are 2 × 10 ticks in `var(--color-success-fg)`. My deaths are 8 × 8 diamonds (a rotated square) in `var(--color-danger-fg)`. Objectives are 6 px dots in `var(--color-info-fg)` above the track, and enemy objectives are hollow dots. It reads like a video player seek bar.
- **Below**, 32 px gap, a row with gap 32.
  - **Highlights** (flex 1): the label "Highlights" (13 px, medium) with "4 clips" muted beside it, and "All clips" as a Polish secondary text button on the right. Under it, 4 clip cards (16:9 thumbnail with a length badge, title and "at 14:32").
  - **This week** (360 px): a surface panel (radius 12, padding 16).
    - "This week" label, with "7–5" on the right in muted text.
    - "58%" at 32 px, weight 600, with "win rate".
    - The results strip: 12 squares of 14 px.
    - "Most played: Ahri · 5 games · 4 wins" (12 px, muted).
- **No status bar.** The recorder capsule carries the state. Storage moves to Settings.

### B · Bento (Polish), x 1400, y 0

Home is a set of tiles with clear sizes, where each tile is one idea.

- **Titlebar**:
  - Left: brand.
  - Navigation segmented control left-aligned, 24 px after the brand.
  - Right: kit Neutral icon buttons for feedback (`comment`) and settings (`settings-gear`), 28 px, 6 px apart. Both are filled, so they match the segmented control.
  - Then the caption buttons.
- **Content**: padding 24, a CSS grid of 4 columns × 3 rows with 12 px gaps, filling 1232 × 668. Each tile is a surface panel: radius 12, `var(--color-surface)`, 1 px `var(--color-border)`, padding 16. Each tile has a 12 px muted label at its top-left.
  1. **Last game**, columns 1–2, rows 1–2.
     - A 16:9 thumbnail across the full tile width with a centred play circle.
     - "Victory" at 32 px success, with "Ahri · 32:14 · 2 h ago".
     - The Polish primary "Watch game" and a Neutral icon `folder`.
  2. **KDA**, column 3, row 1: "8 / 3 / 11" at 34 px, with "27 – 16 team kills" muted below.
  3. **Deaths to review**, column 4, row 1:
     - A ring progress 0 of 3: a 56 px circle, a 4 px track `var(--color-layer-3)` with no fill, and "0/3" in the centre.
     - The kit Neutral "Review deaths" with a Polish key cap "D".
  4. **Story**, columns 3–4, row 2: the kill-lead chart (see C for the style) at 120 px tall, with objective dots and death diamonds.
  5. **Recorder**, column 1, row 3:
     - The 3D hex prism at 56 px, "Ready" at 15 px medium, and "Records your next game" muted.
     - The kit Mini switch at the top-right.
     - A storage bar at the bottom: "112 of 250 GB" and a 4 px bar.
  6. **Highlights**, columns 2–3, row 3: 3 clip cards in a row with titles.
  7. **This week**, column 4, row 3: "58%", "7 wins · 5 losses" and the results strip.
- **No status bar.** The Recorder tile carries the state.

### C · Story (Stock), x 2800, y 0

The game's story is the hero: how the fight went, and where I died.

- **Titlebar**:
  - Brand on the left, the navigation centred, and a kit Neutral 28 px settings icon button on the right.
  - Feedback goes in the status bar.
- **Header row**, padding 32 40 0:
  - Left: "Victory" at 40 px success, followed on the same baseline by "Ahri · 32:14 · 8 / 3 / 11 · 2 hours ago" at 15 px muted. The avatar sits before "Victory".
  - Right: the kit **Submit** button "Watch game" (stock), the kit Neutral "Review deaths" and the kit Neutral icon button `folder`.
- **Story chart**, 24 px below, full width 1200, 200 px tall, `var(--color-surface)`, radius 10, padding 16.
  - A centre zero line, 1 px `var(--color-border)`.
  - The kill-lead line is 2 px `var(--color-text)`. The area above zero is filled `var(--color-success-bg)` and the area below zero is `var(--color-danger-bg)`. Y is ±12 kills.
  - Top band (24 px): objective glyphs. Use 12 px letters in circles: D for dragon, G for grubs, B for baron, H for herald. Mine are `var(--color-info-fg)` and the enemy's are `var(--color-text-faint)`.
  - My deaths are numbered circles ①②③ (18 px, `var(--color-danger-bg)` fill, `var(--color-danger-fg)` border and number) on the line at their times. My kills are 5 px `var(--color-success-fg)` dots on the line.
  - X axis labels every 5 minutes, 11 px faint. A right-edge label reads "+11".
- **Below**, 24 px gap, a 3-column row. Each section is separated by a section label and has no box.
  - **Your deaths** (2 columns wide). The label is "Your deaths", with "0 of 3 reviewed" muted. 3 death rows of 56 px with hairlines between them:
    - Number badge, time (mono), "Killed by Lee Sin" (13 px medium) with "+ Zed" muted.
    - On the right, "Not reviewed" as a kit compact badge, then a chevron.
  - **This week** (column 3): "58%" at 28 px, "7 wins · 5 losses", the results strip, and "Most played: Ahri".
  - Clips appear as a muted "4 clips from this game →" link under the deaths.
- **Status bar** (28 px, stock):
  - Left: the dot, "Ready" and the short text.
  - Right: "1080p 60 · High", the storage text with a 48 px bar, and a **kit Mini switch (36 × 16) with no label**, with a tooltip on hover.
  - A kit Ghost "Feedback" button sits at the far right.

### D · Session (Polish), x 0, y 900

Home is today's session: a list of today's games on the left and the chosen game on the right.

- **Titlebar**: as in C, but the settings button is a Polish-weight Neutral icon button.
- **Left rail**, 320 px, `var(--color-surface)`, right hairline, padding 16.
  - **Recorder card** at the top: a feature card (Polish) at a smaller scale. It has the 40 px 3D prism, "Ready for your next game", "F9 saves the last 30 s" with a Polish key cap, and the kit Mini switch on the right.
  - "Today" label, with "3 games · 2–1 · 1 h 26 m" muted on the right.
  - The 3 today games as kit **TabStrip vertical "With subtitles"** rows (Tab Strip › `2J3-0`): the letter avatar, the title "Ahri — Victory" and the subtitle "8 / 3 / 11 · 32:14 · 2 h ago". The first row is active. The status mark is green for a win and red for a loss.
  - "Earlier this week", with the Lux row.
  - At the bottom: the "This week" block (58%, the strip and "7 wins · 5 losses").
  - Feedback is a ghost row at the very bottom.
- **Detail**, padding 28.
  - Thumbnail 16:9 at the full detail width, capped at 400 px tall, with radius 10 and a centred play circle.
  - Under it: "Victory" at 36 px success, with "Ahri · Summoner's Rift · Classic".
  - Actions on the right: the Polish primary "Watch game" (with a Space key cap), the Neutral "Review deaths" with a D cap, and the Neutral icon `folder`.
  - **Stat strip**: 4 equal cells inside one surface panel, with vertical hairlines between them (KDA, Length, Team kills 27–16, Deaths reviewed 0/3). Values are 20 px and labels 12 px muted.
  - **Lanes timeline**: two labelled lanes, 24 px each, with the labels "You" and "Objectives" in a 72 px label column.
    - "You": my kills as ticks and my deaths as diamonds.
    - "Objectives": D, G, B and H glyph circles, with the enemy's in faint.
    - The time axis runs underneath.
- **No status bar.**

### E · Calm (Polish), x 1400, y 900

Trust first, in the Hex settings language: grouped panels and a feature card on top.

- **Titlebar**: brand on the left, the navigation centred, and a kit Neutral 28 px settings icon button on the right.
- Content width 880, centred, padding-top 28. Section labels sit above the panels, 13 px medium, with a right-aligned 12 px muted hint like Hex's "Runs top to bottom".
- **Recorder feature card**:
  - The 3D prism at 56 px.
  - Title "Greyscreen is ready", and "Your next game records itself. Press F9 in game to save the last 30 seconds." with a Polish F9 key cap inline.
  - On the right: the kit Mini switch.
- **"Last game"** label, with the hint "2 hours ago". It is a grouped panel (radius 12, surface, hairline dividers) with 3 rows:
  1. A 160 × 90 thumbnail, "Victory" (20 px success), "Ahri · 32:14 · 8 / 3 / 11", and on the right the Polish primary "Watch game" and the Neutral icon `folder`.
  2. A row with a 32 px tall ribbon timeline (kills as ticks, deaths as diamonds, objectives as dots).
  3. "3 deaths to review" with a D key cap, a chevron and the muted hint "0 of 3 reviewed".
- Two grouped panels side by side:
  - **"Highlights"** (hint "4 clips") with 2 clip rows: a thumbnail of 96 × 54, a title, "at 14:32" and the length.
  - **"This week"** (hint "12 games"): "58% win rate", the strip, and "Most played: Ahri".
- **No status bar.** Storage shows as a hint on the recorder card: "112 of 250 GB used".

### F · Editorial (Stock), x 2800, y 900

Swiss print: big type, numbered sections, hairlines, no boxes.

- **Titlebar**:
  - The brand mark doubles as the recorder. A 16 px hex outline with a filled `var(--color-success-fg)` centre dot means armed. Next to it, "Greyscreen" and a muted "· Ready".
  - The navigation sits left-aligned after it.
  - Right: a kit Ghost "Feedback" text button and a Neutral settings icon button.
- **Section labels** use the mono font (`var(--font-mono)`), 11 px uppercase, letter-spacing 0.08em, muted, for example "01 — LAST GAME". Each sits on a full-width 1 px hairline with 12 px of space below.
- **01 Last game**: a row.
  - Left (flex 1): "Victory" at 72 px, weight 700, letter-spacing −0.04em, success, line-height 72. Next to it, in `var(--color-text)` at 72 px weight 300, "8/3/11". Below: "Ahri · Summoner's Rift · Classic · 32:14 · 2 hours ago" at 14 px muted.
  - Right (360): a 360 × 203 thumbnail. Under it, the kit Submit "Watch game", the Neutral "Review deaths" and the Neutral icon `folder`.
- **Minute bar**, 16 px below, full width. There are 33 cells (one per minute, 0 to 32), each 32 × 20 px with a 2 px gap and a `var(--color-layer-2)` fill.
  - A minute holding a kill of mine is filled `var(--color-success-bg)` with a 2 px top border in success-fg.
  - A minute holding a death of mine is `var(--color-danger-bg)` with a danger-fg top border.
  - An objective minute has a 4 px info-fg dot above the cell.
  - Minute labels go under every 5th cell.
- **02 Highlights**: 4 clip cards in a row, as the first pass had them.
- **03 This week**, on the same row as 02, 300 px wide: "58%" at 40 px, the strip, "7 wins · 5 losses · 6 h 40 m".
- **No status bar.** The titlebar mark carries the state, and storage moves to Settings.

## Done means (per variation)

- One artboard named `Home — <Letter> <Name>` (for example `Home — A Cinema`) at its x and y on page `p-2-0`.
- Nothing is clipped, and every colour is a token. Ticks are placed from computed times.
- The screenshot passes the Paper review checklist.
- finish_working_on_nodes is called.
