import type { HTMLAttributes } from "react";
import "./kit.css";

export type BadgeVariant = "neutral" | "highContrast" | "subtle" | "accent";
export type BadgeAppearance = "compact" | "standard";

export type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  variant?: BadgeVariant;
  appearance?: BadgeAppearance;
};

export function Badge({
  variant = "neutral",
  appearance = "compact",
  className,
  ...rest
}: BadgeProps) {
  return (
    <span
      {...rest}
      className={className ? `kit-badge ${className}` : "kit-badge"}
      data-variant={variant}
      data-appearance={appearance}
    />
  );
}
