import type { HTMLAttributes } from "react";
import "./kit.css";

export type KeybindVariant = "neutral" | "ghost";

export type KeybindProps = Omit<HTMLAttributes<HTMLSpanElement>, "children"> & {
  keys: string[];
  variant?: KeybindVariant;
  keepCase?: boolean;
};

function label(key: string, keepCase: boolean) {
  return !keepCase && [...key].length === 1 ? key.toUpperCase() : key;
}

export function Keybind({
  keys,
  variant = "neutral",
  keepCase = false,
  className,
  ...rest
}: KeybindProps) {
  return (
    <span
      {...rest}
      className={className ? `kit-keybind ${className}` : "kit-keybind"}
      data-variant={variant}
    >
      {keys.map((key, index) => (
        <kbd key={`${key}-${index}`} className="kit-keycap">
          {label(key, keepCase)}
        </kbd>
      ))}
    </span>
  );
}
