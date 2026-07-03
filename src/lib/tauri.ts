import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DiskInfo } from "../types/disk";
import type {
  CancelledEventPayload,
  ChecksumAlgorithm,
  CompleteEventPayload,
  ProgressEventPayload,
  ScanEventPayload,
  VerificationMode,
} from "../types/job";

export function listDisks(): Promise<DiskInfo[]> {
  return invoke<DiskInfo[]>("list_disks");
}

export function onDisksChanged(
  callback: (disks: DiskInfo[]) => void,
): Promise<UnlistenFn> {
  return listen<DiskInfo[]>("disks-changed", (event) => callback(event.payload));
}

export function startCopy(
  source: string,
  destination: string,
  verificationMode: VerificationMode,
  checksumAlgorithm: ChecksumAlgorithm,
): Promise<string> {
  return invoke<string>("start_copy", {
    source,
    destination,
    verificationMode,
    checksumAlgorithm,
  });
}

export function cancelCopy(jobId: string): Promise<boolean> {
  return invoke<boolean>("cancel_copy", { jobId });
}

export function onCopyScan(
  callback: (payload: ScanEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<ScanEventPayload>("copy-scan", (event) => callback(event.payload));
}

export function onCopyProgress(
  callback: (payload: ProgressEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<ProgressEventPayload>("copy-progress", (event) => callback(event.payload));
}

export function onCopyComplete(
  callback: (payload: CompleteEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<CompleteEventPayload>("copy-complete", (event) => callback(event.payload));
}

export function onCopyCancelled(
  callback: (payload: CancelledEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<CancelledEventPayload>("copy-cancelled", (event) => callback(event.payload));
}
