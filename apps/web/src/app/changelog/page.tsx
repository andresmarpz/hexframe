import type { Metadata } from "next";
import { Changelog } from "../../components/changelog/Changelog";
import { changelog, site } from "../../content/copy";
import { getChangelog } from "../../lib/changelog";

export const metadata: Metadata = {
  title: `${changelog.title} — ${site.name}`,
  description: changelog.intro,
};

export default function ChangelogPage() {
  return <Changelog entries={getChangelog()} />;
}
