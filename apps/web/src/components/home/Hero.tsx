import type { Downloads } from "../../lib/release";
import { DownloadButton } from "../site/DownloadButton";
import { GitHubButton } from "../site/GitHubButton";
import { Mark } from "../site/Mark";
import styles from "./hero/Hero.module.css";

interface Props {
  downloads: Downloads | null;
  stars: number | null;
}

export function Hero({ downloads, stars }: Props) {
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
              <Mark size={104} className={styles.mark} faceClassName={styles.face} />
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
              <GitHubButton initialStars={stars} />
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
