import Link from "next/link";
import type { ReactNode } from "react";
import { Arrow } from "./Arrow";
import styles from "./TextLink.module.css";

interface Props {
  href: string;
  arrow?: boolean;
  className?: string;
  children: ReactNode;
}

export function TextLink({ href, arrow = true, className, children }: Props) {
  const classes = [styles.link, className].filter(Boolean).join(" ");
  const content = (
    <>
      {children}
      {arrow && (
        <span className={styles.arrow}>
          <Arrow />
        </span>
      )}
    </>
  );
  if (href.startsWith("/")) {
    return (
      <Link href={href} className={classes}>
        {content}
      </Link>
    );
  }
  return (
    <a href={href} className={classes} rel="noopener">
      {content}
    </a>
  );
}
