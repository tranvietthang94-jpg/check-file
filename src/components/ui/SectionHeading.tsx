import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

interface SectionHeadingProps {
  as?: "h2" | "h3";
  icon?: ReactNode;
  action?: ReactNode;
  className?: string;
  children: ReactNode;
}

/** Replaces the `text-sm font-semibold uppercase tracking-wide
 * text-neutral-400` heading class string every panel/section previously
 * repeated by hand, with optional leading icon and trailing action slot
 * (e.g. a "Verify" button next to a heading). */
export function SectionHeading({ as: Tag = "h2", icon, action, className, children }: SectionHeadingProps) {
  return (
    <div className={cn("flex items-center justify-between gap-2", className)}>
      <Tag className="flex items-center gap-1.5 text-sm font-semibold uppercase tracking-wide text-neutral-400">
        {icon}
        {children}
      </Tag>
      {action}
    </div>
  );
}
