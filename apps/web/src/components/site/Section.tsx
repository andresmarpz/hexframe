import type { CSSProperties, ElementType, ReactNode } from "react";
import styles from "./Section.module.css";

interface Props {
  as?: ElementType;
  id?: string;
  tone?: "paper" | "fog" | "ink";
  padding?: number | "default";
  paddingMobile?: number | "default";
  className?: string;
  innerClassName?: string;
  children?: ReactNode;
}

export function Section({
  as: Tag = "section",
  id,
  tone = "paper",
  padding = "default",
  paddingMobile = "default",
  className,
  innerClassName,
  children,
}: Props) {
  const style: Record<string, string> = {};
  if (padding !== "default") style["--section-pad-desktop"] = `${padding}px`;
  if (paddingMobile !== "default") style["--section-pad-mobile"] = `${paddingMobile}px`;
  const classes = [styles.section, styles[tone], className].filter(Boolean).join(" ");
  return (
    <Tag id={id} className={classes} style={style as CSSProperties}>
      <div className={[styles.inner, innerClassName].filter(Boolean).join(" ")}>{children}</div>
    </Tag>
  );
}
