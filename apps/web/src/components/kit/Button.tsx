"use client";

import type { AnchorHTMLAttributes, ButtonHTMLAttributes, ReactNode } from "react";
import "./kit.css";

export type ButtonVariant =
  | "neutral"
  | "danger"
  | "warning"
  | "outline"
  | "contrast"
  | "submit"
  | "ghost"
  | "ghostMuted"
  | "ghostFaint"
  | "loading";

export type ButtonSize = "mini" | "small" | "regular" | "large" | "xl" | "xxl";

type Shared = {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: ReactNode;
  disabled?: boolean;
  className?: string;
  children?: ReactNode;
};

export type ButtonProps = Shared &
  (
    | ({ href: string } & Omit<AnchorHTMLAttributes<HTMLAnchorElement>, keyof Shared>)
    | ({ href?: undefined } & Omit<ButtonHTMLAttributes<HTMLButtonElement>, keyof Shared>)
  );

export function Button(props: ButtonProps) {
  const {
    variant = "neutral",
    size = "regular",
    icon,
    disabled,
    className,
    children,
    ...rest
  } = props;
  const classes = className ? `kit-button ${className}` : "kit-button";
  const content = (
    <>
      {icon}
      {children}
    </>
  );

  if (rest.href !== undefined) {
    const { href, onClick, ...anchor } = rest as AnchorHTMLAttributes<HTMLAnchorElement>;
    return (
      <a
        {...anchor}
        href={disabled ? undefined : href}
        onClick={disabled ? (event) => event.preventDefault() : onClick}
        aria-disabled={disabled || undefined}
        className={classes}
        data-variant={variant}
        data-size={size}
      >
        {content}
      </a>
    );
  }

  return (
    <button
      type="button"
      {...(rest as ButtonHTMLAttributes<HTMLButtonElement>)}
      disabled={disabled}
      aria-busy={variant === "loading" || undefined}
      className={classes}
      data-variant={variant}
      data-size={size}
    >
      {content}
    </button>
  );
}
