export const repo = "andresmarpz/greyscreen";
export const githubUrl = `https://github.com/${repo}`;
export const releasesUrl = `${githubUrl}/releases`;
export const issuesUrl = `${githubUrl}/issues`;
export const discordUrl = "";
export const site = {
  name: "Greyscreen",
  title: "Greyscreen — Automatic League of Legends recorder for Windows",
  description:
    "A fast, free and open-source League of Legends recorder for Windows. Records every game and cuts the clips for you. No ads, no account.",
  ogTitle: "Greyscreen — Automatic League of Legends recorder for Windows",
  ogDescription:
    "A fast League of Legends recorder for Windows with no ads and no account. Records every game, marks every death and cuts the clips for you.",
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

export const download = {
  title: "Download Greyscreen",
  intro:
    "One small .exe for Windows. Free, open source, and installs to your user folder without admin rights. Greyscreen is in public beta, so expect rough edges.",
  button: "Download for Windows",
  requirementsTitle: "Requirements",
  requirements: [
    "Windows 10 (version 2004 or newer) or Windows 11, 64-bit",
    "A GPU with hardware H.264 encoding is recommended. Most NVIDIA, AMD and Intel GPUs from the last several years have one. Without it, Greyscreen falls back to Windows' software encoder at up to 1080p 30 fps, which uses more CPU. Tested mostly on NVIDIA so far.",
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
