export type SelectiveCopyMode = "exclude" | "include";

export interface SelectiveCopyFilter {
  mode: SelectiveCopyMode;
  patterns: string[];
}

export interface BundleIgnoreRule {
  name: string;
  maxSizeBytes: number;
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
  };
}
