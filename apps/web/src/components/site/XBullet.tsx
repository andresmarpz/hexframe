interface Props {
  size?: number;
  className?: string;
}

export function XBullet({ size = 20, className }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      className={className}
      style={{ flexShrink: 0 }}
      aria-hidden="true"
      focusable="false"
    >
      <path d="M4 4L16 16M16 4L4 16" stroke="currentColor" strokeWidth="3.5" strokeLinecap="round" />
    </svg>
  );
}
