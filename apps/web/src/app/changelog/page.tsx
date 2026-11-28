import type { Metadata } from "next";
import { Changelog } from "../../components/changelog/Changelog";
import { changelog, site } from "../../content/copy";
import { getChangelog } from "../../lib/changelog";
import { getDownloads } from "../../lib/github-build";

export const metadata: Metadata = {
  title: `${changelog.title} — ${site.name}`,
  description: changelog.intro,
};

export default async function ChangelogPage() {
  const downloads = await getDownloads();
  return <Changelog entries={getChangelog()} downloads={downloads} />;
}
