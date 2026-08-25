import { create } from "zustand";
import { acknowledgeExplorerRequest } from "../lib/tauri";
import type { ExplorerErrorPayload, ExplorerRequest } from "../lib/tauri";
import { useDisksStore } from "./disksStore";

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
      applyRequest(request);
      useExplorerActionStore.setState((state) => ({
        pending: state.pending.filter((pending) => pending.id !== request.id),
        processedIds: [...state.processedIds, request.id],
        feedback: {
          kind: "success",
          message:
            request.action === "setSource"
              ? `Đã đặt Source từ Windows Explorer: ${request.paths.join(", ")}`
              : `Đã đặt Destination từ Windows Explorer: ${request.paths[0]}`,
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

function applyRequest(request: ExplorerRequest): void {
  if (request.paths.length === 0 || request.paths.some((path) => path.trim() === "")) {
    throw new Error("Yêu cầu Windows Explorer không có đường dẫn hợp lệ.");
  }

  const disks = useDisksStore.getState();
  if (request.action === "setSource") {
    for (const path of request.paths) {
      const result = disks.addSourcePath(path);
      if (!result.ok) throw new Error(result.error);
    }
    return;
  }

  if (request.action === "setDestination") {
    if (request.paths.length !== 1) {
      throw new Error("Destination từ Windows Explorer phải có đúng một thư mục.");
    }
    const result = disks.addDestinationPath(request.paths[0]);
    if (!result.ok) throw new Error(result.error);
    return;
  }

  throw new Error("Hành động Windows Explorer không hợp lệ.");
}
