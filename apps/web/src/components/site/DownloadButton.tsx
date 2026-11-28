"use client";

import type { ReactNode } from "react";
import { releasesUrl } from "../../content/copy";
import type { Downloads } from "../../lib/release";
import { useDownloads } from "../../lib/use-downloads";
import { Button, type ButtonSize, type ButtonVariant } from "../kit/Button";
import { WindowsIcon } from "./icons";

interface Props {
  initial: Downloads | null;
  file?: "app" | "cli";
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: boolean;
  className?: string;
  children?: ReactNode;
}

export function DownloadButton({
  initial,
  file = "app",
  variant = "contrast",
  size = "xxl",
  icon = true,
  className,
  children = "Download for Windows",
}: Props) {
  const downloads = useDownloads(initial);
  const href = downloads?.[file]?.url ?? downloads?.pageUrl ?? releasesUrl;

  return (
    <Button
      href={href}
      variant={variant}
      size={size}
      className={className}
      icon={icon ? <WindowsIcon size={size === "xxl" ? 18 : 16} /> : undefined}
    >
      {children}
    </Button>
  );
}
