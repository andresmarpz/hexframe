"use client";

import { displayVersion, type Downloads } from "../../lib/release";
import { useDownloads } from "../../lib/use-downloads";
import { TiltMark } from "../notfound/TiltMark";
import { DownloadButton } from "../site/DownloadButton";
import { RespawnLabel } from "../site/RespawnLabel";
import { Section } from "../site/Section";
import { TextLink } from "../site/TextLink";
import styles from "./Respawn.module.css";

interface Props {
  downloads: Downloads | null;
}

export function Respawn({ downloads: initial }: Props) {
  const downloads = useDownloads(initial);
  const version = downloads ? `${displayVersion(downloads.tag)} · ` : "";

  return (
    <Section id="respawn" innerClassName={styles.inner}>
      <RespawnLabel seconds={0} />
      <TiltMark size={160} className={styles.mark} />
      <h2 className={`display ${styles.title}`}>respawn.</h2>
      <p className={`lead ${styles.lead}`}>Your next game records itself.</p>
      <div className={styles.actions}>
        <DownloadButton initial={initial} className={styles.button} />
        <TextLink href="/changelog/">{`${version}Changelog`}</TextLink>
      </div>
      <p className={`label ${styles.facts}`}>Windows 10 2004+ &amp; 11 · 64-bit · Free</p>
    </Section>
  );
}
