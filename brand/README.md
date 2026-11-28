# Greyscreen Design System

Greyscreen is a quiet, precision Windows utility for recording and reviewing League of Legends matches. The brand is one colour: ink on white, white on ink. The app's own UI keeps its neutral slate greys, hairline borders and cobalt accent, with disciplined semantic colour reserved for match outcomes and timeline events.

---

## 1. Logo Rationale

1. **The dead face:** The mark is the `x_x` face players type in chat when they die, drawn as two X eyes and a short flat mouth.
2. **Knocked over:** The face is rotated -14° inside an upright tile. The tile stands, the face has fallen over. The mouth is crooked on purpose.
3. **One colour only:** Ink `#111111` and white `#FFFFFF`, nothing else. Dark version: ink tile, white face. Light version: white tile, ink face. Never add colour to the mark. The only red in the brand assets is the recording dot in the tray icon.
4. **Built for 16 px:** Round caps, heavy strokes (`5.5 / 64` of the tile) and a pixel-tuned 16 px cut (`app-icon-16.svg`) keep each X and the mouth separate in the taskbar, title bar and tray.
5. **Original and independent:** Pure geometric primitives, no resemblance to Riot's fist, League's "L" or gold Hextech ornament. No Riot logos anywhere.
6. **Wordmark:** `greyscreen` in lowercase, Inter Black (`900`), tracking `-0.06em`, drawn as paths so it never depends on a font.

### Logo Assets & Clearspace

| Asset | File | Usage |
| --- | --- | --- |
| Mark (dark) | `mark.svg` | Ink tile, white face (`1024×1024`, viewBox `0 0 64 64`). For light surfaces. |
| Mark (light) | `mark-light.svg` | White tile, ink face. For dark surfaces. |
| Windows App Icon | `app-icon.svg` | Dark mark with a graphite hairline inner edge (`2/64`) so the ink tile separates from a dark taskbar. |
| Windows 16 px Icon | `app-icon-16.svg` | The 16 px cut, snapped to the `16×16` grid for small taskbar and title bar slots. |
| Web Favicon | `favicon.svg` | `32×32` adaptive: dark mark on light tabs, light mark on dark tabs (`prefers-color-scheme`). |
| Windows Tray (Idle) | `tray-icon.svg` | Monochrome white tile with the face knocked out (`32×32`). |
| Windows Tray (Recording) | `tray-recording.svg` | Same glyph plus a respawn-red `#E8261B` recording dot, with a transparent gap around it. |
| Wordmark | `wordmark.svg`, `wordmark-dark.svg`, `wordmark-light.svg` | Path-drawn `greyscreen` (`currentColor`, white for dark surfaces, ink for light surfaces). |
| Horizontal Lockup | `logo-horizontal.svg`, `logo-horizontal-light.svg`, `logo-horizontal-dark.svg` | Mark + wordmark. Light lockup: dark mark, ink wordmark. Dark lockup: light mark, white wordmark. The plain file is the light lockup. |
| Social Card | `og-image.svg` | `1200×630` Open Graph card: ink ground, light mark, white wordmark, tagline. |

- **Lockup proportions:** Mark height is `1.6×` the cap height of the wordmark. The gap between mark and wordmark is `0.35×` the mark height.
- **Clearspace:** Keep at least `0.5×` the mark's height free around the mark or lockup.
- **Minimum size:** Mark standalone `16 px` (use the 16 px cut); horizontal lockup `20 px` high.
- **Never:** Add colour, gradients, glows or drop shadows to the mark. Rotate the tile (only the face is rotated). Straighten the face or the mouth. Set the wordmark in another font or in capitals. Put red anywhere except the tray recording dot.

### Brand Palette

| Name | Hex | Role |
| --- | --- | --- |
| White | `#FFFFFF` | Light tile, face on dark tile, wordmark on dark surfaces |
| Concrete | `#E4E4E4` | Secondary text on ink, hairlines on white |
| Grey | `#8C8C8C` | Muted text, disabled marks |
| Graphite | `#3B3B3B` | Hairline edge on the app icon, dark borders |
| Ink | `#111111` | Dark tile, face on light tile, wordmark on light surfaces |
| Respawn | `#E8261B` | Recording status dot in the tray icon only |

---

## 2. App UI Colour System

All neutrals share a subtle cool slate tint (OKLCH hue `265°`, chroma `0.004–0.008`) derived directly from the app's Rune neutral kit. Dark mode is the primary experience; light mode mirrors every semantic role with AA/AAA text contrast.

### Backgrounds, Surfaces & Elevations

| Token | Role | Dark (Primary) | Light |
| --- | --- | --- | --- |
| `--gs-bg` | Window / page canvas | `#121314` | `#F0F1F4` |
| `--gs-surface-1` | Primary cards, tables, panels | `#1C1D20` | `#FFFFFF` |
| `--gs-surface-2` | Secondary buttons, active nav pill, raised rows | `#24262A` | `#F5F6F8` |
| `--gs-surface-3` | Control wells, timeline track, toggle off-track | `#2D3036` | `#E6E8EE` |
| `--gs-surface-raised` | Hover cards, tooltips, dropdowns | `#222429` | `#FFFFFF` |
| `--gs-surface-overlay` | Modal dialogs, popovers | `#1F2126` | `#FFFFFF` |
| `--gs-surface-inverse` | High-contrast inverse surface | `#EFF0F2` | `#1A1C1F` |

### Borders & Hairlines

| Token | Role | Dark (Primary) | Light |
| --- | --- | --- | --- |
| `--gs-border-subtle` | Internal card dividers, table hairlines | `#232529` | `#E8EAEE` |
| `--gs-border` | Default card borders, button outlines, inputs | `#2C2E33` | `#DCDFE6` |
| `--gs-border-strong` | Active/hovered input borders, overlay edges | `#3C3F47` | `#C5C9D3` |
| `--gs-border-focus` | Keyboard focus ring | `#4392F7` | `#2F7FE3` |

### Text & Icons

| Token | Role | Dark (Primary) | Light | Contrast (on Surface 1) |
| --- | --- | --- | --- | --- |
| `--gs-text` | Primary copy, champion names, headings | `#EFF0F2` | `#1A1C1F` | `14.8:1` / `17.3:1` |
| `--gs-text-muted` | Subtitles, timestamps, file paths, metadata | `#A2A5AA` | `#606369` | `6.8:1` / `6.1:1` |
| `--gs-text-faint` | KDA slashes, inactive placeholders, subtle icons | `#72757B` | `#8B8F98` | `3.8:1` / `3.3:1` |
| `--gs-text-inverse` | Text on inverse surfaces | `#121314` | `#FFFFFF` | — |

### Single Brand Accent (Cobalt Blue)

Use cobalt blue sparingly: active switches, primary CTA on marketing pages, active video scrubber progress, active clip selection, `WIN` badges, and keyboard focus rings. Never flood large surfaces with accent colour.

| Token | Role | Dark (Primary) | Light |
| --- | --- | --- | --- |
| `--gs-accent` | Primary accent fill, switches, scrubber progress | `#4392F7` | `#2F7FE3` |
| `--gs-accent-hover` | Primary accent hover | `#5BA2FA` | `#246EC9` |
| `--gs-accent-pressed` | Primary accent pressed | `#317EE2` | `#1C5CB0` |
| `--gs-accent-soft` | Tinted accent pill/badge background | `#1B2B40` | `#E5F0FD` |
| `--gs-accent-border` | Tinted accent border | `#294872` | `#B4D3FA` |
| `--gs-accent-text` | Accent links & active clip text | `#6BB0FF` | `#1C62B6` |
| `--gs-accent-fg` | Ink on solid accent buttons | `#FFFFFF` | `#FFFFFF` |

### Status Colours

| Role | Dark BG | Dark FG / Icon | Dark Border | Light BG | Light FG / Icon | Light Border |
| --- | --- | --- | --- | --- | --- | --- |
| **Success** (Victory) | `#1B3121` | `#5DC879` | `#255232` | `#E1F6E6` | `#117A37` / `#188A42` | `#A9DDB6` |
| **Warning** (Caution) | `#362A14` | `#DFA11A` | `#5E4208` | `#FDF0D5` | `#945B00` / `#B47409` | `#EACA92` |
| **Danger** (Defeat / Recording / Error) | `#3C2322` | `#FF847D` / `#F0645E` | `#6C3531` | `#FDE7E5` | `#B3312E` / `#C93632` | `#F8BCB7` |
| **Info** (Highlight / Active) | `#1B2B40` | `#6BB0FF` | `#294872` | `#E5F0FD` | `#1C62B6` / `#2570D4` | `#B4D3FA` |
| **Recording Dot** (Tray & Pill) | — | `#F0524D` | — | — | `#D63934` | — |

### Six In-Game Event Tones

Used for timeline scrubber ticks (`8×18 px`, `2 px` radius), event list dots (`8×8 px`), and event hover cards. Each tone is calibrated so a player scanning a 35-minute replay timeline can separate deaths, kills, big moments, and team vs. enemy objectives in one glance.

| Event Tone | Events Mapped | Dark Marker | Dark Tint BG | Light Marker | Light Tint BG |
| --- | --- | --- | --- | --- | --- |
| **Big moments (Amber)** | First Blood, Multikills, Ace | `#DFA11A` | `#362A14` | `#B47409` | `#FDF0D5` |
| **Death (Red)** | Player deaths (`MyDeath`) | `#F0645E` | `#3C2322` | `#C93632` | `#FDE7E5` |
| **Kill (Blue)** | Player kills (`MyKill`), key plays | `#6BB0FF` | `#1B2B40` | `#2570D4` | `#E5F0FD` |
| **Team objective (Green)** | Allied Dragon, Herald, Baron, Voidgrubs, Turret, Inhibitor | `#5DC879` | `#1B3121` | `#188A42` | `#E1F6E6` |
| **Enemy objective (Coral)** | Enemy Dragon, Baron, Turret, Inhibitor | `#F59573` | `#3D281F` | `#C45534` | `#FBEAE4` |
| **Neutral (Grey)** | Minions spawning, ally kills, routine events | `#72757B` | `#25272C` | `#8A8E96` | `#ECEDF1` |

---

## 3. Typography

### Recommended Free Webfonts

- **UI & Marketing Sans:** **Inter** (SIL OFL — Google Fonts / self-hosted variable WOFF2). Crisp tall x-height, neutral neo-grotesque proportions, native optical sizing (`opsz`), and rock-solid tabular numerals (`tnum`). Fallback: `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`.
- **Monospace (Clocks & Live Feed):** **JetBrains Mono** (SIL OFL). Clean zero/one differentiation for elapsed clocks (`12:34`), live-feed timestamps (`09:47`), and technical diagnostics. Fallback: `ui-monospace, "SF Mono", "Cascadia Mono", Consolas, monospace`.

### Type Scale

| Role | Token | Size | Line Height | Weight | Tracking | Usage |
| --- | --- | --- | --- | --- | --- | --- |
| **Display / Clock** | `--gs-text-display-*` | `40px` | `44px` | `400` (mono) / `600` (web hero) | `-0.025em` | Live elapsed timer (`12:34`), website hero headline (`44–52px` on desktop) |
| **Page Title** | `--gs-text-title-*` | `22px` | `28px` | `500` | `-0.015em` | Screen titles (`Recordings`, `Settings`, `Waiting for a game`, champion name) |
| **Heading** | `--gs-text-heading-*` | `16px` | `22px` | `500` | `-0.01em` | Website feature titles, modal headings |
| **Body Medium** | `--gs-text-body-*` | `14px` | `21px` | `500` | `0` | Card section headers (`Live feed`, `Last game`), setting labels, champion row names |
| **Body** | `--gs-text-body-*` | `14px` | `21px` | `400` | `0` | Page subtitles, event descriptions, empty-state copy |
| **UI Medium** | `--gs-text-ui-*` | `13px` | `20px` | `500` | `0` | Buttons, nav tabs, segmented controls, status pills, settings group headers |
| **UI** | `--gs-text-ui-*` | `13px` | `20px` | `400` | `0` | Table values (`Victory`, `7 / 3 / 11`, `33:12`, `1.8 GB`), recordings summary |
| **Caption / Mono** | `--gs-text-caption-*` | `12px` | `18px` | `400` | `0` | Live feed timestamps, clip time ranges, tooltips |
| **Micro** | `--gs-text-micro-*` | `11px` | `15px` | `400 / 500` | `+0.01em` | Table column headers, setting helper lines, uppercase feed badges (`MY KILL`) |

### Tabular Figures Rule

**Every numeric readout that updates, aligns in columns, or compares quantities MUST use tabular figures** (`font-variant-numeric: tabular-nums; font-feature-settings: "tnum" 1;`):

- Elapsed recording clock (`12:34`) and review scrubber clock (`09:47 / 33:12`)
- Recordings table columns: KDA (`7 / 3 / 11`), date/time (`Yesterday 16:55`), duration (`33:12`), clip count (`6`), file size (`1.8 GB`)
- Summary strip (`8 games · 4W 3L · 3h 51m recorded · 12.4 GB`)
- Event timestamps, clip ranges (`00:01 - 00:06`), FPS options (`30`, `60`), and disk-space estimates (`~55 MB/min`).

---

## 4. Spacing, Radii & Layout

### Spacing Scale (4 px Base Grid)

| Token | Value | Primary Usage |
| --- | --- | --- |
| `--gs-space-1` | `4px` | Nav tab gap, tight badge gap, clip icon-to-count |
| `--gs-space-2` | `8px` | Section label to card, pill icon-to-label, hover card anchor gap |
| `--gs-space-3` | `12px` | Table column gap, review header gap, inline control clusters |
| `--gs-space-4` | `16px` | Page vertical section gap, card internal horizontal padding, setting row gap |
| `--gs-space-5` | `20px` | Status card padding, settings group vertical spacing, navbar horizontal padding |
| `--gs-space-6` | `24px` | App page outer gutter (`px: 24px`, `pb: 24px`) |
| `--gs-space-8` | `32px` | Website card padding, generous vertical rhythm |
| `--gs-space-12` | `48px` | Website section padding (compact) |
| `--gs-space-16` | `64px` | Website major section separation |

- **App container:** Centered column, `max-width: 880px`, top bar `52px` height with bottom `1px` hairline.
- **Standard row heights:** Recordings table row `64px`, Settings row min `56px`, Card header `44px`, Table header `34px`, Live feed row `36px`, Review event/clip row `34px`.

### Border Radii

| Token | Value | Usage |
| --- | --- | --- |
| `--gs-radius-xs` | `2px` | Timeline event markers (`8×18 px`), small accent bars |
| `--gs-radius-sm` | `4px` | Event/result badges (`WIN`, `CLASSIC`, `MY KILL`), keyboard shortcut tags |
| `--gs-radius-md` | `6px` | Buttons, nav tabs, text inputs, selects, active segment pill, hover cards |
| `--gs-radius-lg` | `8px` | Video player frame, champion avatars (`40×40 px`), segmented control track |
| `--gs-radius-xl` | `10px` | Primary cards (`state-card`, `recordings-card`, `settings` groups, `review-panel`) |
| `--gs-radius-2xl` | `12px` | Empty-state icon well (`48×48 px`), website feature frames |
| `--gs-radius-pill` | `9999px` | Status pills (`Recording`, `Idle`), switches, event dots, recording indicator |

---

## 5. Shadows & Elevation

Greyscreen relies on **surface luminance steps and crisp 1 px hairlines** rather than heavy drop shadows. Shadows appear only when a surface lifts above the page plane.

- **Level 0 (Canvas):** `--gs-bg` (`#121314` / `#F0F1F4`). No shadow.
- **Level 1 (Card):** `--gs-surface-1` + `1px solid var(--gs-border)`. Flat in dark mode; crisp hairline border defines structure.
- **Level 2 (Raised / Interactive):** `--gs-shadow-raised` — used on buttons, switches, and segmented active pills (`0 1px 2px` contact shadow + hairline ring).
- **Level 3 (Floating):** `--gs-shadow-floating` — event hover cards, select dropdowns, tooltips.
- **Level 4 (Overlay):** `--gs-shadow-overlay` — toasts, dialogs, website framed app screenshots.

---

## 6. Motion

Motion mirrors Rune's snappy, zero-bounce spring feel: fast, direct, and never theatrical.

| Token | Value | Usage |
| --- | --- | --- |
| `--gs-duration-instant` | `80ms` | Row hover fill, button press feedback, timeline marker hover |
| `--gs-duration-fast` | `120ms` | Switch knob glide, segmented pill slide, badge state changes |
| `--gs-duration-base` | `160ms` | Hover card / tooltip fade-in (after `150ms` hover intent delay), select menu open |
| `--gs-duration-slow` | `220ms` | Toast entrance, accordion/disclosure height transitions |
| `--gs-ease-out` | `cubic-bezier(0.16, 1, 0.3, 1)` | Entering overlays, toasts, panels |
| `--gs-ease-snappy` | `cubic-bezier(0.2, 0, 0, 1)` | Switches, segmented controls, hover fills |

- Always honor `@media (prefers-reduced-motion: reduce)` by collapsing durations to `0ms`.
- Never pulse, glow, or bounce UI chrome. Even the red recording dot stays steady — calm confidence beats anxious blinking.

---

## 7. Iconography Rules

- **Grid:** `16×16` viewBox for UI icons (`13px` inline metadata like clip counts, `16px` standard buttons/controls, `20px` navbar logo mark, `22px` empty-state wells).
- **Stroke:** `1.5px` consistent stroke weight, `stroke-linecap="round"`, `stroke-linejoin="round"`. Simple geometric primitives (rectangles, triangles, straight lines).
- **Colour:** Monochrome `currentColor`, inheriting `--gs-icon` or `--gs-icon-muted`. Never use multi-colour icons inside UI chrome — colour belongs strictly to champion portraits, win/loss edge rails, and timeline event markers.

---

## 8. Components

### Buttons

Three restrained variants. Default height `32px` (small `26px`, website hero `40px`), radius `6px`, horizontal padding `12–14px`, font `13px / 500`.

- **Primary (Accent):** Solid `--gs-accent` background, `#FFFFFF` text. Reserve for the single main action on a website hero (`Download for Windows`) or a primary confirmation dialog. Inside the app, keep controls quiet—prefer Secondary.
- **Secondary (Neutral — App Default):** `--gs-surface-2` background (`#24262A` dark / `#FFFFFF` light), `1px solid var(--gs-border)`, `--gs-text`. Hover shifts to `--gs-surface-3` / `--gs-border-strong`. Used for `Open folder`, `Stop`, `Save clip (last 30 s)`, `Back`, `Change`.
- **Ghost:** Transparent background, `--gs-text-muted` (hover `--gs-text` on `--gs-hover`). Used for inactive nav tabs (`Status`, `Recordings`, `Settings`), icon buttons (`Theater`, `Show in folder`), and low-priority actions (`Open`).

### Tinted Status Pills & Event Badges

- **Status Pills** (`height: 24px`, `radius: 9999px`, `px: 10px`, `gap: 6px`, `13px / 500`): Leading `8×8 px` solid status dot + tinted translucent/low-chroma background + matching legible foreground.
  - Recording: `#3C2322` fill, `#F0645E` dot, `#FF847D` text.
  - Idle / Off / Waiting: `#24262A` fill, `#72757B` dot, `#A2A5AA` text.
  - Saved / Active: `#1B3121` fill, `#5DC879` dot, `#5DC879` text.
- **Event & Result Badges** (`height: 20px`, `radius: 4px`, `px: 6px`, `11px / 600`, uppercase):
  - Accent (`WIN`, `MY KILL`, `MULTIKILL 2`): `--gs-accent-soft` fill + `--gs-accent-text` (or solid `--gs-accent` + `#121314` / `#FFFFFF` when high emphasis is needed in the live feed).
  - Neutral (`CLASSIC`, `BARON`, `HERALD`, `EVENT`): `--gs-surface-2` fill + `--gs-text-muted`.
  - Subtle Danger (`MY DEATH`, `LOSS`): `--gs-danger-bg` fill + `--gs-danger-fg` (or muted dark pill in dense feeds).

### Cards

- Fill `--gs-surface-1`, `1px solid var(--gs-border)`, `radius: 10px`.
- Card headers (`44px` height, `px: 16px`) separated from body content by a full-bleed `1px` `--gs-border-subtle` hairline.
- Grouped rows (Settings) use internal `1px` hairlines between rows—never nest cards inside cards.

### Inputs, Selects & Switches

- **Text Input:** `height: 32px`, `radius: 6px`, `--gs-bg` or `--gs-surface-2` fill, `1px solid var(--gs-border)`, tabular figures for numeric fields (`Seconds before: 15`). Invalid state swaps border to `--gs-danger-icon`. Focus ring uses `--gs-shadow-focus`.
- **Select Trigger:** Matches Secondary button geometry with trailing `16px` chevron (`--gs-icon-muted`), opening a floating `--gs-surface-raised` menu (`--gs-shadow-floating`, `radius: 8px`).
- **Switch:** `36×20 px` pill track. Off: `--gs-control-off`; On: `--gs-accent`. `16×16 px` crisp white `--gs-knob` sliding `16px` with `120ms` `--gs-ease-snappy`.

### Segmented Controls

- Outer recessed track: `--gs-control-track` (`#17181B` / `#E8EAEF`), `1px solid var(--gs-border-subtle)`, `radius: 8px`, `padding: 2px`.
- Active segment: `--gs-surface-2` (`#26282E` dark / `#FFFFFF` light), `radius: 6px`, `--gs-shadow-button`, `--gs-text` (`500` weight). Inactive segments use `--gs-text-muted` and hover subtly. Used for `Balanced | High | Max` and `30 | 60`.

### Tables (Recordings List)

- Contained inside a clipped `10px` card (`--gs-surface-1`).
- Column header row (`34px`, `--gs-text-muted`, `11px / 500`) separated by a `1px` hairline.
- Rows (`64px` height) lead with a **`3×44 px` rounded outcome rail** (`#5DC879` Victory, `#F0645E` Defeat, `#3C3F47` Unknown), followed by a `40×40 px` rounded (`8px`) champion portrait, left-aligned champion/mode stack, and tabular right-aligned KDA, duration, clips, and size.
- Secondary row actions (`Show in folder`) stay hidden at `opacity: 0` until row hover.

### Tooltips & Event Hover Cards

- Appear on `150ms` hover intent over timeline markers or event rows (`8px` anchor gap).
- Surface `--gs-surface-raised`, `radius: 6px`, `--gs-shadow-floating`, `padding: 10px 12px`, `min-width: 200px`, `max-width: 360px`.
- Top row: bold event title left + tabular timestamp (`--gs-text-muted`) right. Body rows: fixed-width muted lead label (`56px`) + primary detail spans.

---

## 9. Website Screenshot & Imagery Rules

- **Real product, zero fabrication:** Capture the real Windows app on a clean `1280×800` or `1120×720` window at `2×` DPI with realistic League match data (varied champions, clean KDA, genuine timeline markers).
- **Framing:** Display screenshots with `10–12px` rounded corners, a `1px solid rgba(255, 255, 255, 0.10)` hairline frame, and `--gs-shadow-overlay`. Never tilt screenshots at 3D perspectives or surround them with neon glows or purple gradients.
- **Backdrop:** Place dark-theme screenshots on `--gs-bg` (`#121314`) or subtle `--gs-surface-1` bands, optionally accented with a single ultra-subtle radial cobalt haze (`opacity <= 0.08`).
- **Focused crops:** For feature callouts (e.g., timeline scrubbing, auto-clip settings), crop tightly around the relevant card + `24px` breathing room so text remains legible at `1:1` scale without squinting.
- **No League IP misuse:** Champion portraits appear only inside authentic app UI screenshots—never extract Riot champion splash art as decorative website backgrounds or hero banners.

---

## 10. Voice & Tone

Greyscreen sounds like a calm, capable tool built by someone who actually plays League and respects your disk space, CPU, and attention.

### Principles

- **Short & literal:** Say what happens in plain words. One sentence per helper line.
- **Factual over hype:** Give real numbers (`~55 MB/min`, `60 fps`, `0% CPU encoding`), not adjectives (`blazing fast`, `next-gen`).
- **Second person, active voice:** "Start when a League game starts, stop when it ends."
- **Zero gamer clichés:** No "dominate the Rift", "cracked plays", "level up your grind", "esports-grade", or exclamation marks in UI copy.

### Do / Don't

| Context | Do | Don't |
| --- | --- | --- |
| **Hero tagline** | Automatic League of Legends recorder and review timeline. | Dominate the Rift and capture every insane play! |
| **Feature subcopy** | Records with your GPU encoder. Saves clips around kills, deaths, and objectives. | Zero-lag AI-powered esports highlight engine built for true gamers. |
| **Idle status** | Waiting for a game. Greyscreen starts recording when a game starts. | Ready to slay! Jump into Summoner's Rift to begin! |
| **Setting helper** | 60 is smoother, 30 uses less disk space. | Crank it to 60 FPS for buttery-smooth pro gameplay! |
| **Empty state** | No recordings yet. Play a game, or run a test recording from Status. | Looks empty in here! Time to queue up and get some pentakills! |
| **Error message** | Encoder unavailable. Check the logs folder in Settings for details. | Oops! Something went wrong while recording your epic match :( |
| **Download CTA** | Download for Windows | GET GREYSCREEN FREE NOW! |
