import type { InputHTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

interface ToggleFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: ReactNode;
  /** "start" aligns the checkbox to the top of a multi-line label (e.g. a
   * title + description pair) instead of centering it. */
  align?: "center" | "start";
  className?: string;
}

const BASE_LABEL_CLASSES =
  "flex gap-2 text-xs text-neutral-300 transition-colors hover:text-neutral-100 has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-40 has-[:disabled]:hover:text-neutral-300";

/** Wraps a native `<input type="checkbox">` (kept for real form semantics
 * and the existing `accent-color` theming in `App.css`) with a consistent
 * label row -- every Preferences tab and `AddTransfersBar` previously built
 * this row by hand with slightly different classes each time. */
export function Checkbox({ label, align = "center", className, ...rest }: ToggleFieldProps) {
  return (
    <label
      className={cn(
        BASE_LABEL_CLASSES,
        align === "start" ? "items-start" : "items-center",
        "cursor-pointer",
        className,
      )}
    >
      <input
        type="checkbox"
        {...rest}
        className={cn("h-3.5 w-3.5 shrink-0 accent-green-500", align === "start" && "mt-0.5")}
      />
      {label}
    </label>
  );
}

export function Radio({ label, align = "center", className, ...rest }: ToggleFieldProps) {
  return (
    <label
      className={cn(
        BASE_LABEL_CLASSES,
        align === "start" ? "items-start" : "items-center",
        "cursor-pointer",
        className,
      )}
    >
      <input
        type="radio"
        {...rest}
        className={cn("h-3.5 w-3.5 shrink-0 accent-green-500", align === "start" && "mt-0.5")}
      />
      {label}
    </label>
  );
}
