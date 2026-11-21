Greyscreen

Greyscreen records your League of Legends games and saves clips of the highlights.

Install

1. Download GreyscreenSetup.exe and run it.
   A setup window opens. Choose Install: Greyscreen installs for your user (no admin needed), adds a Start Menu shortcut and starts.
   You can turn "Start with Windows" on or off in the setup window.
2. To uninstall, open Windows Settings > Apps, find Greyscreen and choose Uninstall.
   Your recordings and settings are kept.

greyscreen.exe in the same release is the same program. The updater inside Greyscreen downloads it, so you do not need it.

First run

1. Play a game (the Practice Tool is fine). Recordings are saved in the Greyscreen folder inside your Videos folder.
2. To test your setup without a game, open Home and choose "Test recording (10 s)".
3. For more detail, open a terminal and run: greyscreen-cli.exe doctor
   It prints your GPU, the hardware video encoders and the capture support.
   Then run: greyscreen-cli.exe record --seconds 10
   It records the primary monitor and system audio to greyscreen-test.mp4. Play the file to check it.
4. If anything fails, send back the files in %LOCALAPPDATA%\Greyscreen\logs

Set League to Borderless window mode. Greyscreen cannot see exclusive fullscreen.
