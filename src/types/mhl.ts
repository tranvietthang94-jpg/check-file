export type MhlEntryStatus =
  | "verified"
  | "mismatch"
  | "missing"
  | "sizeMismatch"
  | "noChecksumRecorded";

export interface MhlEntryResult {
  relativePath: string;
  status: MhlEntryStatus;
}

export interface RepairCandidate {
  root: string;
  path: string;
  checksum: string;
  algorithm: string;
}

export interface RepairPlan {
  mhlPath: string;
  relativePath: string;
  expectedChecksum: string;
  algorithm: string;
  candidates: RepairCandidate[];
}

export interface MhlVerifyReport {
  mhlPath: string;
  results: MhlEntryResult[];
}
