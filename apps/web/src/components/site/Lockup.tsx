import { Mark } from "./Mark";
import styles from "./Lockup.module.css";

interface Props {
  tone?: "dark" | "light";
  href?: string;
  className?: string;
}

export function Lockup({ tone = "dark", href, className }: Props) {
  const classes = [styles.lockup, tone === "light" ? styles.light : "", className].filter(Boolean).join(" ");
  const content = (
    <>
      <Mark tone={tone} size={32} />
      <span className={styles.word}>greyscreen</span>
    </>
  );
  if (href) {
    return (
      <a href={href} className={classes} aria-label="Greyscreen home">
        {content}
      </a>
    );
  }
  return <div className={classes}>{content}</div>;
}
