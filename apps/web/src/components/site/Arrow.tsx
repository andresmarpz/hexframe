import styles from "./Arrow.module.css";

export function Arrow({ className }: { className?: string }) {
  return (
    <svg
      className={className ? `${styles.arrow} ${className}` : styles.arrow}
      width="14"
      height="14"
      viewBox="0 0 14 14"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M1.5 7h11M8 2.5 12.5 7 8 11.5" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
