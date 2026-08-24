import { useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { useDisksStore } from "../state/disksStore";
import { useTransfersStore } from "../state/transfersStore";
import { useMhlVerifyStore } from "../state/mhlVerifyStore";
import { useRecentsStore } from "../state/recentsStore";
import { ejectDisk, renameDisk } from "../lib/tauri";
import { formatBytes } from "../lib/format";
import { DISK_DRAG_MIME, ENDPOINT_REMOVE_MIME } from "../lib/dragTypes";
import { DriveIcon } from "./icons/DriveIcon";
import { DiskContextMenu, type DiskContextMenuItem } from "./DiskContextMenu";
import { Panel } from "./ui/Panel";
import { SectionHeading } from "./ui/SectionHeading";
import { EmptyState } from "./ui/EmptyState";
import { IconButton } from "./ui/IconButton";
import {
  ArrowRightLeft,
  ArrowUpFromLine,
  EyeOff,
  ExternalLink,
  FolderInput,
  FolderOutput,
  HardDrive,
  Menu,
  Pencil,
  Plus,
  ShieldCheck,
  Tag,
} from "./icons";
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
  const removeSource = useDisksStore((s) => s.removeSource);
  const removeDestination = useDisksStore((s) => s.removeDestination);
  const insertDestinationAfter = useDisksStore((s) => s.insertDestinationAfter);
  const setSourceLabel = useDisksStore((s) => s.setSourceLabel);
  const setSourcePath = useDisksStore((s) => s.setSourcePath);
  const setDestinationPath = useDisksStore((s) => s.setDestinationPath);
  const hiddenDiskIds = useDisksStore((s) => s.hiddenDiskIds);
  const hideDisk = useDisksStore((s) => s.hideDisk);
  const jobs = useTransfersStore((s) => s.jobs);
  const runVerify = useMhlVerifyStore((s) => s.runVerify);
  const recentSources = useRecentsStore((s) => s.recentSources);
  const recentDestinations = useRecentsStore((s) => s.recentDestinations);
  const addRecentSource = useRecentsStore((s) => s.addRecentSource);
  const addRecentDestination = useRecentsStore((s) => s.addRecentDestination);
  const clearRecentSources = useRecentsStore((s) => s.clearRecentSources);
  const clearRecentDestinations = useRecentsStore((s) => s.clearRecentDestinations);

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
  const contextMenuReturnFocusRef = useRef<HTMLElement | null>(null);
  const labelReturnFocusRef = useRef<Record<string, HTMLButtonElement | null>>({});

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
    addRecentSource(folder);
  }

  async function chooseDestinationFolder(disk: DiskInfo) {
    const folder = await openFolderDialog({ directory: true, defaultPath: disk.mountPoint });
    if (!folder || Array.isArray(folder)) return;
    if (!destinations.some((d) => d.diskId === disk.id)) addDestination(disk.id);
    setDestinationPath(disk.id, folder);
    addRecentDestination(folder);
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

    const iconClass = "h-3.5 w-3.5";
    const items: DiskContextMenuItem[] = [
      {
        label: "Thêm nhãn…",
        icon: <Tag className={iconClass} />,
        onSelect: () => startEditingLabel(disk),
      },
      {
        label: "Thư mục Nguồn",
        icon: <FolderInput className={iconClass} />,
        children: [
          { label: "Chọn thư mục…", onSelect: () => chooseSourceFolder(disk) },
          ...recentSources.map((path) => ({
            label: path,
            onSelect: () => {
              if (!isSource) addSource(disk.id);
              setSourcePath(disk.id, path);
              addRecentSource(path);
            },
          })),
          ...(recentSources.length > 0
            ? [{ label: "Xóa thư mục gần đây", onSelect: clearRecentSources }]
            : []),
        ],
      },
      {
        label: "Thư mục Đích",
        icon: <FolderOutput className={iconClass} />,
        children: [
          { label: "Chọn thư mục…", onSelect: () => chooseDestinationFolder(disk) },
          ...recentDestinations.map((path) => ({
            label: path,
            onSelect: () => {
              if (!isDestination) addDestination(disk.id);
              setDestinationPath(disk.id, path);
              addRecentDestination(path);
            },
          })),
          ...(recentDestinations.length > 0
            ? [{ label: "Xóa thư mục gần đây", onSelect: clearRecentDestinations }]
            : []),
        ],
      },
      {
        label: "Đặt làm Nguồn",
        icon: <Plus className={iconClass} />,
        onSelect: () => addSource(disk.id),
        disabled: isSource,
      },
      {
        label: "Đặt làm Đích",
        icon: <Plus className={iconClass} />,
        onSelect: () => addDestination(disk.id),
        disabled: isDestination,
      },
      {
        label: "Nối tiếp từ",
        icon: <ArrowRightLeft className={iconClass} />,
        disabled: cascadeFromCandidates.length === 0,
        children: cascadeFromCandidates.map((d) => ({
          label: d.label || disks.find((disk2) => disk2.id === d.diskId)?.name || d.diskId,
          onSelect: () => insertDestinationAfter(disk.id, d.diskId),
        })),
      },
      {
        label: "Xác minh",
        icon: <ShieldCheck className={iconClass} />,
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
        icon: <ArrowUpFromLine className={iconClass} />,
        onSelect: () => handleEject(disk),
        disabled: busy || ejecting[disk.id],
      });
    }
    items.push({
      label: `Đổi tên ${disk.name}`,
      icon: <Pencil className={iconClass} />,
      onSelect: () => startRenamingVolume(disk),
    });
    items.push({
      label: "Ẩn",
      icon: <EyeOff className={iconClass} />,
      onSelect: () => hideDisk(disk.id),
      disabled: assigned,
    });
    items.push({
      label: "Mở trong Explorer",
      icon: <ExternalLink className={iconClass} />,
      onSelect: () => revealItemInDir(disk.mountPoint).catch(console.error),
    });
    return items;
  }

  // OffShoot's real Disks grid only shows disks not already assigned as a
  // Source or Destination -- once assigned, a disk lives in that column
  // instead and disappears from here.
  const availableDisks = visibleDisks.filter(
    (d) => !sources.some((s) => s.diskId === d.id) && !destinations.some((dest) => dest.diskId === d.id),
  );

  return (
    <Panel as="section" className="flex flex-col gap-2 p-3">
      <SectionHeading>Ổ đĩa</SectionHeading>
      {disks.length === 0 && (
        <EmptyState icon={<HardDrive className="h-5 w-5" />}>Không phát hiện ổ đĩa nào.</EmptyState>
      )}
      {disks.length > 0 && visibleDisks.length === 0 && (
        <EmptyState icon={<EyeOff className="h-5 w-5" />}>
          Mọi ổ đĩa phát hiện được đều đang bị ẩn -- bỏ ẩn ở Cài đặt → Ổ đĩa.
        </EmptyState>
      )}
      {visibleDisks.length > 0 && availableDisks.length === 0 && (
        <EmptyState icon={<HardDrive className="h-5 w-5" />}>
          Mọi ổ đĩa đã được gán làm Nguồn hoặc Đích.
        </EmptyState>
      )}
      <ul
        data-testid="available-disk-grid"
        onDragOver={(e) => {
          if (!e.dataTransfer.types.includes(ENDPOINT_REMOVE_MIME)) return;
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
        }}
        onDrop={(e) => {
          const diskId = e.dataTransfer.getData(ENDPOINT_REMOVE_MIME);
          if (!diskId) return;
          e.preventDefault();
          if (sources.some((s) => s.diskId === diskId)) removeSource(diskId);
          if (destinations.some((d) => d.diskId === diskId)) removeDestination(diskId);
        }}
        className="grid grid-cols-3 content-start gap-x-4 gap-y-5 overflow-y-auto px-2 py-3"
      >
        {availableDisks.map((disk) => {
          return (
            <Panel
              as="li"
              key={disk.id}
              data-testid="available-disk-card"
              tabIndex={0}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData(DISK_DRAG_MIME, disk.id);
                e.dataTransfer.effectAllowed = "copy";
              }}
              onContextMenu={(e) => {
                e.preventDefault();
                contextMenuReturnFocusRef.current = e.currentTarget;
                setContextMenu({ x: e.clientX, y: e.clientY, diskId: disk.id });
              }}
              title="Kéo vào Nguồn/Đích, hoặc chuột phải để xem thêm thao tác"
              className="group relative flex min-h-40 min-w-0 cursor-grab flex-col items-center justify-center gap-2 rounded-md border-transparent bg-transparent px-3 py-4 text-center shadow-none hover:border-neutral-700 hover:bg-neutral-800/40 active:cursor-grabbing"
            >
              <div className="flex w-full flex-col items-center gap-1">
                <IconButton
                  aria-label={`Thêm thao tác cho ${disk.name}`}
                  title="Thêm thao tác"
                  icon={<Menu className="h-3.5 w-3.5" />}
                  className="absolute right-1 top-1 opacity-0 group-hover:opacity-100"
                  onClick={(e) => {
                    e.stopPropagation();
                    const rect = e.currentTarget.getBoundingClientRect();
                    setContextMenu({ x: rect.right - 200, y: rect.bottom + 4, diskId: disk.id });
                  }}
                />
                <DriveIcon removable={disk.isRemovable} className="h-12 w-12 text-neutral-400" />
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
                      if (e.key === "Escape") {
                        setEditingLabelDiskId(null);
                        requestAnimationFrame(() => labelReturnFocusRef.current[disk.id]?.focus());
                      }
                    }}
                    placeholder="Nhãn…"
                    autoComplete="off"
                    className="w-full rounded border border-blue-600 bg-neutral-950 px-1 py-0.5 text-center text-xs font-medium"
                  />
                ) : (
                  <button
                    type="button"
                    ref={(node) => {
                      labelReturnFocusRef.current[disk.id] = node;
                    }}
                    title="Bấm để thêm/sửa nhãn -- đặt ổ đĩa này làm Nguồn"
                    onClick={(e) => {
                      e.stopPropagation();
                      startEditingLabel(disk);
                    }}
                    className="w-full truncate text-xs font-medium hover:underline"
                  >
                    {sources.find((s) => s.diskId === disk.id)?.label || disk.name}
                  </button>
                )}
                <span className="truncate text-[10px] text-neutral-500">
                  {formatBytes(disk.availableBytes)} / {formatBytes(disk.totalBytes)}
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
                    className="w-full rounded border border-blue-600 bg-neutral-950 px-1 py-0.5 text-center text-[10px]"
                  />
                )}
                {renameError[disk.id] && (
                  <p className="text-[10px] text-red-400">{renameError[disk.id]}</p>
                )}
                {ejectError[disk.id] && (
                  <p className="text-[10px] text-red-400">{ejectError[disk.id]}</p>
                )}
              </div>
            </Panel>
          );
        })}
      </ul>

      {contextMenu && contextMenuDisk && (
        <DiskContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={buildMenuItems(contextMenuDisk)}
          onClose={() => {
            setContextMenu(null);
            requestAnimationFrame(() => contextMenuReturnFocusRef.current?.focus());
          }}
        />
      )}
    </Panel>
  );
}
