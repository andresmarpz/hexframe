interface Props {
  tone?: "dark" | "light";
  size?: number;
  className?: string;
  faceClassName?: string;
  title?: string;
}

export function Mark({ tone = "dark", size = 32, className, faceClassName, title }: Props) {
  const tile = tone === "dark" ? "var(--color-site-ink)" : "var(--color-site-paper)";
  const face = tone === "dark" ? "var(--color-site-paper)" : "var(--color-site-ink)";
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      fill="none"
      className={className}
      role={title ? "img" : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      focusable="false"
    >
      <rect x="4" y="4" width="56" height="56" rx="16" fill={tile} />
      <path
        className={faceClassName}
        d="M14.54 23.99L26.66 31.27M24.25 21.57L16.96 33.69M33.95 19.15L46.07 26.43M43.65 16.73L36.37 28.85M33.45 46.07L48 38.32"
        stroke={face}
        strokeWidth="5.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
