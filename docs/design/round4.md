# Round 4: Last game refined, and where the recorder lives

This round follows Andres's review of round 3 (2026-11-27). Read `round3.md` and `home-variations.md` first: their rules, sample data and Polish layer still apply. Build on page **Round 4** (`p-6-0`) of the Greyscreen App file. Icons go on page **Icons** (`p-5-0`).

## Andres's decisions

- **Last game:** L1 Cinema is the base.
  - The play button sits in the centre of the whole hero, and the overlay spans the entire hero.
  - Shorter actions: "Watch game" and "Review deaths" are too much text. The Space key cap shows a space-bar glyph, not the word "Space".
  - The flat folder icon does not fit the raised button. It needs an icon with some depth.
  - Clips: put the text under the image, or on the image's bottom over a gradient. Text beside stacked clips looks wrong, and L2's clips were too small.
  - The timeline needs proper tower, dragon, baron and similar icons.
- **Deaths:** L3's Deaths card is dropped. It gives death review too much weight: most players just play the video and click the timeline.
- **Home:** H1 Calm is the base. H2 (List) is dropped: its strips were too cramped and it repeated Watch on every row. H3 (Two columns) is dropped: too many widgets that do not belong together.
- **Recorder:** the Record games on/off belongs somewhere you can reach from every screen, such as the titlebar, a dock or something floating. The state "your next game will be recorded" or "it won't" is what matters. Explore where it goes.

## 1. Icon set (page `Icons`)

**Update (Andres, 2026-11-27):** the custom objective glyphs looked bad. Use **Riot's official minimap icons** for objectives:

- Source: CommunityDragon, `raw.communitydragon.org/latest/game/assets/ux/minimap/icons/`. Local copies are in `~/.local/share/greyscreen-design/lol-icons/`.
- Riot's fan-project terms allow this, and the app already uses Data Dragon champion icons.
- Files:
  - `tower.png`, `inhibitor.png`, `baron.png`, `riftherald.png`, `grub.png`;
  - `dragon_<type>.png` for each elemental dragon (from the event's `dragon_type`), and `dragon_elder.png`.

The custom `folder-reveal` and `space` glyphs below stay. The custom objective glyphs are kept only for reference.

The icons are original drawings, not Riot art. All are drawn on a 16 px grid; the 12 px sizes are scaled for timeline boxes.

| Icon | Shape |
|---|---|
| Turret (`tower`) | A slim tapered tower with a crenellated top and a small window. |
| Inhibitor | A faceted crystal (an elongated diamond) on a short two-step plinth. |
| Dragon | A dragon head in profile with a horn and an open jaw, or a spread wing over a small body. It must read at 12 px. |
| Baron | A serpent head facing front, with two curved horns and a crest of 3 spines. |
| Rift Herald (`herald`) | A round shell with one large central eye. |
| Void grubs (`grubs`) | A small segmented larva with 3 segments and two short legs. |
| Reveal folder (`folder-reveal`) | A folder with depth: a back flap, a front panel tilted open, and a small up-right arrow on the front. |
| Space key (`space`) | The space-bar legend: a wide U-bracket like "␣", 10 px wide. |

Each icon comes in two styles:

- **Line**: a 1.25 px stroke in `currentColor`, round caps and joins, matching the Rune Kit icon catalog (Icons › `43M-0`).
- **Duotone**:
  - fill `color-mix(in srgb, currentColor 28%, transparent)`;
  - the Line stroke on top;
  - a 1 px top highlight with `var(--color-white-12)` where a face catches light.

  This is the "depth" style for raised buttons.

Layout: one artboard `Icons — Game` with a row per icon. Each row shows the Line style at 16 and 12 px, the Duotone style at 16 and 12 px, the icon on a grey timeline box (12 px, on `var(--color-layer-4)`), and on a kit Neutral 28 px icon button. Name each layer `icon / <name> / <style> / <size>`.

## 2. Real frames

The hero and clip images come from real recordings on Andres's PC: `hero-1.jpg`, `hero-2.jpg` and `clip-1.jpg` to `clip-4.jpg`. Their paths are given when they arrive. Use them as `backgroundImage` fills (`background-size: cover`), never stretched.

## 3. Last game, three variants (row y 0)

What changes from L1 in all three:

- **Hero**
  - Hero image `hero-1.jpg`.
  - The scrim covers the whole hero: `var(--color-page)` at 92% at the bottom, then 55% at the middle, then 15% at the top. It is one vertical gradient over the full height.
  - The **play circle** (64 px, `var(--color-scrim)` with a 1 px `var(--color-white-20)` ring and a white triangle) sits in the exact centre of the hero.
- **Actions**
  - Contrast Large "▶ Watch", with a key cap holding the `space` glyph.
  - Neutral Large "Review", with a D key cap.
  - A Neutral 28 px icon button with the **Duotone `folder-reveal`** icon.
- **Timeline**
  - The compact box timeline. In grey objective boxes the **12 px Line icon** sits centred in `var(--color-text)`: dragon, grubs, herald, baron or turret.
  - Add turrets to the sample data:
    - my team: minutes 11, 17, 21 and 28;
    - enemy: minute 23.
  - A box with a kill or death still wins over an objective. Recompute the box states.
- **Breadcrumb and titlebar**: as in L1.

### L4a · Overlay clips (x 0)

- Hero 1232 × 420.
- **Clips**: 4 cards in a row, gap 12, each 16:9 (about 299 × 168), radius 10. The images are `clip-1.jpg` to `clip-4.jpg`.
  - Each card has a bottom gradient over its lower 55%, from `var(--color-page)` at 85% to clear.
  - The title (13 px medium, white) and "14:32" (12 px, `var(--color-text-muted)`) sit on the image's bottom-left, inset 10.
  - The length badge (compact) sits top-right, inset 8.
- The header row "Highlights · 4 clips" with "All clips" sits above the cards.

### L4b · Clips with text below (x 1400)

- Hero 1232 × 400.
- **Clips**: 4 cards. Each has a 16:9 image (radius 8, no overlay, a length badge bottom-right), then 8 px below, the title (13 px medium) and "at 14:32" (12 px muted).

### L4c · Detailed + overlay clips (x 2800)

- Hero 1232 × 300.
- The timeline header has "Detailed" selected. Below it, the **detailed lanes** (You, Objectives, Clips). Objective boxes carry the 12 px icons in their lanes.
- The L4a overlay clip cards sit below at 140 px tall, which is smaller than L4a's but still wider than L2's chips.

## 4. Where the recorder lives (row y 900, base H1 Calm)

What changes from H1 in all three:

- Home's top card loses its switch.
- Images: `hero-2.jpg` for the last-game thumbnail, and clip images for Recent clips.
- Everything else is as H1.

The **record control** is drawn in its "on" state, and the variant also shows a small "off" state copy beside the artboard (an extra 360 × 120 frame next to it named `<variant> / off state`).

- **Off** means `var(--color-text-faint)` and the words "Next game won't be recorded".
- **On** means a success dot and "Next game will be recorded".

### R1 · Titlebar status (x 0)

- In the titlebar, right of the navigation and before Settings, sits a **status button**: a kit Ghost button holding a 16 px hex prism (the 3D mark scaled down, with its state dot), "Recording on", and a chevron.
- Draw its **popover open** under it (kit Popover look: raised card, 280 wide). It contains:
  - "Record games" with the Mini switch;
  - "Next game will be recorded automatically." in muted text;
  - a hairline;
  - "F9 · Save the last 30 s" with a key cap;
  - "112 of 250 GB used" with a 4 px bar;
  - "Recording settings…" as a Ghost button.
- Home's top card becomes a calm sentence with no card: "Ready. Your next game records itself." with the 3D prism at 40 px.

### R2 · Dock (x 1400)

- A **floating dock** centred at the bottom of the window, 16 px above the edge. It is a raised pill 48 px tall with radius 24 and padding 0 8 0 16. Left to right:
  1. a 24 px 3D prism with its state dot, and "Ready" (13 px medium) over "Next game will be recorded" (11 px muted);
  2. the Mini switch;
  3. a 1 × 24 divider;
  4. "Save clip" with an F9 key cap (disabled look while not in a game);
  5. a 1 × 24 divider;
  6. a storage ring (20 px, 45% filled in accent) with "112 GB".
- Home's content ends at least 80 px above the window bottom so the dock never covers it.
- Home's top card is removed. The dock replaces it.

### R3 · Brand is the recorder (x 2800)

- In the titlebar, the **brand mark becomes the recorder**: the 3D prism at 20 px with its state dot, then "Greyscreen", then a kit compact badge-like pill "Recording on" in success-fg text. This is not a restyled kit Badge: it is plain text with a success dot. Then the Mini switch, all on the left.
- The rest of the titlebar keeps the navigation centred and Settings on the right.
- Home's top card becomes the H1 card without its switch: the 3D prism at 56 px, "Greyscreen is ready", the F9 line and the storage hint.

## Done means

- The page `Icons` has `Icons — Game`.
- Page `Round 4` has these artboards:
  - `Last game — L4a Overlay clips`
  - `Last game — L4b Clips below`
  - `Last game — L4c Detailed`
  - `Home — R1 Titlebar status`
  - `Home — R2 Dock`
  - `Home — R3 Brand recorder`

  Each R variant also has its `off state` frame.
- Real frames are used, every colour is a token, and nothing is clipped.
