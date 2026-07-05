import { useState } from "react";
import { open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { DriveIcon } from "./icons/DriveIcon";
import { Panel } from "./ui/Panel";
import { SectionHeading } from "./ui/SectionHeading";
import { EmptyState } from "./ui/EmptyState";
import { IconButton } from "./ui/IconButton";
import { DiskContextMenu, type DiskContextMenuItem } from "./DiskContextMenu";
import { ArrowUpFromLine, ExternalLink, FolderInput, Inbox, Menu as MenuIcon, Plus, Tag, Trash2 } from "./icons";
import { DISK_DRAG_MIME, ENDPOINT_REORDER_MIME } from "../lib/dragTypes";
import { ejectDisk } from "../lib/tauri";
import { formatBytes } from "../lib/format";
import { cn } from "../lib/cn";
import type { DiskInfo, Endpoint } from "../types/disk";

/** Filled blue pill for a manually-typed label, a white-outline pill for an
 * Auto Label -- mirrors OffShoot's label badges. */
function labelPillClass(endpoint: Endpoint): string {
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
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; diskId: string } | null>(
    null,
  );
  const [editingLabelDiskId, setEditingLabelDiskId] = useState<string | null>(null);
  const [labelDraft, setLabelDraft] = useState("");

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

  function startEditingLabel(endpoint: Endpoint) {
    setEditingLabelDiskId(endpoint.diskId);
    setLabelDraft(endpoint.label);
  }

  function commitLabelEdit(diskId: string) {
    setEditingLabelDiskId(null);
    onLabelChange(diskId, labelDraft.trim());
  }

  async function chooseFolderPath(diskId: string, fallbackMountPoint?: string) {
    const folder = await openFolderDialog({ directory: true, defaultPath: fallbackMountPoint });
    if (!folder || Array.isArray(folder)) return;
    onPathChange(diskId, folder);
  }

  /** Real OffShoot's "+" in an empty Sources/Destinations zone opens a
   * native folder picker directly (not tied to a disk card the user already
   * see) -- matched here by finding which recognized disk the chosen folder
   * lives under, then reusing the exact same `onDropDisk`+`onPathChange`
   * calls a drag-and-drop already triggers. */
  async function handleAddViaPicker() {
    if (!onDropDisk) return;
    const folder = await openFolderDialog({ directory: true });
    if (!folder || Array.isArray(folder)) return;
    const normalized = folder.replace(/\\/g, "/").toLowerCase();
    const disk = disks.find((d) =>
      normalized.startsWith(d.mountPoint.replace(/\\/g, "/").toLowerCase()),
    );
    if (!disk) return;
    onDropDisk(disk.id);
    onPathChange(disk.id, folder);
  }

  function buildMenuItems(endpoint: Endpoint, disk: DiskInfo | undefined): DiskContextMenuItem[] {
    const iconClass = "h-3.5 w-3.5";
    const items: DiskContextMenuItem[] = [
      {
        label: "Sửa nhãn…",
        icon: <Tag className={iconClass} />,
        onSelect: () => startEditingLabel(endpoint),
      },
      {
        label: "Đường dẫn thư mục…",
        icon: <FolderInput className={iconClass} />,
        onSelect: () => chooseFolderPath(endpoint.diskId, disk?.mountPoint),
      },
    ];
    if (onBrowse) {
      items.push({
        label: "Xem clip",
        icon: <ExternalLink className={iconClass} />,
        onSelect: () => onBrowse(endpoint.path),
      });
    }
    items.push({
      label: "Xóa",
      icon: <Trash2 className={iconClass} />,
      danger: true,
      onSelect: () => onRemove(endpoint.diskId),
    });
    return items;
  }

  const contextMenuEndpoint = contextMenu
    ? endpoints.find((e) => e.diskId === contextMenu.diskId)
    : undefined;

  return (
    <Panel
      as="section"
      className={cn(
        "flex flex-col gap-2 p-3",
        onDropDisk && "!border-dashed",
        dragOver && "!border-green-500 !bg-neutral-800/60",
      )}
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
      <SectionHeading>{title}</SectionHeading>
      {endpoints.length === 0 && (
        <EmptyState icon={<Inbox className="h-5 w-5" />}>
          {onDropDisk ? "Chưa gán -- kéo một ổ đĩa vào đây." : "Chưa gán."}
          {onDropDisk && (
            <span className="mt-2 flex justify-center">
              <IconButton
                aria-label={`Thêm ${title}`}
                title={`Thêm ${title}`}
                icon={<Plus className="h-4 w-4" />}
                onClick={handleAddViaPicker}
              />
            </span>
          )}
        </EmptyState>
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
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, diskId: endpoint.diskId });
              }}
              title="Kéo để sắp thứ tự (Nối tiếp), hoặc chuột phải để xem thêm thao tác"
              className={`group relative flex items-center gap-2 rounded border px-3 py-2 ${
                reorderOverId === endpoint.diskId
                  ? "border-blue-500 bg-blue-500/10"
                  : "border-neutral-800 bg-neutral-900"
              } ${onReorder ? "cursor-grab active:cursor-grabbing" : ""}`}
            >
              {onReorder && (
                <span className="shrink-0 text-neutral-600" aria-hidden="true">
                  {index + 1}.
                </span>
              )}
              <div className="relative shrink-0">
                <DriveIcon removable={disk?.isRemovable} className="h-6 w-6 text-neutral-400" />
                {disk?.isRemovable && (
                  <button
                    type="button"
                    disabled={ejecting[disk.id]}
                    title={ejecting[disk.id] ? "Đang tháo…" : "Tháo"}
                    onClick={() => handleEject(disk)}
                    className="absolute -right-1.5 -top-1.5 flex h-3.5 w-3.5 items-center justify-center rounded-full bg-green-500 text-neutral-950 disabled:opacity-40"
                  >
                    <ArrowUpFromLine className="h-2 w-2" strokeWidth={3} />
                  </button>
                )}
              </div>
              <div className="flex min-w-0 flex-1 flex-col">
                {editingLabelDiskId === endpoint.diskId ? (
                  <input
                    autoFocus
                    value={labelDraft}
                    onChange={(e) => setLabelDraft(e.currentTarget.value)}
                    onBlur={() => commitLabelEdit(endpoint.diskId)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") commitLabelEdit(endpoint.diskId);
                      if (e.key === "Escape") setEditingLabelDiskId(null);
                    }}
                    placeholder="Nhãn…"
                    autoComplete="off"
                    className="w-28 rounded border border-blue-600 bg-neutral-950 px-1 py-0.5 text-sm font-medium"
                  />
                ) : (
                  <button
                    type="button"
                    title="Bấm để sửa nhãn"
                    onClick={() => startEditingLabel(endpoint)}
                    className={`w-fit truncate text-left font-medium hover:underline ${
                      endpoint.label ? "" : "text-neutral-400"
                    }`}
                  >
                    {endpoint.label ? (
                      <span
                        className={`rounded-full px-2 py-0.5 text-xs ${labelPillClass(endpoint)}`}
                      >
                        {endpoint.label}
                      </span>
                    ) : (
                      (disk?.name ?? endpoint.diskId)
                    )}
                  </button>
                )}
                <span className="truncate text-xs text-neutral-500">
                  {disk?.mountPoint ?? "(đã rút)"}
                  {disk &&
                    ` · ${
                      usageKind === "used"
                        ? `${formatBytes(disk.totalBytes - disk.availableBytes)} đang dùng`
                        : `${formatBytes(disk.availableBytes)} còn trống`
                    }`}
                </span>
                {ejectError[endpoint.diskId] && (
                  <p className="text-[10px] text-red-400">{ejectError[endpoint.diskId]}</p>
                )}
              </div>
              <IconButton
                aria-label={`Thêm thao tác cho ${endpoint.label || disk?.name || endpoint.diskId}`}
                title="Thêm thao tác"
                icon={<MenuIcon className="h-3.5 w-3.5" />}
                className="opacity-0 group-hover:opacity-100"
                onClick={(e) => {
                  const rect = e.currentTarget.getBoundingClientRect();
                  setContextMenu({ x: rect.right - 200, y: rect.bottom + 4, diskId: endpoint.diskId });
                }}
              />
            </li>
          );
        })}
      </ul>

      {contextMenu && contextMenuEndpoint && (
        <DiskContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={buildMenuItems(
            contextMenuEndpoint,
            disks.find((d) => d.id === contextMenu.diskId),
          )}
          onClose={() => setContextMenu(null)}
        />
      )}
    </Panel>
  );
}
