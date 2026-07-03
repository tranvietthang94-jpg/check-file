import type { DiskInfo, Endpoint } from "../types/disk";
import type { TransferJob } from "../types/job";
import { formatBytes, formatSpeed } from "../lib/format";

interface TransfersPanelProps {
  sources: Endpoint[];
  destinations: Endpoint[];
  disks: DiskInfo[];
  jobs: TransferJob[];
  onStart: (source: Endpoint, destination: Endpoint) => void;
  onCancel: (jobId: string) => void;
}

function endpointLabel(endpoint: Endpoint, disks: DiskInfo[]): string {
  const disk = disks.find((d) => d.id === endpoint.diskId);
  return endpoint.label || disk?.name || endpoint.diskId;
}

const STATUS_COLOR: Record<TransferJob["status"], string> = {
  scanning: "bg-neutral-500",
  copying: "bg-blue-500",
  complete: "bg-green-500",
  cancelled: "bg-orange-500",
};

export function TransfersPanel({
  sources,
  destinations,
  disks,
  jobs,
  onStart,
  onCancel,
}: TransfersPanelProps) {
  const pairs = sources.flatMap((source) =>
    destinations.map((destination) => ({ source, destination })),
  );

  return (
    <section className="flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Start Transfer
        </h2>
        {pairs.length === 0 && (
          <p className="text-sm text-neutral-500">
            Assign at least one Source and one Destination to start a transfer.
          </p>
        )}
        <ul className="flex flex-col gap-2">
          {pairs.map(({ source, destination }) => (
            <li
              key={`${source.diskId}->${destination.diskId}`}
              className="flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2"
            >
              <span className="truncate text-sm">
                {endpointLabel(source, disks)} <span className="text-neutral-500">→</span>{" "}
                {endpointLabel(destination, disks)}
              </span>
              <button
                type="button"
                onClick={() => onStart(source, destination)}
                className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs"
              >
                Start
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Transfers
        </h2>
        {jobs.length === 0 && <p className="text-sm text-neutral-500">No transfers yet.</p>}
        <ul className="flex flex-col gap-2">
          {jobs.map((job) => {
            const pct =
              job.totalBytes > 0
                ? Math.min(100, (job.bytesCopied / job.totalBytes) * 100)
                : 0;
            return (
              <li
                key={job.id}
                className="flex flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="truncate text-sm">
                    {job.sourceLabel} <span className="text-neutral-500">→</span>{" "}
                    {job.destinationLabel}
                  </span>
                  {job.status === "copying" || job.status === "scanning" ? (
                    <button
                      type="button"
                      onClick={() => onCancel(job.id)}
                      className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs text-red-400"
                    >
                      Cancel
                    </button>
                  ) : (
                    <span className="shrink-0 text-xs capitalize text-neutral-400">
                      {job.status}
                    </span>
                  )}
                </div>

                <div className="h-2 w-full overflow-hidden rounded bg-neutral-800">
                  <div
                    className={`h-full ${STATUS_COLOR[job.status]} transition-all`}
                    style={{ width: `${pct}%` }}
                  />
                </div>

                <div className="flex justify-between gap-2 text-xs text-neutral-500">
                  <span className="truncate">{job.currentFile || "—"}</span>
                  <span className="shrink-0">
                    {formatBytes(job.bytesCopied)} / {formatBytes(job.totalBytes)}
                    {job.status === "copying" ? ` · ${formatSpeed(job.bytesPerSec)}` : ""}
                  </span>
                </div>

                {job.failedFiles.length > 0 && (
                  <p className="text-xs text-red-400">
                    {job.failedFiles.length} file(s) failed
                  </p>
                )}
              </li>
            );
          })}
        </ul>
      </div>
    </section>
  );
}
