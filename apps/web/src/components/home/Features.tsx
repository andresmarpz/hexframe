import { RespawnLabel } from "../site/RespawnLabel";
import { Section } from "../site/Section";
import styles from "./sections/Features.module.css";

const steps = [
  { time: "00:00", title: "Install and queue up.", body: "One .exe, no admin rights." },
  {
    time: "00:01",
    title: "Play normally.",
    body: "Your GPU records the game window and League's audio. Discord and your mic stay out.",
  },
  { time: "32:14", title: "Review and clip.", body: "Jump to any kill, death or objective." },
];

export function Features() {
  return (
    <Section id="features">
      <RespawnLabel seconds={30} />
      <h2 className={`h2 ${styles.title}`}>it just records.</h2>
      <p className={`body ${styles.intro}`}>
        No record button. Greyscreen waits in the tray, starts when the game loads and stops a few seconds after the Nexus
        falls.
      </p>
      <ol className={styles.steps}>
        {steps.map((step) => (
          <li key={step.time} className={styles.step}>
            <span className={styles.node} aria-hidden="true" />
            <div className={styles.text}>
              <span className="label">{step.time}</span>
              <h3 className="h3">{step.title}</h3>
              <p className={`body ${styles.body}`}>{step.body}</p>
            </div>
          </li>
        ))}
      </ol>
    </Section>
  );
}
