import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export type BadgeTone = "neutral" | "blue" | "green" | "red" | "orange" | "yellow";

const TONE_CLASSES: Record<BadgeTone, string> = {
  neutral: "bg-neutral-500/15 text-neutral-300",
  blue: "bg-blue-500/15 text-blue-400",
  green: "bg-green-500/15 text-green-400",
  red: "bg-red-500/15 text-red-400",
  orange: "bg-orange-500/15 text-orange-400",
  yellow: "bg-yellow-500/15 text-yellow-400",
};

interface BadgeProps {
  tone?: BadgeTone;
  icon?: ReactNode;
  /** Off for values that must stay in their exact case -- e.g. a template
   * token name like `{Source Name}`, which is a literal identifier, not a
   * decorative label. */
  uppercase?: boolean;
  className?: string;
  children: ReactNode;
}

/** Small pill tag -- color/shape spec lifted directly from the token chip
 * style `TemplateBuilder` already established (`bg-{color}-500/15
 * text-{color}-400`), not a new visual language. */
export function Badge({ tone = "neutral", icon, uppercase = true, className, children }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded px-2 py-0.5 font-mono text-[11px] tracking-wide",
        uppercase && "uppercase",
        TONE_CLASSES[tone],
        className,
      )}
    >
      {icon}
      {children}
    </span>
  );
}
