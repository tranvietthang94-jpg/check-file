import { useEffect } from "react";
import { useTransferLogStore } from "../state/transferLogStore";
import { formatBytes } from "../lib/format";
import { Panel } from "./ui/Panel";
import { EmptyState } from "./ui/EmptyState";
import { Button } from "./ui/Button";
import { Badge } from "./ui/Badge";
import { History, RefreshCw } from "./icons";
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
    <Panel as="li" className="flex flex-col gap-1 px-3 py-2 text-xs">
      <div className="flex items-center justify-between gap-2">
        <span className="flex items-center gap-2 font-medium">
          {entry.sourceName}
          {entry.cancelled && <Badge tone="orange">Đã dừng</Badge>}
        </span>
        <span className="text-neutral-500">{formatTimestamp(entry.finishedAt)}</span>
      </div>
      <div className="truncate text-neutral-500" title={`${entry.source} -> ${entry.destination}`}>
        {entry.source} → {entry.destination}
      </div>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-neutral-400">
        <span>
          {entry.filesCopied} tệp · {formatBytes(entry.bytesCopied)}
        </span>
        {entry.skippedFiles.length > 0 && <span>{entry.skippedFiles.length} đã bỏ qua</span>}
        {entry.renamedFiles.length > 0 && <span>{entry.renamedFiles.length} đã đổi tên</span>}
        {entry.failedFiles.length > 0 && (
          <span className="text-red-400">{entry.failedFiles.length} thất bại</span>
        )}
        {entry.deletedSourceFiles.length > 0 && (
          <span title={entry.deletedSourceFiles.join(", ")}>
            {entry.deletedSourceFiles.length} đã di chuyển
          </span>
        )}
        {entry.moveDeleteFailed.length > 0 && (
          <span className="text-orange-400" title={entry.moveDeleteFailed[0].message}>
            {entry.moveDeleteFailed.length} di chuyển-xóa thất bại
          </span>
        )}
        {entry.missingFiles.length > 0 && (
          <span className="text-red-400" title={entry.missingFiles.join(", ")}>
            {entry.missingFiles.length} tệp bị thiếu ở đích
          </span>
        )}
        {mhlName && <span title={entry.mhlPath ?? undefined}>MHL: {mhlName}</span>}
      </div>
      <Button variant="secondary" className="mt-1 w-fit" onClick={() => onViewClips(entry.destination)}>
        Xem clip
      </Button>
    </Panel>
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
    <div className="flex flex-col gap-2">
      <div className="flex justify-end">
        <Button
          variant="secondary"
          icon={<RefreshCw className="h-3.5 w-3.5" />}
          onClick={() => refresh()}
        >
          Làm mới
        </Button>
      </div>

      {loading && <p className="text-xs text-neutral-500">Đang tải…</p>}
      {error && <p className="text-xs text-red-400">{error}</p>}
      {!loading && logs.length === 0 && (
        <EmptyState icon={<History className="h-5 w-5" />}>
          Chưa có lượt truyền nào hoàn tất.
        </EmptyState>
      )}

      <ul className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
        {logs.map((entry) => (
          <LogRow key={entry.jobId} entry={entry} onViewClips={onViewClips} />
        ))}
      </ul>
    </div>
  );
}
