import { useEffect } from "react";
import { useTransferLogStore } from "../state/transferLogStore";
import { formatBytes } from "../lib/format";
import type { TransferLogEntry } from "../types/transferLog";

interface TransferLogPanelProps {
  onViewClips: (path: string) => void;
}

function mhlFileName(path: string | null): string | null {
  if (!path) return null;
  const normalized = path.replace(/\\/g, "/");
  return normalized.substring(normalized.lastIndexOf("/") + 1);
}

function formatTimestamp(iso: string): string {
  const parsed = new Date(iso);
  return Number.isNaN(parsed.getTime()) ? iso : parsed.toLocaleString();
}

function LogRow({ entry, onViewClips }: { entry: TransferLogEntry; onViewClips: (path: string) => void }) {
  const mhlName = mhlFileName(entry.mhlPath);
  return (
    <li className="flex flex-col gap-1 rounded border border-neutral-800 bg-neutral-900 px-3 py-2 text-xs">
      <div className="flex items-center justify-between gap-2">
        <span className="font-medium">
          {entry.sourceName}
          {entry.cancelled && <span className="ml-2 text-orange-400">Stopped</span>}
        </span>
        <span className="text-neutral-500">{formatTimestamp(entry.finishedAt)}</span>
      </div>
      <div className="truncate text-neutral-500" title={`${entry.source} -> ${entry.destination}`}>
        {entry.source} → {entry.destination}
      </div>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-neutral-400">
        <span>
          {entry.filesCopied} file(s) · {formatBytes(entry.bytesCopied)}
        </span>
        {entry.skippedFiles.length > 0 && <span>{entry.skippedFiles.length} skipped</span>}
        {entry.renamedFiles.length > 0 && <span>{entry.renamedFiles.length} renamed</span>}
        {entry.failedFiles.length > 0 && (
          <span className="text-red-400">{entry.failedFiles.length} failed</span>
        )}
        {entry.deletedSourceFiles.length > 0 && (
          <span title={entry.deletedSourceFiles.join(", ")}>
            {entry.deletedSourceFiles.length} moved
          </span>
        )}
        {entry.moveDeleteFailed.length > 0 && (
          <span className="text-orange-400" title={entry.moveDeleteFailed[0].message}>
            {entry.moveDeleteFailed.length} move-delete failed
          </span>
        )}
        {mhlName && <span title={entry.mhlPath ?? undefined}>MHL: {mhlName}</span>}
      </div>
      <button
        type="button"
        onClick={() => onViewClips(entry.destination)}
        className="mt-1 w-fit rounded border border-neutral-700 px-2 py-1 text-xs"
      >
        View clips
      </button>
    </li>
  );
}

export function TransferLogPanel({ onViewClips }: TransferLogPanelProps) {
  const logs = useTransferLogStore((s) => s.logs);
  const loading = useTransferLogStore((s) => s.loading);
  const error = useTransferLogStore((s) => s.error);
  const refresh = useTransferLogStore((s) => s.refresh);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <section className="col-span-full flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Transfer Log
        </h2>
        <button
          type="button"
          onClick={() => refresh()}
          className="rounded border border-neutral-700 px-2 py-1 text-xs"
        >
          Refresh
        </button>
      </div>

      {loading && <p className="text-xs text-neutral-500">Loading…</p>}
      {error && <p className="text-xs text-red-400">{error}</p>}
      {!loading && logs.length === 0 && (
        <p className="text-xs text-neutral-500">No completed transfers yet.</p>
      )}

      <ul className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {logs.map((entry) => (
          <LogRow key={entry.jobId} entry={entry} onViewClips={onViewClips} />
        ))}
      </ul>
    </section>
  );
}
