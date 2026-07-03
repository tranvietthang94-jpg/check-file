import { create } from "zustand";
import { defaultOrganizeSettings } from "../types/organize";
import type { BundleIgnoreRule, OrganizeSettings, SelectiveCopyMode } from "../types/organize";

interface OrganizeState extends OrganizeSettings {
  setRenameTemplate: (template: string) => void;
  setFolderTemplate: (template: string) => void;
  setCounterPadding: (padding: number) => void;
  setSelectiveCopyMode: (mode: SelectiveCopyMode) => void;
  setSelectiveCopyPatterns: (patterns: string[]) => void;
  setBundleIgnore: (rule: BundleIgnoreRule | null) => void;
  setIgnoreEmptyFolders: (value: boolean) => void;
  setFlatten: (value: boolean) => void;
  setContentDateExcludedExtensions: (extensions: string[]) => void;
}

/** Empty string collapses to `null` -- "no template" is the actual default state. */
function toTemplate(value: string): string | null {
  return value.trim() === "" ? null : value;
}

export const useOrganizeStore = create<OrganizeState>((set) => ({
  ...defaultOrganizeSettings(),

  setRenameTemplate: (template) => set({ renameTemplate: toTemplate(template) }),
  setFolderTemplate: (template) => set({ folderTemplate: toTemplate(template) }),
  setCounterPadding: (counterPadding) =>
    set({ counterPadding: Math.min(8, Math.max(1, Math.trunc(counterPadding) || 1)) }),
  setSelectiveCopyMode: (mode) =>
    set((state) => ({ selectiveCopy: { ...state.selectiveCopy, mode } })),
  setSelectiveCopyPatterns: (patterns) =>
    set((state) => ({ selectiveCopy: { ...state.selectiveCopy, patterns } })),
  setBundleIgnore: (bundleIgnore) => set({ bundleIgnore }),
  setIgnoreEmptyFolders: (ignoreEmptyFolders) => set({ ignoreEmptyFolders }),
  setFlatten: (flatten) => set({ flatten }),
  setContentDateExcludedExtensions: (contentDateExcludedExtensions) =>
    set({ contentDateExcludedExtensions }),
}));
