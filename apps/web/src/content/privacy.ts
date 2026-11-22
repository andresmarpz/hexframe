import { githubUrl, issuesUrl } from "./copy";

export const privacy = {
  title: "Privacy",
  description: "What Greyscreen keeps on your PC, what it sends and when. No account, no ads, no tracking.",
  updated: "Last updated 21 November 2026",
  intro:
    "Greyscreen has no account, no ads and no tracking. Your recordings stay on your PC. Greyscreen sends something only in the cases below.",
  localTitle: "Stays on your PC",
  local: [
    "Recordings, clips, thumbnails and each game's timeline file, in the recordings folder (Videos\\Greyscreen by default).",
    "Your settings, in %APPDATA%\\Greyscreen.",
    "Logs and a crash note, in %LOCALAPPDATA%\\Greyscreen\\logs. They never leave your PC unless you tick Include recent logs when you send feedback.",
    "Match events from League. Greyscreen reads them from the Live Client Data API on your own PC (127.0.0.1) and does not send them anywhere.",
  ],
  sentTitle: "What Greyscreen sends, and when",
  sent: [
    {
      title: "Update checks",
      body: "The installed app asks GitHub for the list of Greyscreen releases a few seconds after it starts and every 6 hours. If a newer one exists, it downloads it from GitHub. GitHub sees your IP address and the app version, like any website you visit. Turn this off in Settings > Updates & About.",
    },
    {
      title: "Champion icons",
      body: "Greyscreen downloads champion icons from Riot's public Data Dragon server and keeps them in a cache folder on your PC. Riot sees your IP address.",
    },
    {
      title: "Send feedback",
      body: "Only when you press Send. The report goes to Greyscreen's server at api.greyscreen.app and holds your message and its type (bug, idea or other), the app version, your Windows version, an email if you typed one, and a random ID the app made for itself. The ID is not tied to who you are. If Include recent logs is ticked, it also holds the last 300 log lines, with your user folder name replaced by ~ and Riot ID and summoner name values removed. When no video encoder works on your PC, the report also holds the first encoder error line (your graphics encoder, graphics card name and the Windows error text), with your user folder name replaced by ~, even if logs are not ticked. The server keeps a hash of your IP address to limit spam, not the address itself. We use a report to fix problems and, if you gave an email, to reply to you.",
    },
    {
      title: "Crash reports",
      body: "Planned, opt-in. Greyscreen does not send crash reports today.",
    },
  ],
  websiteTitle: "This website",
  website: [
    "We use Vercel Web Analytics to count page views. It uses no cookies and does not follow you across sites.",
    "The site is hosted on Vercel, which keeps normal server logs. The download page asks GitHub for the latest release, so GitHub sees your IP address.",
  ],
  contactTitle: "Contact and deletion",
  contact:
    "To ask a question, or to have a feedback report deleted, use Send feedback in the app or open an issue on GitHub. Do not put personal details in a public issue. For a deletion, say what you sent and when.",
  contactLinks: [{ label: "GitHub Issues", href: issuesUrl }, { label: "Source code", href: githubUrl }],
  changes: "If this changes, we update this page and say so in the release notes.",
};
