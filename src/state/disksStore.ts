import { create } from "zustand";
import { effectiveJobDate, renderTemplate } from "../lib/tokenEngine";
import { useOrganizeStore } from "./organizeStore";
import { pathLabel } from "../lib/format";
import type { AddEndpointPathResult, DiskInfo, Endpoint } from "../types/disk";

export function normalizeWindowsPath(path: string): string {
  const normalized = path.replace(/\//g, "\\");
  if (/^[a-zA-Z]:\\+$/.test(normalized)) {
    return `${normalized.slice(0, 2).toLowerCase()}\\`;
  }
  if (/^\\\\[^\\]+\\[^\\]+\\*$/.test(normalized)) {
    return `${normalized.replace(/\\+$/, "").toLowerCase()}\\`;
  }
  return normalized.replace(/\\+$/, "").toLowerCase();
}

function diskForPath(path: string, disks: DiskInfo[]): DiskInfo | undefined {
  const pathKey = normalizeWindowsPath(path);
  return [...disks]
    .sort(
      (left, right) =>
        normalizeWindowsPath(right.mountPoint).length -
        normalizeWindowsPath(left.mountPoint).length,
    )
    .find((disk) => {
      const mountKey = normalizeWindowsPath(disk.mountPoint);
      return (
        pathKey === mountKey ||
        pathKey.startsWith(mountKey.endsWith("\\") ? mountKey : `${mountKey}\\`)
      );
    });
}

function pathEndpoint(path: string, disks: DiskInfo[], isAutoLabel: boolean): Endpoint {
  const disk = diskForPath(path, disks);
  return {
    id: `path:${normalizeWindowsPath(path)}`,
    diskId: disk?.id ?? null,
    label: "",
    path,
    isAutoLabel,
  };
}

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
  addSourcePath: (path: string) => AddEndpointPathResult;
  addDestinationPath: (path: string) => AddEndpointPathResult;
  removeSource: (endpointId: string) => void;
  removeDestination: (endpointId: string) => void;
  /** The AddTransfersBar's "clear" (✕) button -- resets both lists so the
   * next Source/Destination pick starts from a clean slate. */
  clearSourcesAndDestinations: () => void;
  setSourceLabel: (endpointId: string, label: string) => void;
  setDestinationLabel: (endpointId: string, label: string) => void;
  setSourcePath: (endpointId: string, path: string) => void;
  setDestinationPath: (endpointId: string, path: string) => void;
  /** Re-renders every still-auto-labeled source's `{Counter}` in sequence, so
   * removing or manually overriding one never leaves a gap or a duplicate --
   * mirrors OffShoot's "recalculates remaining auto-labels" behavior. */
  recomputeAutoLabels: () => void;
  /** Drag-to-reorder within the Destinations list -- this array's order *is*
   * the Cascade hop order now that "Add N Transfers" builds a cascade chain
   * straight from the live Destinations list instead of a per-click
   * composer form. */
  reorderDestinations: (fromEndpointId: string, toEndpointId: string, placement?: "before" | "after") => void;
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
          { id: diskId, diskId, label: "", path: disk?.mountPoint ?? "", isAutoLabel },
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
          { id: diskId, diskId, label: "", path: disk?.mountPoint ?? "", isAutoLabel: false },
        ],
      };
    });
  },

  addSourcePath: (path) => {
    if (path.trim() === "") {
      return { ok: false, added: false, error: "Đường dẫn Source không được để trống." };
    }
    const pathKey = normalizeWindowsPath(path);
    const existing = get().sources.find((source) => normalizeWindowsPath(source.path) === pathKey);
    if (existing) return { ok: true, added: false, endpoint: existing };

    const endpoint = pathEndpoint(path, get().disks, useOrganizeStore.getState().autoLabel.enabled);
    set((state) => ({ sources: [...state.sources, endpoint] }));
    get().recomputeAutoLabels();
    return { ok: true, added: true, endpoint };
  },

  addDestinationPath: (path) => {
    if (path.trim() === "") {
      return { ok: false, added: false, error: "Đường dẫn Destination không được để trống." };
    }
    const pathKey = normalizeWindowsPath(path);
    const existing = get().destinations.find(
      (destination) => normalizeWindowsPath(destination.path) === pathKey,
    );
    if (existing) return { ok: true, added: false, endpoint: existing };

    const endpoint = pathEndpoint(path, get().disks, false);
    set((state) => ({ destinations: [...state.destinations, endpoint] }));
    return { ok: true, added: true, endpoint };
  },

  removeSource: (endpointId) => {
    set((state) => ({
      sources: state.sources.filter((s) => s.id !== endpointId),
    }));
    get().recomputeAutoLabels();
  },

  removeDestination: (endpointId) =>
    set((state) => ({
      destinations: state.destinations.filter((d) => d.id !== endpointId),
    })),

  clearSourcesAndDestinations: () => set({ sources: [], destinations: [] }),

  setSourceLabel: (endpointId, label) => {
    // Typing into the label field is always a manual override, even if this
    // source's label happened to be auto-generated a moment ago.
    set((state) => ({
      sources: state.sources.map((s) =>
        s.id === endpointId ? { ...s, label, isAutoLabel: false } : s,
      ),
    }));
    get().recomputeAutoLabels();
  },

  setDestinationLabel: (endpointId, label) =>
    set((state) => ({
      destinations: state.destinations.map((d) =>
        d.id === endpointId ? { ...d, label } : d,
      ),
    })),

  setSourcePath: (endpointId, path) =>
    set((state) => ({
      sources: state.sources.map((s) => (s.id === endpointId ? { ...s, path } : s)),
    })),

  setDestinationPath: (endpointId, path) =>
    set((state) => ({
      destinations: state.destinations.map((d) =>
        d.id === endpointId ? { ...d, path } : d,
      ),
    })),

  reorderDestinations: (fromEndpointId, toEndpointId, placement = "before") =>
    set((state) => {
      if (fromEndpointId === toEndpointId) return state;
      const fromIndex = state.destinations.findIndex((d) => d.id === fromEndpointId);
      const targetIndex = state.destinations.findIndex((d) => d.id === toEndpointId);
      if (fromIndex === -1 || targetIndex === -1) return state;
      const next = [...state.destinations];
      const [moved] = next.splice(fromIndex, 1);
      const adjustedTargetIndex = next.findIndex((d) => d.id === toEndpointId);
      next.splice(adjustedTargetIndex + (placement === "after" ? 1 : 0), 0, moved);
      return { destinations: next };
    }),

  insertDestinationAfter: (diskId, afterDiskId) =>
    set((state) => {
      const disk = get().disks.find((d) => d.id === diskId);
      const existing = state.destinations.find((d) => d.diskId === diskId);
      const entry: Endpoint = existing ?? {
        id: diskId,
        diskId,
        label: "",
        path: disk?.mountPoint ?? "",
        isAutoLabel: false,
      };
      const withoutMoved = state.destinations.filter((d) => d.id !== entry.id);
      const afterIndex = withoutMoved.findIndex((d) => d.id === afterDiskId);
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
          sourceName: disk?.name ?? pathLabel(s.path),
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
