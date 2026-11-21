Greyscreen

Greyscreen records your League of Legends games and saves clips of the highlights.

Install

1. Download greyscreen.exe and run it.
   It installs itself for your user (no admin needed), adds Greyscreen to the Start Menu and starts.
2. To uninstall, open Windows Settings > Apps, find Greyscreen and choose Uninstall.
   Your recordings and settings are kept.

First run

1. Play a game (the Practice Tool is fine). Recordings are saved in the Greyscreen folder inside your Videos folder.
2. In Settings you can turn on "Start with Windows" to keep Greyscreen in the tray.
3. To test your setup, open a terminal and run: greyscreen-cli.exe doctor
   It prints your GPU, the hardware video encoders and the capture support.
   Then run: greyscreen-cli.exe record --seconds 10
   It records the primary monitor and system audio to test.mp4. Play the file to check it.
4. If anything fails, send back the files in %LOCALAPPDATA%\Greyscreen\logs

Set League to Borderless window mode. Greyscreen cannot see exclusive fullscreen.
