import type { ReactNode } from "react";
import { releasesUrl } from "../content/copy";
import type { Downloads } from "../lib/release";
import { useDownloads } from "../lib/use-downloads";

interface Props {
  initial: Downloads | null;
  file: "app" | "cli";
  className: string;
  icon?: boolean;
  children: ReactNode;
}

export default function DownloadButton({ initial, file, className, icon, children }: Props) {
  const downloads = useDownloads(initial);
  const href = downloads?.[file]?.url ?? downloads?.pageUrl ?? releasesUrl;

  return (
    <a className={className} href={href}>
      {icon && (
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.75"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
          focusable="false"
        >
          <path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19.5h14" />
        </svg>
      )}
      {children}
    </a>
  );
}
