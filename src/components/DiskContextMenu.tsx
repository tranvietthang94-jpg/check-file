import { useEffect, useState } from "react";
import type { ReactNode } from "react";

export interface DiskContextMenuItem {
  label: string;
  /** Small leading icon (e.g. from `components/icons`) -- purely decorative,
   * omit for items where no obviously-matching icon exists. */
  icon?: ReactNode;
  /** Omitted for a parent item that only exists to host `children` -- e.g.
   * "Source Folder ▶" itself does nothing on click, only its submenu items do. */
  onSelect?: () => void;
  disabled?: boolean;
  danger?: boolean;
  /** Renders as a "▶" flyout submenu instead of a clickable action, mirroring
   * OffShoot's "Source Folder ▶" / "Destination Folder ▶" / "Verification ▶". */
  children?: DiskContextMenuItem[];
}

interface DiskContextMenuProps {
  x: number;
  y: number;
  items: DiskContextMenuItem[];
  onClose: () => void;
}

function MenuItems({
  items,
  onClose,
}: {
  items: DiskContextMenuItem[];
  onClose: () => void;
}) {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  return (
    <div
      className="min-w-[200px] rounded border border-neutral-700 bg-neutral-900 py-1 text-xs shadow-lg"
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item, i) => (
        <div key={i} className="relative" onMouseEnter={() => setOpenIndex(i)}>
          <button
            type="button"
            disabled={item.disabled}
            onClick={() => {
              if (item.children) return;
              item.onSelect?.();
              onClose();
            }}
            className={`flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left hover:bg-neutral-800 disabled:opacity-40 disabled:hover:bg-transparent ${
              item.danger ? "text-red-400" : "text-neutral-200"
            }`}
          >
            <span className="flex items-center gap-2">
              {item.icon && <span className="text-neutral-500">{item.icon}</span>}
              {item.label}
            </span>
            {item.children && <span className="text-neutral-500">▶</span>}
          </button>
          {item.children && openIndex === i && !item.disabled && (
            <div className="absolute left-full top-0 z-50">
              <MenuItems items={item.children} onClose={onClose} />
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

export function DiskContextMenu({ x, y, items, onClose }: DiskContextMenuProps) {
  useEffect(() => {
    function handleWindowClick() {
      onClose();
    }
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    // The click/right-click that *opens* this menu is often still bubbling
    // up to `window` at the moment this effect runs (React 18 flushes a
    // discrete event's resulting effects synchronously, before the browser
    // finishes dispatching that same event past `window`) -- attaching
    // these two listeners on the very next tick instead of immediately
    // means the opening click can't also be the closing click.
    const timer = setTimeout(() => {
      window.addEventListener("click", handleWindowClick);
      window.addEventListener("contextmenu", handleWindowClick);
    }, 0);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("click", handleWindowClick);
      window.removeEventListener("contextmenu", handleWindowClick);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose]);

  return (
    <div className="fixed z-50" style={{ left: x, top: y }}>
      <MenuItems items={items} onClose={onClose} />
    </div>
  );
}
