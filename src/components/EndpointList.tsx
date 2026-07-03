import type { DiskInfo, Endpoint } from "../types/disk";

interface EndpointListProps {
  title: string;
  endpoints: Endpoint[];
  disks: DiskInfo[];
  onRemove: (diskId: string) => void;
  onLabelChange: (diskId: string, label: string) => void;
}

export function EndpointList({
  title,
  endpoints,
  disks,
  onRemove,
  onLabelChange,
}: EndpointListProps) {
  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        {title}
      </h2>
      {endpoints.length === 0 && (
        <p className="text-sm text-neutral-500">None assigned yet.</p>
      )}
      <ul className="flex flex-col gap-2">
        {endpoints.map((endpoint) => {
          const disk = disks.find((d) => d.id === endpoint.diskId);
          return (
            <li
              key={endpoint.diskId}
              className="flex flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2"
            >
              <div className="flex flex-col">
                <span className="font-medium">{disk?.name ?? endpoint.diskId}</span>
                <span className="text-xs text-neutral-500">
                  {disk?.mountPoint ?? "(unplugged)"}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <input
                  value={endpoint.label}
                  onChange={(e) => onLabelChange(endpoint.diskId, e.currentTarget.value)}
                  placeholder="Label…"
                  className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
                />
                <button
                  type="button"
                  onClick={() => onRemove(endpoint.diskId)}
                  className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs text-red-400"
                >
                  Remove
                </button>
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
