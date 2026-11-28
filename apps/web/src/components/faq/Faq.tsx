import { faqGroups, helpSection, type FaqItem } from "../../content/faq";
import { withCode } from "../../lib/inline-code";
import { Button } from "../kit/Button";
import styles from "./Faq.module.css";

function Answer({ item }: { item: FaqItem }) {
  return (
    <div className={styles.item} id={item.id}>
      <h3 className={styles.question}>{item.q}</h3>
      <div className={styles.answer}>
        {item.a.map((paragraph) => (
          <p key={paragraph} className="body" dangerouslySetInnerHTML={{ __html: withCode(paragraph) }} />
        ))}
      </div>
    </div>
  );
}

function Help() {
  const [first, ...rest] = helpSection.items;
  return (
    <section className={styles.help} id={helpSection.id}>
      <div className={styles.card}>
        <div className={styles.helpMain}>
          <div className={styles.helpHead}>
            <h2 className="h3">{helpSection.title}</h2>
            <p className="body">{helpSection.intro}</p>
          </div>
          {first && (
            <div className={styles.helpItem}>
              <h3 className={styles.question}>{first.title}</h3>
              <p className="body">{first.body}</p>
            </div>
          )}
          <p className={styles.note} dangerouslySetInnerHTML={{ __html: withCode(helpSection.logsNote) }} />
        </div>
        <div className={styles.helpSide}>
          {rest.map((item) => (
            <div key={item.title} className={styles.helpSide}>
              <p className="body">{item.body}</p>
              {"href" in item && item.href && (
                <Button variant="contrast" size="xxl" href={item.href} rel="noopener">
                  {item.title}
                </Button>
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

export function Faq() {
  return (
    <>
      <div className={styles.head}>
        <h1 className="display">faq.</h1>
      </div>
      <div className={styles.groups}>
        {faqGroups.map((group) => (
          <section key={group.id} id={group.id} className={styles.group} aria-labelledby={`${group.id}-title`}>
            <h2 id={`${group.id}-title`} className={`label ${styles.groupTitle}`}>
              {group.title}
            </h2>
            <div className={styles.column}>
              {group.intro && <p className={`lead ${styles.intro}`}>{group.intro}</p>}
              {group.items.map((item) => (
                <Answer key={item.id} item={item} />
              ))}
            </div>
          </section>
        ))}
      </div>
      <Help />
    </>
  );
}
