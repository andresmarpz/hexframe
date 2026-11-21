import type { Downloads } from "../lib/release";
import { useDownloads } from "../lib/use-downloads";

interface Props {
  initial: Downloads | null;
}

function megabytes(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export default function ReleaseInfo({ initial }: Props) {
  const downloads = useDownloads(initial);

  if (!downloads) return <p className="release-info" aria-hidden="true" />;

  const date = new Date(downloads.date).toLocaleDateString("en", {
    year: "numeric",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });

  return (
    <p className="release-info">
      <span className="release-label">Latest</span>
      <span>
        {downloads.tag} {downloads.prerelease && <span className="pill">Preview</span>}
      </span>
      <span>{date}</span>
      {downloads.app && <span>{megabytes(downloads.app.size)}</span>}
      <a href={downloads.pageUrl} rel="noopener">
        Release notes
      </a>
    </p>
  );
}
