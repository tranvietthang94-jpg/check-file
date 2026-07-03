import { create } from "zustand";
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
}

export const useDisksStore = create<DisksState>((set, get) => ({
  disks: [],
  sources: [],
  destinations: [],

  setDisks: (disks) => set({ disks }),

  addSource: (diskId) =>
    set((state) => {
      if (state.sources.some((s) => s.diskId === diskId)) return state;
      const disk = get().disks.find((d) => d.id === diskId);
      return {
        sources: [...state.sources, { diskId, label: "", path: disk?.mountPoint ?? "" }],
      };
    }),

  addDestination: (diskId) =>
    set((state) => {
      if (state.destinations.some((d) => d.diskId === diskId)) return state;
      const disk = get().disks.find((d) => d.id === diskId);
      return {
        destinations: [
          ...state.destinations,
          { diskId, label: "", path: disk?.mountPoint ?? "" },
        ],
      };
    }),

  removeSource: (diskId) =>
    set((state) => ({
      sources: state.sources.filter((s) => s.diskId !== diskId),
    })),

  removeDestination: (diskId) =>
    set((state) => ({
      destinations: state.destinations.filter((d) => d.diskId !== diskId),
    })),

  setSourceLabel: (diskId, label) =>
    set((state) => ({
      sources: state.sources.map((s) => (s.diskId === diskId ? { ...s, label } : s)),
    })),

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
}));
