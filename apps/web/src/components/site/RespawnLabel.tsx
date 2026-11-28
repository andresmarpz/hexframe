"use client";

import { useEffect, useRef } from "react";
import styles from "./RespawnLabel.module.css";

interface Props {
  seconds: number;
  className?: string;
}

const START_SECONDS = 30;
const DURATION_MS = 600;
const RING_LENGTH = 1;

const easeOut = (t: number) => 1 - Math.pow(1 - t, 3);
const clock = (seconds: number) => `0:${String(Math.max(0, Math.round(seconds))).padStart(2, "0")}`;

export function RespawnLabel({ seconds, className }: Props) {
  const root = useRef<HTMLDivElement>(null);
  const arc = useRef<SVGCircleElement>(null);
  const time = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    const element = root.current;
    const ring = arc.current;
    const text = time.current;
    if (!element || !ring || !text) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches || seconds >= START_SECONDS) return;

    const offsetFor = (value: number) => String(RING_LENGTH * (1 - value / START_SECONDS));
    const paint = (value: number) => {
      ring.style.strokeDashoffset = offsetFor(value);
      ring.style.opacity = value > 0.5 ? "1" : "0";
      text.textContent = clock(value);
    };

    paint(START_SECONDS);
    let frame = 0;
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        const startedAt = performance.now();
        const step = (now: number) => {
          const progress = Math.min(1, (now - startedAt) / DURATION_MS);
          paint(START_SECONDS + (seconds - START_SECONDS) * easeOut(progress));
          if (progress < 1) frame = requestAnimationFrame(step);
        };
        frame = requestAnimationFrame(step);
      },
      { threshold: 0.6 },
    );
    observer.observe(element);

    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
      paint(seconds);
    };
  }, [seconds]);

  const fraction = Math.min(1, Math.max(0, seconds / START_SECONDS));
  return (
    <div ref={root} className={[styles.label, className].filter(Boolean).join(" ")}>
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true" focusable="false">
        <circle cx="8" cy="8" r="7" stroke="var(--color-site-concrete)" strokeWidth="2" />
        <circle
          ref={arc}
          cx="8"
          cy="8"
          r="7"
          stroke="var(--color-site-ink)"
          strokeWidth="2"
          strokeLinecap="round"
          pathLength={RING_LENGTH}
          strokeDasharray={RING_LENGTH}
          strokeDashoffset={RING_LENGTH * (1 - fraction)}
          transform="rotate(-90 8 8)"
          style={{ opacity: fraction > 0 ? 1 : 0 }}
        />
      </svg>
      <span className="sr-only">Respawn {clock(seconds)}</span>
      <span className={styles.text} aria-hidden="true">
        RESPAWN <span ref={time}>{clock(seconds)}</span>
      </span>
    </div>
  );
}
