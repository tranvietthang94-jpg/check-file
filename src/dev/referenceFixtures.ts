import type { DiskInfo, Endpoint } from "../types/disk";
import type { TransferJob } from "../types/job";
import type { TransferGroup } from "../types/transferGroup";

export const referenceDisks: DiskInfo[] = [
  { id: "D:", name: "KHANH VAN", mountPoint: "D:\\", totalBytes: 5_550_000_000_000, availableBytes: 500_000_000_000, isRemovable: true, fileSystem: "NTFS" },
  { id: "G:", name: "Local Disk I", mountPoint: "G:\\", totalBytes: 5_790_000_000_000, availableBytes: 520_000_000_000, isRemovable: false, fileSystem: "NTFS" },
  { id: "F:", name: "TEMP SSD", mountPoint: "F:\\", totalBytes: 725_000_000_000, availableBytes: 100_000_000_000, isRemovable: true, fileSystem: "NTFS" },
  { id: "C:", name: "Local Disk", mountPoint: "C:\\", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: false, fileSystem: "NTFS" },
  { id: "onedrive", name: "OneDrive - Personal", mountPoint: "C:\\Users\\Rin\\OneDrive", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: false, fileSystem: "Cloud" },
  { id: "phone", name: "S24 Ultra RinOP", mountPoint: "Phone:\\", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: true, fileSystem: "MTP" },
  { id: "tablet", name: "Tab S8+ by RinOP", mountPoint: "Tablet:\\", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: true, fileSystem: "MTP" },
  { id: "E:", name: "DATA", mountPoint: "E:\\", totalBytes: 1_000_000_000_000, availableBytes: 276_000_000_000, isRemovable: false, fileSystem: "NTFS" },
];

export const referenceSources: Endpoint[] = [];
export const referenceDestinations: Endpoint[] = [
  { diskId: "E:", label: "", path: "E:\\", isAutoLabel: false },
];

function job(id: string, status: TransferJob["status"], overrides: Partial<TransferJob> = {}): TransferJob {
  return {
    id,
    groupId: "fixture-group",
    hop: 1,
    sourceLabel: "CAM A",
    destinationLabel: "DATA",
    sourcePath: "D:\\",
    destinationPath: "E:\\",
    verificationMode: "sourceAndDestination",
    checksumAlgorithm: "xxh64",
    status,
    currentFile: "",
    bytesCopied: 0,
    totalBytes: 100_000_000,
    filesCopied: 0,
    totalFiles: 10,
    bytesPerSec: 0,
    failedFiles: [],
    verifiedFiles: [],
    skippedFiles: [],
    renamedFiles: [],
    deletedSourceFiles: [],
    moveDeleteFailed: [],
    brokenMediaFiles: [],
    missingFiles: [],
    pendingBrokenMedia: null,
    sourceVolumeSignature: "fixture-volume",
    resumeBlockedReason: null,
    ...overrides,
  };
}

export const referenceJobs: Record<string, TransferJob> = {
  queued: job("queued", "queued"),
  copying: job("copying", "copying", {
    destinationLabel: "BACKUP A",
    currentFile: "clip_004.mov",
    bytesCopied: 50_000_000,
    filesCopied: 4,
    bytesPerSec: 12_000_000,
  }),
  complete: job("complete", "complete", {
    destinationLabel: "BACKUP B",
    bytesCopied: 100_000_000,
    filesCopied: 10,
    verifiedFiles: [{ path: "clip.mov", checksum: "abc", algorithm: "xxh64" }],
  }),
  failed: job("failed", "complete", {
    destinationLabel: "BACKUP C",
    bytesCopied: 90_000_000,
    filesCopied: 9,
    failedFiles: [{ path: "bad.mov", message: "checksum mismatch" }],
  }),
};

export const referenceGroups: Record<string, TransferGroup> = {
  "fixture-group": {
    id: "fixture-group",
    mode: "parallel",
    sourceLabel: "CAM A",
    destinationLabels: ["DATA", "BACKUP A", "BACKUP B", "BACKUP C"],
    jobIds: ["queued", "copying", "complete", "failed"],
  },
};

export function referenceFixture(): "disks" | "transfers" | null {
  if (!import.meta.env.DEV) return null;
  const fixture = new URLSearchParams(window.location.search).get("referenceFixture");
  return fixture === "disks" || fixture === "transfers" ? fixture : null;
}
