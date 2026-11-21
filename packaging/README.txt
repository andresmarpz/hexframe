Greyscreen

Greyscreen records your League of Legends games and saves clips of the highlights.

First run

1. Unzip this folder anywhere.
2. Open a terminal in the folder and run: greyscreen-cli.exe doctor
   It prints your GPU, the hardware video encoders and the capture support.
3. Run: greyscreen-cli.exe record --seconds 10
   It records the primary monitor and system audio to test.mp4. Play the file to check it.
4. Start greyscreen.exe, then play a game (the Practice Tool is fine).
   Recordings are saved in the Greyscreen folder inside your Videos folder.
5. If anything fails, send back the files in %LOCALAPPDATA%\Greyscreen\logs

Set League to Borderless window mode. Greyscreen cannot see exclusive fullscreen.
