import type { Metadata } from "next";
import { Download } from "../../components/download/Download";
import { download, site } from "../../content/copy";
import { getDownloads } from "../../lib/github-build";

export const metadata: Metadata = {
  title: `${download.title} — ${site.name}`,
  description: download.intro,
};

export default async function DownloadPage() {
  const downloads = await getDownloads();
  return <Download downloads={downloads} />;
}
