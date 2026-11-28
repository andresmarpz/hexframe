import type { CSSProperties, ReactNode } from "react";
import styles from "./Callout.module.css";

interface Props {
  label: ReactNode;
  line?: "top" | "bottom" | "none";
  style?: CSSProperties;
  className?: string;
  children?: ReactNode;
}

export function Callout({ label, line = "top", style, className, children }: Props) {
  const classes = [styles.callout, styles[line], className].filter(Boolean).join(" ");
  return (
    <div className={classes} style={style}>
      {line !== "none" && <span className={styles.line} aria-hidden="true" />}
      <div className={styles.card}>
        <div className={styles.label}>{label}</div>
        {children && <div className={styles.text}>{children}</div>}
      </div>
    </div>
  );
}
