export type JobStatus = "queued" | "copying" | "complete" | "cancelled";

export type VerificationMode = "transfer" | "source" | "sourceAndDestination";
export type ChecksumAlgorithm = "xxh64" | "md5" | "sha1" | "c4";

export interface FailedFile {
  path: string;
  message: string;
}

export interface VerifiedFile {
  path: string;
  checksum: string;
  algorithm: ChecksumAlgorithm;
  /** OffShoot's "Also generate legacy checksums" -- a second hash computed
   * alongside the primary one, for interop with tooling expecting an older
   * algorithm. Absent unless that Preferences > Transfers setting is on. */
  legacyChecksum?: string;
  legacyAlgorithm?: ChecksumAlgorithm;
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
  brokenMediaFiles: string[];
  /** OffShoot's "Missing Files Detection" -- destination-relative paths a
   * final post-transfer presence sweep couldn't find on disk. */
  missingFiles: string[];
  /** Non-null while a Broken Media alert is awaiting Continue/Cancel from the user. */
  pendingBrokenMedia: string[] | null;
  /** The source's volume identifier recorded when this job started (see Source Index). */
  sourceVolumeSignature: string | null;
  /** Non-null when a Resume attempt was blocked because the source disk no longer matches. */
  resumeBlockedReason: string | null;
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
  brokenMediaFiles: string[];
  missingFiles: string[];
}

export interface CancelledEventPayload {
  jobId: string;
}

export interface BrokenMediaEventPayload {
  jobId: string;
  files: string[];
}
