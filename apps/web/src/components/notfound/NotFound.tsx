"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { Button } from "../kit/Button";
import { TiltMark } from "../site/TiltMark";
import styles from "./NotFound.module.css";

const START_SECONDS = 5;

export function NotFound() {
  const router = useRouter();
  const [seconds, setSeconds] = useState(START_SECONDS);
  const [cancelled, setCancelled] = useState(false);

  useEffect(() => {
    const cancel = () => setCancelled(true);
    const events = ["pointerdown", "keydown", "wheel", "touchstart"] as const;
    for (const name of events) window.addEventListener(name, cancel, { passive: true, once: true });
    return () => {
      for (const name of events) window.removeEventListener(name, cancel);
    };
  }, []);

  useEffect(() => {
    if (cancelled) return;
    if (seconds <= 0) {
      router.push("/");
      return;
    }
    const timer = window.setTimeout(() => setSeconds((value) => value - 1), 1000);
    return () => window.clearTimeout(timer);
  }, [seconds, cancelled, router]);

  return (
    <section className={styles.page}>
      <TiltMark size={200} className={styles.mark} />
      <h1 className={`display ${styles.headline}`}>this page died.</h1>
      <p className={`lead ${styles.lead}`}>Respawning you on the home page in 0:{String(seconds).padStart(2, "0")}.</p>
      <Button href="/" variant="contrast" size="xxl">
        Back to home
      </Button>
    </section>
  );
}
