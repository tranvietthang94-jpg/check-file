import { useEffect, useLayoutEffect, useRef, useState } from "react";
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
  onBack,
}: {
  items: DiskContextMenuItem[];
  onClose: () => void;
  onBack?: () => void;
}) {
  const [openIndex, setOpenIndex] = useState<number | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  function focusFirstChild(index: number) {
    setOpenIndex(index);
    setTimeout(() => {
      const parent = menuRef.current?.querySelectorAll<HTMLElement>(".context-menu-row")[index];
      const submenu = Array.from(parent?.children ?? []).find((child) =>
        (child as HTMLElement).querySelector(':scope > [role="menu"]'),
      ) as HTMLElement | undefined;
      submenu?.querySelector<HTMLElement>('[role="menuitem"]:not(:disabled)')?.focus();
    });
  }

  return (
    <div
      ref={menuRef}
      role="menu"
      className="min-w-[200px] rounded border border-neutral-700 bg-neutral-900 py-1 text-xs shadow-lg"
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item, i) => (
        <div key={i} className="context-menu-row relative" onMouseEnter={() => setOpenIndex(i)}>
          <button
            type="button"
            role="menuitem"
            tabIndex={-1}
            disabled={item.disabled}
            onKeyDown={(e) => {
              if (e.key === "ArrowRight" && item.children) {
                e.preventDefault();
                e.stopPropagation();
                focusFirstChild(i);
              } else if (e.key === "ArrowLeft" && onBack) {
                e.preventDefault();
                e.stopPropagation();
                onBack();
              }
            }}
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
              <MenuItems
                items={item.children}
                onClose={onClose}
                onBack={() => {
                  setOpenIndex(null);
                  requestAnimationFrame(() => {
                    menuRef.current?.querySelectorAll<HTMLButtonElement>(":scope > .context-menu-row > [role=menuitem]")[i]?.focus();
                  });
                }}
              />
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

export function DiskContextMenu({ x, y, items, onClose }: DiskContextMenuProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ x, y });

  useLayoutEffect(() => {
    const rect = containerRef.current?.getBoundingClientRect();
    if (!rect) return;
    setPosition({
      x: Math.max(8, Math.min(x, window.innerWidth - rect.width - 8)),
      y: Math.max(8, Math.min(y, window.innerHeight - rect.height - 8)),
    });
  }, [x, y]);

  useEffect(() => {
    function enabledItems() {
      return Array.from(
        containerRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? [],
      );
    }
    function handleWindowClick() {
      onClose();
    }
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp" && e.key !== "Home" && e.key !== "End") return;
      e.preventDefault();
      const buttons = enabledItems();
      if (buttons.length === 0) return;
      const activeIndex = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const nextIndex = e.key === "Home"
        ? 0
        : e.key === "End"
          ? buttons.length - 1
          : e.key === "ArrowDown"
            ? (activeIndex + 1 + buttons.length) % buttons.length
            : (activeIndex - 1 + buttons.length) % buttons.length;
      buttons[nextIndex].focus();
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
    <div ref={containerRef} className="fixed z-50" style={{ left: position.x, top: position.y }}>
      <MenuItems items={items} onClose={onClose} />
    </div>
  );
}
