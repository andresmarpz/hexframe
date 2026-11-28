import Link from "next/link";
import { footerLinks, site } from "../../content/copy";
import { Lockup } from "./Lockup";
import styles from "./Footer.module.css";

const links = [...footerLinks, { label: "Changelog", href: "/changelog/" }];

export function Footer() {
  return (
    <footer className={styles.footer}>
      <div className={styles.inner}>
        <div className={styles.top}>
          <Link href="/" className={styles.brand} aria-label="Greyscreen home">
            <Lockup />
          </Link>
          <nav className={styles.links} aria-label="Footer">
            {links.map((item) =>
              item.href.startsWith("/") ? (
                <Link key={item.href} href={item.href} className={styles.link}>
                  {item.label}
                </Link>
              ) : (
                <a key={item.href} href={item.href} className={styles.link} rel="noopener">
                  {item.label}
                </a>
              ),
            )}
          </nav>
        </div>
        <p className={styles.notice}>{site.riotNotice}</p>
        <div className={styles.crop} aria-hidden="true">
          <span className={styles.wordmark}>greyscreen</span>
        </div>
      </div>
    </footer>
  );
}
