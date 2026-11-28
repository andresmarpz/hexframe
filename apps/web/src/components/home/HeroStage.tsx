import { Callout } from "../site/Callout";
import { Mark } from "../site/Mark";
import { RecDot } from "../site/RecDot";
import { LoopRoot } from "./hero/LoopRoot";
import styles from "./hero/HeroStage.module.css";
import motion from "./hero/HeroStageMotion.module.css";

const ALT = "The Greyscreen app showing a finished game with its highlights and timeline";

export function HeroStage() {
  return (
    <section className={styles.section}>
      <div className={styles.desktop}>
        <LoopRoot className={motion.band}>
          <div className={styles.frame}>
            <div className={styles.canvas}>
              <div className={styles.screen}>
                <img className={motion.image} src="/screens/hero-desktop.webp" width={1280} height={712} alt={ALT} fetchPriority="high" />
              </div>

              <div className={motion.markerRing} aria-hidden="true">
                <span className={styles.marker} />
              </div>

              <div className={styles.clipLayer} aria-hidden="true">
                <span className={motion.ghost} />
                <img className={motion.clipCard} src="/screens/hero-clip.webp" width={299} height={168} alt="" loading="lazy" />
              </div>

              <div className={motion.drop} aria-hidden="true">
                <Mark size={96} faceClassName={motion.dropFace} />
              </div>
              <div className={motion.timer} aria-hidden="true">
                <span className={motion.timerText} />
              </div>

              <Callout
                line="none"
                className={`${styles.callout} ${motion.rec}`}
                label={
                  <>
                    <RecDot />
                    <span>
                      REC <span className={motion.recTime} />
                    </span>
                  </>
                }
              >
                Started by itself.
              </Callout>

              <span className={`${styles.hairline} ${motion.deathLine}`} aria-hidden="true" />
              <Callout line="none" className={`${styles.callout} ${motion.death}`} label="DEATH 3 OF 7">
                Tagged: Ganked.
              </Callout>

              <span className={`${styles.hairline} ${motion.clipLine}`} aria-hidden="true" />
              <Callout line="none" className={`${styles.callout} ${motion.clip}`} label="CLIP SAVED">
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
