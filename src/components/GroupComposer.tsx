import { useState } from "react";
import type { DiskInfo, Endpoint } from "../types/disk";
import type { TransferGroupMode } from "../types/transferGroup";

interface GroupComposerProps {
  sources: Endpoint[];
  destinations: Endpoint[];
  disks: DiskInfo[];
  onStart: (source: Endpoint, destinations: Endpoint[], mode: TransferGroupMode) => void;
}

function endpointLabel(endpoint: Endpoint, disks: DiskInfo[]): string {
  const disk = disks.find((d) => d.id === endpoint.diskId);
  return endpoint.label || disk?.name || endpoint.diskId;
}

export function GroupComposer({ sources, destinations, disks, onStart }: GroupComposerProps) {
  const [sourceId, setSourceId] = useState<string | null>(null);
  const [destIds, setDestIds] = useState<string[]>([]); // selection order = cascade hop order
  const [mode, setMode] = useState<TransferGroupMode>("parallel");

  const source = sources.find((s) => s.diskId === sourceId) ?? null;
  const selectedDestinations = destIds
    .map((id) => destinations.find((d) => d.diskId === id))
    .filter((d): d is Endpoint => !!d);

  function toggleDestination(diskId: string) {
    setDestIds((prev) =>
      prev.includes(diskId) ? prev.filter((id) => id !== diskId) : [...prev, diskId],
    );
  }

  function handleStart() {
    if (!source || selectedDestinations.length === 0) return;
    onStart(source, selectedDestinations, mode);
  }

  const canStart = !!source && selectedDestinations.length > 0;

  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Build Transfer
      </h2>

      {sources.length === 0 || destinations.length === 0 ? (
        <p className="text-sm text-neutral-500">
          Assign at least one Source and one Destination to build a transfer.
        </p>
      ) : (
        <>
          <div className="flex flex-col gap-1">
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">Source</span>
            {sources.map((s) => (
              <label key={s.diskId} className="flex items-center gap-2 text-xs">
                <input
                  type="radio"
                  name="group-source"
                  checked={sourceId === s.diskId}
                  onChange={() => setSourceId(s.diskId)}
                />
                {endpointLabel(s, disks)}
              </label>
            ))}
          </div>

          <div className="flex flex-col gap-1">
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">
              Destinations{destIds.length > 1 ? " (order = cascade order)" : ""}
            </span>
            {destinations.map((d) => {
              const order = destIds.indexOf(d.diskId);
              return (
                <label key={d.diskId} className="flex items-center gap-2 text-xs">
                  <input
                    type="checkbox"
                    checked={order !== -1}
                    onChange={() => toggleDestination(d.diskId)}
                  />
                  {endpointLabel(d, disks)}
                  {mode === "cascade" && order !== -1 && (
                    <span className="text-neutral-500">
                      {order === 0 ? "(primary)" : `(#${order + 1})`}
                    </span>
                  )}
                </label>
              );
            })}
          </div>

          <div className="flex flex-col gap-1">
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">Mode</span>
            <label className="flex items-start gap-2 text-xs">
              <input
                type="radio"
                name="group-mode"
                checked={mode === "parallel"}
                onChange={() => setMode("parallel")}
                className="mt-0.5"
              />
              <span>
                <span className="font-medium">Parallel</span>
                <span className="block text-neutral-500">
                  Each destination copies from the source independently
                </span>
              </span>
            </label>
            <label className="flex items-start gap-2 text-xs">
              <input
                type="radio"
                name="group-mode"
                checked={mode === "cascade"}
                onChange={() => setMode("cascade")}
                className="mt-0.5"
              />
              <span>
                <span className="font-medium">Cascade</span>
                <span className="block text-neutral-500">
                  Source copies to the primary destination first, then relays to the rest
                </span>
              </span>
            </label>
          </div>

          <button
            type="button"
            disabled={!canStart}
            onClick={handleStart}
            className="rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
          >
            Start Transfer
          </button>
        </>
      )}
    </section>
  );
}
