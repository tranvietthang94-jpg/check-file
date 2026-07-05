import type { ComponentPropsWithoutRef, ElementType, ReactNode } from "react";
import { cn } from "../../lib/cn";

type PanelProps<T extends ElementType> = {
  as?: T;
  className?: string;
  children?: ReactNode;
} & Omit<ComponentPropsWithoutRef<T>, "as" | "className" | "children">;

/** Consistent card wrapper (soft border + subtle elevation) for the app's
 * top-level sections/modals -- polymorphic via `as` since some call sites
 * need a `<section>` root for semantics (EndpointList, DisksPanel,
 * TransfersPanel) and others just need a `<div>` (modal shells). */
export function Panel<T extends ElementType = "div">({
  as,
  className,
  children,
  ...rest
}: PanelProps<T>) {
  const Tag = (as ?? "div") as ElementType;
  return (
    <Tag
      className={cn("rounded border border-neutral-800 bg-neutral-900 shadow-sm shadow-black/20", className)}
      {...rest}
    >
      {children}
    </Tag>
  );
}
