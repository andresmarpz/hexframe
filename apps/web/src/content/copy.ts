// TODO(copy): strings outside the written copy: download section titles and buttons, notFound buttons, review image alts
export const repo = "andresmarpz/greyscreen";
export const githubUrl = `https://github.com/${repo}`;
export const releasesUrl = `${githubUrl}/releases`;
export const issuesUrl = `${githubUrl}/issues`;
export const discordUrl = "";

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
  { label: "FAQ", href: "/faq/" },
  { label: "Changelog", href: "/changelog/" },
];

export const footerLinks = [
  { label: "Privacy", href: "/privacy/" },
  { label: "Get help", href: "/faq/#help" },
  { label: "GitHub Issues", href: issuesUrl },
  ...(discordUrl ? [{ label: "Discord", href: discordUrl }] : []),
];

export const hero = {
  headline: "Record every League game automatically",
  subheadline:
    "Sits in your Windows tray, records full matches and key clips on your GPU, and maps every play on a timeline for review.",
  primaryCta: "Download for Windows",
  secondaryCta: "View on GitHub",
  screenAlt: "The Greyscreen window while a League game is being recorded, with the games list and the live event feed.",
};

export const trust = [
  "Free",
  "Open source",
  "Public beta",
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
    body: "Recordings stay on your PC as normal MP4s with a small JSON file beside them. No account and no uploads. Nothing is sent unless you press Send feedback. Installs from one .exe without admin rights.",
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
  screenAlt: "The Greyscreen Home screen showing the last recorded game.",
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
  recordingsAlt: "The Greyscreen Games screen with the games list, the video player and the event timeline.",
};

export const safe = {
  title: "Stays out of the game",
  points: [
    {
      title: "Official Riot local API",
      body: "Events come from Riot's Live Client Data API, which League serves on your own PC. Video uses Windows' standard capture of the game window.",
    },
    {
      title: "No memory reading or injection",
      body: "Greyscreen does not read game memory, inject into the game, or change game files.",
    },
    {
      title: "Local files only",
      body: "Recordings stay on your PC as an MP4 with a small JSON file next to it. No account, no uploads. Nothing is sent unless you press Send feedback.",
    },
  ],
  more: "More in the FAQ",
};

export const homeQuestions = {
  title: "Questions",
  items: [
    { label: "Is it safe with Vanguard? Will I get banned?", href: "/faq/#vanguard" },
    { label: "Does it cost FPS?", href: "/faq/#fps" },
    { label: "Where are my recordings, and how much space do they use?", href: "/faq/#storage" },
    { label: "Why isn't it recording?", href: "/faq/#fullscreen" },
    { label: "What does it send?", href: "/privacy/" },
  ],
};

export const closing = {
  headline: "Ready for your next game",
  button: "Download for Windows",
};

export const download = {
  title: "Download Greyscreen",
  intro:
    "One small .exe for Windows. Free, open source, and installs to your user folder without admin rights. Greyscreen is in public beta, so expect rough edges.",
  button: "Download for Windows",
  requirementsTitle: "Requirements",
  requirements: [
    "Windows 10 (version 2004 or newer) or Windows 11, 64-bit",
    "A GPU with hardware H.264 encoding. Most NVIDIA, AMD and Intel GPUs from the last several years have one. Tested mostly on NVIDIA so far.",
    "League of Legends set to Borderless or Windowed. Windows cannot capture exclusive Fullscreen.",
  ],
  smartScreenTitle: "Windows may warn you",
  smartScreenSteps: [
    'Windows may show "Windows protected your PC". The .exe is not code-signed yet, so Windows does not know it.',
    'Click "More info", then "Run anyway".',
    "To check what you run, read the source on GitHub.",
  ],
  smartScreenAppControl:
    'On Windows 11 with Smart App Control turned on, Windows can block the file with no "Run anyway" button. Turn Smart App Control off in Windows Security, or wait for a signed release.',
  firstRunTitle: "First run",
  firstRun: [
    "Run the downloaded .exe. It installs Greyscreen in your user folder with a Start Menu entry and an uninstaller.",
    "In Settings, pick your quality, frame rate, cursor option, and whether Greyscreen starts with Windows.",
    'Set League to Borderless, leave Greyscreen in the tray with "Record games" on, then start a match.',
  ],
  firstRunAlt: "The Greyscreen settings sheet.",
  helpLine: "Something wrong? See the FAQ",
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
