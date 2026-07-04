import type { TransferJob } from "../types/job";
import type { TransferGroup } from "../types/transferGroup";
import { formatBytes, formatSpeed } from "../lib/format";

interface TransfersPanelProps {
  groups: TransferGroup[];
  jobs: Record<string, TransferJob>;
  onCancelJob: (jobId: string) => void;
  onCancelGroup: (group: TransferGroup) => void;
  onResumeJob: (job: TransferJob) => void;
  onResolveBrokenMedia: (jobId: string, proceed: boolean) => void;
}

const STATUS_COLOR: Record<TransferJob["status"], string> = {
  queued: "bg-neutral-500",
  copying: "bg-blue-500",
  complete: "bg-green-500",
  cancelled: "bg-orange-500",
};

const MODE_LABEL: Record<TransferGroup["mode"], string> = {
  parallel: "Parallel",
  cascade: "Cascade",
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
            <span className="mr-1 text-neutral-500" title="Relayed from the primary destination">
              relay →
            </span>
          )}
          {job.destinationLabel}
        </span>
        <span className="flex shrink-0 items-center gap-2">
          {canResume(job) && (
            <button
              type="button"
              onClick={() => onResume(job)}
              title="Start a fresh copy of the same source → destination -- already-offloaded files are skipped automatically"
              className="rounded border border-neutral-700 px-2 py-1 text-xs text-blue-400"
            >
              Resume
            </button>
          )}
          {isActive(job) ? (
            <button
              type="button"
              onClick={() => onCancel(job.id)}
              className="rounded border border-neutral-700 px-2 py-1 text-xs text-red-400"
            >
              Cancel
            </button>
          ) : (
            <span className="text-xs capitalize text-neutral-400">{job.status}</span>
          )}
        </span>
      </div>

      <div className="h-1.5 w-full overflow-hidden rounded bg-neutral-800">
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

      {job.pendingBrokenMedia && (
        <div className="flex flex-col gap-2 rounded border border-orange-700 bg-orange-950/40 px-2 py-2 text-xs">
          <p className="text-orange-300" title={job.pendingBrokenMedia.join(", ")}>
            ⚠ {job.pendingBrokenMedia.length} broken (0-byte) file(s) found on the source — likely a
            card that dropped out mid-recording.
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onResolveBrokenMedia(job.id, true)}
              className="rounded border border-neutral-700 px-2 py-1 text-neutral-200"
            >
              Continue Anyway
            </button>
            <button
              type="button"
              onClick={() => onResolveBrokenMedia(job.id, false)}
              className="rounded border border-neutral-700 px-2 py-1 text-red-400"
            >
              Cancel Job
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
            ? `${job.verifiedFiles.length} file(s) verified (source = destination, `
            : `${job.verifiedFiles.length} file(s) hashed (`}
          {job.verifiedFiles[0].algorithm.toUpperCase()})
        </p>
      )}
      {job.skippedFiles.length > 0 && (
        <p
          className="text-xs text-neutral-400"
          title={job.skippedFiles.map((f) => f.path).join(", ")}
        >
          {job.skippedFiles.length} file(s) skipped (already offloaded)
        </p>
      )}
      {job.renamedFiles.length > 0 && (
        <p
          className="text-xs text-yellow-500"
          title={job.renamedFiles.map((f) => `${f.originalPath} → ${f.renamedTo}`).join(", ")}
        >
          {job.renamedFiles.length} file(s) renamed (name already used by a different file)
        </p>
      )}
      {job.failedFiles.length > 0 && (
        <p className="text-xs text-red-400" title={job.failedFiles[0].message}>
          {job.failedFiles.length} file(s) failed — {job.failedFiles[0].message}
        </p>
      )}
      {job.deletedSourceFiles.length > 0 && (
        <p
          className="text-xs text-neutral-400"
          title={job.deletedSourceFiles.join(", ")}
        >
          {job.deletedSourceFiles.length} file(s) moved (source removed after verified copy)
        </p>
      )}
      {job.moveDeleteFailed.length > 0 && (
        <p className="text-xs text-orange-400" title={job.moveDeleteFailed[0].message}>
          {job.moveDeleteFailed.length} file(s) copied but the source could not be removed —{" "}
          {job.moveDeleteFailed[0].message}
        </p>
      )}
      {!job.pendingBrokenMedia && job.brokenMediaFiles.length > 0 && (
        <p className="text-xs text-orange-400" title={job.brokenMediaFiles.join(", ")}>
          {job.brokenMediaFiles.length} broken (0-byte) file(s) were found on the source
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
        Transfers
      </h2>
      {groups.length === 0 && <p className="text-sm text-neutral-500">No transfers yet.</p>}
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
                  {group.destinationLabels.join(", ")}
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
                    Cancel Group
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
                    <li className="text-xs text-neutral-500">Waiting to relay…</li>
                  )}
              </ul>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
