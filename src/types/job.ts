export type JobStatus = "queued" | "copying" | "complete" | "cancelled";

export type VerificationMode = "transfer" | "source" | "sourceAndDestination";
export type ChecksumAlgorithm = "xxh64" | "md5" | "sha1";

export interface FailedFile {
  path: string;
  message: string;
}

export interface VerifiedFile {
  path: string;
  checksum: string;
  algorithm: ChecksumAlgorithm;
}

export interface SkippedFile {
  path: string;
}

export interface RenamedFile {
  originalPath: string;
  renamedTo: string;
}

export interface TransferJob {
  id: string;
  groupId: string;
  /** 1 = copies from the original source. 2 = relayed from a cascade's primary destination. */
  hop: 1 | 2;
  sourceLabel: string;
  destinationLabel: string;
  sourcePath: string;
  destinationPath: string;
  verificationMode: VerificationMode;
  checksumAlgorithm: ChecksumAlgorithm;
  status: JobStatus;
  currentFile: string;
  bytesCopied: number;
  totalBytes: number;
  filesCopied: number;
  totalFiles: number;
  bytesPerSec: number;
  failedFiles: FailedFile[];
  verifiedFiles: VerifiedFile[];
  skippedFiles: SkippedFile[];
  renamedFiles: RenamedFile[];
  deletedSourceFiles: string[];
  moveDeleteFailed: FailedFile[];
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
  verifiedFiles: VerifiedFile[];
  skippedFiles: SkippedFile[];
  renamedFiles: RenamedFile[];
  deletedSourceFiles: string[];
  moveDeleteFailed: FailedFile[];
}

export interface CancelledEventPayload {
  jobId: string;
}
