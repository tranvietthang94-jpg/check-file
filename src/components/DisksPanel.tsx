import { useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useDisksStore } from "../state/disksStore";
import { useTransfersStore } from "../state/transfersStore";
import { ejectDisk } from "../lib/tauri";
import { formatBytes } from "../lib/format";
import { DISK_DRAG_MIME } from "../lib/dragTypes";
import { DriveIcon } from "./icons/DriveIcon";
import { DiskContextMenu, type DiskContextMenuItem } from "./DiskContextMenu";
import type { DiskInfo } from "../types/disk";
import type { TransferJob } from "../types/job";

function isDiskBusy(disk: DiskInfo, jobs: TransferJob[]): boolean {
  return jobs.some(
    (job) =>
      (job.status === "queued" || job.status === "copying") &&
      (job.sourcePath.startsWith(disk.mountPoint) ||
        job.destinationPath.startsWith(disk.mountPoint)),
  );
}

export function DisksPanel() {
  const disks = useDisksStore((s) => s.disks);
  const sources = useDisksStore((s) => s.sources);
  const destinations = useDisksStore((s) => s.destinations);
  const addSource = useDisksStore((s) => s.addSource);
  const addDestination = useDisksStore((s) => s.addDestination);
  const hiddenDiskIds = useDisksStore((s) => s.hiddenDiskIds);
  const hideDisk = useDisksStore((s) => s.hideDisk);
  const jobs = useTransfersStore((s) => s.jobs);

  const [ejectError, setEjectError] = useState<Record<string, string>>({});
  const [ejecting, setEjecting] = useState<Record<string, boolean>>({});
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; diskId: string } | null>(
    null,
  );

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

  function buildMenuItems(disk: DiskInfo): DiskContextMenuItem[] {
    const isSource = sources.some((s) => s.diskId === disk.id);
    const isDestination = destinations.some((d) => d.diskId === disk.id);
    const assigned = isSource || isDestination;
    const busy = isDiskBusy(disk, Object.values(jobs));

    const items: DiskContextMenuItem[] = [
      { label: "Set as Source", onSelect: () => addSource(disk.id), disabled: isSource },
      {
        label: "Set as Destination",
        onSelect: () => addDestination(disk.id),
        disabled: isDestination,
      },
    ];
    if (disk.isRemovable) {
      items.push({
        label: ejecting[disk.id] ? "Ejecting…" : "Eject",
        onSelect: () => handleEject(disk),
        disabled: busy || ejecting[disk.id],
      });
    }
    items.push({
      label: "Open in Explorer",
      onSelect: () => revealItemInDir(disk.mountPoint).catch(console.error),
    });
    items.push({
      label: "Hide",
      onSelect: () => hideDisk(disk.id),
      disabled: assigned,
    });
    return items;
  }

  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Disks
      </h2>
      {disks.length === 0 && (
        <p className="text-sm text-neutral-500">No volumes detected.</p>
      )}
      {disks.length > 0 && visibleDisks.length === 0 && (
        <p className="text-sm text-neutral-500">
          Every detected drive is hidden -- unhide one in Preferences → Disks.
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
              title="Drag onto Sources/Destinations, or right-click for more actions"
              className="flex cursor-grab flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2 active:cursor-grabbing"
            >
              <div className="flex items-center gap-2">
                <DriveIcon removable={disk.isRemovable} className="h-5 w-5 shrink-0 text-neutral-500" />
                <div className="flex flex-col">
                  <span className="font-medium">{disk.name}</span>
                  <span className="text-xs text-neutral-500">
                    {disk.mountPoint} · {formatBytes(disk.availableBytes)} free of{" "}
                    {formatBytes(disk.totalBytes)}
                    {disk.isRemovable ? " · removable" : ""}
                  </span>
                </div>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  disabled={isSource}
                  onClick={() => addSource(disk.id)}
                  className="rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
                >
                  + Source
                </button>
                <button
                  type="button"
                  disabled={isDestination}
                  onClick={() => addDestination(disk.id)}
                  className="rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
                >
                  + Destination
                </button>
                {disk.isRemovable && (
                  <button
                    type="button"
                    disabled={busy || ejecting[disk.id]}
                    title={busy ? "Wait for active transfers on this disk to finish" : undefined}
                    onClick={() => handleEject(disk)}
                    className="rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
                  >
                    {ejecting[disk.id] ? "Ejecting…" : "Eject"}
                  </button>
                )}
                <button
                  type="button"
                  disabled={assigned}
                  title={
                    assigned
                      ? "Remove it as a Source/Destination first"
                      : "Hide this drive from the list"
                  }
                  onClick={() => hideDisk(disk.id)}
                  className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400 disabled:opacity-40"
                >
                  Hide
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
