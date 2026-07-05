import { useState } from "react";
import { useOrganizeStore } from "../state/organizeStore";
import { DiskContextMenu } from "./DiskContextMenu";
import { IconButton } from "./ui/IconButton";
import { ChevronDown, ChevronUp } from "./icons";

/**
 * Floating panel in the corner of the Disks view for entering each custom
 * Element's value for the job about to be built -- mirrors OffShoot's
 * Review Pane, which auto-appears whenever custom Elements are in use
 * instead of burying value entry inside Preferences. Right-click anywhere
 * on it to Clear every value, same as OffShoot.
 */
export function ElementsReviewPanel() {
  const elements = useOrganizeStore((s) => s.elements);
  const setElementValue = useOrganizeStore((s) => s.setElementValue);
  const clearElementValues = useOrganizeStore((s) => s.clearElementValues);
  const [collapsed, setCollapsed] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  if (elements.length === 0) return null;

  return (
    <div
      className="fixed bottom-20 right-4 z-30 flex w-64 flex-col gap-2 rounded border border-neutral-700 bg-neutral-900 p-3 text-xs shadow-lg"
      onContextMenu={(e) => {
        e.preventDefault();
        setContextMenu({ x: e.clientX, y: e.clientY });
      }}
    >
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-semibold uppercase tracking-wide text-neutral-400">
          Thành phần
        </span>
        <IconButton
          onClick={() => setCollapsed((c) => !c)}
          title={collapsed ? "Mở rộng" : "Thu gọn"}
          aria-label={collapsed ? "Mở rộng bảng Thành phần" : "Thu gọn bảng Thành phần"}
          icon={
            collapsed ? (
              <ChevronUp className="h-3.5 w-3.5" />
            ) : (
              <ChevronDown className="h-3.5 w-3.5" />
            )
          }
        />
      </div>

      {!collapsed && (
        <div className="flex flex-col gap-1.5">
          {elements.map((element) => (
            <label key={element.name} className="flex items-center gap-2">
              <span
                className="w-20 shrink-0 truncate font-mono text-neutral-500"
                title={`{${element.name}}`}
              >
                {`{${element.name}}`}
              </span>
              <input
                value={element.value}
                onChange={(e) => setElementValue(element.name, e.currentTarget.value)}
                placeholder="Giá trị…"
                autoComplete="off"
                className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
              />
            </label>
          ))}
        </div>
      )}

      {contextMenu && (
        <DiskContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={[{ label: "Xóa hết", onSelect: clearElementValues }]}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
