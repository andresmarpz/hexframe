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
    let engaged = false;
    const apply = () => {
      element.dataset.paused = String(!onScreen || document.hidden || engaged);
    };
    const engage = (value: boolean) => () => {
      engaged = value;
      apply();
    };
    const enter = engage(true);
    const leave = engage(false);
    element.addEventListener("pointerenter", enter);
    element.addEventListener("pointerleave", leave);
    element.addEventListener("focusin", enter);
    element.addEventListener("focusout", leave);
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
      element.removeEventListener("pointerenter", enter);
      element.removeEventListener("pointerleave", leave);
      element.removeEventListener("focusin", enter);
      element.removeEventListener("focusout", leave);
      document.removeEventListener("visibilitychange", apply);
    };
  }, []);

  return (
    <div ref={root} className={className} data-paused="true">
      {children}
    </div>
  );
}
