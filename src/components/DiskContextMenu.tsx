import { useEffect } from "react";

export interface DiskContextMenuItem {
  label: string;
  onSelect: () => void;
  disabled?: boolean;
  danger?: boolean;
}

interface DiskContextMenuProps {
  x: number;
  y: number;
  items: DiskContextMenuItem[];
  onClose: () => void;
}

export function DiskContextMenu({ x, y, items, onClose }: DiskContextMenuProps) {
  useEffect(() => {
    function handleWindowClick() {
      onClose();
    }
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("click", handleWindowClick);
    window.addEventListener("contextmenu", handleWindowClick);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("click", handleWindowClick);
      window.removeEventListener("contextmenu", handleWindowClick);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose]);

  return (
    <div
      className="fixed z-50 min-w-[200px] rounded border border-neutral-700 bg-neutral-900 py-1 text-xs shadow-lg"
      style={{ left: x, top: y }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {items.map((item, i) => (
        <button
          key={i}
          type="button"
          disabled={item.disabled}
          onClick={() => {
            item.onSelect();
            onClose();
          }}
          className={`block w-full px-3 py-1.5 text-left hover:bg-neutral-800 disabled:opacity-40 disabled:hover:bg-transparent ${
            item.danger ? "text-red-400" : "text-neutral-200"
          }`}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
