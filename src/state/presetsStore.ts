import { create } from "zustand";
import { deletePreset, listPresets, savePreset } from "../lib/tauri";
import type { Preset } from "../types/preset";

interface PresetsState {
  presets: Preset[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  save: (preset: Preset) => Promise<void>;
  remove: (name: string) => Promise<void>;
}

export const usePresetsStore = create<PresetsState>((set, get) => ({
  presets: [],
  loading: false,
  error: null,

  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const presets = await listPresets();
      set({ presets, loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  save: async (preset) => {
    await savePreset(preset);
    await get().refresh();
  },

  remove: async (name) => {
    await deletePreset(name);
    await get().refresh();
  },
}));
