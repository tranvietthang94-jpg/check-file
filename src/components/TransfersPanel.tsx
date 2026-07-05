import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { TransferJob } from "../types/job";
import type { TransferGroup } from "../types/transferGroup";
import { formatBytes, formatEta, formatSpeed } from "../lib/format";
import { CancelIcon, ResumeIcon, RevealIcon } from "./icons/JobActionIcons";

const ICON_BUTTON =
  "flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-neutral-700 hover:bg-neutral-800";

/** `sourcePath` and a source-relative path (as recorded in
 * `brokenMediaFiles`/`pendingBrokenMedia`) may use either separator --
 * normalizing to "/" before joining works for both Windows and macOS. */
function joinPath(root: string, relative: string): string {
  const normalizedRoot = root.replace(/\\/g, "/").replace(/\/+$/, "");
  const normalizedRelative = relative.replace(/\\/g, "/").replace(/^\/+/, "");
  return `${normalizedRoot}/${normalizedRelative}`;
}

interface TransfersPanelProps {
  groups: TransferGroup[];
  jobs: Record<string, TransferJob>;
  onCancelJob: (jobId: string) => void;
  onCancelGroup: (group: TransferGroup) => void;
  onResumeJob: (job: TransferJob) => void;
  onResolveBrokenMedia: (jobId: string, proceed: boolean) => void;
}

/** Matches OffShoot's progress-bar color coding: grey while queued, blue
 * while copying, green on a clean completion, red for a hardware/checksum
 * failure, orange for a cancelled job or one with an integrity issue
 * (broken/corrupted source media) that was continued anyway. */
function progressBarColor(job: TransferJob): string {
  if (job.status === "cancelled") return "bg-orange-500";
  if (job.status === "complete") {
    if (job.failedFiles.length > 0) return "bg-red-400";
    if (job.brokenMediaFiles.length > 0) return "bg-orange-500";
    return "bg-green-500";
  }
  if (job.status === "copying") return "bg-blue-500";
  return "bg-neutral-500";
}

const MODE_LABEL: Record<TransferGroup["mode"], string> = {
  parallel: "Song song",
  cascade: "Nối tiếp",
};

const STATUS_LABEL: Record<TransferJob["status"], string> = {
  queued: "Đang chờ",
  copying: "Đang sao chép",
  complete: "Hoàn tất",
  cancelled: "Đã hủy",
};

function isActive(job: TransferJob | undefined): boolean {
  return !!job && (job.status === "queued" || job.status === "copying");
}

/** Stopped outright, or finished but with files that never made it -- the
 * same three triggers OffShoot documents for Resume (stopped, failed,
 * completed with warnings). */
function canResume(job: TransferJob): boolean {
  return job.status === "cancelled" || (job.status === "complete" && job.failedFiles.length > 0);
}

function JobRow({
  job,
  onCancel,
  onResume,
  onResolveBrokenMedia,
}: {
  job: TransferJob;
  onCancel: (jobId: string) => void;
  onResume: (job: TransferJob) => void;
  onResolveBrokenMedia: (jobId: string, proceed: boolean) => void;
}) {
  const pct = job.totalBytes > 0 ? Math.min(100, (job.bytesCopied / job.totalBytes) * 100) : 0;

  return (
    <li className="flex flex-col gap-2 rounded border border-neutral-800 bg-neutral-950 px-3 py-2">
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-xs">
          {job.hop === 2 && (
            <span className="mr-1 text-neutral-500" title="Chuyển tiếp từ đích chính">
              chuyển tiếp →
            </span>
          )}
          {job.destinationLabel}
        </span>
        <span className="flex shrink-0 items-center gap-1.5">
          {canResume(job) && (
            <button
              type="button"
              onClick={() => onResume(job)}
              title="Tiếp tục -- sao chép lại từ đầu cùng nguồn → đích -- các tệp đã sao lưu sẽ tự động được bỏ qua"
              aria-label="Tiếp tục"
              className={`${ICON_BUTTON} text-blue-400`}
            >
              <ResumeIcon className="h-3.5 w-3.5" />
            </button>
          )}
          <button
            type="button"
            onClick={() => revealItemInDir(job.destinationPath).catch(console.error)}
            title="Mở đích trong Explorer"
            aria-label="Mở đích trong Explorer"
            className={`${ICON_BUTTON} text-neutral-400`}
          >
            <RevealIcon className="h-3.5 w-3.5" />
          </button>
          {isActive(job) ? (
            <button
              type="button"
              onClick={() => onCancel(job.id)}
              title="Hủy"
              aria-label="Hủy"
              className={`${ICON_BUTTON} text-red-400`}
            >
              <CancelIcon className="h-3.5 w-3.5" />
            </button>
          ) : (
            <span className="text-xs text-neutral-400">{STATUS_LABEL[job.status]}</span>
          )}
        </span>
      </div>

      <div className="h-1.5 w-full overflow-hidden rounded bg-neutral-800">
        <div
          className={`h-full ${progressBarColor(job)} transition-all`}
          style={{ width: `${pct}%` }}
        />
      </div>

      <div className="flex justify-between gap-2 text-xs text-neutral-500">
        <span className="truncate">{job.currentFile || "—"}</span>
        <span className="shrink-0">
          {formatBytes(job.bytesCopied)} / {formatBytes(job.totalBytes)}
          {job.status === "copying" && job.bytesPerSec > 0 ? (
            <>
              {" · "}
              {formatEta((job.totalBytes - job.bytesCopied) / job.bytesPerSec)} (
              {formatSpeed(job.bytesPerSec)})
            </>
          ) : job.status === "copying" ? (
            ` · ${formatSpeed(job.bytesPerSec)}`
          ) : (
            ""
          )}
        </span>
      </div>

      {job.pendingBrokenMedia && (
        <div className="flex flex-col gap-2 rounded border border-orange-700 bg-orange-950/40 px-2 py-2 text-xs">
          <p className="text-orange-300" title={job.pendingBrokenMedia.join(", ")}>
            ⚠ Phát hiện {job.pendingBrokenMedia.length} tệp hỏng (0 byte) ở nguồn — có thể do thẻ nhớ
            bị rút giữa lúc quay.
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() =>
                revealItemInDir(joinPath(job.sourcePath, job.pendingBrokenMedia![0])).catch(
                  console.error,
                )
              }
              className="rounded border border-neutral-700 px-2 py-1 text-neutral-200"
            >
              Hiện trong Explorer
            </button>
            <button
              type="button"
              onClick={() => onResolveBrokenMedia(job.id, true)}
              className="rounded border border-neutral-700 px-2 py-1 text-neutral-200"
            >
              Vẫn tiếp tục
            </button>
            <button
              type="button"
              onClick={() => onResolveBrokenMedia(job.id, false)}
              className="rounded border border-neutral-700 px-2 py-1 text-red-400"
            >
              Hủy công việc
            </button>
          </div>
        </div>
      )}
      {job.resumeBlockedReason && (
        <p className="text-xs text-red-400">⚠ {job.resumeBlockedReason}</p>
      )}
      {job.verifiedFiles.length > 0 && (
        <p className="text-xs text-green-500">
          {job.verificationMode === "sourceAndDestination"
            ? `${job.verifiedFiles.length} tệp đã xác minh (nguồn = đích, `
            : `${job.verifiedFiles.length} tệp đã băm (`}
          {job.verifiedFiles[0].algorithm.toUpperCase()})
        </p>
      )}
      {job.skippedFiles.length > 0 && (
        <p
          className="text-xs text-neutral-400"
          title={job.skippedFiles.map((f) => f.path).join(", ")}
        >
          {job.skippedFiles.length} tệp đã bỏ qua (đã sao lưu trước đó)
        </p>
      )}
      {job.renamedFiles.length > 0 && (
        <p
          className="text-xs text-yellow-500"
          title={job.renamedFiles.map((f) => `${f.originalPath} → ${f.renamedTo}`).join(", ")}
        >
          {job.renamedFiles.length} tệp đã đổi tên (tên đã được dùng bởi tệp khác)
        </p>
      )}
      {job.failedFiles.length > 0 && (
        <p className="text-xs text-red-400" title={job.failedFiles[0].message}>
          {job.failedFiles.length} tệp thất bại — {job.failedFiles[0].message}
        </p>
      )}
      {job.deletedSourceFiles.length > 0 && (
        <p
          className="text-xs text-neutral-400"
          title={job.deletedSourceFiles.join(", ")}
        >
          {job.deletedSourceFiles.length} tệp đã di chuyển (đã xóa nguồn sau khi sao chép được xác minh)
        </p>
      )}
      {job.moveDeleteFailed.length > 0 && (
        <p className="text-xs text-orange-400" title={job.moveDeleteFailed[0].message}>
          {job.moveDeleteFailed.length} tệp đã sao chép nhưng không xóa được nguồn —{" "}
          {job.moveDeleteFailed[0].message}
        </p>
      )}
      {!job.pendingBrokenMedia && job.brokenMediaFiles.length > 0 && (
        <p className="text-xs text-orange-400" title={job.brokenMediaFiles.join(", ")}>
          Phát hiện {job.brokenMediaFiles.length} tệp hỏng (0 byte) ở nguồn
        </p>
      )}
    </li>
  );
}

export function TransfersPanel({
  groups,
  jobs,
  onCancelJob,
  onCancelGroup,
  onResumeJob,
  onResolveBrokenMedia,
}: TransfersPanelProps) {
  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Truyền tải
      </h2>
      {groups.length === 0 && <p className="text-sm text-neutral-500">Chưa có lượt truyền nào.</p>}
      <ul className="flex flex-col gap-3">
        {groups.map((group) => {
          const groupJobs = group.jobIds.map((id) => jobs[id]).filter((j): j is TransferJob => !!j);
          const groupActive = groupJobs.some(isActive);
          return (
            <li
              key={group.id}
              className="flex flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-2"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="truncate text-sm">
                  {group.sourceLabel}{" "}
                  <span className="text-neutral-500">→</span>{" "}
                  {group.destinationLabels.join(
                    group.mode === "cascade" ? " → " : ", ",
                  )}
                  <span className="ml-2 text-[10px] uppercase tracking-wide text-neutral-500">
                    {MODE_LABEL[group.mode]}
                  </span>
                </span>
                {groupActive && (
                  <button
                    type="button"
                    onClick={() => onCancelGroup(group)}
                    className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs text-red-400"
                  >
                    Hủy nhóm
                  </button>
                )}
              </div>

              <ul className="flex flex-col gap-2">
                {groupJobs.map((job) => (
                  <JobRow
                    key={job.id}
                    job={job}
                    onCancel={onCancelJob}
                    onResume={onResumeJob}
                    onResolveBrokenMedia={onResolveBrokenMedia}
                  />
                ))}
                {group.mode === "cascade" &&
                  groupJobs.length < group.destinationLabels.length &&
                  groupJobs.length > 0 &&
                  groupJobs[0].status === "complete" && (
                    <li className="text-xs text-neutral-500">Đang chờ chuyển tiếp…</li>
                  )}
              </ul>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
