import type { Metadata } from "next";
import { Download } from "../../components/download/Download";
import { download, site } from "../../content/copy";
import { getDownloads, getStars } from "../../lib/github-build";

export const metadata: Metadata = {
  title: `${download.title} — ${site.name}`,
  description: download.intro,
};

export default async function DownloadPage() {
  const [downloads, stars] = await Promise.all([getDownloads(), getStars()]);
  return <Download downloads={downloads} stars={stars} />;
}
