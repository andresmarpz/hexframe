import type { Downloads } from "../../lib/release";
import { DownloadButton } from "../site/DownloadButton";
import { GitHubButton } from "../site/GitHubButton";
import { TiltMark } from "../site/TiltMark";
import styles from "./hero/Hero.module.css";

interface Props {
  downloads: Downloads | null;
}

export function Hero({ downloads }: Props) {
  return (
    <section className={styles.hero}>
      <div className={styles.inner}>
        <h1 className={styles.headline}>
          <span>make the grey</span>
          <span>
            screen
            <br className={styles.wrap} />{" "}
            <span className={styles.tail}>
              count
              <TiltMark size={104} markClassName={styles.mark} />
            </span>
          </span>
        </h1>
        <div className={styles.row}>
          <div className={styles.copy}>
            <p className={`lead ${styles.lead}`}>
              Greyscreen records every League game on your GPU, pins every kill, death and dragon to a timeline, and
              cuts the clips for you.
            </p>
            <div className={styles.buttons}>
              <DownloadButton initial={downloads} />
              <GitHubButton />
            </div>
          </div>
          <ul className={`label ${styles.facts}`}>
            <li>Free</li>
            <li>Open source</li>
            <li>Windows 10 &amp; 11</li>
          </ul>
        </div>
      </div>
    </section>
  );
}
