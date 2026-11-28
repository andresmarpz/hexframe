"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { githubUrl, nav } from "../../content/copy";
import { Button } from "../kit/Button";
import { Lockup } from "./Lockup";
import styles from "./Nav.module.css";

export function Nav() {
  const pathname = usePathname();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  const current = (href: string) => (href === pathname ? "page" : undefined);

  return (
    <header className={styles.nav}>
      <div className={styles.bar}>
        <Link href="/" className={styles.brand} aria-label="Greyscreen home" onClick={() => setOpen(false)}>
          <Lockup />
        </Link>
        <nav className={styles.links} aria-label="Main">
          {nav.map((item) => (
            <Link key={item.href} href={item.href} className={styles.link} aria-current={current(item.href)}>
              {item.label}
            </Link>
          ))}
        </nav>
        <div className={styles.actions}>
          <a className={styles.link} href={githubUrl} rel="noopener">
            GitHub →
          </a>
          <Button href="/download/" variant="contrast" size="large">
            Download
          </Button>
        </div>
        <button
          type="button"
          className={styles.menuButton}
          aria-expanded={open}
          aria-controls="mobile-menu"
          aria-label={open ? "Close menu" : "Open menu"}
          onClick={() => setOpen((value) => !value)}
        >
          <span className={styles.line} />
          <span className={styles.line} />
        </button>
      </div>
      {open && (
        <nav id="mobile-menu" className={styles.sheet} aria-label="Menu">
          {nav.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className={styles.sheetLink}
              aria-current={current(item.href)}
              onClick={() => setOpen(false)}
            >
              {item.label}
            </Link>
          ))}
          <a className={styles.sheetLink} href={githubUrl} rel="noopener">
            GitHub →
          </a>
          <Button href="/download/" variant="contrast" size="xxl" onClick={() => setOpen(false)}>
            Download
          </Button>
        </nav>
      )}
    </header>
  );
}
