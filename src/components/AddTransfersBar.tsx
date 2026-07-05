import { useState } from "react";
import { useSettingsStore } from "../state/settingsStore";
import type { DiskInfo, Endpoint } from "../types/disk";
import type { TransferGroupMode } from "../types/transferGroup";

function endpointLabel(endpoint: Endpoint, disks: DiskInfo[]): string {
  const disk = disks.find((d) => d.id === endpoint.diskId);
  return endpoint.label || disk?.name || endpoint.diskId;
}

interface AddTransfersBarProps {
  sources: Endpoint[];
  destinations: Endpoint[];
  disks: DiskInfo[];
  onAdd: (mode: TransferGroupMode, moveAfterTransfer: boolean) => void;
  onClear: () => void;
}

/** Replaces the old per-click "Build Transfer" form: matches OffShoot's own
 * behavior of a confirmation bar that only appears once at least one Source
 * and one Destination exist, and adds every Source's transfer(s) in one
 * click -- Parallel computes the full Source x Destination cross product,
 * Cascade starts one chain per Source through every Destination in the
 * order they're listed (drag rows in the Destinations list to reorder). */
export function AddTransfersBar({
  sources,
  destinations,
  disks,
  onAdd,
  onClear,
}: AddTransfersBarProps) {
  const [mode, setMode] = useState<TransferGroupMode>("parallel");
  const [moveAfterTransfer, setMoveAfterTransfer] = useState(false);
  const verificationMode = useSettingsStore((s) => s.verificationMode);

  if (sources.length === 0 || destinations.length === 0) return null;

  const count = mode === "parallel" ? sources.length * destinations.length : sources.length;
  // Move only makes unambiguous sense with exactly one Destination (more,
  // and every destination reads the same source independently, so there's
  // no single point where deleting it wouldn't race another read), and
  // requires an actual hash comparison -- Transfer mode's size-only check
  // isn't proof enough to delete the only remaining copy of a file.
  const moveEligible = destinations.length === 1 && verificationMode !== "transfer";

  return (
    <div className="fixed inset-x-0 bottom-0 z-40 flex items-center justify-between gap-4 border-t border-neutral-800 bg-neutral-900/95 px-6 py-3 backdrop-blur">
      <button
        type="button"
        onClick={onClear}
        title="Xóa hết Nguồn và Đích"
        className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400 hover:text-neutral-200"
      >
        ✕
      </button>

      <div className="flex flex-1 items-center justify-center gap-4">
        {mode === "cascade" && (
          <span
            className="hidden max-w-xs truncate text-[10px] text-neutral-500 sm:block"
            title="Kéo các dòng trong danh sách Đích để sắp lại thứ tự chuỗi này"
          >
            {destinations.map((d) => endpointLabel(d, disks)).join(" → ")}
          </span>
        )}
        <button
          type="button"
          onClick={() => onAdd(mode, moveAfterTransfer && moveEligible)}
          className="rounded bg-blue-600 px-4 py-1.5 text-sm font-medium text-white hover:bg-blue-500"
        >
          Thêm {count} lượt truyền
        </button>
      </div>

      <div className="flex items-center gap-3 text-xs">
        <div className="flex rounded border border-neutral-700">
          <button
            type="button"
            onClick={() => setMode("parallel")}
            title="Mỗi đích sao chép độc lập từ nguồn của nó"
            className={`px-2 py-1 ${mode === "parallel" ? "bg-neutral-700 text-neutral-100" : "text-neutral-400"}`}
          >
            Song song
          </button>
          <button
            type="button"
            onClick={() => setMode("cascade")}
            title="Nguồn sao chép vào đích đầu tiên, sau đó chuyển tiếp đến các đích còn lại"
            className={`px-2 py-1 ${mode === "cascade" ? "bg-neutral-700 text-neutral-100" : "text-neutral-400"}`}
          >
            Nối tiếp
          </button>
        </div>
        <label
          className={`flex items-center gap-1 ${moveEligible ? "" : "opacity-40"}`}
          title={
            moveEligible
              ? "Xóa từng nguồn sau khi bản sao được xác nhận an toàn"
              : "Chỉ khả dụng khi có đúng 1 Đích và chế độ xác minh là Nguồn/Nguồn & Đích"
          }
        >
          <input
            type="checkbox"
            checked={moveAfterTransfer && moveEligible}
            disabled={!moveEligible}
            onChange={(e) => setMoveAfterTransfer(e.currentTarget.checked)}
          />
          Di chuyển
        </label>
      </div>
    </div>
  );
}
