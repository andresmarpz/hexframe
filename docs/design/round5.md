# Round 5: Last game polish, and the recorder without a switch

This round follows Andres's review of round 4 (2026-11-27). It builds on `round4.md`, `round3.md` and `home-variations.md`, which hold the rules, sample data, Polish layer and box timeline. Build on page **Round 5** (`p-7-0`) of the Greyscreen App file.

## Andres's decisions

- **Last game = L4a**: the hero with player and timeline, overlay clips at the bottom. Overlay clips beat clips with text below.
- **Kill and death boxes:** use League's own kill and death icons, not plain colour. Each dragon shows its own elemental icon.
- **"All clips"** must look like an action that opens the Clips screen, not plain text.
- **Detailed timeline:** keep it, with a proper small toggle. Where it lives is open: on L4a, or as a global setting in Settings.
- **Recorder:** the titlebar status (R1) is his favourite, but not with the "Recording on" text. Show the state with an icon instead. He also really likes the **dock** (R2), but not with that switch. Explore dock variants.

## Assets

- **Real frames:** `~/.local/share/greyscreen-design/frames/hero-1.jpg`, `hero-2.jpg`, `clip-1.jpg` to `clip-4.jpg`. They are 1920×1080. Use them as image fills with `object-fit: cover` (`<img src="paper-asset:///Users/andres/.local/share/greyscreen-design/frames/…">`).
- **Official icons:** `~/.local/share/greyscreen-design/lol-icons/`:
  - the coloured minimap set (`tower.png`, `baron.png`, `riftherald.png`, `grub.png`, `dragon_<type>.png`, `inhibitor.png`);
  - `official/` for kill, death, assist and the monochrome objectives. `official/README.md` lists them.

## Timeline boxes, revised

- **Kill box:** `var(--color-success-bg)` with a 2 px success-fg top border, plus the official kill icon at 12 px in the centre, tinted or left as is so it reads on green.
  - With 2 or more kills in the minute: the icon plus a tiny "×2" (9 px, weight 600) to its right, or the multikill icon if one exists.
- **Death box:** danger-bg with a danger-fg top border, plus the official death icon at 12 px.
- **Objective box:** stays grey with the coloured minimap icon at 14 px.
  - Dragon types:
    - 6:30 Infernal
    - 13:10 Mountain
    - 19:45 Infernal
    - Enemy, 22:10: Ocean
  - Use the matching `dragon_<type>.png`.
- **Empty box:** unchanged.
- Priority is unchanged: death, then kill, then objective.

## Last game (row y 0)

Base: copy **L4a** (`3VU-0`, page `p-6-0`) and apply everything below. The hero image is `hero-1.jpg` and the clip images are `clip-1` to `clip-4`.

The **All clips button** is a kit Neutral button at Large size: "All clips", the count "12" in muted text, and the kit `arrow-right` icon at 14 px. It sits on the right of the Highlights header.

The **timeline mode toggle** is a kit **icon-only segmented control**, the smallest size (Segmented Control › "Icon-only segments"), with 2 segments:

- Compact: a single-row glyph. Use the kit `layout-bottom` icon or a 3-box row.
- Detailed: a 3-lane glyph. Use the kit `bullet-list` or `checklist` icon, or 3 stacked rows.

Its tooltip names each segment. It sits right-aligned on the row just above the timeline, inside the hero, on the same baseline as "8 / 3 / 11 KDA".

### L5a · Compact with toggle (x 0)

- L4a, plus the revised boxes, the All clips button and the toggle set to Compact.

### L5b · Detailed with toggle (x 1400)

- The same screen with the toggle set to Detailed.
- The hero shrinks to 340 px.
- The detailed lanes (You, Objectives, Clips) sit below the hero, as in L4c, using the revised boxes.
- Overlay clips sit below at 132 px tall.

### L5c · Toggle in Settings (x 2800)

- A 760 × 360 frame named `L5c / Settings row`.
- It shows how the setting looks in the Settings sheet:
  - a **Review** section header;
  - a grouped-rows card (Polish) with 3 rows:
    1. "Timeline" with the description "How games show their events", and on the right the kit segmented control **Compact | Detailed** (text labels).
    2. "Show objective icons" with a kit Mini switch, on.
    3. "Jump before an event" with a kit Select "5 s".
- On the Last game screen, this option has **no toggle**: L5a without its toggle is that state. Do not build another screen for it.

## Recorder (row y 900)

Base: **H1 Calm Home** as in R1 (`3UR-0`, page `p-6-0`), with real images (`hero-2`, `clip-1`, `clip-2`). The state goes on a **graphic, never on words** like "Recording on" or "Record games" in the always-visible part.

Recorder states and their colours:

| State | Colour | Notes |
|---|---|---|
| **Armed** (next game will be recorded) | success-fg | |
| **Recording** | danger-fg | Draw one variant's recording state too. |
| **Off** (paused) | text-faint | Use a slash or a hollow mark. |

Every recorder variant also gets a 520 × 120 frame named `<variant> / states` showing all three states side by side, with a small caption under each ("Next game will be recorded", "Recording 12:34", "Recording paused").

### T5 · Titlebar icon (x 0)

- In the titlebar, right of the navigation and before Settings: a kit Ghost icon button, 28 px.
- It holds a **record glyph**: a 16 px ring, 1.5 px stroke in `var(--color-text-muted)`, with a 6 px centre dot coloured by state. There is no text.
- Draw its popover open (as in R1):
  - "Next game will be recorded" (13 px medium);
  - a kit **segmented control** with "Record | Pause" in place of the switch;
  - F9 · Save the last 30 s;
  - storage with a bar;
  - Recording settings….

### D5a · Dock, the record button is the control (x 1400)

- The R2 dock without the switch.
- On the left, a **32 px round record button**: a raised disc in `var(--color-surface)` with an inner 12 px dot in the state colour and a soft glow ring when armed, `color-mix(state 25%, transparent)` with a 6 px spread. Clicking it toggles armed and paused.
- Beside it: "Ready" (13 px medium) over "Next game will be recorded" (11 px muted).
- Then, as in R2: a divider, Save clip F9, a divider, and the storage ring.

### D5b · Dock with an icon segmented control (x 2800)

- The dock's first item is a kit icon-only segmented control with 2 segments:
  - a record dot (filled circle) in the state colour when selected;
  - pause (two bars).
- Then "Next game will be recorded" (12 px muted), a divider, Save clip F9, a divider and storage.

### D5c · Minimal dock (x 4200)

- A smaller dock, 40 px tall, with icons only:
  - the 3D prism at 22 px with its state light, as a button that toggles;
  - a divider;
  - a scissors-style "Save clip" icon button with an F9 cap;
  - a divider;
  - the storage ring.
- Draw a kit tooltip above the prism: "Next game will be recorded · click to pause".

## Done means

- Page `Round 5` holds these frames:
  - `Last game — L5a Compact`
  - `Last game — L5b Detailed`
  - `L5c / Settings row`
  - `Home — T5 Titlebar icon`
  - `Home — D5a Record button dock`
  - `Home — D5b Segmented dock`
  - `Home — D5c Minimal dock`
  - each recorder variant's `states` frame
- Real frames and official icons are used, every colour is a token, and nothing is clipped.
