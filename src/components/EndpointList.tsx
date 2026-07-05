import { useState } from "react";
import { DriveIcon } from "./icons/DriveIcon";
import { DISK_DRAG_MIME } from "../lib/dragTypes";
import type { DiskInfo, Endpoint } from "../types/disk";

/** Blue for a manually-typed label, a white outline for an Auto Label -- mirrors OffShoot. */
function labelBorderClass(endpoint: Endpoint): string {
  if (endpoint.label === "") return "border-neutral-700";
  return endpoint.isAutoLabel ? "border-white" : "border-blue-500";
}

interface EndpointListProps {
  title: string;
  endpoints: Endpoint[];
  disks: DiskInfo[];
  onRemove: (diskId: string) => void;
  onLabelChange: (diskId: string, label: string) => void;
  onPathChange: (diskId: string, path: string) => void;
  onBrowse?: (path: string) => void;
  /** Called with the dragged disk's id when it's dropped here -- typically
   * wired straight to the same store action the "+ Source/Destination"
   * buttons already use, so drag-and-drop is just another way to do the
   * exact same thing. */
  onDropDisk?: (diskId: string) => void;
}

export function EndpointList({
  title,
  endpoints,
  disks,
  onRemove,
  onLabelChange,
  onPathChange,
  onBrowse,
  onDropDisk,
}: EndpointListProps) {
  const [dragOver, setDragOver] = useState(false);

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
          {onDropDisk ? "None assigned yet -- drag a disk here, or use + Source/Destination." : "None assigned yet."}
        </p>
      )}
      <ul className="flex flex-col gap-2">
        {endpoints.map((endpoint) => {
          const disk = disks.find((d) => d.id === endpoint.diskId);
          return (
            <li
              key={endpoint.diskId}
              className="flex flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2"
            >
              <div className="flex items-center gap-2">
                <DriveIcon removable={disk?.isRemovable} className="h-5 w-5 shrink-0 text-neutral-500" />
                <div className="flex flex-col">
                  <span className="font-medium">{disk?.name ?? endpoint.diskId}</span>
                  <span className="text-xs text-neutral-500">
                    {disk?.mountPoint ?? "(unplugged)"}
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <input
                  value={endpoint.label}
                  onChange={(e) => onLabelChange(endpoint.diskId, e.currentTarget.value)}
                  placeholder="Label…"
                  title={endpoint.isAutoLabel ? "Auto-generated label" : "Label"}
                  autoComplete="off"
                  className={`w-24 shrink-0 rounded border bg-neutral-950 px-2 py-1 text-xs ${labelBorderClass(endpoint)}`}
                />
                {onBrowse && (
                  <button
                    type="button"
                    onClick={() => onBrowse(endpoint.path)}
                    className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs"
                  >
                    Browse
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => onRemove(endpoint.diskId)}
                  className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs text-red-400"
                >
                  Remove
                </button>
              </div>
              <div className="flex flex-col gap-1">
                <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                  Folder path
                </span>
                <input
                  value={endpoint.path}
                  onChange={(e) => onPathChange(endpoint.diskId, e.currentTarget.value)}
                  placeholder="Full folder path…"
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
