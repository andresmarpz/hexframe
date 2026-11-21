# Sound effects

Source: the `soft` pack in `resources/audio/sounds/soft/` of https://github.com/anomalyco/hex, MIT licensed (`LICENSE-hex.txt`).

| File | Original | Plays when |
|---|---|---|
| `recording-started.wav` | `start.wav` | a game recording starts |
| `recording-saved.wav` | `stop.wav` | a game recording is saved |
| `clip-saved.wav` | `done.wav` | a clip is saved |
| `failed.wav` | `error.wav` | something fails |

Changes: trailing silence trimmed, a 6 ms fade-out added, gain set so each file has the same RMS (-24 dBFS over the sound, peaks between -10 and -14 dBFS), mono copied to stereo. Format: 48 kHz, 16-bit PCM WAV.

The files are embedded in the binary. At start the app writes them to the cache folder, because Rune's audio player opens files, not memory.
