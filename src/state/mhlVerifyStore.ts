import { create } from "zustand";
import { repairMhlEntry, verifyMhl, verifyMhlsInFolder } from "../lib/tauri";
import type { MhlVerifyReport } from "../types/mhl";

interface MhlVerifyState {
  path: string;
  mode: "file" | "folder";
  busy: boolean;
  reports: MhlVerifyReport[] | null;
  error: string | null;
  setPath: (path: string) => void;
  setMode: (mode: "file" | "folder") => void;
  /** Runs a verify for the given path/mode (or whatever's already set, e.g.
   * from the panel's own inputs). Used both by the panel's Verify button and
   * by a disk's right-click "Verify" context-menu action. */
  runVerify: (path?: string, mode?: "file" | "folder") => Promise<void>;
  repairEntry: (mhlPath: string, relativePath: string, sourceRoot: string) => Promise<void>;
}

export const useMhlVerifyStore = create<MhlVerifyState>((set, get) => ({
  path: "",
  mode: "file",
  busy: false,
  reports: null,
  error: null,

  setPath: (path) => set({ path }),
  setMode: (mode) => set({ mode }),

  repairEntry: async (mhlPath, relativePath, sourceRoot) => {
    set({ busy: true, error: null });
    try {
      const report = await repairMhlEntry(mhlPath, relativePath, sourceRoot, true);
      set((state) => ({
        reports: (state.reports ?? []).map((item) =>
          item.mhlPath === report.mhlPath ? report : item,
        ),
        busy: false,
      }));
    } catch (err) {
      set({ error: String(err), busy: false });
    }
  },

  runVerify: async (pathArg, modeArg) => {
    const path = pathArg ?? get().path;
    const mode = modeArg ?? get().mode;
    if (!path.trim()) return;
    set({ path, mode, busy: true, error: null, reports: null });
    try {
      const result = mode === "file" ? [await verifyMhl(path)] : await verifyMhlsInFolder(path);
      set({ reports: result, busy: false });
    } catch (err) {
      set({ error: String(err), busy: false });
    }
  },
}));
