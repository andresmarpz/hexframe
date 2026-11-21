# Research: competitors

Date of research: 2026-11-21. Scope: League of Legends recorders and clippers on Windows, VOD review tools, built-in recorders, Riot policy that affects them.

Legend: **[V]** read on the vendor's or Riot's own page, fetched during this research. **[S]** only seen in a search-result summary or a third-party page, not re-read at the source. **[U]** could not be confirmed. Vendor claims about themselves or about rivals are marked as claims and are not benchmarks.

Limits of this research:

- Reddit was not reachable (blocked for the search tool, not fetched directly). Trustpilot refused direct fetches (403). User opinions below come from the Overwolf store, OBS forum, Steam forum, vendor-chosen testimonials and search summaries of Trustpilot. Treat vendor-chosen quotes as selected.
- I did not install or run any competitor. No FPS number in this file was measured by me.
- NVIDIA App, AMD Adrenalin, Xbox Game Bar and Skill-Capped pages were JavaScript shells or returned nothing useful, so those sections are thinner and say so.

## Executive summary

1. The market has moved since the Overwolf era: Medal, Ascent and DOR are standalone Windows apps, and Insights dropped Overwolf in its V3. "Needs Overwolf" is no longer a reliable gap for Greyscreen. Native, lightweight, no-ads is now table stakes.
2. **Ascent.gg is the closest rival** (2026): native Rust/Tauri, an open-source OBS fork for recording, League stats and timeline, AI match review, win-probability graph, "Death Replay" clips, 15 GB free cloud. Closed-source app, account (Discord) sign-in, cloud upload.
3. **LeagueRecord (open source, GPLv3, Tauri + libobs)** is the closest in spirit: local only, auto-record, event timeline, no account. It has storage caps, ranked-only, favorites and a marker hotkey that Greyscreen lacks. It has no automatic highlight clips.
4. Greyscreen's real differentiators today: no account and no upload, no Overwolf or Electron, no libobs, crash-safe fragmented MP4, automatic clips cut without re-encoding, Vanguard-safe capture. Nothing else on the list combines all of these.
5. Privacy is a live issue: Medal's policy names a US AI lab as joint controller of clips for model training (opt-out). Insights' Shards program pays gift cards for footage used in generative-model training. Both are opt-out or opt-in toggles, but they make "no upload, no account, verifiable" a strong message.
6. Players complain about: ads (Outplayed), recording that silently fails, huge files that fill the disk, Overwolf as a layer, and OBS Game Capture breaking after Vanguard updates.
7. Greyscreen's gaps are in what happens after the game: no storage management, no match stats or scoreboard, no manual clip hotkey, no trim or share, no death-focused review, no TFT or queue filter.
8. Riot's replay system is getting worse for reviewers (.rofl expires each patch, last 20 games, 10 downloads per hour since patch 26.21). That pushes players towards video recordings and helps every recorder, Greyscreen included.
9. Policy risk is low for a post-game recorder: Riot's stance rejects in-game advice, not post-game review, and says apps using the in-game APIs should keep working under Vanguard. Riot has not mentioned recorders. The risk is Riot changing the local API.
10. Greyscreen has no published performance numbers. Ascent publishes a benchmark that names Medal and Outplayed. Greyscreen needs measured FPS, RAM and disk numbers from a real Windows run before it can make any speed claim.

## Feature comparison

Legend: Yes / No / Partial; `?` = not confirmed. Greyscreen column is the product as of the README (preview, real-hardware testing on Windows under way).

### League-focused recorders

| Feature | Greyscreen | Outplayed | Medal | Insights Capture | Ascent | DOR | LeagueRecord |
|---|---|---|---|---|---|---|---|
| Records full match automatically | Yes | Yes (highlights, full match or session modes) | Partial (clips default; sessions) | Yes | Yes | Yes | Yes |
| Auto clips on League events | Yes (kills, deaths, multikills, objectives, aces; assists and structures off by default) | Yes (event-based; Overwolf clip windows, for example death 12 s before and 3 s after) | Yes | Yes | Yes | Yes (kills, multikills, objectives) | No (markers only) |
| Event timeline over full recording | Yes, with filters and theater mode | Yes (match timeline, APM) | Partial (match recap) [S] | Yes | Yes | ? | Yes, with filters |
| Death-focused review | Partial (death markers and clips) | No | No | Partial (deaths timestamped) | Yes ("Death Replay", 10 to 30 s) | ? | Partial (death markers) |
| Match stats or scoreboard | No | No (APM only) | Yes (post-match recap) | ? | Yes (stats, graphs) | ? | No |
| AI or win-probability analysis | No | No | No | No (VOD review is manual) | Yes (Pro: unlimited AI reviews, win probability) | "AI highlights" | No |
| Manual clip hotkey | No (button in app only) | Yes | Yes (F8) | Yes | Yes (F8) | ? | Marker hotkey only |
| Editor | No | Yes (trim, merge, crop, montage) | Yes (trim, text, effects, captions) | Yes (trim, merge, montage) | Basic trim | ? | No |
| Sharing | None (files only) | outplayed.tv upload, social | Medal links, feed, mobile app | Cloud links, VOD review | Cloud links, communities | dor.gg community | None |
| Account required | No | Overwolf client required; account `?` | Yes to share | Yes (Google or Discord sign-in) | Yes (Discord) | ? | No |
| Uploads gameplay by default | No | Optional | On post | Cloud storage [S] | Cloud (free 15 GB) | ? | No |
| AI-training use of footage | No | ? | Yes, opt-out (joint controller named in policy) | Opt-in via Shards program | Privacy policy has an AI section [U] | ? | No |
| Ads | No | Yes (free tier) | Ad partner named in policy | Basic tier shows ads per pricing page [S] | None (claim) | ? | No |
| Runtime | Native Rust, Rune GPU UI | Overwolf (Chromium) | Standalone, described as Electron by a rival | Standalone (was Overwolf) | Native Rust + C++, Tauri | ? | Tauri + WebView2 + libobs |
| Storage management | No | ? (users report 100 GB growth) | Premium cloud storage | Yes (storage view, cleanup) | Local limit setting, cloud quota | ? | Yes (age, size caps, favorites) |
| Price | Free, MIT | Free with ads, Premium | Free, Premium | Free, Elite/team plans | Free, Pro $4.99/month | Free | Free, GPLv3 |
| Open source | Yes | No | No | No | Recorder only | No | Yes |

### Generic and built-in tools

| Feature | SteelSeries Moments | NVIDIA App / ShadowPlay | AMD Adrenalin ReLive | Xbox Game Bar | OBS Studio | League client replay |
|---|---|---|---|---|---|---|
| Records full match automatically | No (hotkey clip, buffer) | No (Instant Replay buffer, manual record) | No (instant replay buffer) | No (background buffer) | No (manual, or replay buffer) | Replays, not video |
| League event auto clips | Yes (League listed) | Unconfirmed for League (legacy Highlights SDK, unsupported) | Old Gaming Evolved note [S] | No | No | No |
| Event timeline | Yes, markers on timeline | No | No | No | No | Yes (in replay viewer) |
| Match stats | No | No | No | No | No | In replay viewer |
| Editor | Trim, montage, multi-track | Basic | Basic | Trim | No | No |
| Account | ? | ? | ? | ? | No | Riot account |
| Runtime | Native | Driver-level | Driver-level | Windows built-in | Native | Game client |
| Vanguard notes | ? | ? | ReLive failed with League in admin mode plus borderless in a 2019 note [S] | ? | Game Capture broke after Vanguard changes in 2026 per forum | n/a |
| Price | Free | Free (NVIDIA GPU) | Free (AMD GPU) | Free | Free | Free |

## Per-product notes

### Outplayed (Overwolf)

- What it is: Overwolf app. 17.6M downloads, 4.6 rating from 2,497 ratings, "Free, contains ads", Premium removes ads. Version 177.3.14133 on 2026-11-16. [V] [Overwolf listing](https://www.overwolf.com/app/Overwolf-Outplayed)
- Records: highlights, full match or full session. Settings for resolution, FPS and encoder. [S] [Insights comparison, vendor rival](https://insights.gg/blog/insights-vs-outplayed)
- League detection: Overwolf game events. Overwolf's shared `highlights.json` sets the League clip windows, for example death: 12 s before, 3 s after; assist: 12 s before, 8 s after; a "pending" window merges nearby events into one clip. Settings are shared by all Overwolf apps and only Overwolf can change them. [V] [Overwolf dev docs](https://dev.overwolf.com/ow-native/guides/general-tech/auto-highlights-supported-games)
- Review: match timeline, APM review, in-game overlay, second-monitor support. Editor with trim, merge, montage. [V] [Overwolf listing](https://www.overwolf.com/app/Overwolf-Outplayed)
- Performance: Overwolf's own support page says video capture uses RAM, CPU and GPU, that a small impact is normal, and lists FPS drops, game crashes and Overwolf crashes as possible results. Minimum 8 GB RAM, 16 GB recommended. [V] [Overwolf support](https://support.overwolf.com/support/solutions/articles/9000184425-performance-issues-fps-cpu-memory)
- Rival claims (not benchmarks): Ascent says Outplayed used 823 MB while recording and took 7.4 s to open, against 16 MB and 0.1 s for Ascent (measured by Ascent, October 2026). [V] [Ascent home page](https://www.ascent.gg/)
- Complaints: ads, recording that does not start, clips with missing audio, deleted videos that stay on disk, a request for a version without Overwolf. [S] [Trustpilot summary](https://www.trustpilot.com/review/outplayed.tv?page=5)
- Business model: Overwolf requires every app to use Overwolf ads or Tebex subscriptions, and its developer docs say from 2026-11-01 only apps with Overwolf monetization are approved. [V] [Overwolf monetization](https://dev.overwolf.com/ow-native/monetization/overview), [Overwolf developer docs banner](https://dev.overwolf.com/ow-native/guides/general-tech/auto-highlights-supported-games)
- Riot rules for Overwolf apps: apps need Riot's third-party approval, may not show power-spike alerts, enemy cooldown trackers and similar. Nothing about recorders. [V] [Overwolf Riot compliance](https://dev.overwolf.com/ow-native/guides/game-compliance/riot-games)
- Outplayed.tv upload retention: a rival (Medal) says free uploads are deleted after 3 months. [S] [Medal comparison](https://medal.tv/compare/medal-vs-outplayed)

### Medal.tv

- What it is: standalone Windows app plus iOS and Android apps, a social clip platform. Claims 20M+ monthly active users (October 2026). Free, optional Premium (extra cloud storage, higher quality exports). Third-party price listing: from $2.49/month [U]. [V] [Medal features](https://medal.tv/features), [Medal comparison](https://medal.tv/compare/medal-vs-outplayed)
- Records: hotkey clip (F8) from a buffer, session recordings, auto-clipping in supported games including League, up to 4K at 144 fps. Hardware encoding (NVENC, AMF, QuickSync). Separate audio tracks for game, mic and Discord. [V] [Medal League page](https://medal.tv/games/league-of-legends), [Medal features](https://medal.tv/features)
- League detection: "event detection" for League. The exact League events are not published on the pages I read. A November 2026 changelog entry says the post-match recap, with stats and a muted clip loop, appears after every League match. [V] [Medal changelog](https://medal.tv/changelog)
- Review: stats recap, no timeline-driven VOD review. Editor with trim, text, effects, slow-mo, captions. Voice-activated "clip that". Friend feed. [V] [Medal comparison](https://medal.tv/compare/medal-vs-outplayed)
- Performance: Medal claims 100 to 400 MB RAM and no dropped frames. Ascent claims 711 MB and 298 fps against its own 391 fps, and a "3 to 8%" FPS cost for Medal. Both are vendor claims. [V] [Medal features](https://medal.tv/features), [Ascent home page](https://www.ascent.gg/), [Ascent comparison](https://www.ascent.gg/blog/ascent-vs-medal-vs-obs)
- Privacy: policy names an advertising partner relationship and says Medal B.V. is joint controller with General Intuition Inc. for "Training Data" (clips and associated data) used to train AI models, with transfer to the US, identifiers stripped, opt-out in settings. Third-party reviews also report a broad licence. [V] [Medal privacy policy](https://medal.tv/privacy), [Medal opt-out article](https://support.medal.tv/support/solutions/articles/48001281414-opt-out-of-ai-training)
- Monetization: free app, Premium, ad partner, in-app contests and quests (for example a League "Clip That" contest, November 2026). [V] [Medal changelog](https://medal.tv/changelog)

### Insights Capture (Insights.gg)

- What it is: recorder plus a team VOD review site. V3 is a rebuild with no Overwolf. Claims about 40% less FPS impact than V2. Windows only. [V] [Insights V3](https://insights.gg/v3-updates), [Insights League page](https://insights.gg/games/league-of-legends)
- Records: automatic from champion select to the Nexus, manual mode, clips mode. Desktop recording, merge videos, keyboard and mouse overlay, storage view. [V] [Insights League page](https://insights.gg/games/league-of-legends)
- League detection: kills, deaths, dragons and barons timestamped on a timeline of the full recording, pentakill and steal highlights saved. [V] [Insights League page](https://insights.gg/games/league-of-legends)
- Review: timeline, highlight mode, auto montage. The VOD Review site adds timestamped comments, drawing on frames and merged team POVs. [V] [Insights vs Outplayed](https://insights.gg/blog/insights-vs-outplayed)
- Pricing: Basic free with 5 GB active storage. Paid bundles per team: Silver $39/month, Gold $79, Platinum $109, Diamond $229 monthly rates for 5 to 20 Elite seats; yearly rates are lower. "Elite" is described as no ads, permanent upload and advanced features, which implies the free tier shows ads. Insights' own blog says "no ads and no watermark", so the two Insights pages disagree. [V] [Insights pricing](https://insights.gg/pricing)
- Privacy: account via Google or Discord. Footage goes to Wasabi cloud storage. Metadata collected includes in-game events, keyboard and mouse activity timestamps and audio. Opt-in Shards program: footage goes to Storj or Wasabi for "research and development", is used for generative model training, is de-identified and shared with research partners that cannot be named, and opting out does not recall data already shared. Rewards are gift cards. [V] [Insights privacy policy](https://insights.gg/privacy)

### Ascent (ascent.gg)

- What it is: standalone Windows app released in 2026, version 1.0.2 on 2026-11-15. Native Rust and C++ with a Tauri/WebView2 UI, recorder is `ascent-obs`, an open-source fork of OBS Studio running in a separate process. The desktop app (detection, viewer, analytics, cloud) is closed source. [V] [Is Ascent open source](https://www.ascent.gg/blog/is-ascent-open-source), [Ascent vs Medal vs OBS](https://www.ascent.gg/blog/ascent-vs-medal-vs-obs)
- Records: automatic for 100s of PC games, F8 saves the last 30 s, per-game opt-out (F4), AV1 optional, named audio tracks, input overlay on clips. [V] [Ascent changelog](https://www.ascent.gg/changelog)
- League features: stats, graphs, timeline, AI review, a League win-probability graph (Pro), "Ascent Score" built from a win-probability model trained on about 300,000 games (AUC 0.8241), Death Replay (latest clip reopenable, 10 to 30 s). [V] [Ascent pricing](https://www.ascent.gg/pricing), [How Ascent Score works](https://www.ascent.gg/blog/ascent-score), [Ascent changelog](https://www.ascent.gg/changelog)
- Pricing: Free: unlimited recording, 15 GB cloud, cloud videos never expire, clips, stats, share links. Pro $4.99/month: unlimited AI match reviews, 250 GB, video and clip download, win probability. Custom for teams. [V] [Ascent pricing](https://www.ascent.gg/pricing)
- Performance: Ascent claims under 1% FPS impact; the figures come from Ascent's own tests. [V] [Does Ascent use Overwolf](https://www.ascent.gg/blog/does-ascent-use-overwolf)
- Vanguard and League bugs: version 0.5.81 (2026-09-15) fixed a yellow border and lost cursor while recording League. [V] [Ascent changelog](https://www.ascent.gg/changelog)
- Privacy: sign in with Discord, cloud upload, privacy policy has an "AI and Model Training" section that I did not read in full. [V] [Ascent privacy](https://www.ascent.gg/privacy)
- Why it matters: it already has the "record everything, review with a League timeline" product, free, and marks Medal and Outplayed as slow. It leaves room for "no account, no cloud, fully open".

### DOR (clip.dor.gg)

- What it is: free Windows recorder, claims 2.2M+ users, big game list with Korean titles. Records every League match, saves kill, multikill and objective clips, and records replay and spectator playback by default. [V] [DOR home](https://clip.dor.gg/), [DOR LoL clipping guide](https://clip.dor.gg/en/blog/best-lol-clipping-software)
- Calls out that League replays stop playing after a patch and are capped at 10 downloads per hour since 26.21, so recordings are the way to keep a game. [V] [DOR replay guide](https://clip.dor.gg/en/blog/league-of-legends-replay-guide)
- I could not find a privacy policy, pricing tier or review. [U]

### LeagueRecord (open source)

- What it is: open-source Windows recorder for League only. Tauri (WebView2) and libobs. GPLv3. 48 stars, 391 commits. Installer or portable zip. [V] [GitHub](https://github.com/FFFFFFFXXXXXXX/league_record)
- Features: auto-detect and record, tray app, recordings list with favorites, timeline with colored event markers and per-type toggles, keyboard shortcuts, a highlight hotkey that drops a marker, autostart, only-ranked mode, age and size caps that delete non-favorites, filename template, audio modes (none, game, system, system plus mic), resolution and FPS options. [V] [GitHub readme](https://github.com/FFFFFFFXXXXXXX/league_record)
- Performance (author, Ryzen 3600 + RX 5700, Task Manager): recording about 3% CPU, 50 MB RAM, 4% GPU; watching a recording about 160 MB RAM; about 75 MB on disk. [V] [GitHub readme](https://github.com/FFFFFFFXXXXXXX/league_record)
- Known issue: black screen fixed by running as admin. [V] [GitHub readme](https://github.com/FFFFFFFXXXXXXX/league_record)
- Gap versus Greyscreen: no automatic highlight clips, no account, no cloud, no stats.

### miapin.gg (new Overwolf app, League only)

- Records League automatically, uploads after the game, plays back on the web with a timeline from Riot match data (kills, objectives, items). Free: 5 uploads in total. Pro $14.99/month for 30 uploads. Needs Overwolf and an Overwolf account. Version 3.1.8, 2026-11-05. [V] [Overwolf listing](https://www.overwolf.com/app/miapin.gg-miapin.gg)
- Signal: a small team is charging for League-only recording plus web review. The market for review is real, and it is currently paywalled and cloud-only.

### Mobalytics, Porofessor, Blitz, OP.GG, U.GG

- **Mobalytics**: desktop app with "Smart Highlights": after each game it generates the 5 most impactful plays, shown in the post-game Performance Overview. Free tier gets 1 clip per game, Plus gets all 5. Windows only. Mobalytics says it should not have much FPS impact. [S] [Mobalytics FAQ](https://mobalytics.gg/lol/glp/smart-highlights-1) (page blocked direct fetch)
- **Porofessor**: Overwolf/Electron overlay (free with ads, 17.8M downloads). Feature list is scouting, builds, timers and gold difference, no recording. [V] [Overwolf listing](https://www.overwolf.com/app/trebonius-porofessor.gg)
- **Blitz**: no recording feature found. A third-party review describes a stat overlay with reports of minor frame drops in champion select. [S] [WomboCombo review](https://www.wombocombo.gg/blog/game-analytics/blitz-gg-overlay-review)
- **OP.GG Desktop**: patch notes through 3.2.0 (2026-10-28) mention AI voice, chatbot, overlay and "Recommended Videos". I found no recorder. [V] [OP.GG patch notes](https://op.gg/desktop/en/patch-notes)
- **U.GG app**: page blocked, not checked. [U]

### ReplayBook and the League client replay

- Riot replay (`.rofl`): download from the end screen or match history, lasts one patch cycle, only the last 20 games are available, and since patch 26.21 downloads are limited to 10 per hour. The client also has a built-in Highlights recorder that writes `.webm` files. [V] [Riot replays FAQ](https://support.riotgames.com/en-us/league-of-legends/gameplay/replays-faq-pro-tips)
- Riot announced in patch 26.20 that players could only download replays of games they played in and that third-party apps and sites would lose replay access. After backlash Riot delayed it and shipped the 10-per-hour cap in 26.21. Riot says the cap prevents "large-scale abuse of downloaded replays by bad actors using vibecoded apps to scrape thousands upon thousands of replays". [V] [Patch 26.21 notes](https://leagueoflegends.com/en-us/news/game-updates/league-of-legends-patch-26-21-notes), [Insider Gaming](https://insider-gaming.com/riot-delays-private-league-of-legends-replays/)
- ReplayBook: free, open source (GPLv3, 459 stars) manager and player for `.rofl` files. Replays from patches 14.9 to 14.10 lack metadata and are unsupported. I found no statement from ReplayBook on the 2026 changes. [V] [ReplayBook](https://github.com/fraxiinus/ReplayBook)
- Why it matters: a `.rofl` is not a keepsake. A video recording survives the patch, and the replay route is now rate limited. This is a clean story for Greyscreen.

### NVIDIA App / ShadowPlay

- Instant Replay buffer and manual recording. NVIDIA's Highlights (automatic capture of key moments in supported games) is now a legacy, unsupported SDK. League appears in a 2021 GeForce NOW post listing Highlights games. I could not confirm League support in the current NVIDIA App. [V] [NVIDIA Highlights SDK](https://developer.nvidia.com/highlights), [S] [NVIDIA blog](https://blogs.nvidia.com/blog/geforce-now-thursday-june-10)
- NVIDIA GPUs only. Ascent's guide lists the same limits and calls it "low FPS impact, no game analysis, limited sharing". [V] [Ascent League guide](https://www.ascent.gg/blog/how-to-record-league-of-legends)

### AMD Adrenalin (ReLive)

- Instant replay buffer (AMD's FAQ says up to 20 minutes, adjustable in 15 s steps), manual recording. Gaming Evolved once integrated League for automatic replay clips. Community reports of stutter with Instant Replay on some GPUs, and a League-specific failure when League ran as admin in borderless mode (2019 era). I could not read AMD's current page. [S] [AMD FAQ](https://www.amd.com/en/resources/support-articles/faqs/DH3-023.html), [Tom's Hardware](https://www.tomshardware.com/news/amd-gaming-evolved-update,31437.html)

### Xbox Game Bar

- Background recording buffer plus manual capture, built into Windows. No League event detection. Microsoft advises turning off background recording when performance issues show up. A PCWorld test of a different game on a budget machine saw an average of 39.43 fps drop to 37.74 fps. Another source claims 3 to 8% on CPU-bound games without a stated method. League is often CPU-bound. [S] [Xbox news](https://news.xbox.com/en-us/2016/10/14/tips-on-windows-10-and-game-dvr-optimization), [PCWorld](https://www.pcworld.com/article/3315624/gaming/game-bar-for-windows-10-review.html), [Switchblade Gaming](https://www.switchbladegaming.com/game-settings/xbox-game-bar-performance/)

### OBS Studio (replay buffer)

- Open source, native. Manual scenes and sources, no League detection. Game Capture on League broke after Vanguard changes in July 2026 and in December 2024 according to forum posts. Window capture works but is reported as janky. Replay buffer reports are anecdotal. [S] [OBS forum, July 2026](https://obsproject.com/forum/threads/game-capture-black-screen-with-only-league-of-legends.195861/), [win.gg](https://win.gg/obs-game-capture-not-working-valorant-league-of-legends-fix/)
- Ascent says OBS costs 2 to 5% with hardware encoding. This is a competitor claim. [V] [Ascent comparison](https://www.ascent.gg/blog/ascent-vs-medal-vs-obs)

### SteelSeries Moments

- Free native app. Hotkey clipping without an overlay, auto-clip with "Moments AI" for listed games, League included since launch in 2021 (via GameSense then; marketed as Moments AI now). League events named by SteelSeries: multi-kills, kills at low health, objective steals. Trim, montage, multi-track audio, upload to YouTube, Discord, Reddit, X. [V] [Moments page](https://steelseries.com/gg/moments), [S] [2021 press release](https://steelseries.com/press/79-steelseries-launches-moments)
- No review layer beyond event markers on the timeline.

### Skill-Capped and human VOD review

- I could not read Skill-Capped's pages, so I have nothing verified about its product. [U]
- Human VOD review prices found: 1v9.gg about EUR 14.99 per game, Metafy coaches about $20 to $25 per hour session, Blazing Boost from $17.29. [S] [1v9.gg](https://1v9.gg/league-of-legends/services/vod-review), [Metafy](https://metafy.gg/tags/vod-review/league-of-legends), [Blazing Boost](https://blazingboost.com/league-of-legends-boosting/coaching)
- Signal: players pay for VOD review. A good local recording and timeline makes those sessions cheaper and better, since a coach can jump to deaths.

## What players praise and hate

Sources are mixed in quality; see the limits at the top.

### Praise

- Auto recording with no button: "I love that it automatically sorts through the footage to find and make highlights." (Insights testimonial, vendor-chosen) [V] [Insights League page](https://insights.gg/games/league-of-legends)
- No frame drops, small storage: "Love everything about it easy to use and records without any framedrops whatsoever." (Insights testimonial, vendor-chosen) [V] [Insights League page](https://insights.gg/games/league-of-legends)
- Outplayed on League: "great for league, except the PBE server" (Overwolf store review, 2026-09-20) [V] [Overwolf listing](https://www.overwolf.com/app/Overwolf-Outplayed)
- Medal, removable clips: "Clips moments automatically and if I don't like them it's easy to remove." (Trustpilot, Sep 2026, quoted by Medal) [V] [Medal comparison](https://medal.tv/compare/medal-vs-outplayed)
- Outplayed interface, quality and low resource use, according to a summary of Trustpilot reviews. [S] [Trustpilot](https://www.trustpilot.com/review/outplayed.tv?page=5)

### Hate

- Disk growth: "It works, but the videos are huge, before you know it, the app is taking up over 100 gb" (Overwolf store review of Outplayed, 2026-08-15) [V] [Overwolf listing](https://www.overwolf.com/app/Overwolf-Outplayed)
- Ads and FPS: one Trustpilot review says the app "constantly plays ads, lowers my FPS in most games" (as shown in a search summary, not re-read at source) [S] [Trustpilot](https://www.trustpilot.com/review/outplayed.tv?page=5)
- Recording that silently fails: reviewers say Outplayed "sometimes … will just not record despite me having the secondary monitor dashboard telling me it is" [S] [Trustpilot](https://www.trustpilot.com/review/outplayed.tv?page=5)
- Overwolf as a layer: Insights says "What is Overwolf and why is it on my PC?" is one of the most asked questions about Outplayed (vendor rival's words) [V] [Insights vs Outplayed](https://insights.gg/blog/insights-vs-outplayed)
- OBS and Vanguard: "something has changed and since it's not my obs setup I can only conclude it's vanguard. for now, game capture isn't working, try screen capture." (OBS forum, July 2026) and a reply that window capture "makes the game fell a bit stuttery". [V] [OBS forum](https://obsproject.com/forum/threads/game-capture-black-screen-with-only-league-of-legends.195861/)
- Riot replay changes: creator Necrit, quoted by Insider Gaming: "It seems like the issue is that people are vibecoding scripts that scrape the data off of 1000s of public replays…" Creators and coaches reacted angrily; Riot delayed the change to 26.21. [V] [Insider Gaming](https://insider-gaming.com/riot-delays-private-league-of-legends-replays/)
- Privacy and AI training: see Medal and Insights sections. I found no consumer complaint threads about this, only the policies themselves. [V] [Medal privacy](https://medal.tv/privacy), [Insights privacy](https://insights.gg/privacy)

## Gaps and opportunities for Greyscreen, ranked by impact

Impact is my judgement of how much it moves adoption or trust for a free, local, League-only recorder. "Cost" is a rough size.

| # | Opportunity | Why (evidence) | What to build | Cost |
|---|---|---|---|---|
| 1 | Measured performance proof | Ascent, Medal and Insights all publish or claim FPS numbers. Greyscreen has none, and its "near-zero FPS cost" line is unmeasured. | Run a fixed League test on real Windows hardware, record FPS, CPU, RAM and disk with and without Greyscreen, and publish the table and method on the site and in the README. | S |
| 2 | Storage management | "100 gb" complaint on Outplayed. LeagueRecord has age and size caps with favorites. Insights has a storage view. Greyscreen deletes by hand only. | Add max age and max total size settings with a favorites star, a disk-use bar and a "delete after N days unless favorite" default. | M |
| 3 | Death review | Ascent ships Death Replay. League players review deaths first. Greyscreen already marks deaths and cuts death clips. | Add a "Deaths" tab on the review screen: list each death with time, killer and a jump button, and autoplay the 15 s before it. | M |
| 4 | Manual clip hotkey and marker | Medal, Ascent and Outplayed have a hotkey. LeagueRecord has a marker hotkey. Greyscreen has an in-app button only. | Register a global hotkey (default configurable) that saves a clip now and drops a labeled marker on the timeline. | M |
| 5 | Match scoreboard and graphs | Medal and Ascent show a post-match recap. Players want context next to the video. Riot allows post-game analysis. | Store the final scoreboard and per-minute CS, gold and level from the Live Client API in `game.json` and show them beside the review timeline. No AI, no network. | M |
| 6 | Trim and export | Everyone else lets you trim and share. Greyscreen cuts only automatic clips. | Add a trim handle pair in review and "Save clip from selection" using the existing no-re-encode cutter. | M |
| 7 | Privacy as the headline | Medal names an AI lab as joint controller. Insights pays for footage for model training. Greyscreen has no account and no upload, but loads champion icons from Data Dragon, which is a network call. | Write a one-page privacy statement, list every network call (Data Dragon icons, update checks if added), add a setting to disable icon downloads and ship bundled icons, and say "no telemetry" only if code review confirms it. | S |
| 8 | Queue and mode control | LeagueRecord has ranked-only. Ascent, Medal, DOR cover TFT. Greyscreen records neither TFT nor replays and has no per-queue toggle. | Add "record only these queues" (Ranked, Normal, ARAM, Arena) and an opt-in for TFT with a basic timeline. | M |
| 9 | Signed binary and install trust | Unsigned exes show SmartScreen "Windows protected your PC" and Smart App Control can block them. Ascent moved to per-user installs without UAC. | Follow `docs/research/code-signing.md`: sign every release with the same publisher identity. | M |
| 10 | Vanguard and capture resilience as a selling point | OBS Game Capture broke after Vanguard changes (July 2026). Windows Graphics Capture does not hook the game. Ascent had League border and cursor bugs. | Keep `doctor` checks for capture, add a regression run on each Vanguard patch, and publish "what we tested" with the Windows version and League patch. | S |
| 11 | Voice and mic tracks | Medal and Ascent record named audio tracks (game, mic, Discord). LeagueRecord records system plus mic. Coaches want commentary. Greyscreen records game audio only. | Add an optional mic track and a Discord/voice option as separate AAC tracks in the MP4. | L |
| 12 | Replay story | Riot `.rofl` expires each patch and is rate limited. DOR already markets this. | Add a "Why record" page and a first-run note: a recording survives the patch. | S |

Opportunities I would not chase: AI coaching and win-probability models (Ascent has a trained model and a team; Riot's stance warns against in-game conclusions, post-game is fine but a thin model hurts trust), a social feed or cloud (against the product's pitch), and a mobile app.

## Risks

- **Riot policy on recorders.** Riot's third-party stance bans advice or hidden information during play and says approved apps built on official APIs "should rarely have an issue". It names no recorder rule. Post-game review is the pattern Riot says it wants: "we want to see you play the game first, then analyze and reflect later". [V] [Riot third-party stance](https://support-leagueoflegends.riotgames.com/hc/en-us/articles/225266848)
- **Vanguard.** Riot's FAQ says overlays and tools using the API, game client and in-game APIs should keep working, external memory readers will not, and there is no allow list. Greyscreen reads no game memory. [V] [Vanguard FAQ](https://www.riotgames.com/en/DevRel/vanguard-faq)
- **Memory-access block.** From 2026-10-06 Riot blocks game memory access by default for unknown tools and gives approved tools until April 2027. It says this does not cause bans by itself. Greyscreen is not affected, but Overwolf-based rivals may be. [V] [Riot support](https://support.riotgames.com/en-us/riot/performance/game-memory-access-removed-for-third-party-apps)
- **Local API changes.** The Live Client Data API is the single source of events. `docs/research/events.md` notes Atakhan and Feats of Strength were removed in patch 26.1, `HordeKill` was added, and the local API is not owned by DevRel ("cannot offer support or guarantee updates"). A change in event names or the cert would break detection. Keep parsing tolerant and keep the fake server tests.
- **Riot approval.** Overwolf requires Riot's third-party approval for its apps. For a standalone tool using only the local API I found no equivalent registration, but I did not search Riot's developer portal for one. [U]
- **Windows Graphics Capture limits.** Exclusive fullscreen cannot be captured, and Windows 10 shows a yellow border. Ascent also shipped a fix for a League yellow border. This is a support-cost risk, not a policy risk.
- **Competition.** Ascent is funded and moving fast (1.0 on 2026-11-12) and offers a free tier with cloud. Medal has 20M users and a mobile app. Greyscreen wins on a different axis (local, open, no account). It loses if players want sharing or stats first.
- **Privacy claims.** Saying "no upload" while fetching icons from Data Dragon is accurate for gameplay data but invites a "gotcha". State the exact network calls.
- **Dependencies.** Rune is a pre-release UI and the pin has a known resize-lag constraint (`AGENTS.md`). Real-hardware validation on Windows is still in progress per the README.
- **Unverified claims in this file.** Competitor numbers (FPS, RAM) are vendor claims. NVIDIA App and AMD League support, Mobalytics and Blitz details, Skill-Capped, and Reddit sentiment were not confirmed.
