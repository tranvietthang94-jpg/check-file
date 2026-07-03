export type JobStatus = "scanning" | "copying" | "complete" | "cancelled";

export interface FailedFile {
  path: string;
  message: string;
}

export interface TransferJob {
  id: string;
  sourceDiskId: string;
  destinationDiskId: string;
  sourceLabel: string;
  destinationLabel: string;
  sourcePath: string;
  destinationPath: string;
  status: JobStatus;
  currentFile: string;
  bytesCopied: number;
  totalBytes: number;
  filesCopied: number;
  totalFiles: number;
  bytesPerSec: number;
  failedFiles: FailedFile[];
}

export interface ScanEventPayload {
  jobId: string;
  totalFiles: number;
  totalBytes: number;
}

export interface ProgressEventPayload {
  jobId: string;
  currentFile: string;
  bytesCopied: number;
  totalBytes: number;
  filesCopied: number;
  totalFiles: number;
  bytesPerSec: number;
}

export interface CompleteEventPayload {
  jobId: string;
  filesCopied: number;
  bytesCopied: number;
  failedFiles: FailedFile[];
}

export interface CancelledEventPayload {
  jobId: string;
}
