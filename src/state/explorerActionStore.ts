import { create } from "zustand";
import { acknowledgeExplorerRequest, startTransferGroup } from "../lib/tauri";
import type { ExplorerErrorPayload, ExplorerRequest } from "../lib/tauri";
import { useDisksStore } from "./disksStore";
import { useGroupsStore } from "./groupsStore";
import { useOrganizeStore } from "./organizeStore";
import { useSettingsStore } from "./settingsStore";
import { pathLabel } from "../lib/format";

interface ExplorerFeedback {
  kind: "success" | "error";
  message: string;
}

interface ExplorerActionState {
  listenersReady: boolean;
  ready: boolean;
  pending: ExplorerRequest[];
  processedIds: string[];
  errorIds: string[];
  feedback: ExplorerFeedback | null;
  markListenersReady: () => void;
  markReady: () => Promise<void>;
  receiveRequest: (request: ExplorerRequest) => Promise<void>;
  receiveError: (error: ExplorerErrorPayload) => void;
}

let drainPromise: Promise<void> | null = null;

export const useExplorerActionStore = create<ExplorerActionState>((set, get) => ({
  listenersReady: false,
  ready: false,
  pending: [],
  processedIds: [],
  errorIds: [],
  feedback: null,

  markListenersReady: () => set({ listenersReady: true }),

  markReady: async () => {
    set({ ready: true });
    await scheduleDrain();
  },

  receiveRequest: async (request) => {
    const state = get();
    if (state.processedIds.includes(request.id)) return;
    if (!state.pending.some((pending) => pending.id === request.id)) {
      set((current) => ({ pending: [...current.pending, request] }));
    }
    if (get().ready) await scheduleDrain();
  },

  receiveError: (error) => {
    if (get().errorIds.includes(error.id)) return;
    set((state) => ({
      errorIds: [...state.errorIds, error.id],
      feedback: { kind: "error", message: error.message },
    }));
  },
}));

function scheduleDrain(): Promise<void> {
  if (!drainPromise) {
    drainPromise = drainQueue().finally(() => {
      drainPromise = null;
    });
  }
  return drainPromise;
}

async function drainQueue(): Promise<void> {
  while (useExplorerActionStore.getState().ready) {
    const request = useExplorerActionStore.getState().pending[0];
    if (!request) return;

    try {
      const message = await applyRequest(request);
      useExplorerActionStore.setState((state) => ({
        pending: state.pending.filter((pending) => pending.id !== request.id),
        processedIds: [...state.processedIds, request.id],
        feedback: {
          kind: "success",
          message,
        },
      }));
      await acknowledgeExplorerRequest(request.id);
    } catch (error) {
      useExplorerActionStore.setState((state) => ({
        pending: state.pending.filter((pending) => pending.id !== request.id),
        processedIds: [...state.processedIds, request.id],
        feedback: { kind: "error", message: String(error) },
      }));
    }
  }
}

async function applyRequest(request: ExplorerRequest): Promise<string> {
  const integrationSurface = isMacOs() ? "Finder" : "Windows Explorer";
  if (request.paths.length === 0 || request.paths.some((path) => path.trim() === "")) {
    throw new Error(`Yêu cầu ${integrationSurface} không có đường dẫn hợp lệ.`);
  }

  const disks = useDisksStore.getState();
  if (request.action === "setSource") {
    if (request.sourceSelection) {
      const result = disks.addSourceSelection(
        request.sourceSelection.commonRoot,
        request.sourceSelection.selectedPaths,
      );
      if (!result.ok) throw new Error(result.error);
      return `Đã đặt Source từ ${integrationSurface}: ${request.sourceSelection.selectedPaths.join(", ")}`;
    }
    for (const path of request.paths) {
      const result = disks.addSourcePath(path);
      if (!result.ok) throw new Error(result.error);
    }
    return `Đã đặt Source từ ${integrationSurface}: ${request.paths.join(", ")}`;
  }

  if (request.action === "setDestination") {
    if (request.paths.length !== 1) {
      throw new Error(`Destination từ ${integrationSurface} phải có đúng một thư mục.`);
    }
    const result = disks.addDestinationPath(request.paths[0]);
    if (!result.ok) throw new Error(result.error);
    return `Đã đặt Destination từ ${integrationSurface}: ${request.paths[0]}`;
  }

  if (request.action === "copy") {
    return `Đã copy ${request.paths.length} mục bằng ${integrationSurface}.`;
  }

  if (request.action === "paste") {
    const selection = request.sourceSelection;
    const destinationPath = request.destinationPath;
    if (!selection || !destinationPath) {
      throw new Error(`Yêu cầu Paste từ ${integrationSurface} thiếu Source selection hoặc Destination.`);
    }
    const endpoints = disks.replaceForExplorerPaste(
      selection.commonRoot,
      selection.selectedPaths,
      destinationPath,
    );
    const settings = useSettingsStore.getState();
    const organizeState = useOrganizeStore.getState();
    const organize = {
      renameTemplate: organizeState.renameTemplate,
      folderTemplate: organizeState.folderTemplate,
      counterPadding: organizeState.counterPadding,
      selectiveCopy: organizeState.selectiveCopy,
      bundleIgnore: organizeState.bundleIgnore,
      ignoreEmptyFolders: organizeState.ignoreEmptyFolders,
      flatten: organizeState.flatten,
      contentDateExcludedExtensions: organizeState.contentDateExcludedExtensions,
      dateOverride: organizeState.dateOverride,
      elements: organizeState.elements,
      autoLabel: organizeState.autoLabel,
      skipModificationDateCheck: organizeState.skipModificationDateCheck,
      autoContinueOnBrokenMedia: organizeState.autoContinueOnBrokenMedia,
    };
    const sourceLabel = endpoints.source.label || pathLabel(endpoints.source.path);
    const destinationLabel = endpoints.destination.label || pathLabel(endpoints.destination.path);
    const groupId = await startTransferGroup(
      endpoints.source.path,
      endpoints.source.selectedPaths ?? null,
      [endpoints.destination.path],
      "parallel",
      settings.verificationMode,
      settings.checksumAlgorithm,
      sourceLabel,
      organize,
      false,
      false,
      settings.legacyChecksumEnabled ? settings.legacyChecksumAlgorithm : null,
      settings.saveLogToDestination,
      settings.createPerFileMhl,
    );
    useGroupsStore
      .getState()
      .setGroupMeta(groupId, "parallel", sourceLabel, [destinationLabel]);
    return `Đã bắt đầu Paste từ ${integrationSurface} bằng OffloadKit: ${selection.selectedPaths.length} mục.`;
  }

  throw new Error(`Hành động ${integrationSurface} không hợp lệ.`);
}

function isMacOs(): boolean {
  return /Macintosh|Mac OS X/.test(navigator.userAgent);
}
