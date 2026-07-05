import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

interface EmptyStateProps {
  icon: ReactNode;
  children: ReactNode;
  className?: string;
}

/** Replaces a bare `<p className="text-sm text-neutral-500">` used for every
 * "nothing here yet" message across the app with a small icon + message
 * block, so an empty list reads as an intentional state rather than missing
 * content. */
export function EmptyState({ icon, children, className }: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center gap-2 rounded border border-dashed border-neutral-800 px-4 py-6 text-center",
        className,
      )}
    >
      <span className="text-neutral-600">{icon}</span>
      <p className="text-sm text-neutral-500">{children}</p>
    </div>
  );
}
