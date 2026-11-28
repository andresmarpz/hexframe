import { Callout } from "../site/Callout";
import { Mark } from "../site/Mark";
import { RecDot } from "../site/RecDot";
import { LoopRoot } from "./hero/LoopRoot";
import styles from "./hero/HeroStage.module.css";

const ALT = "The Greyscreen app showing a finished game with its highlights and timeline";

export function HeroStage() {
  return (
    <section className={styles.section}>
      <div className={styles.desktop}>
        <LoopRoot className={styles.band}>
          <div className={styles.frame}>
            <div className={styles.canvas}>
              <div className={styles.screen}>
                <img className={styles.image} src="/screens/hero-desktop.webp" width={1280} height={712} alt={ALT} loading="lazy" />
              </div>

              <div className={styles.markerRing} aria-hidden="true">
                <span className={styles.marker} />
              </div>

              <div className={styles.clipLayer} aria-hidden="true">
                <span className={styles.ghost} />
                <img className={styles.clipCard} src="/screens/hero-clip.webp" width={299} height={168} alt="" loading="lazy" />
              </div>

              <div className={styles.drop} aria-hidden="true">
                <Mark size={96} faceClassName={styles.dropFace} />
              </div>
              <div className={styles.timer} aria-hidden="true">
                <span className={styles.timerText} />
              </div>

              <Callout
                line="none"
                className={`${styles.callout} ${styles.rec}`}
                label={
                  <>
                    <RecDot />
                    <span>
                      REC <span className={styles.recTime} />
                    </span>
                  </>
                }
              >
                Started by itself.
              </Callout>

              <span className={`${styles.hairline} ${styles.deathLine}`} aria-hidden="true" />
              <Callout line="none" className={`${styles.callout} ${styles.death}`} label="DEATH 3 OF 7">
                Tagged: Ganked.
              </Callout>

              <span className={`${styles.hairline} ${styles.clipLine}`} aria-hidden="true" />
              <Callout line="none" className={`${styles.callout} ${styles.clip}`} label="CLIP SAVED">
                Double kill · 0:29
              </Callout>
            </div>
          </div>
        </LoopRoot>
      </div>

      <div className={styles.mobile}>
        <div className={styles.mobileBand}>
          <div className={styles.mobileFrame}>
            <div className={styles.mobileScreen}>
              <img src="/screens/hero-mobile.webp" width={350} height={392} alt={ALT} loading="lazy" />
            </div>
            <Callout
              line="none"
              className={`${styles.callout} ${styles.mobileRec}`}
              label={
                <>
                  <RecDot />
                  <span>REC 24:31</span>
                </>
              }
            >
              Started by itself.
            </Callout>
          </div>
        </div>
      </div>
    </section>
  );
}
