import { create } from "zustand";
import { persist } from "zustand/middleware";
import { defaultOrganizeSettings } from "../types/organize";
import type {
  BundleIgnoreRule,
  DateOverrideMode,
  OrganizeSettings,
  SelectiveCopyMode,
} from "../types/organize";

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
  setDateOverrideMode: (mode: DateOverrideMode) => void;
  setManualDate: (date: string | null) => void;
  setRolloverAt4am: (value: boolean) => void;
  /** Adds a new element definition; ignores blank or case-insensitively duplicate names. */
  addElement: (name: string) => void;
  removeElement: (name: string) => void;
  setElementValue: (name: string, value: string) => void;
  /** Resets every element's value to "" but keeps the definitions -- mirrors OffShoot's "Clear". */
  clearElementValues: () => void;
  setAutoLabelEnabled: (enabled: boolean) => void;
  setAutoLabelTemplate: (template: string) => void;
  setAutoLabelStartCounter: (value: number) => void;
  setAutoLabelCounterPadding: (value: number) => void;
  setSkipModificationDateCheck: (value: boolean) => void;
  setAutoContinueOnBrokenMedia: (value: boolean) => void;
  /** Replaces every field at once -- used when applying a loaded preset. */
  loadSettings: (settings: OrganizeSettings) => void;
}

/** Shared by every `AutoLabelSettings` setter -- keeps counters sane the same way `setCounterPadding` does for Organize. */
function clampCounter(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, Math.trunc(value) || min));
}

/** Empty string collapses to `null` -- "no template" is the actual default state. */
function toTemplate(value: string): string | null {
  return value.trim() === "" ? null : value;
}

export const useOrganizeStore = create<OrganizeState>()(
  persist(
    (set) => ({
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
  setDateOverrideMode: (mode) =>
    set((state) => ({ dateOverride: { ...state.dateOverride, mode } })),
  setManualDate: (manualDate) =>
    set((state) => ({ dateOverride: { ...state.dateOverride, manualDate } })),
  setRolloverAt4am: (rolloverAt4am) =>
    set((state) => ({ dateOverride: { ...state.dateOverride, rolloverAt4am } })),
  addElement: (name) =>
    set((state) => {
      const trimmed = name.trim();
      if (trimmed === "") return state;
      const exists = state.elements.some(
        (e) => e.name.toLowerCase() === trimmed.toLowerCase(),
      );
      if (exists) return state;
      return { elements: [...state.elements, { name: trimmed, value: "" }] };
    }),
  removeElement: (name) =>
    set((state) => ({ elements: state.elements.filter((e) => e.name !== name) })),
  setElementValue: (name, value) =>
    set((state) => ({
      elements: state.elements.map((e) => (e.name === name ? { ...e, value } : e)),
    })),
  clearElementValues: () =>
    set((state) => ({ elements: state.elements.map((e) => ({ ...e, value: "" })) })),
  setAutoLabelEnabled: (enabled) =>
    set((state) => ({ autoLabel: { ...state.autoLabel, enabled } })),
  setAutoLabelTemplate: (template) =>
    set((state) => ({ autoLabel: { ...state.autoLabel, template } })),
  setAutoLabelStartCounter: (value) =>
    set((state) => ({
      autoLabel: { ...state.autoLabel, startCounter: clampCounter(value, 0, 999_999) },
    })),
  setAutoLabelCounterPadding: (value) =>
    set((state) => ({
      autoLabel: { ...state.autoLabel, counterPadding: clampCounter(value, 1, 8) },
    })),
  setSkipModificationDateCheck: (skipModificationDateCheck) =>
    set({ skipModificationDateCheck }),
  setAutoContinueOnBrokenMedia: (autoContinueOnBrokenMedia) =>
    set({ autoContinueOnBrokenMedia }),
  loadSettings: (settings) => set({ ...settings }),
    }),
    {
      name: "offloadkit-organize-v1",
      partialize: (state) => ({
        renameTemplate: state.renameTemplate,
        folderTemplate: state.folderTemplate,
        counterPadding: state.counterPadding,
        selectiveCopy: state.selectiveCopy,
        bundleIgnore: state.bundleIgnore,
        ignoreEmptyFolders: state.ignoreEmptyFolders,
        flatten: state.flatten,
        contentDateExcludedExtensions: state.contentDateExcludedExtensions,
        dateOverride: state.dateOverride,
        elements: state.elements,
        autoLabel: state.autoLabel,
        skipModificationDateCheck: state.skipModificationDateCheck,
        autoContinueOnBrokenMedia: state.autoContinueOnBrokenMedia,
      }),
    },
  ),
);
