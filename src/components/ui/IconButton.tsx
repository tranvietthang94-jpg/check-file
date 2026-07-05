import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

type IconButtonTone = "neutral" | "blue" | "red" | "green";

const TONE_CLASSES: Record<IconButtonTone, string> = {
  neutral: "text-neutral-400 hover:text-neutral-200",
  blue: "text-blue-400 hover:text-blue-300",
  red: "text-red-400 hover:text-red-300",
  green: "text-green-500 hover:text-green-400",
};

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  tone?: IconButtonTone;
  icon: ReactNode;
  "aria-label": string;
}

/** Shared circular icon-only button -- replaces the local `ICON_BUTTON`
 * class constant that `TransfersPanel` used to define for itself, so every
 * icon button in the app shares one hover/focus/disabled treatment. */
export function IconButton({ tone = "neutral", icon, className, ...rest }: IconButtonProps) {
  return (
    <button
      type="button"
      {...rest}
      className={cn(
        "flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-neutral-700 transition-colors hover:bg-neutral-800",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/60",
        "disabled:pointer-events-none disabled:opacity-40",
        TONE_CLASSES[tone],
        className,
      )}
    >
      {icon}
    </button>
  );
}
