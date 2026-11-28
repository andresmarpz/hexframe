"use client";

import { useEffect, useRef, useState } from "react";
import { Mark } from "./Mark";
import styles from "./TiltMark.module.css";

interface Props {
  size: number;
  tone?: "dark" | "light";
  className?: string;
  markClassName?: string;
  when?: "load" | "view";
}

export function TiltMark({ size, tone, className, markClassName, when = "load" }: Props) {
  const root = useRef<HTMLSpanElement>(null);
  const [phase, setPhase] = useState<"rest" | "upright" | "tilt">(when === "load" ? "tilt" : "rest");

  useEffect(() => {
    if (when === "load") return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const element = root.current;
    if (!element) return;
    setPhase("upright");
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        setPhase("tilt");
      },
      { threshold: 0.6 },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [when]);

  return (
    <span ref={root} className={[styles.root, className].filter(Boolean).join(" ")} data-phase={phase}>
      <Mark size={size} tone={tone} className={markClassName} faceClassName={styles.face} />
    </span>
  );
}
