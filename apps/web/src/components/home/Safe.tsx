import { Section } from "../site/Section";
import { XBullet } from "../site/XBullet";
import styles from "./Safe.module.css";

const points = [
  { title: "No memory reading.", body: "Events come from Riot's Live Client Data API on your PC." },
  { title: "No injection.", body: "Video comes from Windows Graphics Capture. The game is never touched." },
  { title: "No account.", body: "Recordings stay on your PC as MP4 files. Nothing is uploaded." },
];

export function Safe() {
  return (
    <Section id="safe" tone="ink">
      <h2 className={`h2 ${styles.title}`}>stays out of the game.</h2>
      <ul className={styles.list}>
        {points.map((point) => (
          <li key={point.title} className={styles.item}>
            <XBullet size={28} />
            <h3 className={`h3 ${styles.itemTitle}`}>{point.title}</h3>
            <p className={styles.body}>{point.body}</p>
          </li>
        ))}
      </ul>
    </Section>
  );
}
