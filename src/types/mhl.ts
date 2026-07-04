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

export interface MhlVerifyReport {
  mhlPath: string;
  results: MhlEntryResult[];
}
