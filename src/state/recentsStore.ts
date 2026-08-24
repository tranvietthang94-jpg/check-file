import { create } from "zustand";
import { persist } from "zustand/middleware";

interface RecentsState {
  recentSources: string[];
  recentDestinations: string[];
  addRecentSource: (path: string) => void;
  addRecentDestination: (path: string) => void;
  clearRecentSources: () => void;
  clearRecentDestinations: () => void;
}

const MAX_RECENTS = 8;

function prependUnique(items: string[], path: string): string[] {
  return [path, ...items.filter((item) => item !== path)].slice(0, MAX_RECENTS);
}

export const useRecentsStore = create<RecentsState>()(
  persist(
    (set) => ({
      recentSources: [],
      recentDestinations: [],
      addRecentSource: (path) =>
        set((state) => ({ recentSources: prependUnique(state.recentSources, path) })),
      addRecentDestination: (path) =>
        set((state) => ({ recentDestinations: prependUnique(state.recentDestinations, path) })),
      clearRecentSources: () => set({ recentSources: [] }),
      clearRecentDestinations: () => set({ recentDestinations: [] }),
    }),
    {
      name: "offloadkit-recents-v1",
      partialize: (state) => ({
        recentSources: state.recentSources,
        recentDestinations: state.recentDestinations,
      }),
    },
  ),
);
