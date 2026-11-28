# greyscreen.app website, design spec

Paper file "Greyscreen App" (fileId `01M8FDXHR3EEGZQPC4QVGZ6E9X`). Website pages:

- `Web — Home` (pageId `p-E-0`): Home desktop 1440 and Home mobile 390.
- `Web — Download & 404` (pageId `p-F-0`).
- `Web — Motion` (pageId `p-G-0`): storyboard of the animations.

Do not touch any other page. Always pass `pageId` to page-scoped tools (create_artboard, get_basic_info). Andres may be looking at another page.

## Idea

In League, the screen goes grey when you die and a respawn timer counts down. The website is that grey screen: white paper, ink, concrete greys. Colour lives only inside "screens": the real Greyscreen app (dark UI, green kills, red deaths, blue objectives). Everything outside a screen is black, white and grey.

Two playful devices, and only these two:

1. **The x_x mark is the mascot.** It is the full stop of the hero headline, the 404 face, and the hero of the motion loop.
2. **Sections count down like a respawn timer.** Each section label reads `RESPAWN 0:30`, `0:20`, `0:10`, and the last section is `0:00`, "respawn." with the download button.

Swiss editorial: huge heavy lowercase display type, light body text, mono labels, lots of white space. Minimal, tidy, precise. No gradients, no glows, no drop shadows on anything except app screenshots (one soft shadow allowed, see Stage).

## Tokens (already in the file, use them, never literals)

Colours: `--color-site-paper` #FFF, `--color-site-fog` #F3F3F3 (alternate band), `--color-site-concrete` #E4E4E4 (hairlines on paper), `--color-site-graphite` #5C5C5C (muted text on paper), `--color-site-ink` #111 (text, buttons, ink bands), `--color-site-ink-2` #1C1C1C (raised on ink), `--color-site-ash` #A3A3A3 (muted text on ink), `--color-site-respawn` #E8261B (REC dot ONLY, never anything else).
App tokens (`--color-page`, `--color-surface`, `--color-success-fg`, `--color-danger-fg`, `--color-info-fg`, ...) are allowed only inside dark app screens.

Sizes: `--text-site-label` 13, `--text-site-body` 16, `--text-site-lead` 20, `--text-site-h3` 28, `--text-site-h2` 72, `--text-site-display` 136. Radius `--radius-site-stage` 28.

Fonts: `Inter` and `IBM Plex Mono` (both verified available). Call get_font_family_info once before typing anyway.

## Type

| Role | Style |
| --- | --- |
| Display (hero, final CTA, 404) | Inter 900, 136px / 120px line, -0.06em, lowercase, ink |
| H2 (section title) | Inter 900, 72px / 72px, -0.05em, lowercase, ink |
| H3 | Inter 700, 28px / 32px, -0.02em, ink |
| Lead | Inter 400, 20px / 30px, graphite |
| Body | Inter 400, 16px / 24px, graphite |
| Label | IBM Plex Mono 500, 13px / 16px, uppercase, +0.08em, ink or graphite |
| Nav link | Inter 500, 15px, ink |

Headlines are lowercase and end with a full stop. That echoes the lowercase wordmark.

## Layout

- Desktop artboard 1440 wide, `height: fit-content`, background `--color-site-paper`, `overflow: hidden`.
- Side gutter 64px. Content max width 1312. Sections: 160px top and bottom padding. Inside a section, 24px between label and H2, 32px between H2 and body, 64-80px before the visual.
- 8px grid. Gaps: 8 / 12 / 16 / 24 / 32 / 48 / 64 / 96 / 160.
- Prefer asymmetry: text column 520-560 wide next to a large visual, never centred everything.
- Mobile 390: gutter 20, display 56px / 52px, H2 40px / 40px, sections 96px padding.

## Shared pieces (paste these exactly, so every page matches)

**Mark, dark (ink tile, white face), for paper.** Size by width/height.
```html
<svg layer-name="Mark" width="32" height="32" viewBox="0 0 64 64" fill="none"><rect x="4" y="4" width="56" height="56" rx="16" fill="var(--color-site-ink)"/><path d="M14.54 23.99L26.66 31.27M24.25 21.57L16.96 33.69M33.95 19.15L46.07 26.43M43.65 16.73L36.37 28.85M33.45 46.07L48 38.32" stroke="var(--color-site-paper)" stroke-width="5.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
```
**Mark, light (white tile, ink face), for ink.** Same SVG with the fill and stroke tokens swapped.

Brand rules for the mark: never rotate the tile, never add colour, shadow or glow, never straighten the face in a static design.

**Lockup:** a row with gap 10, align center: Mark 32px + text `greyscreen` in Inter 900, 24px, -0.06em, ink, lowercase.

**Buttons, badges and keycaps come from the Rune Kit** (Paper file "Rune Kit", page Button / Badge / Keybind), with their own look in Rune's light theme: each `--color-X` becomes `--color-light-X`. Never restyle them. The website uses:
- Primary CTA: Button Contrast, Extra large, leading Windows glyph (fill `--color-light-surface`), "Download for Windows".
- Secondary CTA: Button Outline, Extra large, "View on GitHub".
- Nav: Button Contrast, Large, "Download".
- Export options: Button Outline, Regular.
- Version: Badge neutral.
- Reference instances: page `Web — Components`, section "Website buttons in use". React port: `apps/web/src/components/kit/` (`Button`, `Badge`, `Keybind`).

Windows glyph:
```html
<svg width="16" height="16" viewBox="0 0 16 16"><path d="M0 0h7.5v7.5H0zM8.5 0H16v7.5H8.5zM0 8.5h7.5V16H0zM8.5 8.5H16V16H8.5z" fill="var(--color-site-paper)"/></svg>
```

**Text link:** Inter 500 16px ink, with a trailing `→`.

**REC dot:** an 8px circle in `--color-site-respawn`. It appears only inside the REC callout on the hero stage.

**Respawn label (section label):** row, gap 10, align center. A 16px ring: a circle 16x16 with a 2px concrete stroke and an ink arc showing the fraction left (0:30 = full ring, 0:20 = 2/3, 0:10 = 1/3, 0:00 = empty ring). Draw it as SVG (circle plus a path arc, stroke-linecap round). Then mono Label text `RESPAWN 0:20` in graphite.

**X bullet:** the mark's eye as a list bullet: `<svg width="20" height="20" viewBox="0 0 20 20"><path d="M4 4L16 16M16 4L4 16" stroke="currentColor" stroke-width="3.5" stroke-linecap="round"/></svg>` with stroke set to the token.

**Stage:** the frame that holds a real app screen. Background `--color-site-fog`, radius `--radius-site-stage`, padding 48, overflow hidden. Inside, the app screen keeps its own radius 12, a 1px `--color-site-concrete` border, and one shadow `0 40px 80px -24px rgba(17,17,17,0.28)`.

**Callout (sticker):** a small ink card floating over the stage edge (absolute): background ink, radius 12, padding 12 16, mono Label in paper on line 1, Inter 500 14px ash on line 2. Joined to its target with a 1px ink hairline (absolute div). Use 2-3 callouts per stage, never more.

## Using the real app

Clone the real app screens. Never redraw them.

- Last game screen: artboard `72F-0` (1280x760, page "Last game"). Highlights row, timeline row and hero are inside it.
- Home screen: artboard `79Y-0` (1280x760, page "Home").
- Use `<x-paper-clone node-id="72F-0" style="...">` inside write_html, or `duplicate_nodes` with a `parentId`. Then get_tree_summary on the clone to find its children.
- The app screens still say "Greyscreen" with a hexagon icon in the top left. In every clone, set that text to `Greyscreen` and replace the hexagon icon with the LIGHT mark at the same size (white tile, ink face). Leave the source artboards untouched.
- To show a part of a screen (for example the timeline row), clone just that child node into your layout.
- A clone keeps its 1280 width. Where you need it smaller, set `transform: scale(x)` with `transform-origin: top left` and wrap it in a fixed-size box with overflow hidden. Check it in a screenshot.

## Copy rules

- Real facts only. The app has kills, deaths, assists, multikills, first blood, aces, dragons, heralds, barons, void grubs, turrets, inhibitors. There is NO rank, LP, CS, gold, items, runes, role or patch. Never mock those.
- Facts you can use: free, open source, public beta, Windows 10 2004+ and 11 (64-bit), League in Borderless or Windowed, records on the GPU's hardware H.264 encoder, 1080p60 High by default, about 55 MB per minute, MP4 + small JSON file per game, no account, no uploads, installs from one .exe without admin rights, sits in the tray, F9 saves the last 30 seconds, export presets "Original quality" and "For Discord (under 10 MB)", 7 death tags (Out of position, Overextended, No vision, Bad trade, Ganked, Teamfight, Unavoidable), events come from Riot's Live Client Data API on your own PC, video uses Windows Graphics Capture, no memory reading, no injection, no game file changes, game audio only (Discord and mic stay out).
- Current release: `v0.1.20 preview 28`. Download file `GreyscreenSetup.exe`. GitHub repo `andresmarpz/greyscreen`.
- No Riot logos or art. The Riot legal notice goes in the footer, 13px graphite: "Greyscreen isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc."
- Tone: short, dry, a little funny. Player words. No exclamation marks, no emoji.

## Page: Home desktop (artboard "Home — Desktop", 1440)

1. **Nav**, height 88, padding 0 64, space-between, align center. Left: Lockup. Middle: links Features, Download, FAQ, Changelog (gap 32). Right: text link `GitHub` and a small primary button `Download`.
2. **Hero**, padding 96 64 0.
   - No label above the headline. Display headline on two lines: `make the grey` / `screen count` followed by the dark Mark, 104px, sitting on the baseline in place of the full stop (a row with align-items flex-end; put the second line and the mark in one row with gap 20).
   - 48px below, a row with two columns, space-between, align flex-end. Left (width 560): Lead "Greyscreen records every League game on your GPU, pins every kill, death and dragon to a timeline, and cuts the clips for you." Then 32px below, the Primary CTA, gap 12, then the Secondary CTA. Right: a mono stack (gap 10, align right), graphite: `FREE`, `OPEN SOURCE`, `WINDOWS 10 & 11`.
3. **Hero stage**, padding 72 64 0. A Stage, full width 1312. Inside, a clone of the Last game screen (rebranded), centred, at scale 1 (stage 1312 minus 48 padding each side is 1216, so scale the clone to 0.95 inside a 1216x722 box). Callouts:
   - top-left over the edge: `● REC 24:31` / "Started by itself."
   - over the timeline, pointing at a red death marker: `DEATH 3 OF 7` / "Tagged: Ganked."
   - bottom-right, over the highlights row: `CLIP SAVED` / "Double kill · 0:29"
4. **0:30, "it just records."** Section with id-like layer name "Features". Respawn label `RESPAWN 0:30`, H2 `it just records.`, body "No record button. Greyscreen waits in the tray, starts when the game loads and stops a few seconds after the Nexus falls." Then a three-step timeline row 1312 wide: a 2px concrete line across, three nodes on it (12px ink circles), each node with a mono step time (`00:00`, `00:01`, `32:14`), an H3 and body underneath. Steps: "Install and queue up." / "One .exe, no admin rights." · "Play normally." / "Your GPU records the game window and League's audio. Discord and your mic stay out." · "Review and clip." / "Jump to any kill, death or objective."
5. **0:20, "every death, on the timeline."** Fog band, full bleed (1440), padding 160 64. Two columns, gap 96. Left 480: respawn label, H2, body "Scrubbing a 32 minute game for one fight is a waste of a grey screen. Greyscreen puts every event on the seek bar, so you jump straight to the play." Then 3 points as a list (X bullet, H3-ish 18px Inter 600 title + body): "Filters and pre-roll" (jump to 5 seconds before any event), "Death review" (step through each death, tag it, write a note), "Theater mode and speed". Right (rest): an ink panel, radius 28, padding 40: on top a clone of the timeline row from the Last game screen; below it a "Death review" card built from app tokens: title `Death 3 of 7` (Inter 600 16 `--color-text`), mono time `14:32`, then the 7 tags as chips (`--color-layer-2` background, `--color-text-muted` text, radius 999, 13px), "Ganked" selected (`--color-danger-bg` background, `--color-danger-fg` text, `--color-danger-border` border), then a note field with the text "Warded the wrong bush. Again."
6. **0:10, "clips cut themselves."** Paper, padding 160 64. Two columns. Left (wide): a 2x2 grid (gap 16) of the 4 highlight clip cards cloned from the Last game screen. Right 440: respawn label, H2, body "Kills, multikills, aces, dragons, barons: clipped while you play. Saw something the game didn't count? Press F9 to save the last 30 seconds." Then a big keycap: 120x120 box, 2px ink border, 4px ink bottom border, radius 20, text `F9` Inter 900 48px ink. Then a mono `EXPORT` label and two chips in one row (1px concrete border, radius 999, height 36, padding 0 14, Inter 500 14 ink): `Original quality`, `For Discord (under 10 MB)`.
7. **"stays out of the game."** Ink band, full bleed, padding 160 64. H2 in paper, no label (this section sits outside the countdown). Then three columns (gap 48), each: X bullet in paper 28px, H3 in paper, body in ash: "No memory reading." / "Events come from Riot's Live Client Data API on your PC." · "No injection." / "Video comes from Windows Graphics Capture. The game is never touched." · "No account." / "Recordings stay on your PC as MP4 files. Nothing is uploaded."
8. **Numbers strip**, paper, padding 120 64. Three cells in a row split by 1px concrete vertical lines. Each: number Inter 900 56px -0.05em ink, then mono label graphite. `1080p60` / `DEFAULT QUALITY` · `~55 MB` / `PER MINUTE` · `1 .exe` / `NO ADMIN RIGHTS`.
9. **0:00, "respawn."** Paper, padding 160 64, centred this time (the one centred moment). Respawn label `RESPAWN 0:00` with the empty ring, the dark Mark at 160px, Display `respawn.`, lead "Your next game records itself.", primary button `Download for Windows` + text link `v0.1.20 preview 28 · Changelog →`. Mono line under it in graphite: `WINDOWS 10 2004+ & 11 · 64-BIT · FREE`.
10. **Footer**, padding 64 64 0, top border 1px concrete. Row: Lockup left; links right (Privacy, Get help, GitHub Issues, Changelog). Riot notice below (max width 760). Then the giant wordmark `greyscreen` in Inter 900, about 330px, -0.07em, ink, spanning the full width, cropped by the artboard bottom so only the top 75% of the letters show (put it in a box with fixed height and overflow hidden).

## Page: Home mobile (artboard "Home — Mobile", 390, next to desktop)

Same sections, stacked. Nav: Lockup + a 40px menu button (two 2px ink lines). Hero display 56px. The stage shows the Home screen clone, cropped to the left 60%, with one callout. Timeline steps stack vertically with the line running down the left. Numbers strip is 2x2. Footer wordmark about 96px.

## Page: Download & 404

**Download desktop** (1440, fit-content): same Nav. Hero: respawn label `RESPAWN 0:05`, Display `download.`, lead "One file. No admin rights, no account." Then a big card (fog, radius 28, padding 48, width 1312, row): left: mono `v0.1.20 PREVIEW 28 · PUBLIC BETA`, H3 `GreyscreenSetup.exe`, body "Windows 10 2004+ and 11, 64-bit."; right: primary button `Download for Windows`, then text link `All releases on GitHub →`. Then "first run" as 3 numbered steps in a row (mono `01 02 03`, H3, body): "Run it." / "Windows may show SmartScreen because the app is new. Click More info, then Run anyway." · "Pick a folder." / "Videos\Greyscreen by default." · "Queue up." / "Your next game records itself." Then "Requirements" as a two-column definition list with 1px concrete row lines (Windows sits on the download card, not here): League / "Borderless or Windowed" · Graphics / "A GPU with a hardware H.264 encoder (software fallback works, uses more CPU)" · Disk / "About 2 GB per game at the defaults". Same footer as Home (you can clone it from the Home page once it exists, otherwise build it to the spec).

**404** (1440x900): paper, centred column. Dark Mark 200px, Display `this page died.`, lead "Respawning you on the home page in 0:05.", primary button `Back to home`. No respawn label: the lead already counts down.

## Page: Motion (storyboard)

Static frames that describe each animation. One artboard per animation, 1440 wide, fit-content, paper, padding 64. Each has a title (H2 40px), a one-line description, then frames left to right (gap 24) with a mono caption under each: time `0.0s`, what moves, easing. All motion respects `prefers-reduced-motion` (then the final frame shows, no movement).

1. **Hero loop** (the main showcase, about 9 s, loops). Frames are 16:10, built from a scaled clone of the Last game screen inside a Stage:
   - `0.0s` Playing: full colour, REC chip timer running `● REC 24:31`.
   - `2.4s` Death: the screen goes greyscale over 400 ms (`filter: grayscale(1)` on the screen), the dark Mark drops in from above centre and lands with a small overshoot (spring, 500 ms). Its face falls from upright to its -14° tilt as it lands. A mono `RESPAWN 0:12` counts down under it.
   - `4.2s` Marker: a red death marker pops onto the timeline (scale 0 → 1.15 → 1, 300 ms) and the callout `DEATH 3 OF 7 · Ganked` appears.
   - `6.0s` Respawn: colour comes back (400 ms ease-out), the Mark lifts away.
   - `7.2s` Clip: a highlight card slides 24px up out of the stage edge with `CLIP SAVED · Double kill · 0:29`, holds, fades. Loop.
2. **Headline mark** (on load, once): the full-stop Mark in the hero starts with its face upright, then at 600 ms the face tips over to -14° with a small bounce (spring, 450 ms). The tile never moves.
3. **Respawn labels** (on scroll): when a section enters, its ring arc draws from full to its value (600 ms ease-out) and the time ticks down to its value in mono.
4. **Hover and press**: kit buttons use their own Hover and Pressed states; the Primary CTA's Windows glyph also shifts 2px right. Text links: the arrow moves 4px right. Callouts: no hover. Nothing else animates.
5. **REC dot**: opacity 1 → 0.35 → 1, 1.6 s, ease-in-out, forever. The only looping thing outside the hero.

Show each micro-interaction as 2-3 small frames (rest, hover, press) built from the shared pieces.

## Quality bar

- Screenshot after every section. Check spacing rhythm, alignment to the 64px gutter, contrast, and clipping.
- Name every layer (`layer-name`). Sections are named `Nav`, `Hero`, `Stage`, `Features`, `Timeline`, `Clips`, `Safe`, `Numbers`, `Respawn`, `Footer`.
- Write small: one visual group per write_html call.
- Colours only through tokens.
- Call finish_working_on_nodes on your artboards when done.

## Copy rule

Every string must earn its place (clean-copy ladder). Cut captions that restate a title or a visual, and never say the same fact twice on one page. Keep compatibility facts next to every download button.
