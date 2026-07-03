import { useState } from "react";
import { useOrganizeStore } from "../state/organizeStore";
import { effectiveJobDate, previewDestinationPath } from "../lib/tokenEngine";
import type { DateOverrideMode, SelectiveCopyMode } from "../types/organize";

const TOKEN_REFERENCE =
  "{Source Name} {Counter} {YYYY}{YY}{MM}{DD}{hh}{mm}{ss} · " +
  "{Filename} {File Counter} {File Extension} {File YYYY}.. · {Content YYYY}..";

const NOW = new Date();

const PREVIEW_CONTEXT = {
  sourceName: "A-Cam",
  counter: 1,
  fileStem: "C0001",
  fileExtension: "MP4",
  now: NOW,
};

function parseList(value: string): string[] {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

export function OrganizePanel() {
  const renameTemplate = useOrganizeStore((s) => s.renameTemplate);
  const folderTemplate = useOrganizeStore((s) => s.folderTemplate);
  const counterPadding = useOrganizeStore((s) => s.counterPadding);
  const selectiveCopy = useOrganizeStore((s) => s.selectiveCopy);
  const bundleIgnore = useOrganizeStore((s) => s.bundleIgnore);
  const ignoreEmptyFolders = useOrganizeStore((s) => s.ignoreEmptyFolders);
  const flatten = useOrganizeStore((s) => s.flatten);
  const contentDateExcludedExtensions = useOrganizeStore((s) => s.contentDateExcludedExtensions);
  const dateOverride = useOrganizeStore((s) => s.dateOverride);

  const setRenameTemplate = useOrganizeStore((s) => s.setRenameTemplate);
  const setFolderTemplate = useOrganizeStore((s) => s.setFolderTemplate);
  const setCounterPadding = useOrganizeStore((s) => s.setCounterPadding);
  const setSelectiveCopyMode = useOrganizeStore((s) => s.setSelectiveCopyMode);
  const setSelectiveCopyPatterns = useOrganizeStore((s) => s.setSelectiveCopyPatterns);
  const setBundleIgnore = useOrganizeStore((s) => s.setBundleIgnore);
  const setIgnoreEmptyFolders = useOrganizeStore((s) => s.setIgnoreEmptyFolders);
  const setFlatten = useOrganizeStore((s) => s.setFlatten);
  const setContentDateExcludedExtensions = useOrganizeStore((s) => s.setContentDateExcludedExtensions);
  const setDateOverrideMode = useOrganizeStore((s) => s.setDateOverrideMode);
  const setManualDate = useOrganizeStore((s) => s.setManualDate);
  const setRolloverAt4am = useOrganizeStore((s) => s.setRolloverAt4am);

  const [patternsText, setPatternsText] = useState(selectiveCopy.patterns.join(", "));
  const [excludedExtText, setExcludedExtText] = useState(
    contentDateExcludedExtensions.join(", "),
  );
  const [bundleEnabled, setBundleEnabled] = useState(!!bundleIgnore);
  const [bundleName, setBundleName] = useState(bundleIgnore?.name ?? "");
  const [bundleMaxMb, setBundleMaxMb] = useState(
    bundleIgnore ? (bundleIgnore.maxSizeBytes / (1024 * 1024)).toString() : "50",
  );

  function commitBundle(enabled: boolean, name: string, maxMb: string) {
    if (!enabled || name.trim() === "") {
      setBundleIgnore(null);
      return;
    }
    const mb = Number.parseFloat(maxMb);
    setBundleIgnore({
      name: name.trim(),
      maxSizeBytes: Math.max(0, Number.isFinite(mb) ? mb : 0) * 1024 * 1024,
    });
  }

  const preview = previewDestinationPath(
    {
      renameTemplate,
      folderTemplate,
      counterPadding,
      selectiveCopy,
      bundleIgnore,
      ignoreEmptyFolders,
      flatten,
      contentDateExcludedExtensions,
      dateOverride,
    },
    { ...PREVIEW_CONTEXT, counterPadding, jobStarted: effectiveJobDate(NOW, dateOverride) },
  );

  return (
    <section className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Organize
      </h2>

      <label className="flex flex-col gap-1 text-xs">
        <span className="text-[10px] uppercase tracking-wide text-neutral-500">
          Rename template
        </span>
        <input
          value={renameTemplate ?? ""}
          onChange={(e) => setRenameTemplate(e.currentTarget.value)}
          placeholder="Keep original filename…"
          autoComplete="off"
          className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
        />
      </label>

      <label className="flex flex-col gap-1 text-xs">
        <span className="text-[10px] uppercase tracking-wide text-neutral-500">
          Folder template
        </span>
        <input
          value={folderTemplate ?? ""}
          onChange={(e) => setFolderTemplate(e.currentTarget.value)}
          placeholder="Keep original folder structure…"
          disabled={flatten}
          autoComplete="off"
          className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs disabled:opacity-40"
        />
      </label>

      <p className="truncate rounded border border-neutral-800 bg-neutral-900 px-2 py-1 font-mono text-[10px] text-neutral-400" title={preview}>
        Preview: {preview}
      </p>
      <p className="text-[10px] leading-relaxed text-neutral-600">{TOKEN_REFERENCE}</p>

      <label className="flex items-center gap-2 text-xs">
        <span className="text-[10px] uppercase tracking-wide text-neutral-500">
          {"{Counter}"} padding
        </span>
        <input
          type="number"
          min={1}
          max={8}
          value={counterPadding}
          onChange={(e) => setCounterPadding(Number.parseInt(e.currentTarget.value, 10))}
          className="w-14 rounded border border-neutral-700 bg-neutral-950 px-2 py-1"
        />
      </label>

      <div className="flex flex-col gap-1">
        <span className="text-[10px] uppercase tracking-wide text-neutral-500">
          Selective copy
        </span>
        <div className="flex gap-3 text-xs">
          {(["exclude", "include"] as SelectiveCopyMode[]).map((m) => (
            <label key={m} className="flex items-center gap-1">
              <input
                type="radio"
                name="selective-copy-mode"
                checked={selectiveCopy.mode === m}
                onChange={() => setSelectiveCopyMode(m)}
              />
              {m === "exclude" ? "Do not copy" : "Copy only"}
            </label>
          ))}
        </div>
        <input
          value={patternsText}
          onChange={(e) => {
            setPatternsText(e.currentTarget.value);
            setSelectiveCopyPatterns(parseList(e.currentTarget.value));
          }}
          placeholder=".xml, proxy, .tmp"
          autoComplete="off"
          className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
        />
      </div>

      <div className="flex flex-col gap-1">
        <label className="flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            checked={bundleEnabled}
            onChange={(e) => {
              setBundleEnabled(e.currentTarget.checked);
              commitBundle(e.currentTarget.checked, bundleName, bundleMaxMb);
            }}
          />
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Ignore bundle folder
          </span>
        </label>
        {bundleEnabled && (
          <div className="flex gap-2">
            <input
              value={bundleName}
              onChange={(e) => {
                setBundleName(e.currentTarget.value);
                commitBundle(bundleEnabled, e.currentTarget.value, bundleMaxMb);
              }}
              placeholder="Folder name (e.g. PRIVATE)"
              autoComplete="off"
              className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
            />
            <input
              type="number"
              min={0}
              value={bundleMaxMb}
              onChange={(e) => {
                setBundleMaxMb(e.currentTarget.value);
                commitBundle(bundleEnabled, bundleName, e.currentTarget.value);
              }}
              className="w-16 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
            />
            <span className="self-center text-[10px] text-neutral-500">MB max</span>
          </div>
        )}
      </div>

      <label className="flex items-center gap-2 text-xs">
        <input
          type="checkbox"
          checked={flatten}
          onChange={(e) => setFlatten(e.currentTarget.checked)}
        />
        Flatten (discard original subfolders)
      </label>

      <label className="flex items-center gap-2 text-xs">
        <input
          type="checkbox"
          checked={ignoreEmptyFolders}
          onChange={(e) => setIgnoreEmptyFolders(e.currentTarget.checked)}
          disabled={flatten}
        />
        <span className={flatten ? "opacity-40" : undefined}>Ignore empty folders</span>
      </label>

      <label className="flex flex-col gap-1 text-xs">
        <span className="text-[10px] uppercase tracking-wide text-neutral-500">
          {"{Content *}"} excludes extensions
        </span>
        <input
          value={excludedExtText}
          onChange={(e) => {
            setExcludedExtText(e.currentTarget.value);
            setContentDateExcludedExtensions(parseList(e.currentTarget.value));
          }}
          placeholder=".xml"
          autoComplete="off"
          className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
        />
      </label>

      <div className="flex flex-col gap-1">
        <span className="text-[10px] uppercase tracking-wide text-neutral-500">
          {"{YYYY}{MM}{DD}"} shoot date
        </span>
        <div className="flex gap-3 text-xs">
          {(["automatic", "manual"] as DateOverrideMode[]).map((m) => (
            <label key={m} className="flex items-center gap-1">
              <input
                type="radio"
                name="date-override-mode"
                checked={dateOverride.mode === m}
                onChange={() => setDateOverrideMode(m)}
              />
              {m === "automatic" ? "Follow system clock" : "Set manually"}
            </label>
          ))}
        </div>

        {dateOverride.mode === "automatic" ? (
          <label className="flex items-center gap-2 text-xs">
            <input
              type="checkbox"
              checked={dateOverride.rolloverAt4am}
              onChange={(e) => setRolloverAt4am(e.currentTarget.checked)}
            />
            Roll over at 4am (overnight shoots keep yesterday's date)
          </label>
        ) : (
          <div className="flex items-center gap-2">
            <input
              type="date"
              title="Shoot date"
              value={dateOverride.manualDate ?? ""}
              onChange={(e) => setManualDate(e.currentTarget.value || null)}
              className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
            />
            <button
              type="button"
              onClick={() => {
                setDateOverrideMode("automatic");
                setManualDate(null);
              }}
              className="rounded border border-neutral-700 px-2 py-1 text-[10px] uppercase tracking-wide text-neutral-400 hover:bg-neutral-800"
            >
              Now
            </button>
          </div>
        )}
      </div>
    </section>
  );
}
