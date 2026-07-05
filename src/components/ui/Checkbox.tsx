import type { InputHTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

interface ToggleFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: ReactNode;
  className?: string;
}

const LABEL_CLASSES =
  "flex items-center gap-2 text-xs text-neutral-300 transition-colors hover:text-neutral-100 has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-40 has-[:disabled]:hover:text-neutral-300";

/** Wraps a native `<input type="checkbox">` (kept for real form semantics
 * and the existing `accent-color` theming in `App.css`) with a consistent
 * label row -- every Preferences tab and `AddTransfersBar` previously built
 * this row by hand with slightly different classes each time. */
export function Checkbox({ label, className, ...rest }: ToggleFieldProps) {
  return (
    <label className={cn(LABEL_CLASSES, "cursor-pointer", className)}>
      <input type="checkbox" {...rest} className="h-3.5 w-3.5 accent-green-500" />
      {label}
    </label>
  );
}

export function Radio({ label, className, ...rest }: ToggleFieldProps) {
  return (
    <label className={cn(LABEL_CLASSES, "cursor-pointer", className)}>
      <input type="radio" {...rest} className="h-3.5 w-3.5 accent-green-500" />
      {label}
    </label>
  );
}
