import { releasesUrl } from "../../content/copy";
import type { ChangelogEntry } from "../../lib/changelog";
import type { Downloads } from "../../lib/release";
import { Badge } from "../kit/Badge";
import { TextLink } from "../site/TextLink";
import { XBullet } from "../site/XBullet";
import styles from "./Changelog.module.css";

interface Props {
  entries: ChangelogEntry[];
  downloads: Downloads | null;
}

const dateFormat = new Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", year: "numeric", timeZone: "UTC" });

const versionLabel = (version: string) => `v${version}`.replace(/-/g, " ").replace(/(preview)\./, "$1 ");

export function Changelog({ entries }: Props) {
  return (
    <>
      <div className={styles.head}>
        <h1 className="display">changelog.</h1>
      </div>
      <div className={styles.list}>
        {entries.map((entry, index) => (
          <article key={entry.version} id={entry.version} className={styles.release}>
            <div className={styles.meta}>
              <div className={styles.version}>
                <h2 className="label">{versionLabel(entry.version)}</h2>
                {index === 0 && <Badge>Latest</Badge>}
              </div>
              <time className="body" dateTime={entry.date}>
                {dateFormat.format(new Date(entry.date))}
              </time>
            </div>
            <div className={styles.body}>
              <h3 className="h3">{entry.title}</h3>
              <ul className={styles.bullets}>
                {entry.bullets.map((bullet) => (
                  <li key={bullet} className={styles.bullet}>
                    <XBullet size={14} />
                    <p className="body">
                      {bullet.split("`").map((part, i) => (i % 2 ? <code key={i}>{part}</code> : part))}
                    </p>
                  </li>
                ))}
              </ul>
            </div>
          </article>
        ))}
      </div>
      <div className={styles.more}>
        <TextLink href={releasesUrl} className={styles.moreLink}>
          All releases on GitHub
        </TextLink>
      </div>
    </>
  );
}
