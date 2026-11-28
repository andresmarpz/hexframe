import type { Downloads } from "../../lib/release";
import { RespawnLabel } from "../site/RespawnLabel";
import { DownloadCard } from "./DownloadCard";
import styles from "./Download.module.css";

interface Props {
  downloads: Downloads | null;
  stars: number | null;
}

const steps = [
  {
    title: "Run it.",
    body: "Windows may show SmartScreen because the app is new. Click More info, then Run anyway.",
  },
  { title: "Pick a folder.", body: "Videos\\Greyscreen by default." },
  { title: "Queue up.", body: "Your next game records itself." },
];

const requirements = [
  { term: "League", detail: "Borderless or Windowed" },
  { term: "Graphics", detail: "A GPU with a hardware H.264 encoder (software fallback works, uses more CPU)" },
  { term: "Disk", detail: "About 2 GB per game at the defaults" },
];

export function Download({ downloads }: Props) {
  return (
    <div className={styles.page}>
      <section className={styles.hero}>
        <RespawnLabel seconds={5} />
        <h1 className={`display ${styles.display}`}>download.</h1>
        <p className={`lead ${styles.lead}`}>One file. No admin rights, no account.</p>
      </section>

      <section className={styles.card} aria-label="Installer">
        <DownloadCard initial={downloads} />
      </section>

      <section className={styles.section}>
        <h2 className="h2">first run.</h2>
        <ol className={styles.steps}>
          {steps.map((step, index) => (
            <li key={step.title} className={styles.step}>
              <span className={`label ${styles.number}`}>{String(index + 1).padStart(2, "0")}</span>
              <h3 className="h3">{step.title}</h3>
              <p className="body">{step.body}</p>
            </li>
          ))}
        </ol>
      </section>

      <section className={styles.section}>
        <h2 className="h2">requirements.</h2>
        <dl className={styles.requirements}>
          {requirements.map((row) => (
            <div key={row.term} className={styles.row}>
              <dt className={`label ${styles.term}`}>{row.term}</dt>
              <dd className={`body ${styles.detail}`}>{row.detail}</dd>
            </div>
          ))}
        </dl>
      </section>
    </div>
  );
}
