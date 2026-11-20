# Research: game detection and events

Date of research: 2026-11-20. Scope: Windows League of Legends client, Live Client Data API, LCU, TLS, time mapping, process watching, policy, data model.

Legend: **VERIFIED** = seen in a Riot primary source or in a Riot-hosted file I downloaded and parsed. **COMMUNITY** = from community docs or code, not re-checked against a live game. **UNVERIFIED** = not confirmed by anything I could reach. I could not run a game from macOS, so no payload below was captured live by me.

## Summary

- The Live Client Data API (`https://127.0.0.1:2999/liveclientdata/...`) alone is enough for v1: it gives game start, all events, Riot IDs, game mode and win/loss. The LCU is not needed and is officially unsupported.
- Poll `/liveclientdata/eventdata` and `/liveclientdata/gamestats` about once per second. De-duplicate by `EventID`. Convert `EventTime` to recording time with a per-poll offset: `offset = mono_midpoint(gamestats request) - gameTime`, then `rec_time = EventTime + offset`.
- Detect the game by polling `CreateToolhelp32Snapshot` every 1 s for `League of Legends.exe`. Treat the API answering 200 on `/gamestats` as "game ready". Stop on `GameEnd` plus a short tail, or when the connection is refused or the process exits.
- TLS: the cert chain is signed by `riotgames.pem` (expires 2043-11-27, SHA-1, X.509 v1). The leaf cert's name is UNVERIFIED. `ureq` cannot skip only the hostname check, so use `rustls` 0.23 directly with a custom verifier that wraps the WebPKI verifier and forgives name errors. Fall back to loopback-only trust if chain checks fail. This is the main item to test on the Windows PC.
- Atakhan and Feats of Strength were removed in patch 26.1 (January 2026). `HordeKill` (voidgrub) was confirmed live on 2026-11-18. Use of local game APIs is allowed per Riot's Vanguard FAQ. Memory reading is blocked from 2026-10-06. Recording software is not mentioned by Riot.

## 1. Live Client Data API endpoint reference

Base URL `https://127.0.0.1:2999` (also `https://localhost:2999`). Local only, "for native applications". HTTPS only, GET only, JSON. VERIFIED from https://developer.riotgames.com/docs/lol (Game Client API section).

| Endpoint | Returns | Notes |
|---|---|---|
| `/liveclientdata/allgamedata` | Object with `activePlayer`, `allPlayers`, `events`, `gameData` | VERIFIED keys from Riot sample file. One request returns events and `gameTime` atomically. Large. |
| `/liveclientdata/activeplayer` | `abilities`, `championStats`, `currentGold`, `fullRunes`, `level`, `summonerName`, `riotId`, `riotIdGameName`, `riotIdTagLine` | VERIFIED field list. |
| `/liveclientdata/activeplayername` | JSON string, e.g. `"Riot Tuxedo#TXC1"` | VERIFIED from Riot docs: `GameName#TAG`. |
| `/liveclientdata/activeplayerabilities` | Abilities of the active player | |
| `/liveclientdata/activeplayerrunes` | Full rune list of the active player | |
| `/liveclientdata/playerlist` | Array of players | Fields below. Also accepts `?teamID=` (UNVERIFIED, not in the Riot page). |
| `/liveclientdata/playerscores?riotId=` | `{assists, creepScore, deaths, kills, wardScore}` | `riotId` param only; see Riot ID section. |
| `/liveclientdata/playersummonerspells?riotId=` | `summonerSpellOne`, `summonerSpellTwo` | |
| `/liveclientdata/playermainrunes?riotId=` | Keystone and rune trees | |
| `/liveclientdata/playeritems?riotId=` | Array of `{displayName, itemID, slot, ...}` | |
| `/liveclientdata/eventdata` | `{"Events": [...]}` | Optional `?eventID=N` (see below). |
| `/liveclientdata/gamestats` | `{gameMode, gameTime, mapName, mapNumber, mapTerrain}` | VERIFIED from Riot sample. |
| `/swagger/v2/swagger.json`, `/swagger/v3/openapi.json` | OpenAPI spec of the live server | VERIFIED listed on Riot page. Fetch these on the PC to confirm every field below. |

Example `/gamestats` (Riot sample file `liveclientdata_sample.json`, VERIFIED):

```json
{ "gameMode": "CLASSIC", "gameTime": 0.0, "mapName": "Map11", "mapNumber": 11, "mapTerrain": "Default" }
```

`/playerlist` entry fields (VERIFIED from Riot page and sample): `championName`, `isBot`, `isDead`, `items`, `level`, `position`, `rawChampionName`, `respawnTimer`, `runes`, `scores`, `skinID`, `summonerName`, `riotId`, `riotIdGameName`, `riotIdTagLine`, `summonerSpells`, `team` (`"ORDER"` or `"CHAOS"`). Note the Riot sample file is older: it only contains `summonerName`, not the three `riotId*` fields. The docs page says the three fields exist now.

### `eventID` query parameter

COMMUNITY (LeagueCustomLobby wiki): `eventID` is a sequence number; `/eventdata?eventID=N` returns events after `N`, `eventID=0` returns from the start. It is not on Riot's doc page, and I could not read the live swagger. UNVERIFIED. Do not depend on it: always fetch the full list and de-duplicate by `EventID`. The list is small (a few hundred entries at most). One community report claims the list is a rolling window (UNVERIFIED, single source); dedup by a seen-set handles both cases.

### Bandwidth

Riot's sample `allgamedata` with one player is 6.5 KB. With 10 players, items and runes I estimate 40 to 80 KB (UNVERIFIED estimate). `eventdata` is tens of bytes per event. `gamestats` is about 100 bytes. Polling `eventdata` plus `gamestats` once per second is under 20 KB/s late in a game, and is cheaper for the game process than serializing `allplayers` each second (impact on game frame time is UNVERIFIED).

## 2. Event reference (`/liveclientdata/eventdata`)

Response shape (VERIFIED): `{"Events": [ { "EventID": 0, "EventName": "GameStart", "EventTime": 0.0325561985373497 } ]}`. Every event has `EventID` (integer, increasing, GameStart is 0 in live games; the static sample file shows all zeros, ignore that), `EventName` (string) and `EventTime` (float, game seconds).

The Riot sample events file (`https://static.developer.riotgames.com/docs/lol/liveclientdata_events.json`, downloaded and parsed) does not contain `FirstBlood`, `GameEnd`, `HordeKill`, `InhibRespawningSoon`, `InhibRespawned` or `AtakhanKill`. Fields for those come from community docs.

| EventName | Extra fields | Source | Example payload |
|---|---|---|---|
| `GameStart` | none | VERIFIED | `{"EventID":0,"EventName":"GameStart","EventTime":0.03}` |
| `MinionsSpawning` | none | VERIFIED | `{"EventID":1,"EventName":"MinionsSpawning","EventTime":65.0}` |
| `FirstBlood` | `Recipient` (name of killer) | COMMUNITY (do-something.jp field list) | `{"EventID":8,"EventName":"FirstBlood","EventTime":181.2,"Recipient":"Faker"}` |
| `ChampionKill` | `KillerName`, `VictimName`, `Assisters` (array of names, may be empty) | VERIFIED | `{"EventID":9,"EventName":"ChampionKill","EventTime":181.2,"KillerName":"Faker","VictimName":"Chovy","Assisters":["Keria"]}` |
| `Multikill` | `KillerName`, `KillStreak` (int 2 to 5) | VERIFIED | `{"EventID":12,"EventName":"Multikill","EventTime":900.1,"KillerName":"Faker","KillStreak":2}` |
| `Ace` | `Acer`, `AcingTeam` (`"ORDER"`/`"CHAOS"`) | VERIFIED | `{"EventID":40,"EventName":"Ace","EventTime":1500.0,"Acer":"Faker","AcingTeam":"ORDER"}` |
| `FirstBrick` | `KillerName` | VERIFIED | `{"EventID":15,"EventName":"FirstBrick","EventTime":600.0,"KillerName":"Faker"}` |
| `TurretKilled` | `TurretKilled` (structure id), `KillerName`, `Assisters` | VERIFIED | `{"EventName":"TurretKilled","TurretKilled":"Turret_T2_L_03_A","KillerName":"Faker","Assisters":["Keria"],...}` |
| `InhibKilled` | `InhibKilled` (e.g. `Barracks_T2_R1`), `KillerName`, `Assisters` | VERIFIED | `{"EventName":"InhibKilled","InhibKilled":"Barracks_T2_R1","KillerName":"Faker","Assisters":[],...}` |
| `InhibRespawningSoon` | `InhibRespawningSoon` (inhib id) | COMMUNITY | field value is the inhib name; no killer |
| `InhibRespawned` | `InhibRespawned` (inhib id) | COMMUNITY | same |
| `DragonKill` | `DragonType`, `Stolen` (string `"True"`/`"False"`), `KillerName`, `Assisters` | VERIFIED shape | `{"EventName":"DragonKill","DragonType":"Earth","Stolen":"False","KillerName":"Faker","Assisters":[],...}` |
| `HeraldKill` | `Stolen`, `KillerName`, `Assisters` | VERIFIED | |
| `BaronKill` | `Stolen`, `KillerName`, `Assisters` | VERIFIED | |
| `HordeKill` | assume `KillerName`, `Assisters` (and maybe `Stolen`) | Name VERIFIED live 2026-11-18 by a third-party project (Practice Tool, GitHub RyeRaven/lol-broadcast-hud issue 4); fields UNVERIFIED | One event per grub. Fields other than `EventID` were identical for repeat kills by one player. |
| `AtakhanKill` | unknown | COMMUNITY: listed as "valid through season 2025" | Atakhan was removed in patch 26.1 (2026-01-07). Keep parsing it as `Unknown` and ignore. |
| `GameEnd` | `Result` (`"Win"` or `"Lose"`) | Field name COMMUNITY (do-something.jp lists `Result`). Values `Win`/`Lose` COMMUNITY (single GitHub issue, not captured live). UNVERIFIED | `{"EventName":"GameEnd","EventTime":1850.2,"Result":"Win"}` |

Notes:

- `DragonType` documented values: `Water`, `Fire`, `Air`, `Earth`, `Elder` (COMMUNITY). `Hextech` and `Chemtech` also exist as dragon types in game (UNVERIFIED in the API). Treat `DragonType` as an open string.
- `Stolen` is a string in Riot's sample, not a JSON bool. Parse both.
- Structure ids encode team: `T1` is ORDER (blue), `T2` is CHAOS (red). `L`, `C`, `R` are top, mid, bot. `01` to `03` are outer, inner, inhibitor turrets; `04`/`05` are nexus turrets. `Barracks_T2_L1` is the CHAOS top inhibitor. COMMUNITY (LeagueCustomLobby wiki). The team of the destroyed structure is the team in the id. The killer's team is the opposite.
- `KillerName` is not always a player. It can be a minion (`Minion_T100L0S...`), a turret (`Turret_T2_L_03_A`), or a jungle monster. Always look the name up in `/playerlist`; if absent, it is an environment kill.
- Observer mode: Japanese community doc says some events (dragon, void grub, herald, baron) are not available from the API in some spectate modes. UNVERIFIED which.
- Also reported by Overwolf for game flow: `announcer` events and `jungle_camps` exist in Overwolf's own layer, not in this API. Ignore.

## 3. Riot ID changes

VERIFIED from the Riot docs page: endpoints that returned `summonerName` now also return `riotId`, `riotIdGameName`, `riotIdTagLine`. Endpoints that took `summonerName` as a parameter now accept only `riotId`. Lookup order for the parameter is `riotId`, then `riotIdGameName`, then `summonerName`.

- `/activeplayername` returns `"GameName#TAG"`. VERIFIED in the Riot docs example.
- `KillerName`, `VictimName`, `Assisters`, `Recipient`, `Acer`: the Riot sample file still shows a bare name (`"Riot Tuxedo"`), and a community doc writes `<riotIdGameName>` as the placeholder. So: game name without tag. COMMUNITY, not captured live. Implement tolerant matching: strip anything from the last `#`, then compare to `riotIdGameName`, then `summonerName`.
- Ambiguity: two players with the same game name and different tags would have identical event names. UNVERIFIED how Riot disambiguates. Rare. If the ambiguity touches me, mark the event `me_ambiguous` and resolve with a kills/deaths delta from `/playerscores`.
- Bots: `isBot: true`, names are champion names.

Robust "me" identification, in order:

1. `GET /activeplayername` returns `"Name#TAG"`. Split at the last `#`.
2. `GET /playerlist`. Find the entry whose `riotId == activeplayername`. Fall back to `riotIdGameName == name`, then `summonerName == name`, case-insensitive.
3. Store `me = { game_name, tag, team, champion_name }`. `team` (`ORDER`/`CHAOS`) is the key for own/enemy objective classification.
4. Refresh the `team` and `me` lookup once after `GameStart`, since the list can be partial while loading.

If `/activeplayername` does not return 200 after `/gamestats` does, assume spectator or replay and skip the session (replay detail below).

## 4. Game state, loading, modes

- Not in a game: the port is closed. Connection refused. COMMUNITY (LeagueCustomLobby wiki, gist author treats refused as "game gone").
- Loading screen: UNVERIFIED. A gist commenter saw `404` at game start; treat any non-200 or non-JSON as "not ready" and retry. Treat as ready when `/gamestats` returns 200 with a numeric `gameTime`.
- Mid-game attach (reconnect or app restart): the first events response already contains history. Process all unseen `EventID`s but mark events older than the recording start as `before_recording` (not placed on the timeline).
- `/gamestats` modes: `gameMode` examples seen: `CLASSIC` (VERIFIED from sample), `PRACTICETOOL`, `ARAM`, `CHERRY` (Arena), `TFT`. Only `CLASSIC` is VERIFIED here. The rest are COMMUNITY/UNVERIFIED. Summoner's Rift is `Map11` / `mapNumber 11` (VERIFIED). Other map numbers (Howling Abyss 12, Arena 30, TFT 22) are from memory and UNVERIFIED.
- TFT also runs in `League of Legends.exe`. Ignore any session where `gameMode == "TFT"`. Recommend ignoring unknown modes only if they lack a `GameStart` event. Overwolf reports `game_mode` values `lol`, `tft`, `arena`, `classic` (not the same strings).
- Practice tool: events are produced (HordeKill was confirmed there). Game can be paused in practice tool, so `gameTime` can stall.
- Arena (`CHERRY`): no turrets or dragons. Kills and deaths work. `GameEnd.Result` meaning with placements is UNVERIFIED.
- Replays and spectator: the same port 2999 serves the Replay API (`/replay/playback`, `/replay/game`...), COMMUNITY/UNVERIFIED in this research. In spectate or replay, `activeplayer` is not meaningful for "me". Probe `GET /replay/playback`: 200 means replay, skip recording. If it 404s and `/activeplayername` fails, assume spectator, skip.

## 5. TLS

Facts:

- Riot publishes the root as `https://static.developer.riotgames.com/docs/lol/riotgames.pem`. I downloaded and parsed it:
  - Subject and issuer: `C=US, ST=California, L=Santa Monica, O=Riot Games, OU=LoL Game Engineering, CN=LoL Game Engineering Certificate Authority, emailAddress=gametechnologies@riotgames.com`
  - X.509 **version 1** (no extensions, so no `basicConstraints`), RSA 2048, signature `sha1WithRSAEncryption`
  - Not before 2013-12-04, **not after 2043-11-27**
  - SHA-256 fingerprint `CA:8C:9D:32:5B:4C:DC:46:4C:6C:94:A5:85:C8:5E:91:EC:23:D4:0B:A5:BF:3A:E2:82:2B:95:1A:4A:50:4E:A3`
  - Valid as of 2026. Note: a separate client-side certificate expired in January 2026 and caused a login outage; press reports say Riot renewed it for a long term. That event is about the Riot Client, not this root. Do not hard-code a leaf cert.
- The leaf cert served on 2999 is signed by this CA. Its CN, SAN and signature algorithm are UNVERIFIED: I found no source and could not connect. A Rust crate note says the LCU's cert CN does not match `127.0.0.1`. The `irelia` crate (LCU client) uses `rustls` with only this root and default hostname checks, which suggests that cert works with hostname checks, but that is for the LCU, not 2999, and it is indirect evidence.
- `ureq` 3.5.2 `TlsConfig` supports `RootCerts::Specific`, `PlatformVerifier`, `WebPki`, and `disable_verification()` (turns off everything). It has no hostname-only skip and no custom verifier hook. VERIFIED from docs.rs.
- `native-tls` 0.2.18 has `danger_accept_invalid_hostnames`, but on Windows it uses SChannel, which may reject a v1 CA and SHA-1 signed chain. Not recommended.
- `reqwest` blocking works but pulls in tokio. Rejected.

First thing to run on the Windows PC during a game (also fills the UNVERIFIED items):

```
curl.exe -k https://127.0.0.1:2999/swagger/v3/openapi.json -o openapi.json
openssl s_client -connect 127.0.0.1:2999 -showcerts </dev/null | openssl x509 -noout -subject -issuer -ext subjectAltName -text
curl.exe --cacert riotgames.pem https://127.0.0.1:2999/liveclientdata/gamestats
```

Recommended Rust approach (no decisions left):

- Crate: `rustls = { version = "0.23.46", default-features = false, features = ["ring", "std", "tls12", "logging"] }`, `rustls-pki-types = "1"` (PEM parsing via `CertificateDer::from_pem_slice`). `ring` cross-compiles to `x86_64-pc-windows-msvc` from macOS with `cargo-xwin`; avoid `aws-lc-rs` (needs cmake and NASM).
- Write a tiny blocking HTTP/1.1 GET over `rustls::StreamOwned<ClientConnection, TcpStream>` (about 50 lines): send `GET {path} HTTP/1.1\r\nHost: 127.0.0.1:2999\r\nConnection: close\r\n\r\n`, read to EOF, split headers and body, handle `Content-Length` and `Transfer-Encoding: chunked`. One connection per request. Connect to the literal IP `127.0.0.1`, never a hostname. Set 1.5 s connect, read and write timeouts. Use the server name `ServerName::IpAddress(127.0.0.1)`.
- Verifier: implement `rustls::client::danger::ServerCertVerifier` wrapping `WebPkiServerVerifier::builder_with_provider(roots, provider).build()` where `roots` contains only `riotgames.pem`. In `verify_server_cert`, call the inner verifier. If it returns `Err(Error::InvalidCertificate(CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. }))`, return `Ok(ServerCertVerified::assertion())`. Chain, expiry and signature are still checked. Delegate `verify_tls12_signature`, `verify_tls13_signature` and `supported_verify_schemes` to the inner verifier.
- If the first test on the PC shows chain errors other than name (`UnsupportedSignatureAlgorithm`, `BadSignature`, `UnknownIssuer`, `CaUsedAsEndEntity`), because rustls-webpki rejects SHA-1 signatures and the root is a v1 cert, switch the verifier to loopback-trust: accept any certificate, but only when the peer address is `127.0.0.1`. This is acceptable: the endpoint is loopback only, Riot's own docs say "ignore the errors", and the data is read-only. Log the chosen mode once at startup. Make this a runtime setting, not a rebuild.
- Plan B if `rustls` cannot complete the handshake at all (for example the server offers only static-RSA key exchange, which rustls does not implement; UNVERIFIED): `native-tls` 0.2.18 with `add_root_certificate`, `danger_accept_invalid_hostnames(true)`, then `danger_accept_invalid_certs(true)` if the chain fails.

## 6. Mapping `EventTime` to recording time

Definitions: `rec_t0` is the monotonic timestamp that the video pipeline treats as time 0. Use the same clock as capture (QPC; `std::time::Instant` is QPC-based on Windows). If capture exposes a QPC value for frame 0, convert it to `Instant` terms once.

Per poll (1 Hz):

1. `a = Instant::now()`, `GET /eventdata`, parse events.
2. `b = Instant::now()`, `GET /gamestats`, `c = Instant::now()`, parse `gameTime`.
3. `mid = b + (c - b) / 2`. `offset = (mid - rec_t0).as_secs_f64() - gameTime`.
4. For each new event (`EventID` not seen): `rec_time = EventTime + offset`. Because `eventdata` is fetched before `gamestats`, `EventTime <= gameTime` holds, so no event lands in the future.
5. Drop events where `rec_time < 0` (before recording started).

Why per-poll offset and no long-term fit:

- Pause (practice tool, replay pause): `gameTime` stops while the video continues, so `offset` grows. Events after a pause use the fresh offset and land correctly. An event that occurred in the same second as the pause start is off by at most 1 s. Accept.
- Replay speed or lag: the game clock rate differs from 1.0, so any fitted constant would drift. A fresh offset per poll avoids this.
- Error budget: the request round trip on loopback is about 1 to 5 ms. Using the midpoint gives under 5 ms error. Whether `gameTime` is smooth or updates only per server tick is UNVERIFIED. If it is stepped (about 33 ms), the error is still negligible.
- Smoothing: do not smooth. If `gameTime` goes backwards (new process attached, reconnect), keep working; the offset recomputes. If `gamestats` fails but `eventdata` succeeded, reuse the last offset, and flag the events `approximate`.
- Optional sanity check: log `offset` per poll. Warn if it changes by more than 0.25 s between polls without a pause.

Cheaper alternative: `GET /allgamedata` once per poll gives events and `gameData.gameTime` atomically; offset uses the request midpoint. Rejected for v1 because of payload size and unknown game-side cost.

## 7. Detecting game start and end

Processes:

- `League of Legends.exe`: the game. Also used for TFT. Path normally `C:\Riot Games\League of Legends\Game\League of Legends.exe`.
- `LeagueClient.exe`, `LeagueClientUx.exe`, `LeagueClientUxRender.exe`: the client. Open before and after games. Do not use for game detection.

Watching for process start in Rust:

- Recommended: poll `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` plus `Process32FirstW`/`Process32NextW` every 1 s using the `windows` crate (0.62.2, features `Win32_System_Diagnostics_ToolHelp`, `Win32_Foundation`). Cost is about 1 ms per scan. Compare `szExeFile` case-insensitively with `League of Legends.exe`. No admin needed and nothing touches the game process.
- WMI process-start events (`Win32_ProcessStartTrace`) require admin; `__InstanceCreationEvent WITHIN n` polling works unprivileged but adds a COM dependency and is no cheaper. Rejected.
- `sysinfo` 0.40.4 works but refreshes far more data per call. Rejected.
- Do not read other processes' command lines (`wmic`, PEB reads): this reads game-process memory, which the Vanguard rules treat as suspect, and `wmic` is being removed from Windows 11. Use the API instead.

State machine:

1. `Idle`: scan every 1 s. When the process exists, go to `Waiting`.
2. `Waiting`: every 1 s `GET /gamestats`. Connection refused, non-200 or bad JSON means stay. On 200 with numeric `gameTime`: check `gameMode` is not `TFT` and not a replay (probe `/replay/playback`), identify "me", set `rec_t0` consideration (start recording now, or earlier if the user wants the loading screen; recording the process window from process start is a capture-side decision). Go to `Recording`.
3. `Recording`: poll per section 6. Stop when any of: `GameEnd` event seen plus a 5 s tail (so the Nexus explosion is on tape); connection refused for 3 consecutive polls (game closed); process gone; hard cap of 3 h. Then go to `Cooldown` until the process is gone, so one game is never recorded twice.
4. `Cooldown` until `League of Legends.exe` disappears, then `Idle`.

What happens after `GameEnd`: the game process stays alive on the end-of-game screen until the player continues or the client closes it (COMMUNITY, e.g. a script kills the process on `GameEnd`; how long the API keeps answering after `GameEnd` is UNVERIFIED). Never use process exit alone as the stop signal.

Remake or early surrender: expect `GameEnd` with `Result` (UNVERIFIED for remakes). If the process exits with no `GameEnd`, tag the recording `result: unknown`.

## 8. LCU (League Client API)

Not needed for v1. Riot: LCU is "not officially supported for third party applications"; the Vanguard FAQ says apps built on the LCU and in-game APIs "are still expected to work" but Developer Relations does not own them and gives no guarantee. Using the LCU requires registering the product via the Developer Portal (VERIFIED from Riot policy page). Reference only:

- Lockfile: file `lockfile` in the League install directory (usually `C:\Riot Games\League of Legends\`, can differ). One line: `LeagueClient:<pid>:<port>:<password>:https`. COMMUNITY (hextechdocs.dev). The install directory can be derived from the image path of `LeagueClientUx.exe` via `QueryFullProcessImageNameW`, without reading process memory.
- Auth: HTTP Basic, user `riot`, password from the lockfile (changes on every client start). Base URL `https://127.0.0.1:<port>`. Same `riotgames.pem` and the same TLS concerns.
- `GET /lol-gameflow/v1/gameflow-phase` returns a JSON string. Enum (COMMUNITY, from a third-party schema dump about 9 months old, not checked live): `None`, `Lobby`, `Matchmaking`, `CheckedIntoTournament`, `ReadyCheck`, `ChampSelect`, `GameStart`, `FailedToLaunch`, `InProgress`, `Reconnect`, `WaitingForStats`, `PreEndOfGame`, `EndOfGame`, `TerminatedInError`. Normal order: `None, Lobby, Matchmaking, ReadyCheck, ChampSelect, GameStart, InProgress, WaitingForStats, PreEndOfGame, EndOfGame`.
- `GET /lol-gameflow/v1/session`: includes `phase` and `gameData`; a community answer places the mode at `gameData.queue.gameMode`. `gameData.gameId`, queue id and map fields are UNVERIFIED (no source found).
- `GET /lol-end-of-game/v1/eog-stats-block`: UNVERIFIED (no source found in this research).
- The LCU would add: exact queue id and name (to tell ranked from normal, and TFT cleanly), a `gameId` for match history, and end-of-game stats. Defer to v2 behind a feature flag.

## 9. Riot policy

- The Live Client Data API is documented by Riot as "served by the game client over HTTPS on your own PC, at 127.0.0.1:2999, during a game" for native applications. VERIFIED, https://developer.riotgames.com/docs/lol.
- Riot Vanguard FAQ (https://www.riotgames.com/en/DevRel/vanguard-faq): apps that use the API, game client and in-game APIs should keep working; external tools reading memory will no longer work; overlays using the APIs continue; no allow list. OBS and recording software are not mentioned.
- Riot support, posted 2026-09-29, effective 2026-10-06: unknown third-party apps are blocked from reading game memory by default across League, TFT and VALORANT; official APIs are the only allowed mechanism; existing approved tools have a grace period to April 2027; the change "does not result in bans by itself". VERIFIED, https://support.riotgames.com/en-us/riot/performance/game-memory-access-removed-for-third-party-apps. Greyscreen uses no memory access, so it is in the allowed category.
- Developer policy (https://developer.riotgames.com/policies/general): "All products must be registered in, and audited by Riot Games through the Developer Portal." Applies to products that use Riot APIs; the page names the LCU case explicitly and does not mention the Live Client Data API by name. Registration is required if the LCU is used. For a free personal tool that uses only the Live Client Data API it is not clear that registration is required; ask Riot DevRel before any public distribution or monetization (UNVERIFIED either way). Monetization requires an Approved or Acknowledged status.
- Game integrity rules (same policy): do not use information not present in the game client for a competitive edge, do not de-anonymize hidden players, do not alter the goal of the game. A post-game recorder with an event timeline for the user's own games shows no information the player could not see. Do not show live enemy-ability or hidden information.
- Overwolf: apps run on Overwolf when they follow Riot's third-party policy; Overwolf's compliance page prohibits power-spike notifications, enemy cooldown tracking, and aggregated League Classic data display. Overwolf's list of allowed data is not authoritative for a native app, but its limits match the game integrity rules. Overwolf is not required.

## 10. Recommendation for v1

Dependencies:

| Crate | Version | Use |
|---|---|---|
| `rustls` | 0.23.46 (features `ring`, `std`, `tls12`, `logging`; no defaults) | TLS client and custom verifier |
| `rustls-pki-types` | 1 | PEM and DER types |
| `serde`, `serde_json` | 1 | JSON |
| `windows` | 0.62.2 (features `Win32_Foundation`, `Win32_System_Diagnostics_ToolHelp`) | Process scan |
| `crossbeam-channel` or `std::sync::mpsc` | latest | Event output to the recorder |

Embed `riotgames.pem` with `include_bytes!`. Pin: check at startup that the SHA-256 of the DER equals `CA:8C:9D:32:...:4E:A3` (fingerprint above).

Algorithm, in order:

1. Loop every 1 s: scan processes for `League of Legends.exe`. If absent, sleep and repeat.
2. When present, loop every 1 s: `GET /liveclientdata/gamestats`. Not 200 means continue. If `gameMode == "TFT"` or `/replay/playback` returns 200, set `Cooldown` and ignore the session.
3. On first ready: `GET /liveclientdata/activeplayername`, then `GET /liveclientdata/playerlist`. Resolve `me` (section 3). Emit `RecordingStart` to the recorder, and set `rec_t0` to the monotonic time at which recording actually began.
4. Every 1 s: `GET /eventdata`, then `GET /gamestats`, compute `offset` (section 6). Parse unseen `EventID`s into `GameEvent`. Refresh `playerlist` every 10 s (team and dead state only; not needed for classification after the first fetch except for new names) or whenever an event names an unknown player.
5. On `GameEnd`: emit the event, then emit `RecordingStop` after 5 s. Also stop on 3 consecutive refused connections or process exit.
6. Wait for the process to exit before arming again.

Errors: log every non-200 and parse failure at debug level. Never panic on an unknown `EventName` or missing field; deserialize with `#[serde(default)]` and map unknown names to `GameEvent::Unknown`.

## 11. Rust data model

```rust
pub struct Me { pub game_name: String, pub tag: String, pub team: Team, pub champion: String }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Team { Order, Chaos }

pub struct TimedEvent {
    pub id: u32,
    pub game_secs: f64,
    pub rec_secs: f64,          // EventTime + offset
    pub approximate: bool,      // offset reused
    pub kind: GameEvent,
}

pub enum GameEvent {
    GameStart,
    MinionsSpawning,
    FirstBlood { recipient: String },
    ChampionKill { killer: String, victim: String, assisters: Vec<String> },
    Multikill { killer: String, streak: u8 },
    Ace { acer: String, acing_team: Team },
    FirstBrick { killer: String },
    TurretKilled { turret: String, killer: String, assisters: Vec<String> },
    InhibKilled { inhib: String, killer: String, assisters: Vec<String> },
    InhibRespawningSoon { inhib: String },
    InhibRespawned { inhib: String },
    DragonKill { dragon_type: String, stolen: bool, killer: String, assisters: Vec<String> },
    HeraldKill { stolen: bool, killer: String, assisters: Vec<String> },
    BaronKill { stolen: bool, killer: String, assisters: Vec<String> },
    HordeKill { killer: String, assisters: Vec<String> },
    GameEnd { result: GameResult },
    Unknown { name: String },
}

pub enum GameResult { Win, Loss, Unknown }
```

Me-centric classification (derive from `GameEvent` plus `Me` plus a name-to-team map from `playerlist`):

```rust
pub enum Classified {
    MyKill, MyDeath, MyAssist,
    MyMultikill { streak: u8 },
    MyFirstBlood,
    AllyKill, EnemyKill,            // champion kills not involving me
    TeamObjective { objective: Objective },   // killer on my team
    EnemyObjective { objective: Objective },
    TeamStructureDestroyed { kind: Structure }, // I or allies took an enemy structure
    EnemyStructureDestroyed { kind: Structure }, // enemy took my team's structure
    MyAce, EnemyAce,
    GameWon, GameLost,
    Ignored,
}
pub enum Objective { Dragon(String), Herald, Baron, VoidGrub }
pub enum Structure { Turret, Inhibitor }
```

Rules:

- `ChampionKill`: `killer` matches me gives `MyKill`; `victim` matches me gives `MyDeath`; me in `assisters` gives `MyAssist`. One event can be both, so return a list.
- `Multikill` with killer = me gives `MyMultikill`; others are ignored.
- Objective events (`DragonKill`, `HeraldKill`, `BaronKill`, `HordeKill`): team of `killer` from the map; same as `me.team` gives `TeamObjective`, otherwise `EnemyObjective`. Unknown killer: `Ignored`.
- Structures: team comes from the structure id (`T1` is Order, `T2` is Chaos). Structure team equal to `me.team` means an enemy took our structure: `EnemyStructureDestroyed`. Otherwise `TeamStructureDestroyed`.
- `Ace`: `acing_team == me.team` gives `MyAce`, else `EnemyAce`.
- `GameEnd`: `Win` gives `GameWon`, `Lose` gives `GameLost`. Unknown result: derive from the nexus inhibitor events if present, else `Unknown`.
- Name matching is case-insensitive on `game_name` after stripping any `#tag`.

## 12. Pitfalls

- `Stolen` is the string `"True"`/`"False"` in Riot's sample. Parse as string and bool.
- `EventID` from the static sample file is all 0. Live `EventID` starts at 0 for `GameStart` and rises. Never use `EventID == 0` as "no events".
- Names in events may omit the tag; `/activeplayername` includes it. Strip on both sides before comparing.
- `KillerName` can be a minion, turret or monster. Do not assume it is in `playerlist`.
- `gameTime` pauses in practice tool and replays; never derive recording time from wall-clock minus game start.
- During loading, responses may be 404 or empty, not just connection refused. Gate on a numeric `gameTime`.
- TFT uses `League of Legends.exe` and port 2999 too. Filter on `gameMode`.
- The process can outlive `GameEnd`. Never use process exit as the only stop.
- Reconnect or app restart mid-game produces history that is already past. Do not place events whose `rec_time < 0`.
- Riot's certificate root is X.509 v1 with SHA-1. A strict `rustls` or SChannel chain check may fail for reasons unrelated to name. Test on the PC before shipping.
- Do not use `ureq` for 2999: no hostname-only bypass. Do not use `reqwest` (tokio).
- `wmic` is deprecated; do not shell out to it.
- Do not read process memory, ever, including command lines of Riot processes.
- Voidgrub: one `HordeKill` per grub. A fast multi-kill by one player yields several events with identical fields except `EventID`. Dedup on `EventID`, not on content. A third-party project hit this bug.
- Atakhan and Feats of Strength no longer exist (patch 26.1). Do not build UI for them.
- Public docs list `GameEnd.Result` only through community sources; confirm the exact strings on the PC and log unknown values.

## 13. Open items to verify on the Windows PC

1. Leaf cert subject, SAN, signature algorithm on 2999 (commands in section 5).
2. `GameEnd` payload with `Result` for win, loss and remake.
3. `HordeKill`, `FirstBlood`, `InhibRespawningSoon`, `InhibRespawned` payloads.
4. Whether `KillerName` in events includes `#tag` in 2026.
5. API behaviour while loading and for 10+ seconds after `GameEnd`.
6. Whether `/eventdata?eventID=N` filters.
7. Behaviour in replay, spectator, Arena (`CHERRY`), ARAM and Practice Tool.
8. Whether the server keeps the connection alive (keep-alive would cut handshake cost).

## Sources

- https://developer.riotgames.com/docs/lol (Game Client API, League Client API, certificate link)
- https://developer.riotgames.com/policies/general (product registration, game integrity)
- https://static.developer.riotgames.com/docs/lol/riotgames.pem (downloaded, parsed with openssl)
- https://static.developer.riotgames.com/docs/lol/liveclientdata_sample.json (downloaded)
- https://static.developer.riotgames.com/docs/lol/liveclientdata_events.json (downloaded)
- https://www.riotgames.com/en/DevRel/vanguard-faq
- https://support.riotgames.com/en-us/riot/performance/game-memory-access-removed-for-third-party-apps
- https://senseos.gg/guides/league-of-legends/third-party-apps-after-memory-block/ (secondary summary)
- https://hextechdocs.dev/getting-started-with-the-lcu-api/ (lockfile format, auth, cert)
- https://github.com/XHXIAIEIN/LeagueCustomLobby/wiki/client:--game-client (event list, `eventID`, structure ids)
- https://do-something.jp/lol-liveclienapi/ (event names and field list, Atakhan note)
- https://github.com/RyeRaven/lol-broadcast-hud/issues/4 (`HordeKill` confirmed live 2026-11-18)
- https://gist.github.com/NanoAi/cbc79b2fa694a079bdc2588c46746681 (GameEnd polling, process kill)
- https://esports.gg/news/league-of-legends/atakhan-feats-of-strength-league-of-legends-2026/ (Atakhan removed in 26.1)
- https://docs.rs/ureq/latest/ureq/tls/struct.TlsConfig.html and https://docs.rs/ureq/latest/ureq/tls/enum.RootCerts.html
- https://docs.rs/crate/irelia/0.11.2/source/src/tls/rustls.rs and https://docs.rs/crate/irelia/0.11.2/source/src/tls/nativetls.rs (how an LCU crate pins the Riot root)
- https://lobehub.com/skills/hoangvu12-ame-lcu-api (gameflow phase enum, community)
- https://dev.overwolf.com/ow-native/live-game-data-gep/supported-games/league-of-legends/ and https://dev.overwolf.com/ow-native/guides/game-compliance/riot-games/ (Overwolf layer and compliance)
- https://crates.io/api/v1/crates/{ureq,rustls,native-tls,windows,sysinfo} (versions as of 2026-11-20)
