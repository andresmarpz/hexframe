import styles from "./DeathReview.module.css";

const tags = ["Out of position", "Overextended", "No vision", "Bad trade", "Ganked", "Teamfight", "Unavoidable"];
const selectedTag = "Ganked";

export function DeathReview() {
  return (
    <div className={styles.card}>
      <div className={styles.head}>
        <span className={styles.title}>Death 3 of 7</span>
        <span className={styles.time}>14:32</span>
      </div>
      <ul className={styles.tags}>
        {tags.map((tag) => (
          <li key={tag} className={tag === selectedTag ? `${styles.tag} ${styles.selected}` : styles.tag}>
            {tag}
          </li>
        ))}
      </ul>
      <div className={styles.note}>Warded the wrong bush. Again.</div>
    </div>
  );
}
