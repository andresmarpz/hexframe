# Round 6: chosen direction, polished

These are Andres's picks after round 5 (2026-11-27):

- **Last game:** L5a.
- **Timeline mode:** a setting in Settings (L5c), not a toggle on the screen.
- **Recorder:** the dock with a record button (D5a).

Build on page **Round 6** (`p-8-0`). The artboards are copies of L5a and D5a, so edit them in place.

## L6 · Last game (`72F-0`)

He liked it a lot but found it cluttered ("five buttons is too much").

- **Remove the Watch button.** The centre play button already plays the game.
- **Remove the timeline mode toggle.** It now lives in Settings.
- **Actions:**
  - What stays: Neutral Large "Review" with its D key cap, and the Neutral 28 px folder icon button. The gap between them is 8.
  - Place them at the hero's bottom right, on the row just above the timeline, where the toggle was. Align their right edge with the timeline's right edge, and bottom-align them with the "8 / 3 / 11 KDA" line.
- Nothing else changes.

## H6 · Home with the dock (`79Y-0`)

Dock geometry: the dock is 48 tall with a 24 radius.

- **Record button** (32 px): it sits **concentric with the dock's rounded left end**, with an 8 px inset on top, bottom and left. Set the dock's left padding to 8. Its glow ring stays inside the dock.
- **Spacing:**
  - 12 px gap between items.
  - Dividers have 4 px of extra air on each side.
  - Right padding 8.
- **Storage** becomes a **kit Ghost button**: 32 px tall, padding 0 10, radius `var(--radius-md)`, a 16 px storage ring and "112 GB". Its rounded right end sits concentric with the dock's right end (an 8 px inset).
- **Storage popover**: draw it **open**, anchored above the storage button and right-aligned to the dock. It is a Polish raised card, 300 wide, padding 14 16. The storage button shows its hover fill (`var(--color-hover)` or `var(--color-layer-2)`) to read as pressed. Contents:
  - The title "Storage" (13 px, weight 530), with the muted "Videos\Greyscreen" on the right.
  - A 40 px ring at 45%, with "112 GB of 250 GB" (15 px medium) and "Storage limit · oldest videos deleted first" (12 px muted).
  - A breakdown of 3 rows, 24 px each. Each row has a label, a value and a 4 px bar beneath it, scaled to the limit:
    - Game videos, 104.0 GB (`var(--color-accent)`);
    - Clips, 6.2 GB (`var(--color-info-fg)`);
    - Thumbnails & timelines, 1.8 GB (`var(--color-text-muted)`).
  - "Free on drive C: 418 GB", 12 px muted.
  - A hairline.
  - Two Ghost rows: "Storage settings…" (with a settings-gear icon) and "Open recordings folder" (with the folder icon).

## Done means

- Both artboards are edited in place, nothing is clipped, and every colour is a token.
- finish_working_on_nodes is called.
