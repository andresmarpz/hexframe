import { Button } from "../kit/Button";
import { RespawnLabel } from "../site/RespawnLabel";
import { Section } from "../site/Section";
import { Stage } from "../site/Stage";
import styles from "./sections/Clips.module.css";

const cards = [
  { src: "/screens/clip-1.webp", alt: "Kill clip, 20 seconds, at 4:12" },
  { src: "/screens/clip-2.webp", alt: "Double kill clip, 29 seconds, at 14:32" },
  { src: "/screens/clip-3.webp", alt: "Baron clip, 20 seconds, at 25:40" },
  { src: "/screens/clip-4.webp", alt: "Kill clip, 20 seconds, at 30:02" },
];

export function Clips() {
  return (
    <Section innerClassName={styles.columns}>
      <Stage className={styles.grid}>
        {cards.map((card, index) => (
          <img
            key={card.src}
            className={`${styles.card} ${styles[`card${index + 1}`]}`}
            src={card.src}
            width={299}
            height={168}
            alt={card.alt}
          />
        ))}
      </Stage>
      <div className={styles.right}>
        <RespawnLabel seconds={10} />
        <h2 className={`h2 ${styles.title}`}>clips cut themselves.</h2>
        <p className={`body ${styles.intro}`}>
          Kills, multikills, aces, dragons, barons: clipped while you play. Saw something the game didn&apos;t count? Press F9
          to save the last 30 seconds.
        </p>
        <kbd className={styles.keycap}>F9</kbd>
        <div className={styles.export}>
          <span className={`label ${styles.exportLabel}`}>Export</span>
          <Button variant="outline" size="regular">
            Original quality
          </Button>
          <Button variant="outline" size="regular">
            For Discord (under 10 MB)
          </Button>
        </div>
      </div>
    </Section>
  );
}
