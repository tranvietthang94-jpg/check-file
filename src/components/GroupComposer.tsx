import { useState } from "react";
import { useSettingsStore } from "../state/settingsStore";
import type { DiskInfo, Endpoint } from "../types/disk";
import type { TransferGroupMode } from "../types/transferGroup";

const CASCADE_REORDER_MIME = "application/x-offloadkit-cascade-reorder";

interface GroupComposerProps {
  sources: Endpoint[];
  destinations: Endpoint[];
  disks: DiskInfo[];
  onStart: (
    source: Endpoint,
    destinations: Endpoint[],
    mode: TransferGroupMode,
    moveAfterTransfer: boolean,
  ) => void;
}

function endpointLabel(endpoint: Endpoint, disks: DiskInfo[]): string {
  const disk = disks.find((d) => d.id === endpoint.diskId);
  return endpoint.label || disk?.name || endpoint.diskId;
}

export function GroupComposer({ sources, destinations, disks, onStart }: GroupComposerProps) {
  const [sourceId, setSourceId] = useState<string | null>(null);
  const [destIds, setDestIds] = useState<string[]>([]); // selection order = cascade hop order
  const [mode, setMode] = useState<TransferGroupMode>("parallel");
  const [moveAfterTransfer, setMoveAfterTransfer] = useState(false);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);
  const verificationMode = useSettingsStore((s) => s.verificationMode);

  /** Drag one cascade destination onto another to relocate it in the relay
   * order -- replaces reordering-by-deselect-and-reselect with OffShoot's
   * drag-a-destination-card interaction. */
  function moveDestination(from: number, to: number) {
    setDestIds((prev) => {
      if (from === to || from < 0 || from >= prev.length) return prev;
      const next = [...prev];
      const [moved] = next.splice(from, 1);
      next.splice(to, 0, moved);
      return next;
    });
  }

  const source = sources.find((s) => s.diskId === sourceId) ?? null;
  const selectedDestinations = destIds
    .map((id) => destinations.find((d) => d.diskId === id))
    .filter((d): d is Endpoint => !!d);
  // Move only ever makes unambiguous sense with exactly one destination --
  // with more, every destination reads the same source independently, so
  // there's no single point where deleting it wouldn't race another read.
  // It also requires an actual hash comparison: Transfer mode's size-only
  // check is never enough proof to delete the only remaining copy of a file.
  const moveEligible = selectedDestinations.length === 1 && verificationMode !== "transfer";

  function toggleDestination(diskId: string) {
    setDestIds((prev) =>
      prev.includes(diskId) ? prev.filter((id) => id !== diskId) : [...prev, diskId],
    );
  }

  function handleStart() {
    if (!source || selectedDestinations.length === 0) return;
    onStart(source, selectedDestinations, mode, moveAfterTransfer && moveEligible);
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
              Destinations
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
                </label>
              );
            })}
          </div>

          {mode === "cascade" && selectedDestinations.length > 0 && (
            <div className="flex flex-col gap-1">
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                Cascade order -- drag to reorder
              </span>
              <div className="flex flex-col">
                {selectedDestinations.map((d, i) => (
                  <div key={d.diskId} className="flex flex-col items-stretch">
                    <div
                      draggable
                      onDragStart={(e) => {
                        e.dataTransfer.setData(CASCADE_REORDER_MIME, String(i));
                        e.dataTransfer.effectAllowed = "move";
                      }}
                      onDragOver={(e) => {
                        e.preventDefault();
                        setDragOverIndex(i);
                      }}
                      onDragLeave={() => setDragOverIndex((cur) => (cur === i ? null : cur))}
                      onDrop={(e) => {
                        e.preventDefault();
                        setDragOverIndex(null);
                        const from = Number(e.dataTransfer.getData(CASCADE_REORDER_MIME));
                        if (!Number.isNaN(from)) moveDestination(from, i);
                      }}
                      className={`flex cursor-grab items-center justify-between rounded border px-2 py-1 text-xs active:cursor-grabbing ${
                        dragOverIndex === i
                          ? "border-blue-500 bg-blue-500/10"
                          : "border-neutral-700 bg-neutral-900"
                      }`}
                    >
                      <span>{endpointLabel(d, disks)}</span>
                      <span className="text-neutral-500">
                        {i === 0 ? "primary" : `hop ${i + 1}`}
                      </span>
                    </div>
                    {i < selectedDestinations.length - 1 && (
                      <span className="py-0.5 text-center text-neutral-600" aria-hidden="true">
                        ↓
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

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

          <label className="flex items-start gap-2 text-xs">
            <input
              type="checkbox"
              checked={moveAfterTransfer && moveEligible}
              disabled={!moveEligible}
              onChange={(e) => setMoveAfterTransfer(e.currentTarget.checked)}
              className="mt-0.5"
            />
            <span className={moveEligible ? undefined : "opacity-40"}>
              <span className="font-medium">Move (delete source after verified copy)</span>
              <span className="block text-neutral-500">
                {moveEligible
                  ? "Only removes a file once it's confirmed safe at the destination."
                  : selectedDestinations.length !== 1
                    ? "Only available with exactly one destination."
                    : "Requires Source or Source & Destination verification (not Transfer-only)."}
              </span>
            </span>
          </label>

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
