import type { ReactNode } from "react";
import styles from "./Stage.module.css";

interface Props {
  className?: string;
  children?: ReactNode;
}

export function Stage({ className, children }: Props) {
  return <div className={[styles.stage, className].filter(Boolean).join(" ")}>{children}</div>;
}

export function StageScreen({ className, children }: Props) {
  return <div className={[styles.screen, className].filter(Boolean).join(" ")}>{children}</div>;
}
