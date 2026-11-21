# Greyscreen Design System

Greyscreen is a quiet, precision Windows utility for recording and reviewing League of Legends matches. Its visual language comes directly from the native app: cool neutral slate greys, hairline borders, a single cobalt-blue accent, crisp tabular typography, and disciplined semantic colour reserved exclusively for match outcomes and timeline events.

---

## 1. Logo Rationale

1. **Hex + Frame, refined:** The mark evolves Greyscreen's point-top hexagon and play triangle into a concentric machined frame with smooth tangent-arc outer and inner corners.
2. **Exact isometric lock:** The equilateral play prism shares the hexagon's exact 30° / 60° / 90° axes and sits centered so its clearance to all three facing inner walls is mathematically identical.
3. **App-native palette, zero gamer clichés:** Replacing the old neon-purple squircle with a dark graphite tile (`#181A1F`), cobalt-blue frame (`#4392F7`), and crisp off-white prism (`#F4F6FA`) ties the desktop icon directly to the app's dark card-and-hairline UI.
4. **Built for 16 px:** Heavy wall proportions (~10% of the canvas), bold inner contrast, and a pixel-tuned 16 px variant keep the silhouette razor-sharp in the Windows taskbar, title bar, and system tray.
5. **Original & independent:** Pure geometric primitives avoid any resemblance to Riot's fist, League's "L", or gold Hextech ornamentation while pairing with a custom neo-grotesque wordmark matched to the hex wall's stroke weight.

### Logo Assets & Clearspace

| Asset | File | Usage |
| --- | --- | --- |
| Brand Mark | `mark.svg` | Standalone cobalt `#4392F7` mark (`1024×1024`), legible on both dark and light surfaces. |
| Windows App Icon | `app-icon.svg` | Full-resolution icon (`1024×1024`) on a dark graphite tile with hairline bevel. |
| Windows 16 px Icon | `app-icon-16.svg` | Simplified, bolder proportions snapped to the `16×16` grid for small taskbar/titlebar slots. |
| Web Favicon | `favicon.svg` | `32×32` adaptive icon supporting `prefers-color-scheme: light` and `dark`. |
| Windows Tray (Idle) | `tray-icon.svg` | Monochrome `#FFFFFF` glyph (`32×32`) for the Windows notification area. |
| Windows Tray (Recording) | `tray-recording.svg` | Monochrome `#FFFFFF` glyph with a masked cutout and red `#F0524D` recording dot. |
| Wordmark | `wordmark.svg`, `wordmark-dark.svg`, `wordmark-light.svg` | Path-drawn `"Greyscreen"` logotype (`currentColor`, dark-surface `#EFF0F2`, light-surface `#1A1C1F`). |
| Horizontal Lockup | `logo-horizontal.svg`, `logo-horizontal-dark.svg`, `logo-horizontal-light.svg` | Mark + `"Greyscreen"` lockup with cobalt frame and ink-matched play prism. |
| Social Card | `og-image.svg` | `1200×630` Open Graph card featuring the dark lockup, tagline, and vector timeline preview. |

- **Clearspace:** Maintain at least `0.5×` the mark's height around the mark or horizontal lockup.
- **Minimum size:** Mark standalone `16 px`; horizontal lockup `20 px` height (`~98 px` wide).
- **Never:** Apply neon outer glows, RGB gradients, drop shadows behind the mark on light backgrounds, or stretch the hexagon out of its regular 1:1.155 aspect ratio.

---

## 2. Colour System

All neutrals share a subtle cool slate tint (OKLCH hue `265°`, chroma `0.004–0.008`) derived directly from the app's Rune neutral kit. Dark mode is the primary experience; light mode mirrors every semantic role with AA/AAA text contrast.

### Backgrounds, Surfaces & Elevations

| Token | Role | Dark (Primary) | Light |
| --- | --- | --- | --- |
| `--hf-bg` | Window / page canvas | `#121314` | `#F0F1F4` |
| `--hf-surface-1` | Primary cards, tables, panels | `#1C1D20` | `#FFFFFF` |
| `--hf-surface-2` | Secondary buttons, active nav pill, raised rows | `#24262A` | `#F5F6F8` |
| `--hf-surface-3` | Control wells, timeline track, toggle off-track | `#2D3036` | `#E6E8EE` |
| `--hf-surface-raised` | Hover cards, tooltips, dropdowns | `#222429` | `#FFFFFF` |
| `--hf-surface-overlay` | Modal dialogs, popovers | `#1F2126` | `#FFFFFF` |
| `--hf-surface-inverse` | High-contrast inverse surface | `#EFF0F2` | `#1A1C1F` |

### Borders & Hairlines

| Token | Role | Dark (Primary) | Light |
| --- | --- | --- | --- |
| `--hf-border-subtle` | Internal card dividers, table hairlines | `#232529` | `#E8EAEE` |
| `--hf-border` | Default card borders, button outlines, inputs | `#2C2E33` | `#DCDFE6` |
| `--hf-border-strong` | Active/hovered input borders, overlay edges | `#3C3F47` | `#C5C9D3` |
| `--hf-border-focus` | Keyboard focus ring | `#4392F7` | `#2F7FE3` |

### Text & Icons

| Token | Role | Dark (Primary) | Light | Contrast (on Surface 1) |
| --- | --- | --- | --- | --- |
| `--hf-text` | Primary copy, champion names, headings | `#EFF0F2` | `#1A1C1F` | `14.8:1` / `17.3:1` |
| `--hf-text-muted` | Subtitles, timestamps, file paths, metadata | `#A2A5AA` | `#606369` | `6.8:1` / `6.1:1` |
| `--hf-text-faint` | KDA slashes, inactive placeholders, subtle icons | `#72757B` | `#8B8F98` | `3.8:1` / `3.3:1` |
| `--hf-text-inverse` | Text on inverse surfaces | `#121314` | `#FFFFFF` | — |

### Single Brand Accent (Cobalt Blue)

Use cobalt blue sparingly: active switches, primary CTA on marketing pages, active video scrubber progress, active clip selection, `WIN` badges, and keyboard focus rings. Never flood large surfaces with accent colour.

| Token | Role | Dark (Primary) | Light |
| --- | --- | --- | --- |
| `--hf-accent` | Primary accent fill, switches, scrubber progress | `#4392F7` | `#2F7FE3` |
| `--hf-accent-hover` | Primary accent hover | `#5BA2FA` | `#246EC9` |
| `--hf-accent-pressed` | Primary accent pressed | `#317EE2` | `#1C5CB0` |
| `--hf-accent-soft` | Tinted accent pill/badge background | `#1B2B40` | `#E5F0FD` |
| `--hf-accent-border` | Tinted accent border | `#294872` | `#B4D3FA` |
| `--hf-accent-text` | Accent links & active clip text | `#6BB0FF` | `#1C62B6` |
| `--hf-accent-fg` | Ink on solid accent buttons | `#FFFFFF` | `#FFFFFF` |

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
| **Display / Clock** | `--hf-text-display-*` | `40px` | `44px` | `400` (mono) / `600` (web hero) | `-0.025em` | Live elapsed timer (`12:34`), website hero headline (`44–52px` on desktop) |
| **Page Title** | `--hf-text-title-*` | `22px` | `28px` | `500` | `-0.015em` | Screen titles (`Recordings`, `Settings`, `Waiting for a game`, champion name) |
| **Heading** | `--hf-text-heading-*` | `16px` | `22px` | `500` | `-0.01em` | Website feature titles, modal headings |
| **Body Medium** | `--hf-text-body-*` | `14px` | `21px` | `500` | `0` | Card section headers (`Live feed`, `Last game`), setting labels, champion row names |
| **Body** | `--hf-text-body-*` | `14px` | `21px` | `400` | `0` | Page subtitles, event descriptions, empty-state copy |
| **UI Medium** | `--hf-text-ui-*` | `13px` | `20px` | `500` | `0` | Buttons, nav tabs, segmented controls, status pills, settings group headers |
| **UI** | `--hf-text-ui-*` | `13px` | `20px` | `400` | `0` | Table values (`Victory`, `7 / 3 / 11`, `33:12`, `1.8 GB`), recordings summary |
| **Caption / Mono** | `--hf-text-caption-*` | `12px` | `18px` | `400` | `0` | Live feed timestamps, clip time ranges, tooltips |
| **Micro** | `--hf-text-micro-*` | `11px` | `15px` | `400 / 500` | `+0.01em` | Table column headers, setting helper lines, uppercase feed badges (`MY KILL`) |

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
| `--hf-space-1` | `4px` | Nav tab gap, tight badge gap, clip icon-to-count |
| `--hf-space-2` | `8px` | Section label to card, pill icon-to-label, hover card anchor gap |
| `--hf-space-3` | `12px` | Table column gap, review header gap, inline control clusters |
| `--hf-space-4` | `16px` | Page vertical section gap, card internal horizontal padding, setting row gap |
| `--hf-space-5` | `20px` | Status card padding, settings group vertical spacing, navbar horizontal padding |
| `--hf-space-6` | `24px` | App page outer gutter (`px: 24px`, `pb: 24px`) |
| `--hf-space-8` | `32px` | Website card padding, generous vertical rhythm |
| `--hf-space-12` | `48px` | Website section padding (compact) |
| `--hf-space-16` | `64px` | Website major section separation |

- **App container:** Centered column, `max-width: 880px`, top bar `52px` height with bottom `1px` hairline.
- **Standard row heights:** Recordings table row `64px`, Settings row min `56px`, Card header `44px`, Table header `34px`, Live feed row `36px`, Review event/clip row `34px`.

### Border Radii

| Token | Value | Usage |
| --- | --- | --- |
| `--hf-radius-xs` | `2px` | Timeline event markers (`8×18 px`), small accent bars |
| `--hf-radius-sm` | `4px` | Event/result badges (`WIN`, `CLASSIC`, `MY KILL`), keyboard shortcut tags |
| `--hf-radius-md` | `6px` | Buttons, nav tabs, text inputs, selects, active segment pill, hover cards |
| `--hf-radius-lg` | `8px` | Video player frame, champion avatars (`40×40 px`), segmented control track |
| `--hf-radius-xl` | `10px` | Primary cards (`state-card`, `recordings-card`, `settings` groups, `review-panel`) |
| `--hf-radius-2xl` | `12px` | Empty-state icon well (`48×48 px`), website feature frames |
| `--hf-radius-pill` | `9999px` | Status pills (`Recording`, `Idle`), switches, event dots, recording indicator |

---

## 5. Shadows & Elevation

Greyscreen relies on **surface luminance steps and crisp 1 px hairlines** rather than heavy drop shadows. Shadows appear only when a surface lifts above the page plane.

- **Level 0 (Canvas):** `--hf-bg` (`#121314` / `#F0F1F4`). No shadow.
- **Level 1 (Card):** `--hf-surface-1` + `1px solid var(--hf-border)`. Flat in dark mode; crisp hairline border defines structure.
- **Level 2 (Raised / Interactive):** `--hf-shadow-raised` — used on buttons, switches, and segmented active pills (`0 1px 2px` contact shadow + hairline ring).
- **Level 3 (Floating):** `--hf-shadow-floating` — event hover cards, select dropdowns, tooltips.
- **Level 4 (Overlay):** `--hf-shadow-overlay` — toasts, dialogs, website framed app screenshots.

---

## 6. Motion

Motion mirrors Rune's snappy, zero-bounce spring feel: fast, direct, and never theatrical.

| Token | Value | Usage |
| --- | --- | --- |
| `--hf-duration-instant` | `80ms` | Row hover fill, button press feedback, timeline marker hover |
| `--hf-duration-fast` | `120ms` | Switch knob glide, segmented pill slide, badge state changes |
| `--hf-duration-base` | `160ms` | Hover card / tooltip fade-in (after `150ms` hover intent delay), select menu open |
| `--hf-duration-slow` | `220ms` | Toast entrance, accordion/disclosure height transitions |
| `--hf-ease-out` | `cubic-bezier(0.16, 1, 0.3, 1)` | Entering overlays, toasts, panels |
| `--hf-ease-snappy` | `cubic-bezier(0.2, 0, 0, 1)` | Switches, segmented controls, hover fills |

- Always honor `@media (prefers-reduced-motion: reduce)` by collapsing durations to `0ms`.
- Never pulse, glow, or bounce UI chrome. Even the red recording dot stays steady — calm confidence beats anxious blinking.

---

## 7. Iconography Rules

- **Grid:** `16×16` viewBox for UI icons (`13px` inline metadata like clip counts, `16px` standard buttons/controls, `20px` navbar logo mark, `22px` empty-state wells).
- **Stroke:** `1.5px` consistent stroke weight, `stroke-linecap="round"`, `stroke-linejoin="round"`. Simple geometric primitives (rectangles, triangles, straight lines).
- **Colour:** Monochrome `currentColor`, inheriting `--hf-icon` or `--hf-icon-muted`. Never use multi-colour icons inside UI chrome — colour belongs strictly to the brand mark, champion portraits, win/loss edge rails, and timeline event markers.

---

## 8. Components

### Buttons

Three restrained variants. Default height `32px` (small `26px`, website hero `40px`), radius `6px`, horizontal padding `12–14px`, font `13px / 500`.

- **Primary (Accent):** Solid `--hf-accent` background, `#FFFFFF` text. Reserve for the single main action on a website hero (`Download for Windows`) or a primary confirmation dialog. Inside the app, keep controls quiet—prefer Secondary.
- **Secondary (Neutral — App Default):** `--hf-surface-2` background (`#24262A` dark / `#FFFFFF` light), `1px solid var(--hf-border)`, `--hf-text`. Hover shifts to `--hf-surface-3` / `--hf-border-strong`. Used for `Open folder`, `Stop`, `Save clip (last 30 s)`, `Back`, `Change`.
- **Ghost:** Transparent background, `--hf-text-muted` (hover `--hf-text` on `--hf-hover`). Used for inactive nav tabs (`Status`, `Recordings`, `Settings`), icon buttons (`Theater`, `Show in folder`), and low-priority actions (`Open`).

### Tinted Status Pills & Event Badges

- **Status Pills** (`height: 24px`, `radius: 9999px`, `px: 10px`, `gap: 6px`, `13px / 500`): Leading `8×8 px` solid status dot + tinted translucent/low-chroma background + matching legible foreground.
  - Recording: `#3C2322` fill, `#F0645E` dot, `#FF847D` text.
  - Idle / Off / Waiting: `#24262A` fill, `#72757B` dot, `#A2A5AA` text.
  - Saved / Active: `#1B3121` fill, `#5DC879` dot, `#5DC879` text.
- **Event & Result Badges** (`height: 20px`, `radius: 4px`, `px: 6px`, `11px / 600`, uppercase):
  - Accent (`WIN`, `MY KILL`, `MULTIKILL 2`): `--hf-accent-soft` fill + `--hf-accent-text` (or solid `--hf-accent` + `#121314` / `#FFFFFF` when high emphasis is needed in the live feed).
  - Neutral (`CLASSIC`, `BARON`, `HERALD`, `EVENT`): `--hf-surface-2` fill + `--hf-text-muted`.
  - Subtle Danger (`MY DEATH`, `LOSS`): `--hf-danger-bg` fill + `--hf-danger-fg` (or muted dark pill in dense feeds).

### Cards

- Fill `--hf-surface-1`, `1px solid var(--hf-border)`, `radius: 10px`.
- Card headers (`44px` height, `px: 16px`) separated from body content by a full-bleed `1px` `--hf-border-subtle` hairline.
- Grouped rows (Settings) use internal `1px` hairlines between rows—never nest cards inside cards.

### Inputs, Selects & Switches

- **Text Input:** `height: 32px`, `radius: 6px`, `--hf-bg` or `--hf-surface-2` fill, `1px solid var(--hf-border)`, tabular figures for numeric fields (`Seconds before: 15`). Invalid state swaps border to `--hf-danger-icon`. Focus ring uses `--hf-shadow-focus`.
- **Select Trigger:** Matches Secondary button geometry with trailing `16px` chevron (`--hf-icon-muted`), opening a floating `--hf-surface-raised` menu (`--hf-shadow-floating`, `radius: 8px`).
- **Switch:** `36×20 px` pill track. Off: `--hf-control-off`; On: `--hf-accent`. `16×16 px` crisp white `--hf-knob` sliding `16px` with `120ms` `--hf-ease-snappy`.

### Segmented Controls

- Outer recessed track: `--hf-control-track` (`#17181B` / `#E8EAEF`), `1px solid var(--hf-border-subtle)`, `radius: 8px`, `padding: 2px`.
- Active segment: `--hf-surface-2` (`#26282E` dark / `#FFFFFF` light), `radius: 6px`, `--hf-shadow-button`, `--hf-text` (`500` weight). Inactive segments use `--hf-text-muted` and hover subtly. Used for `Balanced | High | Max` and `30 | 60`.

### Tables (Recordings List)

- Contained inside a clipped `10px` card (`--hf-surface-1`).
- Column header row (`34px`, `--hf-text-muted`, `11px / 500`) separated by a `1px` hairline.
- Rows (`64px` height) lead with a **`3×44 px` rounded outcome rail** (`#5DC879` Victory, `#F0645E` Defeat, `#3C3F47` Unknown), followed by a `40×40 px` rounded (`8px`) champion portrait, left-aligned champion/mode stack, and tabular right-aligned KDA, duration, clips, and size.
- Secondary row actions (`Show in folder`) stay hidden at `opacity: 0` until row hover.

### Tooltips & Event Hover Cards

- Appear on `150ms` hover intent over timeline markers or event rows (`8px` anchor gap).
- Surface `--hf-surface-raised`, `radius: 6px`, `--hf-shadow-floating`, `padding: 10px 12px`, `min-width: 200px`, `max-width: 360px`.
- Top row: bold event title left + tabular timestamp (`--hf-text-muted`) right. Body rows: fixed-width muted lead label (`56px`) + primary detail spans.

---

## 9. Website Screenshot & Imagery Rules

- **Real product, zero fabrication:** Capture the real Windows app on a clean `1280×800` or `1120×720` window at `2×` DPI with realistic League match data (varied champions, clean KDA, genuine timeline markers).
- **Framing:** Display screenshots with `10–12px` rounded corners, a `1px solid rgba(255, 255, 255, 0.10)` hairline frame, and `--hf-shadow-overlay`. Never tilt screenshots at 3D perspectives or surround them with neon glows or purple gradients.
- **Backdrop:** Place dark-theme screenshots on `--hf-bg` (`#121314`) or subtle `--hf-surface-1` bands, optionally accented with a single ultra-subtle radial cobalt haze (`opacity <= 0.08`).
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
