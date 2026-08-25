import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DiskInfo } from "../types/disk";
import type {
  BrokenMediaEventPayload,
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
import type { Preset } from "../types/preset";
import type { ReportRequest } from "../types/report";
import type { MhlVerifyReport, RepairPlan } from "../types/mhl";
import type { TransferLogEntry } from "../types/transferLog";
import type { QueueMode } from "../types/queue";

export type ExplorerAction = "setSource" | "setDestination";

export interface ExplorerRequest {
  id: string;
  action: ExplorerAction;
  paths: string[];
}

export interface ExplorerErrorPayload {
  id: string;
  message: string;
}

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function listenForExplorerEvent<T>(
  eventName: string,
  browserTestEventName: string,
  callback: (payload: T) => void,
): Promise<UnlistenFn> {
  if (hasTauriRuntime()) {
    return listen<T>(eventName, (event) => callback(event.payload));
  }

  const handler = ((event: CustomEvent<T>) => callback(event.detail)) as EventListener;
  window.addEventListener(browserTestEventName, handler);
  return Promise.resolve(() => window.removeEventListener(browserTestEventName, handler));
}

export function onExplorerRequest(
  callback: (request: ExplorerRequest) => void,
): Promise<UnlistenFn> {
  return listenForExplorerEvent("explorer://request", "offloadkit-test:explorer-request", callback);
}

export function onExplorerError(
  callback: (error: ExplorerErrorPayload) => void,
): Promise<UnlistenFn> {
  return listenForExplorerEvent("explorer://error", "offloadkit-test:explorer-error", callback);
}

export function explorerFrontendReady(): Promise<void> {
  if (!hasTauriRuntime()) return Promise.resolve();
  return invoke<void>("explorer_frontend_ready");
}

export function acknowledgeExplorerRequest(requestId: string): Promise<void> {
  if (!hasTauriRuntime()) {
    window.dispatchEvent(new CustomEvent("offloadkit-test:explorer-ack", { detail: requestId }));
    return Promise.resolve();
  }
  return invoke<void>("acknowledge_explorer_request", { requestId });
}

export function listDisks(): Promise<DiskInfo[]> {
  return invoke<DiskInfo[]>("list_disks");
}

/** The stable volume identifier backing `path` right now, or `null` if it can't be determined. */
export function getVolumeSignature(path: string): Promise<string | null> {
  return invoke<string | null>("get_volume_signature", { path });
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
  moveAfterTransfer: boolean,
  moveSameVolume: boolean,
  legacyChecksumAlgorithm: ChecksumAlgorithm | null,
  saveLogToDestination: boolean,
  createPerFileMhl: boolean,
): Promise<string> {
  return invoke<string>("start_transfer_group", {
    source,
    destinations,
    mode,
    verificationMode,
    checksumAlgorithm,
    sourceName,
    organize,
    moveAfterTransfer,
    moveSameVolume,
    legacyChecksumAlgorithm,
    saveLogToDestination,
    createPerFileMhl,
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

export function onBrokenMediaDetected(
  callback: (payload: BrokenMediaEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<BrokenMediaEventPayload>("copy-broken-media", (event) => callback(event.payload));
}

/** Resolves a pending Broken Media alert: `proceed = true` continues the copy, `false` aborts it. */
export function resolveBrokenMedia(jobId: string, proceed: boolean): Promise<void> {
  return invoke<void>("resolve_broken_media", { jobId, proceed });
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

export function savePreset(preset: Preset): Promise<void> {
  return invoke<void>("save_preset", { preset });
}

export function listPresets(): Promise<Preset[]> {
  return invoke<Preset[]>("list_presets");
}

export function deletePreset(name: string): Promise<void> {
  return invoke<void>("delete_preset", { name });
}

export function listTransferLogs(): Promise<TransferLogEntry[]> {
  return invoke<TransferLogEntry[]>("list_transfer_logs");
}

export function ejectDisk(mountPoint: string): Promise<void> {
  return invoke<void>("eject_disk", { mountPoint });
}

export function renameDisk(mountPoint: string, label: string): Promise<void> {
  return invoke<void>("rename_disk", { mountPoint, label });
}

export function setPreventSleepEnabled(enabled: boolean): Promise<void> {
  return invoke<void>("set_prevent_sleep", { enabled });
}

export function setQueueMode(mode: QueueMode): Promise<void> {
  return invoke<void>("set_queue_mode", { mode });
}

/** Returns the absolute path to the generated report HTML file. */
export function generateReport(request: ReportRequest): Promise<string> {
  return invoke<string>("generate_report", { request });
}

/** Verifies one .mhl file against the real files on disk, without a transfer. */
export function verifyMhl(path: string): Promise<MhlVerifyReport> {
  return invoke<MhlVerifyReport>("verify_mhl", { path });
}

export function planMhlRepair(
  mhlPath: string,
  relativePath: string,
  candidateRoots: string[],
): Promise<RepairPlan> {
  return invoke<RepairPlan>("plan_mhl_repair", { mhlPath, relativePath, candidateRoots });
}

export function repairMhlEntry(
  mhlPath: string,
  relativePath: string,
  sourceRoot: string,
  approved: boolean,
): Promise<MhlVerifyReport> {
  return invoke<MhlVerifyReport>("repair_mhl_entry", {
    mhlPath,
    relativePath,
    sourceRoot,
    approved,
  });
}

/** Verifies every .mhl file found directly inside `folder`. */
export function verifyMhlsInFolder(folder: string): Promise<MhlVerifyReport[]> {
  return invoke<MhlVerifyReport[]>("verify_mhls_in_folder", { folder });
}
