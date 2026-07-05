import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md";

const VARIANT_CLASSES: Record<ButtonVariant, string> = {
  primary:
    "border border-blue-600 bg-blue-600 font-medium text-white hover:border-blue-500 hover:bg-blue-500",
  secondary:
    "border border-neutral-700 bg-neutral-900 text-neutral-200 hover:border-neutral-600 hover:bg-neutral-800",
  ghost: "border border-transparent text-neutral-400 hover:bg-neutral-800 hover:text-neutral-200",
  danger:
    "border border-neutral-700 bg-neutral-900 text-red-400 hover:border-red-800 hover:bg-red-950/40",
};

/** Extra classes layered on top of `VARIANT_CLASSES` when `active` is set --
 * only meaningful for `ghost`, used for tab/toggle-style buttons (view
 * switcher, Preferences tabs, Parallel/Cascade mode picker) where one option
 * needs to read as "currently selected" rather than just hoverable. */
const ACTIVE_GHOST = "bg-neutral-800 text-neutral-100";

const SIZE_CLASSES: Record<ButtonSize, string> = {
  sm: "gap-1.5 rounded px-2 py-1 text-xs",
  md: "gap-2 rounded px-3 py-1.5 text-sm",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: ReactNode;
  /** Marks a `ghost` button as the currently-selected option in a
   * tab/toggle group. */
  active?: boolean;
}

export function Button({
  variant = "secondary",
  size = "sm",
  icon,
  active,
  className,
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      type="button"
      {...rest}
      className={cn(
        "inline-flex shrink-0 items-center justify-center transition-colors",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/60",
        "disabled:pointer-events-none disabled:opacity-40",
        VARIANT_CLASSES[variant],
        variant === "ghost" && active && ACTIVE_GHOST,
        SIZE_CLASSES[size],
        className,
      )}
    >
      {icon}
      {children}
    </button>
  );
}
