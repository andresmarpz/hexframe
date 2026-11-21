// TODO(copy): strings outside the written copy: faq.title, download section titles and buttons, notFound buttons, review image alts
export const repo = "andresmarpz/greyscreen";
export const githubUrl = `https://github.com/${repo}`;
export const releasesUrl = `${githubUrl}/releases`;
export const downloadUrl = `${githubUrl}/releases/latest/download/greyscreen.exe`;
export const cliDownloadUrl = `${githubUrl}/releases/latest/download/greyscreen-cli.exe`;

export const site = {
  name: "Greyscreen",
  title: "Greyscreen — Automatic League of Legends recorder for Windows",
  description:
    "Free, open-source League of Legends recorder for Windows. Auto-records games and key clips on your GPU. Local MP4 files, no account.",
  ogTitle: "Greyscreen — Automatic League of Legends recorder for Windows",
  ogDescription:
    "Free, open-source Windows recorder for League of Legends. Auto-captures full games and clips on your GPU, with an event timeline for review.",
  tagline: "Free, open-source League of Legends recorder for Windows.",
  riotNotice:
    "Greyscreen isn't endorsed by Riot Games and doesn't reflect the views or opinions of Riot Games or anyone officially involved in producing or managing Riot Games properties. Riot Games, and all associated properties are trademarks or registered trademarks of Riot Games, Inc.",
};

export const nav = [
  { label: "Features", href: "/#features" },
  { label: "Download", href: "/download/" },
  { label: "Changelog", href: "/changelog/" },
];

export const hero = {
  headline: "Record every League game automatically",
  subheadline:
    "Sits in your Windows tray, records full matches and key clips on your GPU, and maps every play on a timeline for review.",
  primaryCta: "Download for Windows",
  secondaryCta: "View on GitHub",
  screenAlt: "The Greyscreen Status screen while a League game is being recorded.",
};

export const trust = [
  "Free",
  "Open source",
  "Windows 10 and 11 (64-bit)",
  "Local MP4s, no account",
];

export type IconName = "detect" | "gpu" | "clips" | "audio" | "library" | "local";

export const features: { icon: IconName; title: string; body: string }[] = [
  {
    icon: "detect",
    title: "Automatic game detection",
    body: 'Starts with Windows if you want, sits in the tray, and records each match automatically. Flip the "Record games" switch whenever you want to pause.',
  },
  {
    icon: "gpu",
    title: "Hardware GPU encoding",
    body: "Encodes H.264 video on your graphics card instead of your CPU. Choose Balanced, High, or Max quality at 30 or 60 fps, with size estimates in Settings.",
  },
  {
    icon: "clips",
    title: "Auto clips and manual saves",
    body: "Saves short clips around kills, deaths, multikills, first blood, aces, towers, inhibitors, dragons, heralds, barons, and void grubs. Also supports manually saving the last 30 seconds.",
  },
  {
    icon: "audio",
    title: "Game audio only",
    body: "Records only League's own audio. Discord and your microphone stay out of your videos. Show or hide the mouse cursor to analyze your movement.",
  },
  {
    icon: "library",
    title: "Match library",
    body: "Browse games by champion icon, Victory or Defeat, KDA, duration, clips, and size. Filter your recordings, or delete old ones to the Recycle Bin.",
  },
  {
    icon: "local",
    title: "Local files, no account",
    body: "Recordings stay on your PC as normal MP4s with a small JSON file beside them. No account, no uploads, no telemetry. Installs from one .exe without admin rights.",
  },
];

export const howItWorks = {
  title: "How it works",
  steps: [
    {
      title: "Install and queue up.",
      body: "Run the single .exe to install Greyscreen in your user folder without admin rights. It sits in the tray and waits for League.",
    },
    {
      title: "Play normally.",
      body: "When a game starts, Greyscreen records the game window on your GPU, captures League's audio, and logs match events automatically.",
    },
    {
      title: "Review and clip.",
      body: "Open the library after your match to jump to any kill, death, or objective on the timeline, or grab the clips already saved for you.",
    },
  ],
  screenAlt: "The Greyscreen Status screen showing the last recorded game.",
};

export const review = {
  title: "Built for reviewing",
  intro:
    "Scrubbing through a full match to find one fight wastes time. Greyscreen pairs your video with an event timeline so you can go straight to the plays you want to check.",
  points: [
    {
      title: "Color-coded markers",
      body: "Hover a marker to see who killed whom, which tower fell, or every kill of a pentakill.",
    },
    {
      title: "Event filters and pre-roll",
      body: "Filter to deaths, kills, or objectives, and jump to 5 seconds before any event.",
    },
    {
      title: "Fullscreen theater mode",
      body: "Watch fullscreen while keeping the event timeline on screen.",
    },
    {
      title: "Playback speed",
      body: "Adjust playback speed to inspect close fights.",
    },
  ],
  reviewAlt: "The Greyscreen review screen with the match video and its event timeline.",
  theaterAlt: "The Greyscreen review screen in fullscreen theater mode.",
  recordingsAlt: "The Greyscreen Recordings screen listing recorded games by champion.",
};

export const safe = {
  title: "Safe by design",
  points: [
    {
      title: "Official Riot local API",
      body: "Events come from Riot's official Live Client Data API served by League on your PC, and video uses Windows' standard capture of the game window.",
    },
    {
      title: "No memory reading or injection",
      body: "Greyscreen does not read game memory, inject into the game, or modify game files.",
    },
    {
      title: "Local files only",
      body: "Every recording stays on your PC as an MP4 with a small JSON file next to it. No account, no uploads, and no telemetry.",
    },
  ],
};

export const closing = {
  headline: "Ready for your next game",
  button: "Download for Windows",
};

export const faq = {
  title: "Questions",
  items: [
    {
      q: "Is Greyscreen allowed? Will I get banned?",
      a: "Greyscreen only uses Riot's official local Live Client Data API and standard Windows screen capture, like other well-known recorders. It does not read game memory, inject into the game, or modify game files. Riot's terms and enforcement are always Riot's call.",
    },
    {
      q: "Will recording hurt my FPS?",
      a: "Greyscreen uses your GPU's hardware H.264 encoder, so video encoding runs on the graphics card instead of your CPU. You can also choose Balanced, High, or Max quality and 30 or 60 fps in Settings.",
    },
    {
      q: "Which GPUs are supported?",
      a: "Greyscreen runs on 64-bit Windows 10 and 11 with a GPU that supports hardware H.264 encoding. It has been tested on NVIDIA so far; AMD and Intel use the same Windows encoder path.",
    },
    {
      q: "How much disk space do recordings use?",
      a: "Size depends on your display resolution, frame rate (30 or 60 fps), and quality preset (Balanced, High, or Max). At 1080p60 High, recordings use about 55 MB per minute. Settings shows an estimated size for your configuration.",
    },
    {
      q: "Where are my recordings saved?",
      a: "Files stay on your PC as normal MP4 videos with a small JSON file next to each one. Nothing is uploaded and no account is required. You can manage recordings and delete them to the Recycle Bin from the library.",
    },
    {
      q: "Does it record Discord or my microphone?",
      a: "No. Greyscreen records only League's own audio. Discord and your microphone are not recorded.",
    },
    {
      q: "Can I show or hide my mouse cursor?",
      a: "Yes. Showing the mouse cursor in recordings is optional, so you can turn it on to analyze your mouse movement or hide it.",
    },
    {
      q: "How do I uninstall Greyscreen?",
      a: "Greyscreen installs to your user folder with a Start Menu entry and an uninstaller, with no admin rights required. Run the uninstaller to remove it.",
    },
    {
      q: "Why does Windows SmartScreen show a warning?",
      a: 'The download is not code-signed yet, so Windows SmartScreen may warn before running it. Click "More info" and then "Run anyway", or inspect the source code on GitHub.',
    },
    {
      q: "Is Greyscreen free and open source?",
      a: "Yes. Greyscreen is free and open source on GitHub, with no account, no uploads, and no telemetry.",
    },
  ],
};

export const download = {
  title: "Download Greyscreen",
  intro:
    "One small .exe for Windows. Free, open source, and installs to your user folder without admin rights.",
  button: "Download for Windows",
  requirementsTitle: "Requirements",
  requirements: [
    "Windows 10 or Windows 11 (64-bit)",
    "GPU with hardware H.264 encoding (tested on NVIDIA; AMD and Intel use the same Windows encoder path)",
    "League of Legends on the same PC",
  ],
  smartScreenTitle: "Windows SmartScreen",
  smartScreen:
    'The download is not code-signed yet, so Windows SmartScreen may show a warning. Click "More info" and then "Run anyway" to continue.',
  firstRunTitle: "First run",
  firstRun: [
    "Run the downloaded .exe to install Greyscreen in your user folder with a Start Menu entry and an uninstaller.",
    'Open Settings to pick your quality preset (Balanced, High, or Max), frame rate (30 or 60 fps), cursor option, and whether Greyscreen starts with Windows.',
    'Leave Greyscreen in the tray with "Record games" turned on, then start a match.',
  ],
  firstRunAlt: "The Greyscreen Settings screen.",
  cliTitle: "Command-line tool",
  cliBody:
    "Use greyscreen-cli.exe to check your encoder (greyscreen-cli doctor) or run a 10-second test recording (greyscreen-cli record --seconds 10).",
  cliButton: "Download greyscreen-cli.exe",
  allReleases: "All releases on GitHub",
};

export const changelog = {
  title: "Changelog",
  intro: "Release notes and updates for Greyscreen on Windows.",
};

export const notFound = {
  headline: "Page not found",
  body: "The link you followed does not exist. Return to the home page or go to the Windows download.",
  home: "Home",
  download: "Download",
};
