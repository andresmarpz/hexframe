import { discordUrl, githubUrl, issuesUrl } from "./copy";

export interface FaqItem {
  id: string;
  q: string;
  a: string[];
  steps?: string[];
}

export interface FaqGroup {
  id: string;
  title: string;
  intro?: string;
  items: FaqItem[];
}

export const faqPage = {
  title: "FAQ",
  description: "Answers about Greyscreen: Vanguard, FPS, where recordings go, why it is not recording, updates and privacy.",
  intro: "Short answers. If yours is not here, see Get help at the bottom.",
};

export const faqGroups: FaqGroup[] = [
  {
    id: "safety",
    title: "Safety and speed",
    items: [
      {
        id: "vanguard",
        q: "Is it safe with Vanguard? Will I get banned?",
        a: [
          "Greyscreen stays out of the game. It records the League window with Windows' built-in screen capture and League's own sound with Windows' per-app audio capture. It reads match events from Riot's Live Client Data API, which League serves on your own PC at 127.0.0.1.",
          "It does not read game memory, inject code or hook into League, and it does not change game files. You can check this in the source on GitHub.",
          "Riot does not publish a list of approved recorders and changes its anti-cheat from time to time, so we cannot promise anything on Riot's behalf.",
        ],
      },
      {
        id: "fps",
        q: "Does it cost FPS?",
        a: [
          "Greyscreen encodes video on your graphics card's hardware H.264 encoder, not on the CPU, so the cost should be small. We have not measured it on many PCs yet, so we cannot give you a number.",
          "If your FPS drops, lower the quality or the frame rate in Settings > Recording (for example Balanced at 30 fps). After a game, the Recording health button in the review toolbar shows dropped frames and sound gaps. If it looks bad, tell us with Send feedback and name your GPU.",
        ],
      },
    ],
  },
  {
    id: "recordings",
    title: "Recordings",
    items: [
      {
        id: "storage",
        q: "Where are my recordings, and how much space do they use?",
        a: [
          "Each game gets its own folder in Videos\\Greyscreen in your user folder. It holds game.mp4, a game.json file with the event timeline, and a clips folder. Change the folder in Settings > Storage.",
          "Size depends on resolution, frame rate and quality. At the defaults (1080p, 60 fps, High) expect roughly 55 MB per minute, about 2 GB for a 35-minute game. This is an estimate. Settings > Recording shows one for your settings.",
          "To cap the space, set a storage limit in Settings > Storage (50, 100, 250 or 500 GB). It is off by default. After each game, and when Greyscreen starts, it deletes the oldest game videos first (their timeline and clips stay), then whole old games. It never deletes the game being recorded.",
          "Delete a game in the library and it goes to the Recycle Bin.",
        ],
      },
      {
        id: "low-disk",
        q: "My disk is nearly full",
        a: [
          "A full drive can make a recording fail or stop early. Free some space, move the recordings folder to another drive in Settings > Storage, or set a storage limit so Greyscreen deletes the oldest videos for you.",
        ],
      },
      {
        id: "death-review",
        q: "How do death review, clips and F9 work?",
        a: [
          "Open Games, pick a match, and click any marker on the timeline to jump to that moment. Press D to review your deaths one by one, from 12 seconds before to 3 seconds after. Tag the cause and add a note for each.",
          "Greyscreen saves clips around your kills, deaths, multikills, objectives and aces by default. Turn each kind on or off in Settings > Clips.",
          "Press F9 while a game is recording to save the last 30 seconds as a clip. F9 works in game. Change it in Settings > Keybinds.",
        ],
      },
    ],
  },
  {
    id: "not-recording",
    title: "Why isn't it recording?",
    intro:
      "First check that Greyscreen is in the tray, Record games is on, and the game is a normal match. TFT, replays and spectator games are not recorded. Test recording (10 s) on Home checks capture on your main monitor.",
    items: [
      {
        id: "fullscreen",
        q: "Black video, or nothing, in Fullscreen mode",
        a: [
          "Windows cannot capture a game that runs in exclusive Fullscreen. In League, open Settings > Video and set Window Mode to Borderless (or Windowed). Borderless looks the same.",
          "On a laptop with two GPUs, black video can also mean League runs on the other GPU. In Windows Graphics settings, set Greyscreen to High performance.",
          "On Windows 10, Windows draws a thin yellow border around a captured window. You cannot remove it.",
        ],
      },
      {
        id: "window-not-found",
        q: "Greyscreen does not find the game window",
        a: [
          "Greyscreen looks for the League game window and for the Live Client Data API at 127.0.0.1:2999. Both start when the match loads. Wait for the loading screen to end, keep the game window open, and check that Greyscreen shows Waiting or Recording on Home.",
          "If it stays stuck, quit Greyscreen from the tray, start it again, then start the match. A security tool that blocks local connections can also cause this.",
        ],
      },
      {
        id: "run-as-admin",
        q: "League runs as administrator",
        a: [
          "Windows keeps apps with fewer rights from working with an app that runs as administrator. If League runs as administrator, Greyscreen may not see it, and F9 may not work.",
          "Start League without administrator rights. If you need them, start Greyscreen as administrator too: right-click Greyscreen and pick Run as administrator.",
        ],
      },
      {
        id: "folder-not-writable",
        q: "Greyscreen cannot write to the recordings folder",
        a: [
          "Pick another folder in Settings > Storage. Folders under Program Files, folders protected by Controlled folder access in Windows Security, and some cloud or network folders can refuse writes. The default, Videos\\Greyscreen in your user folder, normally works.",
        ],
      },
      {
        id: "windows-version",
        q: "Windows is too old to record",
        a: [
          "Greyscreen uses Windows Graphics Capture, which needs Windows 10 version 1903 or newer. Run Windows Update, then start Greyscreen again.",
        ],
      },
      {
        id: "no-encoder",
        q: "No video encoder found",
        a: [
          "Greyscreen needs a GPU with hardware H.264 encoding. Update your graphics driver and check that the GPU is enabled in Device Manager. Virtual machines and remote desktop sessions usually have no hardware encoder.",
          "To see what Greyscreen finds, run greyscreen-cli doctor (greyscreen-cli.exe is on the Download page) and send the output with Send feedback.",
        ],
      },
    ],
  },
  {
    id: "app",
    title: "The app",
    items: [
      {
        id: "updates",
        q: "How do updates work?",
        a: [
          "The installed copy asks GitHub for new releases a few seconds after it starts and every 6 hours. Turn this off under Settings > Updates & About.",
          "When a new version is ready, a banner offers Restart to update or Later. It stays hidden while you record. Greyscreen checks the downloaded file before it runs it. Preview builds also get preview releases.",
        ],
      },
      {
        id: "uninstall",
        q: "How do I uninstall it?",
        a: [
          "Open Windows Settings > Apps > Installed apps, find Greyscreen and uninstall it. This removes the app, its Start Menu entry and its start-with-Windows setting.",
          "Your recordings and settings stay. To remove them, delete Videos\\Greyscreen, %APPDATA%\\Greyscreen and %LOCALAPPDATA%\\Greyscreen.",
        ],
      },
      {
        id: "privacy",
        q: "What does Greyscreen send? Does it track me?",
        a: [
          "No account, no ads, no tracking. Recordings and settings stay on your PC. Nothing is sent unless you press Send feedback, apart from update checks to GitHub and champion icons from Riot's public server.",
          "The details are on the privacy page.",
        ],
      },
      {
        id: "free",
        q: "Is it free and open source?",
        a: [
          "Yes. Greyscreen is free and MIT licensed, and the source is on GitHub. It is a public beta, so expect rough edges.",
        ],
      },
    ],
  },
];

export const helpSection = {
  id: "help",
  title: "Get help",
  intro: "Pick what is easiest. Say which version you run and what you saw.",
  items: [
    {
      title: "Send feedback in the app",
      body: "Use the Send feedback button in the sidebar or dock, or in Settings > Updates & About. Tick Include recent logs so we can see what happened.",
    },
    {
      title: "GitHub Issues",
      body: "Report a bug or ask for a feature in public.",
      href: issuesUrl,
      linkLabel: "Open GitHub Issues",
    },
    ...(discordUrl
      ? [{ title: "Discord", body: "Chat with other players.", href: discordUrl, linkLabel: "Join the Discord" }]
      : []),
  ],
  logsNote: "Logs are in %LOCALAPPDATA%\\Greyscreen\\logs. Do not post them in public without reading them first.",
  sourceLabel: "Source on GitHub",
  sourceUrl: githubUrl,
};
