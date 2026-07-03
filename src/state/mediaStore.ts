import { create } from "zustand";
import type { MediaEntry } from "../types/media";

export interface MediaScanState {
  scanId: string;
  folder: string;
  entries: MediaEntry[];
  status: "scanning" | "complete";
  total: number;
}

interface MediaState {
  scans: Record<string, MediaScanState>;
  activeScanId: string | null;
  /** Upsert: like groupsStore, the scan's own events can arrive before the
   * command's return value does (two independent IPC channels), so this
   * can't assume a scan record already exists. */
  startScan: (scanId: string, folder: string) => void;
  addEntry: (scanId: string, entry: MediaEntry) => void;
  completeScan: (scanId: string, total: number) => void;
  setActiveScan: (scanId: string | null) => void;
}

function emptyScan(scanId: string, folder = ""): MediaScanState {
  return { scanId, folder, entries: [], status: "scanning", total: 0 };
}

export const useMediaStore = create<MediaState>((set) => ({
  scans: {},
  activeScanId: null,

  startScan: (scanId, folder) =>
    set((state) => {
      const existing = state.scans[scanId] ?? emptyScan(scanId);
      return {
        scans: { ...state.scans, [scanId]: { ...existing, folder } },
        activeScanId: scanId,
      };
    }),

  addEntry: (scanId, entry) =>
    set((state) => {
      const scan = state.scans[scanId] ?? emptyScan(scanId);
      return {
        scans: {
          ...state.scans,
          [scanId]: { ...scan, entries: [...scan.entries, entry] },
        },
      };
    }),

  completeScan: (scanId, total) =>
    set((state) => {
      const scan = state.scans[scanId] ?? emptyScan(scanId);
      return {
        scans: {
          ...state.scans,
          [scanId]: { ...scan, status: "complete", total },
        },
      };
    }),

  setActiveScan: (scanId) => set({ activeScanId: scanId }),
}));
