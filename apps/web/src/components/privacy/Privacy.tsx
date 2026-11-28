import { privacy } from "../../content/privacy";
import { TextLink } from "../site/TextLink";
import { XBullet } from "../site/XBullet";
import styles from "./Privacy.module.css";

function Bullets({ items }: { items: string[] }) {
  return (
    <ul className={styles.bullets}>
      {items.map((item) => (
        <li key={item} className={styles.bullet}>
          <XBullet size={14} />
          <p className="body">{item}</p>
        </li>
      ))}
    </ul>
  );
}

export function Privacy() {
  return (
    <>
      <div className={styles.head}>
        <h1 className="display">privacy.</h1>
        <p className={`lead ${styles.intro}`}>{privacy.intro}</p>
        <p className={`label ${styles.updated}`}>{privacy.updated}</p>
      </div>
      <div className={styles.sections}>
        <section className={styles.section}>
          <h2 className={`h3 ${styles.title}`}>{privacy.localTitle}</h2>
          <div className={styles.content}>
            <Bullets items={privacy.local} />
          </div>
        </section>
        <section className={styles.section}>
          <h2 className={`h3 ${styles.title}`}>{privacy.sentTitle}</h2>
          <div className={`${styles.content} ${styles.sent}`}>
            {privacy.sent.map((entry) => (
              <div key={entry.title} className={styles.entry}>
                <h3 className={styles.entryTitle}>{entry.title}</h3>
                <p className="body">{entry.body}</p>
              </div>
            ))}
          </div>
        </section>
        <section className={styles.section}>
          <h2 className={`h3 ${styles.title}`}>{privacy.websiteTitle}</h2>
          <div className={styles.content}>
            <Bullets items={privacy.website} />
          </div>
        </section>
        <section className={styles.section}>
          <h2 className={`h3 ${styles.title}`}>{privacy.contactTitle}</h2>
          <div className={`${styles.content} ${styles.contact}`}>
            <p className="body">{privacy.contact}</p>
            <div className={styles.links}>
              {privacy.contactLinks.map((link) => (
                <TextLink key={link.href} href={link.href}>
                  {link.label}
                </TextLink>
              ))}
            </div>
            <p className="body">{privacy.changes}</p>
          </div>
        </section>
      </div>
    </>
  );
}
