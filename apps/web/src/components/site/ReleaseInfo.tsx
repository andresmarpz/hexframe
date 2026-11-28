"use client";

import { displayVersion, formatMegabytes, formatReleaseDate, type Downloads } from "../../lib/release";
import { useDownloads } from "../../lib/use-downloads";
import { Badge } from "../kit/Badge";

interface Props {
  initial: Downloads | null;
  className?: string;
}

export function ReleaseInfo({ initial, className }: Props) {
  const downloads = useDownloads(initial);
  if (!downloads) return null;

  return (
    <p className={className}>
      <Badge>{displayVersion(downloads.tag)}</Badge> {formatReleaseDate(downloads.date)}
      {downloads.app && ` · ${formatMegabytes(downloads.app.size)}`}
    </p>
  );
}
