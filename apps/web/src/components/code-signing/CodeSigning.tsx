import { codeSigning } from "../../content/code-signing";
import styles from "../privacy/Privacy.module.css";
import own from "./CodeSigning.module.css";
import { TextLink } from "../site/TextLink";
import { XBullet } from "../site/XBullet";

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

export function CodeSigning() {
  const { sponsor } = codeSigning;
  return (
    <>
      <div className={styles.head}>
        <h1 className="display">code signing policy.</h1>
        <p className={`lead ${styles.intro}`}>
          {sponsor.before}
          <a href={sponsor.signPath.href} className={own.inline} rel="noopener">
            {sponsor.signPath.label}
          </a>
          {sponsor.middle}
          <a href={sponsor.foundation.href} className={own.inline} rel="noopener">
            {sponsor.foundation.label}
          </a>
        </p>
        <p className={`label ${styles.updated}`}>{codeSigning.updated}</p>
      </div>
      <div className={styles.sections}>
        <section className={styles.section}>
          <h2 className={`h3 ${styles.title}`}>{codeSigning.signedTitle}</h2>
          <div className={`${styles.content} ${styles.contact}`}>
            <Bullets items={codeSigning.signed} />
            <div className={styles.links}>
              {codeSigning.links.map((link) => (
                <TextLink key={link.href} href={link.href}>
                  {link.label}
                </TextLink>
              ))}
            </div>
          </div>
        </section>
        <section className={styles.section}>
          <h2 className={`h3 ${styles.title}`}>{codeSigning.teamTitle}</h2>
          <div className={`${styles.content} ${styles.sent}`}>
            {codeSigning.team.map((entry) => (
              <div key={entry.role} className={styles.entry}>
                <h3 className={styles.entryTitle}>{entry.role}</h3>
                <p className="body">
                  {entry.people.map((person) => (
                    <a key={person.href} href={person.href} className={own.inline} rel="noopener">
                      {person.label}
                    </a>
                  ))}
                </p>
              </div>
            ))}
            <p className="body">{codeSigning.teamNote}</p>
          </div>
        </section>
        <section className={styles.section}>
          <h2 className={`h3 ${styles.title}`}>{codeSigning.privacyTitle}</h2>
          <div className={`${styles.content} ${styles.contact}`}>
            <p className="body">{codeSigning.privacy}</p>
            <div className={styles.links}>
              <TextLink href={codeSigning.privacyLink.href}>{codeSigning.privacyLink.label}</TextLink>
            </div>
          </div>
        </section>
      </div>
    </>
  );
}
