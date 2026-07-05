import { useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { useDisksStore } from "../state/disksStore";
import { useTransfersStore } from "../state/transfersStore";
import { useMhlVerifyStore } from "../state/mhlVerifyStore";
import { ejectDisk, renameDisk } from "../lib/tauri";
import { formatBytes } from "../lib/format";
import { DISK_DRAG_MIME } from "../lib/dragTypes";
import { DriveIcon } from "./icons/DriveIcon";
import { DiskContextMenu, type DiskContextMenuItem } from "./DiskContextMenu";
import type { DiskInfo } from "../types/disk";
import type { TransferJob } from "../types/job";

interface DisksPanelProps {
  /** Lets the panel jump to the Transfers view after "Verify" is chosen from
   * a disk's context menu, since that's where the Verify MHL results panel
   * lives -- optional so the component still works standalone/in tests. */
  onVerifyRequested?: () => void;
}

function isDiskBusy(disk: DiskInfo, jobs: TransferJob[]): boolean {
  return jobs.some(
    (job) =>
      (job.status === "queued" || job.status === "copying") &&
      (job.sourcePath.startsWith(disk.mountPoint) ||
        job.destinationPath.startsWith(disk.mountPoint)),
  );
}

export function DisksPanel({ onVerifyRequested }: DisksPanelProps) {
  const disks = useDisksStore((s) => s.disks);
  const sources = useDisksStore((s) => s.sources);
  const destinations = useDisksStore((s) => s.destinations);
  const addSource = useDisksStore((s) => s.addSource);
  const addDestination = useDisksStore((s) => s.addDestination);
  const insertDestinationAfter = useDisksStore((s) => s.insertDestinationAfter);
  const setSourceLabel = useDisksStore((s) => s.setSourceLabel);
  const setSourcePath = useDisksStore((s) => s.setSourcePath);
  const setDestinationPath = useDisksStore((s) => s.setDestinationPath);
  const hiddenDiskIds = useDisksStore((s) => s.hiddenDiskIds);
  const hideDisk = useDisksStore((s) => s.hideDisk);
  const jobs = useTransfersStore((s) => s.jobs);
  const runVerify = useMhlVerifyStore((s) => s.runVerify);

  const [ejectError, setEjectError] = useState<Record<string, string>>({});
  const [ejecting, setEjecting] = useState<Record<string, boolean>>({});
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; diskId: string } | null>(
    null,
  );
  const [editingLabelDiskId, setEditingLabelDiskId] = useState<string | null>(null);
  const [labelDraft, setLabelDraft] = useState("");
  const [renamingDiskId, setRenamingDiskId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const [renameError, setRenameError] = useState<Record<string, string>>({});

  // Renames the actual OS volume (Win32 SetVolumeLabelW / macOS `diskutil
  // rename`) -- distinct from the app-only "Label" above, which never
  // touches the real disk. The disk-watcher poll picks up the new name on
  // its own; no optimistic local update needed.
  function startRenamingVolume(disk: DiskInfo) {
    setRenamingDiskId(disk.id);
    setRenameDraft(disk.name);
    setRenameError((prev) => ({ ...prev, [disk.id]: "" }));
  }

  async function commitRename(disk: DiskInfo) {
    const value = renameDraft.trim();
    setRenamingDiskId(null);
    if (!value || value === disk.name) return;
    try {
      await renameDisk(disk.mountPoint, value);
    } catch (err) {
      setRenameError((prev) => ({ ...prev, [disk.id]: String(err) }));
    }
  }

  // Matches OffShoot: clicking a disk's name (before it's even assigned)
  // lets you type a label and hitting Return both saves it and sets that
  // disk as a Source in one step -- labels only live on Source/Destination
  // endpoints, not on the raw disk, so there's nothing to save until then.
  function startEditingLabel(disk: DiskInfo) {
    const existingSource = sources.find((s) => s.diskId === disk.id);
    setEditingLabelDiskId(disk.id);
    setLabelDraft(existingSource?.label ?? "");
  }

  function commitLabelEdit(diskId: string) {
    const value = labelDraft.trim();
    setEditingLabelDiskId(null);
    if (!value) return;
    if (!sources.some((s) => s.diskId === diskId)) {
      addSource(diskId);
    }
    setSourceLabel(diskId, value);
  }

  const visibleDisks = disks.filter((d) => !hiddenDiskIds.includes(d.id));
  const contextMenuDisk = contextMenu
    ? visibleDisks.find((d) => d.id === contextMenu.diskId)
    : undefined;

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

  async function chooseSourceFolder(disk: DiskInfo) {
    const folder = await openFolderDialog({ directory: true, defaultPath: disk.mountPoint });
    if (!folder || Array.isArray(folder)) return;
    if (!sources.some((s) => s.diskId === disk.id)) addSource(disk.id);
    setSourcePath(disk.id, folder);
  }

  async function chooseDestinationFolder(disk: DiskInfo) {
    const folder = await openFolderDialog({ directory: true, defaultPath: disk.mountPoint });
    if (!folder || Array.isArray(folder)) return;
    if (!destinations.some((d) => d.diskId === disk.id)) addDestination(disk.id);
    setDestinationPath(disk.id, folder);
  }

  async function verifyOtherFolder() {
    const folder = await openFolderDialog({ directory: true });
    if (!folder || Array.isArray(folder)) return;
    runVerify(folder, "folder");
    onVerifyRequested?.();
  }

  function buildMenuItems(disk: DiskInfo): DiskContextMenuItem[] {
    const isSource = sources.some((s) => s.diskId === disk.id);
    const isDestination = destinations.some((d) => d.diskId === disk.id);
    const assigned = isSource || isDestination;
    const busy = isDiskBusy(disk, Object.values(jobs));
    const diskLabel = sources.find((s) => s.diskId === disk.id)?.label || disk.name;
    // Every *other* current Destination -- picking one wires this disk in
    // right after it in the chain (Cascade mode reads the Destinations
    // list's order as its hop order), without first having to "+
    // Destination" it and then drag it into place by hand.
    const cascadeFromCandidates = destinations.filter((d) => d.diskId !== disk.id);

    const items: DiskContextMenuItem[] = [
      { label: "Thêm nhãn…", onSelect: () => startEditingLabel(disk) },
      {
        label: "Thư mục Nguồn",
        children: [{ label: "Chọn thư mục…", onSelect: () => chooseSourceFolder(disk) }],
      },
      {
        label: "Thư mục Đích",
        children: [{ label: "Chọn thư mục…", onSelect: () => chooseDestinationFolder(disk) }],
      },
      { label: "Đặt làm Nguồn", onSelect: () => addSource(disk.id), disabled: isSource },
      {
        label: "Đặt làm Đích",
        onSelect: () => addDestination(disk.id),
        disabled: isDestination,
      },
      {
        label: "Nối tiếp từ",
        disabled: cascadeFromCandidates.length === 0,
        children: cascadeFromCandidates.map((d) => ({
          label: d.label || disks.find((disk2) => disk2.id === d.diskId)?.name || d.diskId,
          onSelect: () => insertDestinationAfter(disk.id, d.diskId),
        })),
      },
      {
        label: "Xác minh",
        children: [
          {
            label: `Xác minh ${diskLabel}…`,
            onSelect: () => {
              runVerify(disk.mountPoint, "folder");
              onVerifyRequested?.();
            },
          },
          { label: "Xác minh thư mục…", onSelect: () => verifyOtherFolder() },
        ],
      },
    ];
    if (disk.isRemovable) {
      items.push({
        label: ejecting[disk.id] ? "Đang tháo…" : "Tháo",
        onSelect: () => handleEject(disk),
        disabled: busy || ejecting[disk.id],
      });
    }
    items.push({
      label: `Đổi tên ${disk.name}`,
      onSelect: () => startRenamingVolume(disk),
    });
    items.push({
      label: "Ẩn",
      onSelect: () => hideDisk(disk.id),
      disabled: assigned,
    });
    items.push({
      label: "Mở trong Explorer",
      onSelect: () => revealItemInDir(disk.mountPoint).catch(console.error),
    });
    return items;
  }

  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Ổ đĩa
      </h2>
      {disks.length === 0 && (
        <p className="text-sm text-neutral-500">Không phát hiện ổ đĩa nào.</p>
      )}
      {disks.length > 0 && visibleDisks.length === 0 && (
        <p className="text-sm text-neutral-500">
          Mọi ổ đĩa phát hiện được đều đang bị ẩn -- bỏ ẩn ở Cài đặt → Ổ đĩa.
        </p>
      )}
      <ul className="flex flex-col gap-2">
        {visibleDisks.map((disk) => {
          const isSource = sources.some((s) => s.diskId === disk.id);
          const isDestination = destinations.some((d) => d.diskId === disk.id);
          const assigned = isSource || isDestination;
          const busy = isDiskBusy(disk, Object.values(jobs));
          return (
            <li
              key={disk.id}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData(DISK_DRAG_MIME, disk.id);
                e.dataTransfer.effectAllowed = "copy";
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                setContextMenu({ x: e.clientX, y: e.clientY, diskId: disk.id });
              }}
              title="Kéo vào Nguồn/Đích, hoặc chuột phải để xem thêm thao tác"
              className="flex cursor-grab flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2 active:cursor-grabbing"
            >
              <div className="flex items-center gap-2">
                <DriveIcon removable={disk.isRemovable} className="h-5 w-5 shrink-0 text-neutral-500" />
                <div className="flex flex-col">
                  {editingLabelDiskId === disk.id ? (
                    <input
                      autoFocus
                      value={labelDraft}
                      onChange={(e) => setLabelDraft(e.currentTarget.value)}
                      onClick={(e) => e.stopPropagation()}
                      onDragStart={(e) => e.stopPropagation()}
                      onBlur={() => commitLabelEdit(disk.id)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") commitLabelEdit(disk.id);
                        if (e.key === "Escape") setEditingLabelDiskId(null);
                      }}
                      placeholder="Nhãn…"
                      autoComplete="off"
                      className="w-32 rounded border border-blue-600 bg-neutral-950 px-1 py-0.5 text-sm font-medium"
                    />
                  ) : (
                    <button
                      type="button"
                      title="Bấm để thêm/sửa nhãn -- đặt ổ đĩa này làm Nguồn"
                      onClick={(e) => {
                        e.stopPropagation();
                        startEditingLabel(disk);
                      }}
                      className="w-fit text-left font-medium hover:underline"
                    >
                      {sources.find((s) => s.diskId === disk.id)?.label || disk.name}
                    </button>
                  )}
                  <span className="text-xs text-neutral-500">
                    {disk.mountPoint} · còn trống {formatBytes(disk.availableBytes)} /{" "}
                    {formatBytes(disk.totalBytes)}
                    {disk.isRemovable ? " · di động" : ""}
                  </span>
                  {renamingDiskId === disk.id && (
                    <input
                      autoFocus
                      value={renameDraft}
                      onChange={(e) => setRenameDraft(e.currentTarget.value)}
                      onClick={(e) => e.stopPropagation()}
                      onDragStart={(e) => e.stopPropagation()}
                      onBlur={() => commitRename(disk)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") commitRename(disk);
                        if (e.key === "Escape") setRenamingDiskId(null);
                      }}
                      title="Đổi tên volume thật của ổ đĩa (khác với Nhãn chỉ dùng trong app)"
                      autoComplete="off"
                      className="mt-1 w-32 rounded border border-blue-600 bg-neutral-950 px-1 py-0.5 text-xs"
                    />
                  )}
                  {renameError[disk.id] && (
                    <p className="text-[10px] text-red-400">{renameError[disk.id]}</p>
                  )}
                </div>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  disabled={isSource}
                  onClick={() => addSource(disk.id)}
                  className="rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
                >
                  + Nguồn
                </button>
                <button
                  type="button"
                  disabled={isDestination}
                  onClick={() => addDestination(disk.id)}
                  className="rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
                >
                  + Đích
                </button>
                {disk.isRemovable && (
                  <button
                    type="button"
                    disabled={busy || ejecting[disk.id]}
                    title={busy ? "Đợi các lượt truyền đang chạy trên ổ đĩa này xong đã" : undefined}
                    onClick={() => handleEject(disk)}
                    className="rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
                  >
                    {ejecting[disk.id] ? "Đang tháo…" : "Tháo"}
                  </button>
                )}
                <button
                  type="button"
                  disabled={assigned}
                  title={
                    assigned
                      ? "Gỡ khỏi Nguồn/Đích trước đã"
                      : "Ẩn ổ đĩa này khỏi danh sách"
                  }
                  onClick={() => hideDisk(disk.id)}
                  className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400 disabled:opacity-40"
                >
                  Ẩn
                </button>
              </div>
              {ejectError[disk.id] && (
                <p className="text-[10px] text-red-400">{ejectError[disk.id]}</p>
              )}
            </li>
          );
        })}
      </ul>

      {contextMenu && contextMenuDisk && (
        <DiskContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={buildMenuItems(contextMenuDisk)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </section>
  );
}
