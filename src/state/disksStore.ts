import { create } from "zustand";
import { effectiveJobDate, renderTemplate } from "../lib/tokenEngine";
import { useOrganizeStore } from "./organizeStore";
import type { DiskInfo, Endpoint } from "../types/disk";

interface DisksState {
  disks: DiskInfo[];
  sources: Endpoint[];
  destinations: Endpoint[];
  setDisks: (disks: DiskInfo[]) => void;
  addSource: (diskId: string) => void;
  addDestination: (diskId: string) => void;
  removeSource: (diskId: string) => void;
  removeDestination: (diskId: string) => void;
  setSourceLabel: (diskId: string, label: string) => void;
  setDestinationLabel: (diskId: string, label: string) => void;
  setSourcePath: (diskId: string, path: string) => void;
  setDestinationPath: (diskId: string, path: string) => void;
  /** Re-renders every still-auto-labeled source's `{Counter}` in sequence, so
   * removing or manually overriding one never leaves a gap or a duplicate --
   * mirrors OffShoot's "recalculates remaining auto-labels" behavior. */
  recomputeAutoLabels: () => void;
}

export const useDisksStore = create<DisksState>((set, get) => ({
  disks: [],
  sources: [],
  destinations: [],

  setDisks: (disks) => set({ disks }),

  addSource: (diskId) => {
    set((state) => {
      if (state.sources.some((s) => s.diskId === diskId)) return state;
      const disk = get().disks.find((d) => d.id === diskId);
      const isAutoLabel = useOrganizeStore.getState().autoLabel.enabled;
      return {
        sources: [
          ...state.sources,
          { diskId, label: "", path: disk?.mountPoint ?? "", isAutoLabel },
        ],
      };
    });
    get().recomputeAutoLabels();
  },

  addDestination: (diskId) =>
    set((state) => {
      if (state.destinations.some((d) => d.diskId === diskId)) return state;
      const disk = get().disks.find((d) => d.id === diskId);
      return {
        destinations: [
          ...state.destinations,
          { diskId, label: "", path: disk?.mountPoint ?? "", isAutoLabel: false },
        ],
      };
    }),

  removeSource: (diskId) => {
    set((state) => ({
      sources: state.sources.filter((s) => s.diskId !== diskId),
    }));
    get().recomputeAutoLabels();
  },

  removeDestination: (diskId) =>
    set((state) => ({
      destinations: state.destinations.filter((d) => d.diskId !== diskId),
    })),

  setSourceLabel: (diskId, label) => {
    // Typing into the label field is always a manual override, even if this
    // source's label happened to be auto-generated a moment ago.
    set((state) => ({
      sources: state.sources.map((s) =>
        s.diskId === diskId ? { ...s, label, isAutoLabel: false } : s,
      ),
    }));
    get().recomputeAutoLabels();
  },

  setDestinationLabel: (diskId, label) =>
    set((state) => ({
      destinations: state.destinations.map((d) =>
        d.diskId === diskId ? { ...d, label } : d,
      ),
    })),

  setSourcePath: (diskId, path) =>
    set((state) => ({
      sources: state.sources.map((s) => (s.diskId === diskId ? { ...s, path } : s)),
    })),

  setDestinationPath: (diskId, path) =>
    set((state) => ({
      destinations: state.destinations.map((d) =>
        d.diskId === diskId ? { ...d, path } : d,
      ),
    })),

  recomputeAutoLabels: () => {
    const { autoLabel, dateOverride, elements } = useOrganizeStore.getState();
    if (!autoLabel.enabled) return;
    set((state) => {
      const now = new Date();
      const jobStarted = effectiveJobDate(now, dateOverride);
      let counter = autoLabel.startCounter;
      const sources = state.sources.map((s) => {
        if (!s.isAutoLabel) return s;
        const disk = state.disks.find((d) => d.id === s.diskId);
        const label = renderTemplate(autoLabel.template, {
          sourceName: disk?.name ?? s.diskId,
          counter,
          counterPadding: autoLabel.counterPadding,
          fileStem: "",
          fileExtension: "",
          now,
          jobStarted,
          elements,
        });
        counter += 1;
        return { ...s, label };
      });
      return { sources };
    });
  },
}));
