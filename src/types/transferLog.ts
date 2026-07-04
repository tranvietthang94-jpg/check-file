import type {
  ChecksumAlgorithm,
  FailedFile,
  RenamedFile,
  SkippedFile,
  VerificationMode,
  VerifiedFile,
} from "./job";

export interface TransferLogEntry {
  jobId: string;
  sourceName: string;
  source: string;
  destination: string;
  verificationMode: VerificationMode;
  checksumAlgorithm: ChecksumAlgorithm;
  startedAt: string;
  finishedAt: string;
  filesCopied: number;
  bytesCopied: number;
  failedFiles: FailedFile[];
  verifiedFiles: VerifiedFile[];
  skippedFiles: SkippedFile[];
  renamedFiles: RenamedFile[];
  deletedSourceFiles: string[];
  moveDeleteFailed: FailedFile[];
  mhlPath: string | null;
  cancelled: boolean;
}
