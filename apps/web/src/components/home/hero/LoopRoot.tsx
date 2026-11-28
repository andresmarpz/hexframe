"use client";

import { useEffect, useRef, type ReactNode } from "react";

interface Props {
  className?: string;
  children?: ReactNode;
}

export function LoopRoot({ className, children }: Props) {
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = root.current;
    if (!element) return;
    let onScreen = false;
    const apply = () => {
      element.dataset.paused = String(!onScreen || document.hidden);
    };
    const observer = new IntersectionObserver(
      (entries) => {
        onScreen = entries.some((entry) => entry.isIntersecting);
        apply();
      },
      { threshold: 0.25 },
    );
    observer.observe(element);
    document.addEventListener("visibilitychange", apply);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", apply);
    };
  }, []);

  return (
    <div ref={root} className={className} data-paused="true">
      {children}
    </div>
  );
}
