import { useState } from "react";
import { DriveIcon } from "./icons/DriveIcon";
import { DISK_DRAG_MIME, ENDPOINT_REORDER_MIME } from "../lib/dragTypes";
import { ejectDisk } from "../lib/tauri";
import { formatBytes } from "../lib/format";
import type { DiskInfo, Endpoint } from "../types/disk";

/** Filled blue pill for a manually-typed label, a white-outline pill for an
 * Auto Label, plain input otherwise -- mirrors OffShoot's label badges. */
function labelPillClass(endpoint: Endpoint): string {
  if (endpoint.label === "") {
    return "border border-neutral-700 bg-neutral-950 text-neutral-100";
  }
  return endpoint.isAutoLabel
    ? "border-2 border-white bg-transparent text-white"
    : "border border-blue-600 bg-blue-600 text-white";
}

interface EndpointListProps {
  title: string;
  endpoints: Endpoint[];
  disks: DiskInfo[];
  onRemove: (diskId: string) => void;
  onLabelChange: (diskId: string, label: string) => void;
  onPathChange: (diskId: string, path: string) => void;
  onBrowse?: (path: string) => void;
  /** "in use" (Sources) vs "free" (Destinations) -- matches which stat
   * OffShoot surfaces under the label on each side. */
  usageKind?: "used" | "free";
  /** Called with the dragged disk's id when it's dropped here -- typically
   * wired straight to the same store action the "+ Source/Destination"
   * buttons already use, so drag-and-drop is just another way to do the
   * exact same thing. */
  onDropDisk?: (diskId: string) => void;
  /** Drag one row onto another to reorder the list in place -- only wired
   * up for Destinations, where list order doubles as Cascade hop order. */
  onReorder?: (fromDiskId: string, toDiskId: string) => void;
}

export function EndpointList({
  title,
  endpoints,
  disks,
  onRemove,
  onLabelChange,
  onPathChange,
  onBrowse,
  usageKind = "used",
  onDropDisk,
  onReorder,
}: EndpointListProps) {
  const [dragOver, setDragOver] = useState(false);
  const [reorderOverId, setReorderOverId] = useState<string | null>(null);
  const [ejecting, setEjecting] = useState<Record<string, boolean>>({});
  const [ejectError, setEjectError] = useState<Record<string, string>>({});

  async function handleEject(disk: DiskInfo) {
    setEjecting((prev) => ({ ...prev, [disk.id]: true }));
    setEjectError((prev) => ({ ...prev, [disk.id]: "" }));
    try {
      await ejectDisk(disk.mountPoint);
    } catch (err) {
      setEjectError((prev) => ({ ...prev, [disk.id]: String(err) }));
    } finally {
      setEjecting((prev) => ({ ...prev, [disk.id]: false }));
    }
  }

  return (
    <section
      className={`flex flex-col gap-2 rounded ${
        onDropDisk ? `border border-dashed p-2 ${dragOver ? "border-green-500 bg-neutral-900/60" : "border-neutral-700"}` : ""
      }`}
      onDragOver={(e) => {
        if (!onDropDisk) return;
        e.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(e) => {
        if (!onDropDisk) return;
        e.preventDefault();
        setDragOver(false);
        const diskId = e.dataTransfer.getData(DISK_DRAG_MIME);
        if (diskId) onDropDisk(diskId);
      }}
    >
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        {title}
      </h2>
      {endpoints.length === 0 && (
        <p className="text-sm text-neutral-500">
          {onDropDisk ? "Chưa gán -- kéo một ổ đĩa vào đây, hoặc dùng nút + Nguồn/Đích." : "Chưa gán."}
        </p>
      )}
      <ul className="flex flex-col gap-2">
        {endpoints.map((endpoint, index) => {
          const disk = disks.find((d) => d.id === endpoint.diskId);
          return (
            <li
              key={endpoint.diskId}
              draggable={!!onReorder}
              onDragStart={(e) => {
                if (!onReorder) return;
                e.dataTransfer.setData(ENDPOINT_REORDER_MIME, endpoint.diskId);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragOver={(e) => {
                if (!onReorder) return;
                e.preventDefault();
                e.stopPropagation();
                setReorderOverId(endpoint.diskId);
              }}
              onDragLeave={() => setReorderOverId((cur) => (cur === endpoint.diskId ? null : cur))}
              onDrop={(e) => {
                if (!onReorder) return;
                const fromDiskId = e.dataTransfer.getData(ENDPOINT_REORDER_MIME);
                if (!fromDiskId) return;
                e.preventDefault();
                e.stopPropagation();
                setReorderOverId(null);
                onReorder(fromDiskId, endpoint.diskId);
              }}
              title={onReorder ? "Kéo để sắp thứ tự -- thứ tự này là thứ tự chuyển tiếp Nối tiếp (Cascade)" : undefined}
              className={`flex flex-col gap-2 rounded border px-3 py-2 ${
                reorderOverId === endpoint.diskId
                  ? "border-blue-500 bg-blue-500/10"
                  : "border-neutral-800 bg-neutral-900"
              } ${onReorder ? "cursor-grab active:cursor-grabbing" : ""}`}
            >
              <div className="flex items-center gap-2">
                {onReorder && (
                  <span className="shrink-0 text-neutral-600" aria-hidden="true">
                    {index + 1}.
                  </span>
                )}
                <div className="relative shrink-0">
                  <DriveIcon removable={disk?.isRemovable} className="h-5 w-5 text-neutral-500" />
                  {disk?.isRemovable && (
                    <button
                      type="button"
                      disabled={ejecting[disk.id]}
                      title={ejecting[disk.id] ? "Đang tháo…" : "Tháo"}
                      onClick={() => handleEject(disk)}
                      className="absolute -right-1.5 -top-1.5 flex h-3 w-3 items-center justify-center rounded-full bg-green-500 text-[6px] text-neutral-950 disabled:opacity-40"
                    >
                      ▲
                    </button>
                  )}
                </div>
                <div className="flex flex-col">
                  <span className="font-medium">{disk?.name ?? endpoint.diskId}</span>
                  <span className="text-xs text-neutral-500">
                    {disk?.mountPoint ?? "(đã rút)"}
                  </span>
                  {disk && (
                    <span className="text-[10px] text-neutral-600">
                      {usageKind === "used"
                        ? `${formatBytes(disk.totalBytes - disk.availableBytes)} đang dùng`
                        : `${formatBytes(disk.availableBytes)} còn trống`}
                    </span>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-2">
                <input
                  value={endpoint.label}
                  onChange={(e) => onLabelChange(endpoint.diskId, e.currentTarget.value)}
                  placeholder="Nhãn…"
                  title={endpoint.isAutoLabel ? "Nhãn tự động" : "Nhãn"}
                  autoComplete="off"
                  className={`w-24 shrink-0 rounded-full px-2 py-1 text-center text-xs ${labelPillClass(endpoint)}`}
                />
                {onBrowse && (
                  <button
                    type="button"
                    onClick={() => onBrowse(endpoint.path)}
                    className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs"
                  >
                    Duyệt
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => onRemove(endpoint.diskId)}
                  className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs text-red-400"
                >
                  Xóa
                </button>
              </div>
              {ejectError[endpoint.diskId] && (
                <p className="text-[10px] text-red-400">{ejectError[endpoint.diskId]}</p>
              )}
              <div className="flex flex-col gap-1">
                <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                  Đường dẫn thư mục
                </span>
                <input
                  value={endpoint.path}
                  onChange={(e) => onPathChange(endpoint.diskId, e.currentTarget.value)}
                  placeholder="Đường dẫn thư mục đầy đủ…"
                  autoComplete="off"
                  className="min-w-0 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
                />
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
