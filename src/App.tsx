import { useEffect } from "react";
import {
  cancelCopy,
  getVolumeSignature,
  listDisks,
  onBrokenMediaDetected,
  onCopyCancelled,
  onCopyComplete,
  onCopyProgress,
  onCopyScan,
  onDisksChanged,
  onMediaScanComplete,
  onMediaScanItem,
  onTransferGroupJobAdded,
  resolveBrokenMedia,
  startMediaScan,
  startTransferGroup,
} from "./lib/tauri";
import { useDisksStore } from "./state/disksStore";
import { useTransfersStore } from "./state/transfersStore";
import { useSettingsStore } from "./state/settingsStore";
import { useGroupsStore } from "./state/groupsStore";
import { useMediaStore } from "./state/mediaStore";
import { useOrganizeStore } from "./state/organizeStore";
import { useTransferLogStore } from "./state/transferLogStore";
import { DisksPanel } from "./components/DisksPanel";
import { EndpointList } from "./components/EndpointList";
import { TransfersPanel } from "./components/TransfersPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { GroupComposer } from "./components/GroupComposer";
import { MediaBrowser } from "./components/MediaBrowser";
import { OrganizePanel } from "./components/OrganizePanel";
import { PresetsPanel } from "./components/PresetsPanel";
import { ReportsPanel } from "./components/ReportsPanel";
import { MhlVerifyPanel } from "./components/MhlVerifyPanel";
import { TransferLogPanel } from "./components/TransferLogPanel";
import { pathLabel, formatBytes } from "./lib/format";
import { notifyTransfer } from "./lib/notify";
import type { DiskInfo, Endpoint } from "./types/disk";
import type { TransferJob } from "./types/job";
import type { GroupJobAddedEventPayload, TransferGroup, TransferGroupMode } from "./types/transferGroup";
import type { MediaScanCompletePayload, MediaScanItemPayload } from "./types/media";
import "./App.css";

function endpointLabel(endpoint: Endpoint, disks: DiskInfo[]) {
  const disk = disks.find((d) => d.id === endpoint.diskId);
  return endpoint.label || disk?.name || endpoint.diskId;
}

function App() {
  const disks = useDisksStore((s) => s.disks);
  const sources = useDisksStore((s) => s.sources);
  const destinations = useDisksStore((s) => s.destinations);
  const setDisks = useDisksStore((s) => s.setDisks);
  const removeSource = useDisksStore((s) => s.removeSource);
  const removeDestination = useDisksStore((s) => s.removeDestination);
  const setSourceLabel = useDisksStore((s) => s.setSourceLabel);
  const setDestinationLabel = useDisksStore((s) => s.setDestinationLabel);
  const setSourcePath = useDisksStore((s) => s.setSourcePath);
  const setDestinationPath = useDisksStore((s) => s.setDestinationPath);

  const jobs = useTransfersStore((s) => s.jobs);
  const addJob = useTransfersStore((s) => s.addJob);
  const applyScan = useTransfersStore((s) => s.applyScan);
  const applyProgress = useTransfersStore((s) => s.applyProgress);
  const applyComplete = useTransfersStore((s) => s.applyComplete);
  const applyCancelled = useTransfersStore((s) => s.applyCancelled);
  const applyBrokenMedia = useTransfersStore((s) => s.applyBrokenMedia);
  const clearBrokenMediaAlert = useTransfersStore((s) => s.clearBrokenMediaAlert);
  const setResumeBlockedReason = useTransfersStore((s) => s.setResumeBlockedReason);

  const groups = useGroupsStore((s) => s.groups);
  const setGroupMeta = useGroupsStore((s) => s.setGroupMeta);
  const addJobToGroup = useGroupsStore((s) => s.addJobToGroup);

  const mediaScans = useMediaStore((s) => s.scans);
  const activeScanId = useMediaStore((s) => s.activeScanId);
  const startMediaScanState = useMediaStore((s) => s.startScan);
  const addMediaEntry = useMediaStore((s) => s.addEntry);
  const completeMediaScan = useMediaStore((s) => s.completeScan);
  const setActiveScan = useMediaStore((s) => s.setActiveScan);

  const verificationMode = useSettingsStore((s) => s.verificationMode);
  const checksumAlgorithm = useSettingsStore((s) => s.checksumAlgorithm);
  const desktopNotifications = useSettingsStore((s) => s.desktopNotifications);

  const refreshTransferLogs = useTransferLogStore((s) => s.refresh);

  const organizeRenameTemplate = useOrganizeStore((s) => s.renameTemplate);
  const organizeFolderTemplate = useOrganizeStore((s) => s.folderTemplate);
  const organizeCounterPadding = useOrganizeStore((s) => s.counterPadding);
  const organizeSelectiveCopy = useOrganizeStore((s) => s.selectiveCopy);
  const organizeBundleIgnore = useOrganizeStore((s) => s.bundleIgnore);
  const organizeIgnoreEmptyFolders = useOrganizeStore((s) => s.ignoreEmptyFolders);
  const organizeFlatten = useOrganizeStore((s) => s.flatten);
  const organizeContentDateExcludedExtensions = useOrganizeStore(
    (s) => s.contentDateExcludedExtensions,
  );
  const organizeDateOverride = useOrganizeStore((s) => s.dateOverride);
  const organizeElements = useOrganizeStore((s) => s.elements);
  const organizeAutoLabel = useOrganizeStore((s) => s.autoLabel);
  const organizeSkipModificationDateCheck = useOrganizeStore((s) => s.skipModificationDateCheck);
  const organizeAutoContinueOnBrokenMedia = useOrganizeStore((s) => s.autoContinueOnBrokenMedia);
  // Assembled fresh each render from the primitive selections above --
  // deliberately not itself a selector return value, since Zustand's
  // useSyncExternalStore compares each render's snapshot by reference and a
  // freshly-built object there never stabilizes, causing an infinite loop.
  const organize = {
    renameTemplate: organizeRenameTemplate,
    folderTemplate: organizeFolderTemplate,
    counterPadding: organizeCounterPadding,
    selectiveCopy: organizeSelectiveCopy,
    bundleIgnore: organizeBundleIgnore,
    ignoreEmptyFolders: organizeIgnoreEmptyFolders,
    flatten: organizeFlatten,
    contentDateExcludedExtensions: organizeContentDateExcludedExtensions,
    dateOverride: organizeDateOverride,
    elements: organizeElements,
    autoLabel: organizeAutoLabel,
    skipModificationDateCheck: organizeSkipModificationDateCheck,
    autoContinueOnBrokenMedia: organizeAutoContinueOnBrokenMedia,
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listDisks().then(setDisks).catch(console.error);
    onDisksChanged(setDisks).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, [setDisks]);

  useEffect(() => {
    function handleGroupJobAdded(payload: GroupJobAddedEventPayload) {
      addJobToGroup(payload.groupId, payload.jobId);
      addJob({
        id: payload.jobId,
        groupId: payload.groupId,
        hop: payload.hop,
        sourceLabel: pathLabel(payload.source),
        destinationLabel: pathLabel(payload.destination),
        sourcePath: payload.source,
        destinationPath: payload.destination,
        verificationMode,
        checksumAlgorithm,
        status: "queued",
        currentFile: "",
        bytesCopied: 0,
        totalBytes: 0,
        filesCopied: 0,
        totalFiles: 0,
        bytesPerSec: 0,
        failedFiles: [],
        verifiedFiles: [],
        skippedFiles: [],
        renamedFiles: [],
        deletedSourceFiles: [],
        moveDeleteFailed: [],
        brokenMediaFiles: [],
        pendingBrokenMedia: null,
        sourceVolumeSignature: payload.sourceVolumeSignature,
        resumeBlockedReason: null,
      });
    }

    function handleMediaScanItem(payload: MediaScanItemPayload) {
      addMediaEntry(payload.scanId, payload.entry);
    }

    function handleMediaScanComplete(payload: MediaScanCompletePayload) {
      completeMediaScan(payload.scanId, payload.total);
    }

    function handleCopyComplete(payload: Parameters<typeof applyComplete>[0]) {
      applyComplete(payload);
      // The backend writes the transfer log/MHL right after emitting this
      // event, on the same thread -- usually already on disk by the time
      // this handler runs, but the panel also offers a manual refresh in
      // case this call ever races the write.
      refreshTransferLogs();

      if (desktopNotifications) {
        const job = useTransfersStore.getState().jobs[payload.jobId];
        const label = job ? `${job.sourceLabel} → ${job.destinationLabel}` : payload.jobId;
        if (payload.failedFiles.length > 0) {
          notifyTransfer(
            "Transfer finished with errors",
            `${label}: ${payload.failedFiles.length} file(s) failed`,
          );
        } else {
          notifyTransfer(
            "Transfer complete",
            `${label}: ${payload.filesCopied} file(s), ${formatBytes(payload.bytesCopied)}`,
          );
        }
      }
    }

    function handleCopyCancelled(payload: Parameters<typeof applyCancelled>[0]) {
      applyCancelled(payload);
      if (desktopNotifications) {
        const job = useTransfersStore.getState().jobs[payload.jobId];
        const label = job ? `${job.sourceLabel} → ${job.destinationLabel}` : payload.jobId;
        notifyTransfer("Transfer cancelled", label);
      }
    }

    const unlistenPromises = [
      onCopyScan(applyScan),
      onCopyProgress(applyProgress),
      onCopyComplete(handleCopyComplete),
      onCopyCancelled(handleCopyCancelled),
      onTransferGroupJobAdded(handleGroupJobAdded),
      onMediaScanItem(handleMediaScanItem),
      onMediaScanComplete(handleMediaScanComplete),
      onBrokenMediaDetected(applyBrokenMedia),
    ];

    return () => {
      unlistenPromises.forEach((p) => p.then((fn) => fn()));
    };
  }, [
    applyScan,
    applyProgress,
    applyComplete,
    applyCancelled,
    applyBrokenMedia,
    addJob,
    addJobToGroup,
    addMediaEntry,
    completeMediaScan,
    verificationMode,
    checksumAlgorithm,
    refreshTransferLogs,
    desktopNotifications,
  ]);

  async function handleStartGroup(
    source: Endpoint,
    destinationEndpoints: Endpoint[],
    mode: TransferGroupMode,
    moveAfterTransfer: boolean,
  ) {
    const groupId = await startTransferGroup(
      source.path,
      destinationEndpoints.map((d) => d.path),
      mode,
      verificationMode,
      checksumAlgorithm,
      endpointLabel(source, disks),
      organize,
      moveAfterTransfer,
    );
    setGroupMeta(
      groupId,
      mode,
      endpointLabel(source, disks),
      destinationEndpoints.map((d) => endpointLabel(d, disks)),
    );
  }

  function handleCancelJob(jobId: string) {
    cancelCopy(jobId).catch(console.error);
  }

  function handleCancelGroup(group: TransferGroup) {
    group.jobIds.forEach((jobId) => handleCancelJob(jobId));
  }

  function handleResolveBrokenMedia(jobId: string, proceed: boolean) {
    clearBrokenMediaAlert(jobId);
    resolveBrokenMedia(jobId, proceed).catch(console.error);
  }

  // Resume = a fresh Parallel transfer over the exact same source →
  // destination the job already used, with the same Verification/Checksum
  // settings it was started with. Duplicate Detection (and, when the
  // algorithm matches, MHL Awareness) then naturally skips whatever that job
  // already got through, so only what's missing gets copied. Move is
  // deliberately never re-enabled here -- it shouldn't silently start
  // deleting sources as a side effect of a Resume click; redo it from Build
  // Transfer if that's really what's wanted.
  //
  // Source Index check: confirms the disk currently at `job.sourcePath` is
  // still the same physical volume the job originally read from, not a
  // different card that happens to have mounted at the same drive letter
  // since. Only blocks on a *confirmed* mismatch -- if either signature is
  // unavailable (unsupported platform, or a path that isn't a recognizable
  // local volume), there's nothing to contradict, so it fails open rather
  // than blocking Resume on ambiguity.
  async function handleResumeJob(job: TransferJob) {
    const currentSignature = await getVolumeSignature(job.sourcePath).catch(() => null);
    if (
      job.sourceVolumeSignature &&
      currentSignature &&
      currentSignature !== job.sourceVolumeSignature
    ) {
      setResumeBlockedReason(
        job.id,
        "Source disk has changed since this transfer started -- reconnect the original source before resuming.",
      );
      return;
    }
    setResumeBlockedReason(job.id, null);

    const groupId = await startTransferGroup(
      job.sourcePath,
      [job.destinationPath],
      "parallel",
      job.verificationMode,
      job.checksumAlgorithm,
      job.sourceLabel,
      organize,
      false,
    );
    setGroupMeta(groupId, "parallel", job.sourceLabel, [job.destinationLabel]);
  }

  async function handleBrowse(path: string) {
    const scanId = await startMediaScan(path);
    startMediaScanState(scanId, path);
  }

  const activeScan = activeScanId ? mediaScans[activeScanId] : undefined;

  return (
    <main className="min-h-screen bg-neutral-950 p-6 text-neutral-100">
      <h1 className="mb-6 text-xl font-semibold">OffloadKit</h1>
      <div className="grid grid-cols-1 gap-6 md:grid-cols-5">
        <DisksPanel />
        <EndpointList
          title="Sources"
          endpoints={sources}
          disks={disks}
          onRemove={removeSource}
          onLabelChange={setSourceLabel}
          onPathChange={setSourcePath}
          onBrowse={handleBrowse}
        />
        <EndpointList
          title="Destinations"
          endpoints={destinations}
          disks={disks}
          onRemove={removeDestination}
          onLabelChange={setDestinationLabel}
          onPathChange={setDestinationPath}
        />
        <div className="flex flex-col gap-6">
          <SettingsPanel />
          <OrganizePanel />
          <PresetsPanel />
          <GroupComposer
            sources={sources}
            destinations={destinations}
            disks={disks}
            onStart={handleStartGroup}
          />
        </div>
        <TransfersPanel
          groups={Object.values(groups)}
          jobs={jobs}
          onCancelJob={handleCancelJob}
          onCancelGroup={handleCancelGroup}
          onResumeJob={handleResumeJob}
          onResolveBrokenMedia={handleResolveBrokenMedia}
        />
        <TransferLogPanel onViewClips={handleBrowse} />
        <ReportsPanel />
        <MhlVerifyPanel />
        {activeScan && (
          <MediaBrowser
            folder={activeScan.folder}
            entries={activeScan.entries}
            status={activeScan.status}
            total={activeScan.total}
            onClose={() => setActiveScan(null)}
          />
        )}
      </div>
    </main>
  );
}

export default App;
