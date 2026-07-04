import { create } from "zustand";
import type {
  BrokenMediaEventPayload,
  CancelledEventPayload,
  CompleteEventPayload,
  ProgressEventPayload,
  ScanEventPayload,
  TransferJob,
} from "../types/job";

interface TransfersState {
  jobs: Record<string, TransferJob>;
  addJob: (job: TransferJob) => void;
  applyScan: (payload: ScanEventPayload) => void;
  applyProgress: (payload: ProgressEventPayload) => void;
  applyComplete: (payload: CompleteEventPayload) => void;
  applyCancelled: (payload: CancelledEventPayload) => void;
  applyBrokenMedia: (payload: BrokenMediaEventPayload) => void;
  clearBrokenMediaAlert: (jobId: string) => void;
}

function updateJob(
  jobs: Record<string, TransferJob>,
  jobId: string,
  patch: Partial<TransferJob>,
): Record<string, TransferJob> {
  const job = jobs[jobId];
  if (!job) return jobs;
  return { ...jobs, [jobId]: { ...job, ...patch } };
}

export const useTransfersStore = create<TransfersState>((set) => ({
  jobs: {},

  addJob: (job) => set((state) => ({ jobs: { ...state.jobs, [job.id]: job } })),

  applyScan: (payload) =>
    set((state) => ({
      jobs: updateJob(state.jobs, payload.jobId, {
        status: "copying",
        totalFiles: payload.totalFiles,
        totalBytes: payload.totalBytes,
      }),
    })),

  applyProgress: (payload) =>
    set((state) => ({
      jobs: updateJob(state.jobs, payload.jobId, {
        status: "copying",
        currentFile: payload.currentFile,
        bytesCopied: payload.bytesCopied,
        totalBytes: payload.totalBytes,
        filesCopied: payload.filesCopied,
        totalFiles: payload.totalFiles,
        bytesPerSec: payload.bytesPerSec,
      }),
    })),

  applyComplete: (payload) =>
    set((state) => ({
      jobs: updateJob(state.jobs, payload.jobId, {
        status: "complete",
        bytesCopied: payload.bytesCopied,
        filesCopied: payload.filesCopied,
        failedFiles: payload.failedFiles,
        verifiedFiles: payload.verifiedFiles,
        skippedFiles: payload.skippedFiles,
        renamedFiles: payload.renamedFiles,
        deletedSourceFiles: payload.deletedSourceFiles,
        moveDeleteFailed: payload.moveDeleteFailed,
        brokenMediaFiles: payload.brokenMediaFiles,
        currentFile: "",
      }),
    })),

  applyCancelled: (payload) =>
    set((state) => ({
      jobs: updateJob(state.jobs, payload.jobId, {
        status: "cancelled",
        currentFile: "",
        pendingBrokenMedia: null,
      }),
    })),

  applyBrokenMedia: (payload) =>
    set((state) => ({
      jobs: updateJob(state.jobs, payload.jobId, {
        pendingBrokenMedia: payload.files,
      }),
    })),

  clearBrokenMediaAlert: (jobId) =>
    set((state) => ({
      jobs: updateJob(state.jobs, jobId, { pendingBrokenMedia: null }),
    })),
}));
