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
import type { GroupJobAddedEventPayload, TransferGroupMode } from "../types/transferGroup";
import type { MediaScanCompletePayload, MediaScanItemPayload } from "../types/media";
import type { OrganizeSettings } from "../types/organize";

export function listDisks(): Promise<DiskInfo[]> {
  return invoke<DiskInfo[]>("list_disks");
}

export function onDisksChanged(
  callback: (disks: DiskInfo[]) => void,
): Promise<UnlistenFn> {
  return listen<DiskInfo[]>("disks-changed", (event) => callback(event.payload));
}

export function startTransferGroup(
  source: string,
  destinations: string[],
  mode: TransferGroupMode,
  verificationMode: VerificationMode,
  checksumAlgorithm: ChecksumAlgorithm,
  sourceName: string,
  organize: OrganizeSettings,
): Promise<string> {
  return invoke<string>("start_transfer_group", {
    source,
    destinations,
    mode,
    verificationMode,
    checksumAlgorithm,
    sourceName,
    organize,
  });
}

export function cancelCopy(jobId: string): Promise<boolean> {
  return invoke<boolean>("cancel_copy", { jobId });
}

export function onTransferGroupJobAdded(
  callback: (payload: GroupJobAddedEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<GroupJobAddedEventPayload>("transfer-group-job-added", (event) =>
    callback(event.payload),
  );
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

export function startMediaScan(folder: string): Promise<string> {
  return invoke<string>("start_media_scan", { folder });
}

export function onMediaScanItem(
  callback: (payload: MediaScanItemPayload) => void,
): Promise<UnlistenFn> {
  return listen<MediaScanItemPayload>("media-scan-item", (event) => callback(event.payload));
}

export function onMediaScanComplete(
  callback: (payload: MediaScanCompletePayload) => void,
): Promise<UnlistenFn> {
  return listen<MediaScanCompletePayload>("media-scan-complete", (event) =>
    callback(event.payload),
  );
}
