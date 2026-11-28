"use client";

import { releasesUrl } from "../../content/copy";
import { APP_ASSET, displayVersion, type Downloads } from "../../lib/release";
import { useDownloads } from "../../lib/use-downloads";
import { DownloadButton } from "../site/DownloadButton";
import { TextLink } from "../site/TextLink";
import styles from "./Download.module.css";

interface Props {
  initial: Downloads | null;
}

export function DownloadCard({ initial }: Props) {
  const downloads = useDownloads(initial);
  const cliHref = downloads?.cli?.url ?? downloads?.pageUrl ?? releasesUrl;
  const version = downloads ? `${displayVersion(downloads.tag)} · ` : "";

  return (
    <>
      <div className={styles.cardText}>
        <p className={`label ${styles.meta}`}>{`${version}Public beta`}</p>
        <h2 className="h3">{downloads?.app?.name ?? APP_ASSET}</h2>
        <p className="body">Windows 10 2004+ and 11, 64-bit.</p>
      </div>
      <div className={styles.cardActions}>
        <DownloadButton initial={initial} className={styles.button} />
        <TextLink href={releasesUrl}>All releases on GitHub</TextLink>
        <TextLink href={cliHref}>greyscreen-cli.exe</TextLink>
      </div>
    </>
  );
}
