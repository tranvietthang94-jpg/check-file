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
}

export const useDisksStore = create<DisksState>((set) => ({
  disks: [],
  sources: [],
  destinations: [],

  setDisks: (disks) => set({ disks }),

  addSource: (diskId) =>
    set((state) =>
      state.sources.some((s) => s.diskId === diskId)
        ? state
        : { sources: [...state.sources, { diskId, label: "" }] },
    ),

  addDestination: (diskId) =>
    set((state) =>
      state.destinations.some((d) => d.diskId === diskId)
        ? state
        : { destinations: [...state.destinations, { diskId, label: "" }] },
    ),

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
}));
