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

## Round 6b: Andres's follow-up (2026-11-27)

- **Objective icons:**
  - Use the **flat one-colour style** of the kill and death icons, not the coloured minimap set.
  - Source: Riot's scoreboard and match-history objective silhouettes (baron, dragon, elder, herald, grubs, inhibitor, turret), traced to SVG in `~/.local/share/greyscreen-design/lol-icons/match-history/svg/*.svg`.
  - Inline them, 12 px, with the `fill` attribute set to:
    - `var(--color-text)` for my team's objectives;
    - `var(--color-text-muted)` for the enemy's.
  - Every dragon uses `dragon.svg` and an elder uses `elder.svg`. Grey objective boxes are unchanged.
- **Home card actions:** "Watch" and "Open" did the same thing. The new actions:
  - Click the card's thumbnail or title to open the Last game screen.
  - On the right, two icon-only buttons:
    - a kit **Contrast** 28 px icon button with the play triangle (plays the game);
    - a Neutral 28 px icon button with the **Duotone `folder-reveal`** icon (`3RB-0` on page `p-5-0`), which replaces the flat folder.
  - Remove "Open" and the word "Watch".

## Icon rules (Andres, 2026-11-27)

- **Folder:** always the custom `folder-reveal` icon (Duotone `3RB-0`, Line `3R1-0`, page `p-5-0`). The flat kit folder is retired.
- **Settings, storage settings and any settings entry:** always the **Rune kit `settings-gear`** icon. The custom gear was deleted (Andres, 2026-11-27).
- **Objectives:** flat traced Riot silhouettes (`lol-icons/match-history/svg/`).
- **Kill:** Riot's crossed swords.
- **Death:** the skull.
- **Dock:** every item is 28 px tall inside the 48 px dock, with 10 px insets. The record button is 28 px with a 3 px glow. The storage button is a Ghost button with padding 0 8.

## Timeline boxes (reference: Icons › "Timeline boxes")

- Every box is 20 px tall: a 2 px top border plus 18 px of content. An icon never touches the border.
- **Multikill** (2 to 4):
  - The kill icon sits 1 px from the top of the content.
  - A badge circle 8 px wide, filled with the box colour, starts at y 10 and holds the number (7 px, weight 700). It knocks out only the handle tips.
- **Penta:** an inverted box, solid success-fg with dark swords, plus a soft green glow.

## File layout (Andres, 2026-11-27)

- **Official screens:** page `Home` (`p-9-0`) and page `Last game` (`p-A-0`), one artboard each. Edit them in place. Do not make round copies any more.
- **App-only components:** page `Components` (`p-B-0`), one sheet per component in the Rune Kit convention (layers named `Component / Variant / State`):
  - the recording dock;
  - the record button;
  - the storage popover;
  - the last game card;
  - the clip card;
  - game actions;
  - the timeline box.

  Rune Kit stays the shared kit. Generic parts can be promoted into it later.
- **Icons:** page `Icons`.
- **Older rounds:** archived under the bottom divider.
