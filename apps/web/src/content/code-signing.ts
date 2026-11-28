import { githubUrl, releasesUrl } from "./copy";

const maintainer = { label: "andresmarpz", href: "https://github.com/andresmarpz" };

export const codeSigning = {
  title: "Code signing policy",
  description: "How Greyscreen's Windows files are built and signed, and who approves each release.",
  updated: "Last updated 28 November 2026",
  sponsor: {
    before: "Free code signing provided by ",
    signPath: { label: "SignPath.io", href: "https://about.signpath.io" },
    middle: ", certificate by ",
    foundation: { label: "SignPath Foundation", href: "https://signpath.org" },
  },
  signedTitle: "What is signed",
  signed: [
    "GreyscreenSetup.exe, greyscreen.exe and greyscreen-cli.exe on the GitHub releases page. The in-app updater downloads the same signed greyscreen.exe.",
    "GitHub Actions builds each release from the source code in the public repository. No file built on a personal computer is signed.",
    "Windows shows SignPath Foundation as the publisher.",
  ],
  links: [
    { label: "Releases", href: releasesUrl },
    { label: "Source code", href: githubUrl },
  ],
  teamTitle: "Team",
  team: [
    { role: "Committers and reviewers", people: [maintainer] },
    { role: "Approvers", people: [maintainer] },
  ],
  teamNote:
    "A team member reviews every change from outside the team before it is merged. A team member approves every signing request by hand.",
  privacyTitle: "Privacy",
  privacy:
    "Greyscreen sends data to other networked systems only as described in the privacy policy, and you can turn off update checks in Settings.",
  privacyLink: { label: "Privacy policy", href: "/privacy/" },
};
