import { RespawnLabel } from "../site/RespawnLabel";
import { Section } from "../site/Section";
import { XBullet } from "../site/XBullet";
import { DeathReview } from "./sections/DeathReview";
import styles from "./sections/Timeline.module.css";

const points = [
  { title: "Filters and pre-roll", body: "Show only what you want. Jumps start 5 seconds early." },
  { title: "Death review", body: "Step through each death, tag it, write a note." },
  { title: "Theater mode and speed", body: "Fill the window, or slow it down to see what went wrong." },
];

export function Timeline() {
  return (
    <Section tone="fog" innerClassName={styles.columns}>
      <div className={styles.left}>
        <RespawnLabel seconds={20} />
        <h2 className={`h2 ${styles.title}`}>every death, on the timeline.</h2>
        <p className={`body ${styles.intro}`}>
          Every kill, death and objective gets a marker on the seek bar. Click one and you&apos;re at the fight.
        </p>
        <ul className={styles.points}>
          {points.map((point) => (
            <li key={point.title} className={styles.point}>
              <XBullet />
              <div className={styles.pointText}>
                <h3 className={styles.pointTitle}>{point.title}</h3>
                <p className="body">{point.body}</p>
              </div>
            </li>
          ))}
        </ul>
      </div>
      <div className={styles.panel}>
        <picture>
          <source media="(max-width: 899px)" srcSet="/screens/timeline-mobile.webp" />
          <img
            className={styles.seekbar}
            src="/screens/timeline-desktop.webp"
            width={656}
            height={56}
            alt="The seek bar with a marker for every minute: kills, deaths and objectives."
          />
        </picture>
        <DeathReview />
      </div>
    </Section>
  );
}
