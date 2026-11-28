"use client";

import { useEffect, useRef, useState } from "react";
import { Mark } from "../site/Mark";
import styles from "./TiltMark.module.css";

interface Props {
  size: number;
  className?: string;
  trigger?: "load" | "view";
}

export function TiltMark({ size, className, trigger = "view" }: Props) {
  const root = useRef<HTMLSpanElement>(null);
  const [phase, setPhase] = useState<"rest" | "upright" | "tilt">("rest");

  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const element = root.current;
    if (trigger === "load" || !element) {
      setPhase("tilt");
      return;
    }
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
  }, [trigger]);

  return (
    <span ref={root} className={[styles.root, className].filter(Boolean).join(" ")} data-phase={phase}>
      <Mark size={size} faceClassName={styles.face} />
    </span>
  );
}
