export type SelectiveCopyMode = "exclude" | "include";

export interface SelectiveCopyFilter {
  mode: SelectiveCopyMode;
  patterns: string[];
}

export interface BundleIgnoreRule {
  name: string;
  maxSizeBytes: number;
}

export type DateOverrideMode = "automatic" | "manual";

export interface DateTimeOverride {
  mode: DateOverrideMode;
  /** ISO `YYYY-MM-DD`. Ignored unless `mode` is `"manual"`. */
  manualDate: string | null;
  /** Automatic-mode only: keeps times before 4am on the previous shoot day. */
  rolloverAt4am: boolean;
}

/** A user-defined custom token (e.g. `{Location}`) -- OffShoot calls these "Elements". */
export interface ElementDefinition {
  /** Token name without braces, e.g. `"Location"` for `{Location}`. */
  name: string;
  value: string;
}

export interface OrganizeSettings {
  renameTemplate: string | null;
  folderTemplate: string | null;
  counterPadding: number;
  selectiveCopy: SelectiveCopyFilter;
  bundleIgnore: BundleIgnoreRule | null;
  ignoreEmptyFolders: boolean;
  flatten: boolean;
  contentDateExcludedExtensions: string[];
  dateOverride: DateTimeOverride;
  elements: ElementDefinition[];
}

export function defaultOrganizeSettings(): OrganizeSettings {
  return {
    renameTemplate: null,
    folderTemplate: null,
    counterPadding: 3,
    selectiveCopy: { mode: "exclude", patterns: [] },
    bundleIgnore: null,
    ignoreEmptyFolders: true,
    flatten: false,
    contentDateExcludedExtensions: [],
    dateOverride: { mode: "automatic", manualDate: null, rolloverAt4am: false },
    elements: [],
  };
}
