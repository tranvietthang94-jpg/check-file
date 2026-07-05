import { useDisksStore } from "../../state/disksStore";

export function DisksPreferences() {
  const disks = useDisksStore((s) => s.disks);
  const hiddenDiskIds = useDisksStore((s) => s.hiddenDiskIds);
  const unhideDisk = useDisksStore((s) => s.unhideDisk);

  const hiddenDisks = disks.filter((d) => hiddenDiskIds.includes(d.id));

  return (
    <div className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Hidden drives
      </h3>
      {hiddenDisks.length === 0 ? (
        <p className="text-xs text-neutral-500">
          No hidden drives. Use "Hide" on a drive in the Disks view to hide it from the list.
        </p>
      ) : (
        <ul className="flex flex-col gap-1">
          {hiddenDisks.map((disk) => (
            <li
              key={disk.id}
              className="flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-1.5 text-xs"
            >
              <span className="truncate text-neutral-400">
                {disk.name} · {disk.mountPoint}
              </span>
              <button
                type="button"
                onClick={() => unhideDisk(disk.id)}
                className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs"
              >
                Unhide
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
