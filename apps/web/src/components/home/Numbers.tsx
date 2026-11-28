import { Section } from "../site/Section";
import styles from "./Numbers.module.css";

const numbers = [
  { value: "1080p60", label: "DEFAULT QUALITY" },
  { value: "AGPL-3.0", label: "OPEN SOURCE" },
  { value: "1 .exe", label: "NO ADMIN RIGHTS" },
];

export function Numbers() {
  return (
    <Section id="numbers" padding={120} paddingMobile={72}>
      <ul className={styles.row}>
        {numbers.map((item) => (
          <li key={item.label} className={styles.cell}>
            <span className={styles.value}>{item.value}</span>
            <span className={`label ${styles.caption}`}>{item.label}</span>
          </li>
        ))}
      </ul>
    </Section>
  );
}
