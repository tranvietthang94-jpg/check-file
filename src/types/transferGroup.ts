export type TransferGroupMode = "parallel" | "cascade";

export interface TransferGroup {
  id: string;
  mode: TransferGroupMode;
  sourceLabel: string;
  destinationLabels: string[];
  expectedJobCount: number;
  jobIds: string[];
}

export interface GroupJobAddedEventPayload {
  groupId: string;
  jobId: string;
  source: string;
  selectedPaths?: string[];
  destination: string;
  hop: 1 | 2;
  sourceVolumeSignature: string | null;
}
