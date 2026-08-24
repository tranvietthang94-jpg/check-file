import { create } from "zustand";
import { planMhlRepair, repairMhlEntry, verifyMhl, verifyMhlsInFolder } from "../lib/tauri";
import type { MhlVerifyReport, RepairPlan } from "../types/mhl";

interface MhlVerifyState {
  path: string;
  mode: "file" | "folder";
  busy: boolean;
  reports: MhlVerifyReport[] | null;
  error: string | null;
  repairPlan: RepairPlan | null;
  selectedCandidateRoot: string | null;
  manualCandidateRoot: string;
  setPath: (path: string) => void;
  setMode: (mode: "file" | "folder") => void;
  /** Runs a verify for the given path/mode (or whatever's already set, e.g.
   * from the panel's own inputs). Used both by the panel's Verify button and
   * by a disk's right-click "Verify" context-menu action. */
  runVerify: (path?: string, mode?: "file" | "folder") => Promise<void>;
  planRepair: (mhlPath: string, relativePath: string, candidateRoots: string[]) => Promise<void>;
  closeRepairPlan: () => void;
  setManualCandidateRoot: (root: string) => void;
  selectCandidateRoot: (root: string) => void;
  repairSelected: () => Promise<void>;
  repairEntry: (mhlPath: string, relativePath: string, sourceRoot: string) => Promise<void>;
}

export const useMhlVerifyStore = create<MhlVerifyState>((set, get) => ({
  path: "",
  mode: "file",
  busy: false,
  reports: null,
  error: null,
  repairPlan: null,
  selectedCandidateRoot: null,
  manualCandidateRoot: "",

  setPath: (path) => set({ path }),
  setMode: (mode) => set({ mode }),
  setManualCandidateRoot: (manualCandidateRoot) => set({ manualCandidateRoot }),
  selectCandidateRoot: (selectedCandidateRoot) => set({ selectedCandidateRoot }),
  closeRepairPlan: () =>
    set({ repairPlan: null, selectedCandidateRoot: null, manualCandidateRoot: "", error: null }),

  planRepair: async (mhlPath, relativePath, candidateRoots) => {
    const manual = get().manualCandidateRoot.trim();
    const roots = [...candidateRoots, ...(manual ? [manual] : [])].filter(
      (root, index, all) => root.trim() && all.indexOf(root) === index,
    );
    set({ busy: true, error: null, repairPlan: null, selectedCandidateRoot: null });
    try {
      const repairPlan = await planMhlRepair(mhlPath, relativePath, roots);
      set({
        repairPlan,
        selectedCandidateRoot: repairPlan.candidates[0]?.root ?? null,
        busy: false,
      });
    } catch (err) {
      set({ error: String(err), busy: false });
    }
  },

  repairSelected: async () => {
    const plan = get().repairPlan;
    const root = get().selectedCandidateRoot;
    if (!plan || !root) return;
    await get().repairEntry(plan.mhlPath, plan.relativePath, root);
    if (!get().error) get().closeRepairPlan();
  },

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
