import { useDisksStore } from "../state/disksStore";
import { formatBytes } from "../lib/format";

export function DisksPanel() {
  const disks = useDisksStore((s) => s.disks);
  const sources = useDisksStore((s) => s.sources);
  const destinations = useDisksStore((s) => s.destinations);
  const addSource = useDisksStore((s) => s.addSource);
  const addDestination = useDisksStore((s) => s.addDestination);

  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Disks
      </h2>
      {disks.length === 0 && (
        <p className="text-sm text-neutral-500">No volumes detected.</p>
      )}
      <ul className="flex flex-col gap-2">
        {disks.map((disk) => {
          const isSource = sources.some((s) => s.diskId === disk.id);
          const isDestination = destinations.some((d) => d.diskId === disk.id);
          return (
            <li
              key={disk.id}
              className="flex flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2"
            >
              <div className="flex flex-col">
                <span className="font-medium">{disk.name}</span>
                <span className="text-xs text-neutral-500">
                  {disk.mountPoint} · {formatBytes(disk.availableBytes)} free of{" "}
                  {formatBytes(disk.totalBytes)}
                  {disk.isRemovable ? " · removable" : ""}
                </span>
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
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
