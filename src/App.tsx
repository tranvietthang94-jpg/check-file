import { useEffect, useRef, useState } from "react";
import {
  cancelCopy,
  getVolumeSignature,
  ejectDisk,
  listDisks,
  onBrokenMediaDetected,
  onCopyCancelled,
  onCopyComplete,
  onCopyProgress,
  onCopyScan,
  onDisksChanged,
  onExplorerError,
  onExplorerRequest,
  onMediaScanComplete,
  onMediaScanItem,
  onTransferGroupJobAdded,
  resolveBrokenMedia,
  startMediaScan,
  startTransferGroup,
  explorerFrontendReady,
} from "./lib/tauri";
import { useDisksStore } from "./state/disksStore";
import { useTransfersStore } from "./state/transfersStore";
import { useSettingsStore } from "./state/settingsStore";
import { useGroupsStore } from "./state/groupsStore";
import { useMediaStore } from "./state/mediaStore";
import { useOrganizeStore } from "./state/organizeStore";
import { useTransferLogStore } from "./state/transferLogStore";
import { useExplorerActionStore } from "./state/explorerActionStore";
import { DisksPanel } from "./components/DisksPanel";
import { ElementsReviewPanel } from "./components/ElementsReviewPanel";
import { EndpointList } from "./components/EndpointList";
import { TransfersPanel } from "./components/TransfersPanel";
import { AddTransfersBar } from "./components/AddTransfersBar";
import { MediaBrowser } from "./components/MediaBrowser";
import { AboutDialog } from "./components/AboutDialog";
import { StartTransferFailureDialog } from "./components/StartTransferFailureDialog";
import { PreferencesModal } from "./components/PreferencesModal";
import { ReportsPanel } from "./components/ReportsPanel";
import { MhlVerifyPanel } from "./components/MhlVerifyPanel";
import { TransferLogPanel } from "./components/TransferLogPanel";
import { Button } from "./components/ui/Button";
import { IconButton } from "./components/ui/IconButton";
import { Modal } from "./components/ui/Modal";
import { DiskContextMenu, type DiskContextMenuItem } from "./components/DiskContextMenu";
import { ArrowLeftRight, HardDrive, History, Info, Menu, FileText, Settings } from "./components/icons";
import { pathLabel, formatBytes } from "./lib/format";
import { notifyTransfer } from "./lib/notify";
import type { DiskInfo, Endpoint } from "./types/disk";
import type { TransferJob } from "./types/job";
import type { GroupJobAddedEventPayload, TransferGroup, TransferGroupMode } from "./types/transferGroup";
import type { MediaScanCompletePayload, MediaScanItemPayload } from "./types/media";
import {
  autoEjectGroups,
  autoEjectJobs,
  autoEjectPendingGroups,
  referenceDestinations,
  referenceDisks,
  referenceFixture,
  referenceGroups,
  referenceJobs,
  referenceSources,
} from "./dev/referenceFixtures";
import "./App.css";

function endpointLabel(endpoint: Endpoint, disks: DiskInfo[]) {
  const disk = disks.find((d) => d.id === endpoint.diskId);
  return endpoint.label || (endpoint.id === endpoint.diskId ? disk?.name : undefined) || pathLabel(endpoint.path);
}

function wildcardMatches(value: string, pattern: string): boolean {
  const escaped = pattern.trim().replace(/[.+?^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*");
  return escaped !== "" && new RegExp(`^${escaped}$`, "i").test(value);
}

function App() {
  const [view, setView] = useState<"disks" | "transfers">("disks");
  const [preferencesOpen, setPreferencesOpen] = useState(false);
  const [preferencesTab, setPreferencesTab] = useState<"general" | "disks" | "organize" | "transfers">("general");
  const [transferLogOpen, setTransferLogOpen] = useState(false);
  const [reportsOpen, setReportsOpen] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [startTransferError, setStartTransferError] = useState<string | null>(null);
  const [endpointStoresReady, setEndpointStoresReady] = useState(false);
  const autoEjectedGroups = useRef(new Set<string>());
  // Matches OffShoot's own hamburger menu -- Transfer Logs/Reports/Settings
  // open as their own windows from here rather than living permanently
  // stacked in the Transfers view.
  const [appMenu, setAppMenu] = useState<{ x: number; y: number } | null>(null);

  const disks = useDisksStore((s) => s.disks);
  const sources = useDisksStore((s) => s.sources);
  const destinations = useDisksStore((s) => s.destinations);
  const setDisks = useDisksStore((s) => s.setDisks);
  const setEndpoints = useDisksStore((s) => s.setEndpoints);
  const addSource = useDisksStore((s) => s.addSource);
  const addDestination = useDisksStore((s) => s.addDestination);
  const removeSource = useDisksStore((s) => s.removeSource);
  const removeDestination = useDisksStore((s) => s.removeDestination);
  const setSourceLabel = useDisksStore((s) => s.setSourceLabel);
  const setDestinationLabel = useDisksStore((s) => s.setDestinationLabel);
  const setSourcePath = useDisksStore((s) => s.setSourcePath);
  const setDestinationPath = useDisksStore((s) => s.setDestinationPath);
  const reorderDestinations = useDisksStore((s) => s.reorderDestinations);
  const clearSourcesAndDestinations = useDisksStore((s) => s.clearSourcesAndDestinations);

  const explorerListenersReady = useExplorerActionStore((s) => s.listenersReady);
  const explorerFeedback = useExplorerActionStore((s) => s.feedback);
  const receiveExplorerRequest = useExplorerActionStore((s) => s.receiveRequest);
  const receiveExplorerError = useExplorerActionStore((s) => s.receiveError);
  const markExplorerListenersReady = useExplorerActionStore((s) => s.markListenersReady);
  const markExplorerReady = useExplorerActionStore((s) => s.markReady);

  const jobs = useTransfersStore((s) => s.jobs);
  const addJob = useTransfersStore((s) => s.addJob);
  const setJobs = useTransfersStore((s) => s.setJobs);
  const applyScan = useTransfersStore((s) => s.applyScan);
  const applyProgress = useTransfersStore((s) => s.applyProgress);
  const applyComplete = useTransfersStore((s) => s.applyComplete);
  const applyCancelled = useTransfersStore((s) => s.applyCancelled);
  const applyBrokenMedia = useTransfersStore((s) => s.applyBrokenMedia);
  const clearBrokenMediaAlert = useTransfersStore((s) => s.clearBrokenMediaAlert);
  const setResumeBlockedReason = useTransfersStore((s) => s.setResumeBlockedReason);

  const groups = useGroupsStore((s) => s.groups);
  const setGroupMeta = useGroupsStore((s) => s.setGroupMeta);
  const setGroups = useGroupsStore((s) => s.setGroups);
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
  const moveSameVolume = useSettingsStore((s) => s.moveSameVolume);
  const legacyChecksumEnabled = useSettingsStore((s) => s.legacyChecksumEnabled);
  const legacyChecksumAlgorithm = useSettingsStore((s) => s.legacyChecksumAlgorithm);
  const effectiveLegacyChecksumAlgorithm = legacyChecksumEnabled ? legacyChecksumAlgorithm : null;
  const saveLogToDestination = useSettingsStore((s) => s.saveLogToDestination);
  const createPerFileMhl = useSettingsStore((s) => s.createPerFileMhl);
  const autoSourceEnabled = useSettingsStore((s) => s.autoSourceEnabled);
  const autoSourcePattern = useSettingsStore((s) => s.autoSourcePattern);
  const autoEjectEnabled = useSettingsStore((s) => s.autoEjectEnabled);

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
    function handleShortcut(event: KeyboardEvent) {
      if (!event.ctrlKey || event.altKey || event.metaKey) return;
      const key = event.key.toLowerCase();
      if (key === "d") setView("disks");
      else if (key === "t") setView("transfers");
      else if (key === "l") setTransferLogOpen(true);
      else if (key === ",") {
        setPreferencesTab("general");
        setPreferencesOpen(true);
      } else if (key === "1" || key === "2" || key === "3") {
        setPreferencesTab(key === "1" ? "general" : key === "2" ? "transfers" : "organize");
        setPreferencesOpen(true);
      } else return;
      event.preventDefault();
    }
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisteners: Array<() => void> = [];
    Promise.all([
      onExplorerRequest((request) => void receiveExplorerRequest(request)),
      onExplorerError(receiveExplorerError),
    ])
      .then((registered) => {
        if (disposed) registered.forEach((unlisten) => unlisten());
        else {
          unlisteners = registered;
          markExplorerListenersReady();
        }
      })
      .catch((error) =>
        receiveExplorerError({ id: "explorer-listener-registration", message: String(error) }),
      );
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [markExplorerListenersReady, receiveExplorerError, receiveExplorerRequest]);

  useEffect(() => {
    if (!endpointStoresReady || !explorerListenersReady) return;
    let started = false;
    const start = () => {
      if (started) return;
      if (!useSettingsStore.persist.hasHydrated() || !useOrganizeStore.persist.hasHydrated()) {
        return;
      }
      started = true;
      markExplorerReady()
        .then(explorerFrontendReady)
        .catch((error) =>
          receiveExplorerError({ id: "explorer-frontend-ready", message: String(error) }),
        );
    };
    const unlistenSettings = useSettingsStore.persist.onFinishHydration(start);
    const unlistenOrganize = useOrganizeStore.persist.onFinishHydration(start);
    start();
    return () => {
      unlistenSettings();
      unlistenOrganize();
    };
  }, [
    endpointStoresReady,
    explorerListenersReady,
    markExplorerReady,
    receiveExplorerError,
  ]);

  useEffect(() => {
    const fixture = referenceFixture();
    if (fixture) {
      setDisks(referenceDisks);
      setEndpoints(referenceSources, referenceDestinations);
      if (fixture === "transfers") {
        setJobs(referenceJobs);
        setGroups(referenceGroups);
        setView("transfers");
      } else if (fixture === "autoEject" || fixture === "autoEjectPending") {
        setJobs(autoEjectJobs);
        setGroups(fixture === "autoEject" ? autoEjectGroups : autoEjectPendingGroups);
        setView("transfers");
      }
      setEndpointStoresReady(true);
      return;
    }

    let unlisten: (() => void) | undefined;

    listDisks().then(setDisks).catch(console.error).finally(() => setEndpointStoresReady(true));
    onDisksChanged(setDisks).then((fn) => {
      unlisten = fn;
    });

    return () => unlisten?.();
  }, [setDisks, setEndpoints, setGroups, setJobs]);

  useEffect(() => {
    if (!autoSourceEnabled) return;
    for (const disk of disks) {
      if (disk.isRemovable && wildcardMatches(disk.name, autoSourcePattern)) addSource(disk.id);
    }
  }, [addSource, autoSourceEnabled, autoSourcePattern, disks]);

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
        selectedPaths: payload.selectedPaths,
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
        missingFiles: [],
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
            "Lượt truyền hoàn tất nhưng có lỗi",
            `${label}: ${payload.failedFiles.length} tệp thất bại`,
          );
        } else if (payload.missingFiles.length > 0) {
          notifyTransfer(
            "Phát hiện tệp bị thiếu ở đích",
            `${label}: ${payload.missingFiles.length} tệp không thấy trên đích sau khi truyền xong`,
          );
        } else {
          notifyTransfer(
            "Lượt truyền hoàn tất",
            `${label}: ${payload.filesCopied} tệp, ${formatBytes(payload.bytesCopied)}`,
          );
        }
      }
    }

    function handleCopyCancelled(payload: Parameters<typeof applyCancelled>[0]) {
      applyCancelled(payload);
      if (desktopNotifications) {
        const job = useTransfersStore.getState().jobs[payload.jobId];
        const label = job ? `${job.sourceLabel} → ${job.destinationLabel}` : payload.jobId;
        notifyTransfer("Đã hủy lượt truyền", label);
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

  useEffect(() => {
    if (!autoEjectEnabled) return;
    for (const group of Object.values(groups)) {
      if (
        autoEjectedGroups.current.has(group.id) ||
        group.expectedJobCount === 0 ||
        group.jobIds.length !== group.expectedJobCount
      )
        continue;
      const groupJobs = group.jobIds.map((id) => jobs[id]);
      if (groupJobs.some((job) => !job || job.status !== "complete")) continue;
      if (
        groupJobs.some(
          (job) =>
            job.failedFiles.length > 0 ||
            job.missingFiles.length > 0 ||
            job.moveDeleteFailed.length > 0 ||
            job.brokenMediaFiles.length > 0,
        )
      )
        continue;
      const sourcePath = groupJobs[0].sourcePath;
      const normalizedSource = sourcePath.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
      const disk = disks.find((candidate) => {
        if (!candidate.isRemovable) return false;
        const mount = candidate.mountPoint.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
        return normalizedSource === mount || normalizedSource.startsWith(`${mount}/`);
      });
      if (!disk) continue;
      autoEjectedGroups.current.add(group.id);
      const testEject = (window as Window & { __OFFLOADKIT_TEST_EJECT__?: (mount: string) => void })
        .__OFFLOADKIT_TEST_EJECT__;
      const eject = testEject ? Promise.resolve(testEject(disk.mountPoint)) : ejectDisk(disk.mountPoint);
      eject.catch((error) => {
        autoEjectedGroups.current.delete(group.id);
        console.error(error);
      });
    }
  }, [autoEjectEnabled, disks, groups, jobs]);

  async function handleStartGroup(
    source: Endpoint,
    destinationEndpoints: Endpoint[],
    mode: TransferGroupMode,
    moveAfterTransfer: boolean,
  ) {
    if (!source.path.trim()) {
      throw new Error(`Nguồn “${endpointLabel(source, disks)}” chưa có đường dẫn thư mục.`);
    }
    const missingDestination = destinationEndpoints.find((destination) => !destination.path.trim());
    if (missingDestination) {
      throw new Error(`Đích “${endpointLabel(missingDestination, disks)}” chưa có đường dẫn thư mục.`);
    }
    const normalizeTransferPath = (path: string) =>
      path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
    const sourceRoots = (source.selectedPaths?.length ? source.selectedPaths : [source.path])
      .map(normalizeTransferPath);
    if (destinationEndpoints.some((destination) => {
      const destinationRoot = normalizeTransferPath(destination.path);
      return sourceRoots.some(
        (sourceRoot) =>
          destinationRoot === sourceRoot ||
          destinationRoot.startsWith(`${sourceRoot}/`) ||
          sourceRoot.startsWith(`${destinationRoot}/`),
      );
    })) {
      throw new Error(`Nguồn “${endpointLabel(source, disks)}” và một Đích đang trùng hoặc chồng lấn đường dẫn.`);
    }
    const groupId = await startTransferGroup(
      source.path,
      source.selectedPaths ?? null,
      destinationEndpoints.map((d) => d.path),
      mode,
      verificationMode,
      checksumAlgorithm,
      endpointLabel(source, disks),
      organize,
      moveAfterTransfer,
      moveSameVolume,
      effectiveLegacyChecksumAlgorithm,
      saveLogToDestination,
      createPerFileMhl,
    );
    setGroupMeta(
      groupId,
      mode,
      endpointLabel(source, disks),
      destinationEndpoints.map((d) => endpointLabel(d, disks)),
    );
  }

  // AddTransfersBar's "Add N Transfers" -- matches OffShoot's own behavior
  // of building every Source's transfer(s) against the full Destinations
  // list in one click, instead of a per-click single-Source composer form.
  // Parallel starts one group per Source (each fanning out to every
  // Destination independently); Cascade starts one chain per Source through
  // every Destination in the order the Destinations list is currently in.
  async function handleAddTransfers(mode: TransferGroupMode, moveAfterTransfer: boolean) {
    try {
      for (const source of sources) {
        await handleStartGroup(source, destinations, mode, moveAfterTransfer);
      }
    } catch (error) {
      setStartTransferError(error instanceof Error ? error.message : String(error));
    }
  }

  function handleCancelJob(jobId: string) {
    cancelCopy(jobId).catch(console.error);
  }

  function handleCancelGroup(group: TransferGroup) {
    group.jobIds.forEach((jobId) => handleCancelJob(jobId));
  }

  async function handleResolveBrokenMedia(jobId: string, proceed: boolean) {
    try {
      await resolveBrokenMedia(jobId, proceed);
      clearBrokenMediaAlert(jobId);
    } catch (error) {
      console.error(error);
    }
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
        "Ổ đĩa nguồn đã thay đổi kể từ khi lượt truyền này bắt đầu -- kết nối lại đúng ổ đĩa nguồn trước khi tiếp tục.",
      );
      return;
    }
    setResumeBlockedReason(job.id, null);

    const groupId = await startTransferGroup(
      job.sourcePath,
      job.selectedPaths ?? null,
      [job.destinationPath],
      "parallel",
      job.verificationMode,
      job.checksumAlgorithm,
      job.sourceLabel,
      organize,
      false,
      false,
      effectiveLegacyChecksumAlgorithm,
      saveLogToDestination,
      createPerFileMhl,
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
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-xl font-semibold">OffloadKit</h1>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 rounded border border-neutral-800 p-1">
            {(
              [
                { id: "disks", label: "Ổ đĩa", icon: <HardDrive className="h-3.5 w-3.5" /> },
                {
                  id: "transfers",
                  label: "Truyền tải",
                  icon: <ArrowLeftRight className="h-3.5 w-3.5" />,
                },
              ] as const
            ).map((v) => (
              <Button
                key={v.id}
                variant="ghost"
                active={view === v.id}
                aria-pressed={view === v.id}
                icon={v.icon}
                onClick={() => setView(v.id)}
              >
                {v.label}
              </Button>
            ))}
          </div>
          <IconButton
            aria-label="Menu ứng dụng"
            title="Menu"
            icon={<Menu className="h-4 w-4" />}
            onClick={(e) => {
              const rect = e.currentTarget.getBoundingClientRect();
              // Anchored to the button's right edge (it sits at the far
              // right of the header) -- DiskContextMenu only positions from
              // its top-left corner, so the menu's ~200px width has to be
              // subtracted back from the button's right edge here, or it
              // would render mostly off-screen.
              setAppMenu({ x: Math.max(8, rect.right - 200), y: rect.bottom + 4 });
            }}
          />
        </div>
      </div>

      {explorerFeedback && (
        <div
          role={explorerFeedback.kind === "error" ? "alert" : "status"}
          data-testid="explorer-feedback"
          className={`mb-4 rounded border px-3 py-2 text-xs ${
            explorerFeedback.kind === "error"
              ? "border-red-800 bg-red-950/60 text-red-300"
              : "border-green-800 bg-green-950/60 text-green-300"
          }`}
        >
          {explorerFeedback.message}
        </div>
      )}

      {appMenu && (
        <DiskContextMenu
          x={appMenu.x}
          y={appMenu.y}
          onClose={() => setAppMenu(null)}
          items={
            [
              {
                label: "Nhật ký truyền tải",
                icon: <History className="h-3.5 w-3.5" />,
                onSelect: () => setTransferLogOpen(true),
              },
              {
                label: "Báo cáo",
                icon: <FileText className="h-3.5 w-3.5" />,
                onSelect: () => setReportsOpen(true),
              },
              {
                label: "Giới thiệu OffloadKit",
                icon: <Info className="h-3.5 w-3.5" />,
                onSelect: () => setAboutOpen(true),
              },
              {
                label: "Cài đặt",
                icon: <Settings className="h-3.5 w-3.5" />,
                onSelect: () => {
                  setPreferencesTab("general");
                  setPreferencesOpen(true);
                },
              },
            ] satisfies DiskContextMenuItem[]
          }
        />
      )}

      {view === "disks" ? (
        <div
          data-testid="disks-shell"
          className="grid grid-cols-[220px_minmax(0,1fr)_220px] gap-0 overflow-hidden"
        >
          <div data-testid="sources-column" className="min-w-0">
            <EndpointList
              title="Nguồn"
              endpoints={sources}
              disks={disks}
              onRemove={removeSource}
              onLabelChange={setSourceLabel}
              onPathChange={setSourcePath}
              onBrowse={handleBrowse}
              onDropDisk={addSource}
            />
          </div>
          <div data-testid="disks-column" className="min-w-0">
            <DisksPanel onVerifyRequested={() => setView("transfers")} />
          </div>
          <div data-testid="destinations-column" className="min-w-0">
            <EndpointList
              title="Đích"
              endpoints={destinations}
              disks={disks}
              onRemove={removeDestination}
              onLabelChange={setDestinationLabel}
              onPathChange={setDestinationPath}
              usageKind="free"
              onDropDisk={addDestination}
              onReorder={reorderDestinations}
            />
          </div>
          <ElementsReviewPanel />
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-6 md:grid-cols-4">
          <EndpointList
            title="Nguồn"
            endpoints={sources}
            disks={disks}
            onRemove={removeSource}
            onLabelChange={setSourceLabel}
            onPathChange={setSourcePath}
            onBrowse={handleBrowse}
            onDropDisk={addSource}
          />
          <div className="flex flex-col gap-6 md:col-span-2">
            <TransfersPanel
              groups={Object.values(groups)}
              jobs={jobs}
              onCancelJob={handleCancelJob}
              onCancelGroup={handleCancelGroup}
              onResumeJob={handleResumeJob}
              onResolveBrokenMedia={handleResolveBrokenMedia}
            />
            <MhlVerifyPanel />
          </div>
          <EndpointList
            title="Đích"
            endpoints={destinations}
            disks={disks}
            onRemove={removeDestination}
            onLabelChange={setDestinationLabel}
            onPathChange={setDestinationPath}
            usageKind="free"
            onDropDisk={addDestination}
            onReorder={reorderDestinations}
          />
        </div>
      )}

      <AddTransfersBar
        sources={sources}
        destinations={destinations}
        disks={disks}
        onAdd={handleAddTransfers}
        onClear={clearSourcesAndDestinations}
      />

      <PreferencesModal
        open={preferencesOpen}
        initialTab={preferencesTab}
        onClose={() => setPreferencesOpen(false)}
      />

      <Modal open={transferLogOpen} onClose={() => setTransferLogOpen(false)} title="Nhật ký truyền tải">
        <TransferLogPanel onViewClips={handleBrowse} />
      </Modal>

      <Modal open={reportsOpen} onClose={() => setReportsOpen(false)} title="Báo cáo">
        <ReportsPanel />
      </Modal>

      <AboutDialog open={aboutOpen} onClose={() => setAboutOpen(false)} />

      <StartTransferFailureDialog
        message={startTransferError}
        onClose={() => setStartTransferError(null)}
      />

      {activeScan && (
        <MediaBrowser
          folder={activeScan.folder}
          entries={activeScan.entries}
          status={activeScan.status}
          total={activeScan.total}
          onClose={() => setActiveScan(null)}
        />
      )}
    </main>
  );
}

export default App;
