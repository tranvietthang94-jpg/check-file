import { create } from "zustand";
import { effectiveJobDate, renderTemplate } from "../lib/tokenEngine";
import { useOrganizeStore } from "./organizeStore";
import type { DiskInfo, Endpoint } from "../types/disk";

interface DisksState {
  disks: DiskInfo[];
  sources: Endpoint[];
  destinations: Endpoint[];
  /** Drives hidden from the Disks list -- e.g. the system drive that's never
   * a real Source/Destination and just adds noise. Session-only, like every
   * other setting in this app (nothing here persists across a restart yet). */
  hiddenDiskIds: string[];
  hideDisk: (diskId: string) => void;
  unhideDisk: (diskId: string) => void;
  setDisks: (disks: DiskInfo[]) => void;
  setEndpoints: (sources: Endpoint[], destinations: Endpoint[]) => void;
  addSource: (diskId: string) => void;
  addDestination: (diskId: string) => void;
  removeSource: (diskId: string) => void;
  removeDestination: (diskId: string) => void;
  /** The AddTransfersBar's "clear" (✕) button -- resets both lists so the
   * next Source/Destination pick starts from a clean slate. */
  clearSourcesAndDestinations: () => void;
  setSourceLabel: (diskId: string, label: string) => void;
  setDestinationLabel: (diskId: string, label: string) => void;
  setSourcePath: (diskId: string, path: string) => void;
  setDestinationPath: (diskId: string, path: string) => void;
  /** Re-renders every still-auto-labeled source's `{Counter}` in sequence, so
   * removing or manually overriding one never leaves a gap or a duplicate --
   * mirrors OffShoot's "recalculates remaining auto-labels" behavior. */
  recomputeAutoLabels: () => void;
  /** Drag-to-reorder within the Destinations list -- this array's order *is*
   * the Cascade hop order now that "Add N Transfers" builds a cascade chain
   * straight from the live Destinations list instead of a per-click
   * composer form. */
  reorderDestinations: (fromDiskId: string, toDiskId: string) => void;
  /** The disk context menu's "Cascade from ▶ [existing destination]" --
   * adds `diskId` as a Destination (if it isn't already one) positioned
   * right after `afterDiskId` in the list, i.e. it now receives from that
   * hop instead of landing wherever `addDestination` would otherwise
   * append it. */
  insertDestinationAfter: (diskId: string, afterDiskId: string) => void;
}

export const useDisksStore = create<DisksState>((set, get) => ({
  disks: [],
  sources: [],
  destinations: [],
  hiddenDiskIds: [],

  hideDisk: (diskId) =>
    set((state) =>
      state.hiddenDiskIds.includes(diskId)
        ? state
        : { hiddenDiskIds: [...state.hiddenDiskIds, diskId] },
    ),

  unhideDisk: (diskId) =>
    set((state) => ({
      hiddenDiskIds: state.hiddenDiskIds.filter((id) => id !== diskId),
    })),

  setDisks: (disks) => set({ disks }),
  setEndpoints: (sources, destinations) => set({ sources, destinations }),

  addSource: (diskId) => {
    if (!get().disks.some((disk) => disk.id === diskId)) return;
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

  addDestination: (diskId) => {
    if (!get().disks.some((disk) => disk.id === diskId)) return;
    set((state) => {
      if (state.destinations.some((d) => d.diskId === diskId)) return state;
      const disk = get().disks.find((d) => d.id === diskId);
      return {
        destinations: [
          ...state.destinations,
          { diskId, label: "", path: disk?.mountPoint ?? "", isAutoLabel: false },
        ],
      };
    });
  },

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

  clearSourcesAndDestinations: () => set({ sources: [], destinations: [] }),

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

  reorderDestinations: (fromDiskId, toDiskId) =>
    set((state) => {
      if (fromDiskId === toDiskId) return state;
      const fromIndex = state.destinations.findIndex((d) => d.diskId === fromDiskId);
      const toIndex = state.destinations.findIndex((d) => d.diskId === toDiskId);
      if (fromIndex === -1 || toIndex === -1) return state;
      const next = [...state.destinations];
      const [moved] = next.splice(fromIndex, 1);
      next.splice(toIndex, 0, moved);
      return { destinations: next };
    }),

  insertDestinationAfter: (diskId, afterDiskId) =>
    set((state) => {
      const disk = get().disks.find((d) => d.id === diskId);
      const existing = state.destinations.find((d) => d.diskId === diskId);
      const entry: Endpoint = existing ?? {
        diskId,
        label: "",
        path: disk?.mountPoint ?? "",
        isAutoLabel: false,
      };
      const withoutMoved = state.destinations.filter((d) => d.diskId !== diskId);
      const afterIndex = withoutMoved.findIndex((d) => d.diskId === afterDiskId);
      const next = [...withoutMoved];
      next.splice(afterIndex === -1 ? next.length : afterIndex + 1, 0, entry);
      return { destinations: next };
    }),

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
