import { useEffect, useState } from "react";
import { useOrganizeStore } from "../../state/organizeStore";
import { useDisksStore } from "../../state/disksStore";
import { usePresetsStore } from "../../state/presetsStore";
import { useSettingsStore } from "../../state/settingsStore";
import { effectiveJobDate, previewDestinationPath, renderTemplate } from "../../lib/tokenEngine";
import { TemplateBuilder, type TemplateToken } from "../organize/TemplateBuilder";
import type { SelectiveCopyMode } from "../../types/organize";

const DATE_SUB_TOKENS = ["YYYY", "YY", "MM", "DD", "hh", "mm", "ss"];

const BUILTIN_TOKENS: TemplateToken[] = [
  { name: "Source Name", group: "General" },
  { name: "Counter", group: "General" },
  { name: "Filename", group: "General" },
  { name: "File Counter", group: "General" },
  { name: "File Extension", group: "General" },
  ...DATE_SUB_TOKENS.map((t) => ({ name: t, group: "Shoot Date" })),
  ...DATE_SUB_TOKENS.map((t) => ({ name: `File ${t}`, group: "File Date" })),
  ...DATE_SUB_TOKENS.map((t) => ({ name: `Content ${t}`, group: "Content Date" })),
];

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

function PresetsSection() {
  const presets = usePresetsStore((s) => s.presets);
  const loading = usePresetsStore((s) => s.loading);
  const error = usePresetsStore((s) => s.error);
  const refresh = usePresetsStore((s) => s.refresh);
  const save = usePresetsStore((s) => s.save);
  const remove = usePresetsStore((s) => s.remove);

  const verificationMode = useSettingsStore((s) => s.verificationMode);
  const checksumAlgorithm = useSettingsStore((s) => s.checksumAlgorithm);
  const setVerificationMode = useSettingsStore((s) => s.setVerificationMode);
  const setChecksumAlgorithm = useSettingsStore((s) => s.setChecksumAlgorithm);

  const renameTemplate = useOrganizeStore((s) => s.renameTemplate);
  const folderTemplate = useOrganizeStore((s) => s.folderTemplate);
  const counterPadding = useOrganizeStore((s) => s.counterPadding);
  const selectiveCopy = useOrganizeStore((s) => s.selectiveCopy);
  const bundleIgnore = useOrganizeStore((s) => s.bundleIgnore);
  const ignoreEmptyFolders = useOrganizeStore((s) => s.ignoreEmptyFolders);
  const flatten = useOrganizeStore((s) => s.flatten);
  const contentDateExcludedExtensions = useOrganizeStore((s) => s.contentDateExcludedExtensions);
  const dateOverride = useOrganizeStore((s) => s.dateOverride);
  const elements = useOrganizeStore((s) => s.elements);
  const autoLabel = useOrganizeStore((s) => s.autoLabel);
  const skipModificationDateCheck = useOrganizeStore((s) => s.skipModificationDateCheck);
  const autoContinueOnBrokenMedia = useOrganizeStore((s) => s.autoContinueOnBrokenMedia);
  const loadOrganizeSettings = useOrganizeStore((s) => s.loadSettings);

  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleSave() {
    if (newName.trim() === "") return;
    setBusy(true);
    try {
      await save({
        name: newName.trim(),
        verificationMode,
        checksumAlgorithm,
        organize: {
          renameTemplate,
          folderTemplate,
          counterPadding,
          selectiveCopy,
          bundleIgnore,
          ignoreEmptyFolders,
          flatten,
          contentDateExcludedExtensions,
          dateOverride,
          elements,
          autoLabel,
          skipModificationDateCheck,
          autoContinueOnBrokenMedia,
        },
      });
      setNewName("");
    } finally {
      setBusy(false);
    }
  }

  function handleLoad(name: string) {
    const preset = presets.find((p) => p.name === name);
    if (!preset) return;
    setVerificationMode(preset.verificationMode);
    setChecksumAlgorithm(preset.checksumAlgorithm);
    loadOrganizeSettings(preset.organize);
  }

  async function handleDelete(name: string) {
    setBusy(true);
    try {
      await remove(name);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">Presets</h3>

      {loading && <p className="text-xs text-neutral-500">Loading…</p>}
      {error && <p className="text-xs text-red-400">{error}</p>}
      {!loading && presets.length === 0 && (
        <p className="text-xs text-neutral-500">No presets saved yet.</p>
      )}

      <ul className="flex flex-col gap-1">
        {presets.map((preset) => (
          <li
            key={preset.name}
            className="flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs"
          >
            <span className="truncate font-medium" title={preset.name}>
              {preset.name}
            </span>
            <span className="flex shrink-0 gap-1">
              <button
                type="button"
                disabled={busy}
                onClick={() => handleLoad(preset.name)}
                className="rounded border border-neutral-700 px-2 py-1 disabled:opacity-40"
              >
                Load
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => handleDelete(preset.name)}
                className="rounded border border-neutral-700 px-2 py-1 text-red-400 disabled:opacity-40"
              >
                Delete
              </button>
            </span>
          </li>
        ))}
      </ul>

      <div className="flex gap-2">
        <input
          value={newName}
          onChange={(e) => setNewName(e.currentTarget.value)}
          placeholder="Preset name…"
          autoComplete="off"
          className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
        />
        <button
          type="button"
          disabled={busy || newName.trim() === ""}
          onClick={handleSave}
          className="shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
        >
          Save current
        </button>
      </div>
    </section>
  );
}

export function OrganizePreferences() {
  const renameTemplate = useOrganizeStore((s) => s.renameTemplate);
  const folderTemplate = useOrganizeStore((s) => s.folderTemplate);
  const counterPadding = useOrganizeStore((s) => s.counterPadding);
  const selectiveCopy = useOrganizeStore((s) => s.selectiveCopy);
  const bundleIgnore = useOrganizeStore((s) => s.bundleIgnore);
  const ignoreEmptyFolders = useOrganizeStore((s) => s.ignoreEmptyFolders);
  const flatten = useOrganizeStore((s) => s.flatten);
  const contentDateExcludedExtensions = useOrganizeStore((s) => s.contentDateExcludedExtensions);
  const dateOverride = useOrganizeStore((s) => s.dateOverride);
  const elements = useOrganizeStore((s) => s.elements);
  const autoLabel = useOrganizeStore((s) => s.autoLabel);
  const skipModificationDateCheck = useOrganizeStore((s) => s.skipModificationDateCheck);
  const autoContinueOnBrokenMedia = useOrganizeStore((s) => s.autoContinueOnBrokenMedia);

  const setRenameTemplate = useOrganizeStore((s) => s.setRenameTemplate);
  const setFolderTemplate = useOrganizeStore((s) => s.setFolderTemplate);
  const setCounterPadding = useOrganizeStore((s) => s.setCounterPadding);
  const setSelectiveCopyMode = useOrganizeStore((s) => s.setSelectiveCopyMode);
  const setSelectiveCopyPatterns = useOrganizeStore((s) => s.setSelectiveCopyPatterns);
  const setBundleIgnore = useOrganizeStore((s) => s.setBundleIgnore);
  const setIgnoreEmptyFolders = useOrganizeStore((s) => s.setIgnoreEmptyFolders);
  const setFlatten = useOrganizeStore((s) => s.setFlatten);
  const setContentDateExcludedExtensions = useOrganizeStore((s) => s.setContentDateExcludedExtensions);
  const addElement = useOrganizeStore((s) => s.addElement);
  const removeElement = useOrganizeStore((s) => s.removeElement);
  const setAutoLabelEnabled = useOrganizeStore((s) => s.setAutoLabelEnabled);
  const setAutoLabelTemplate = useOrganizeStore((s) => s.setAutoLabelTemplate);
  const setAutoLabelStartCounter = useOrganizeStore((s) => s.setAutoLabelStartCounter);
  const setAutoLabelCounterPadding = useOrganizeStore((s) => s.setAutoLabelCounterPadding);
  const setSkipModificationDateCheck = useOrganizeStore((s) => s.setSkipModificationDateCheck);
  const setAutoContinueOnBrokenMedia = useOrganizeStore((s) => s.setAutoContinueOnBrokenMedia);

  const [patternsText, setPatternsText] = useState(selectiveCopy.patterns.join(", "));
  const [excludedExtText, setExcludedExtText] = useState(
    contentDateExcludedExtensions.join(", "),
  );
  const [bundleEnabled, setBundleEnabled] = useState(!!bundleIgnore);
  const [bundleName, setBundleName] = useState(bundleIgnore?.name ?? "");
  const [bundleMaxMb, setBundleMaxMb] = useState(
    bundleIgnore ? (bundleIgnore.maxSizeBytes / (1024 * 1024)).toString() : "50",
  );
  const [newElementName, setNewElementName] = useState("");

  function commitNewElement() {
    if (newElementName.trim() === "") return;
    addElement(newElementName);
    setNewElementName("");
  }

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

  const autoLabelPreview = renderTemplate(autoLabel.template, {
    ...PREVIEW_CONTEXT,
    counter: autoLabel.startCounter,
    counterPadding: autoLabel.counterPadding,
    jobStarted: effectiveJobDate(NOW, dateOverride),
    elements,
  });

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
      elements,
      autoLabel,
      skipModificationDateCheck,
      autoContinueOnBrokenMedia,
    },
    {
      ...PREVIEW_CONTEXT,
      counterPadding,
      jobStarted: effectiveJobDate(NOW, dateOverride),
      elements,
    },
  );

  const allTokens: TemplateToken[] = [
    ...BUILTIN_TOKENS,
    ...elements
      .filter((e) => e.name.trim() !== "")
      .map((e) => ({ name: e.name.trim(), group: "Elements" })),
  ];

  return (
    <div className="flex flex-col gap-6">
      <PresetsSection />

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Organize
        </h3>

        <div className="flex flex-col gap-1 text-xs">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Rename template
          </span>
          <TemplateBuilder
            value={renameTemplate ?? ""}
            onChange={setRenameTemplate}
            tokens={allTokens}
            placeholder="Keep original filename…"
          />
        </div>

        <div className="flex flex-col gap-1 text-xs">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Folder template
          </span>
          <TemplateBuilder
            value={folderTemplate ?? ""}
            onChange={setFolderTemplate}
            tokens={allTokens}
            disabled={flatten}
            placeholder="Keep original folder structure…"
          />
        </div>

        <p
          className="truncate rounded border border-neutral-800 bg-neutral-900 px-2 py-1 font-mono text-[10px] text-neutral-400"
          title={preview}
        >
          Preview: {preview}
        </p>

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

        <label className="flex items-start gap-2 text-xs">
          <input
            type="checkbox"
            checked={skipModificationDateCheck}
            onChange={(e) => setSkipModificationDateCheck(e.currentTarget.checked)}
            className="mt-0.5"
          />
          <span>
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">
              Skip Modification Date Check
            </span>
            <span className="block text-neutral-500">
              Duplicate Detection compares name + size only -- for workflows where a file's
              modified time can't be trusted to still match.
            </span>
          </span>
        </label>

        <label className="flex items-start gap-2 text-xs">
          <input
            type="checkbox"
            checked={autoContinueOnBrokenMedia}
            onChange={(e) => setAutoContinueOnBrokenMedia(e.currentTarget.checked)}
            className="mt-0.5"
          />
          <span>
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">
              Auto-Continue on Broken Media
            </span>
            <span className="block text-neutral-500">
              Skips the alert when a 0-byte file is found on the source and copies anyway. Off by
              default, so a dropped card gets flagged before anything is copied.
            </span>
          </span>
        </label>

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
            Elements (custom tokens)
          </span>

          {elements.map((element) => (
            <div key={element.name} className="flex items-center gap-2">
              <span
                className="min-w-0 flex-1 truncate font-mono text-xs text-neutral-400"
                title={`{${element.name}}`}
              >
                {`{${element.name}}`}
              </span>
              <button
                type="button"
                onClick={() => removeElement(element.name)}
                title={`Remove {${element.name}}`}
                className="rounded border border-neutral-700 px-2 py-1 text-xs hover:bg-neutral-800"
              >
                ×
              </button>
            </div>
          ))}

          <div className="flex gap-2">
            <input
              value={newElementName}
              onChange={(e) => setNewElementName(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitNewElement();
              }}
              placeholder="New element name (e.g. Location)…"
              autoComplete="off"
              className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
            />
            <button
              type="button"
              onClick={commitNewElement}
              className="rounded border border-neutral-700 px-2 py-1 text-xs hover:bg-neutral-800"
            >
              + Add
            </button>
          </div>
          <p className="text-[10px] leading-relaxed text-neutral-600">
            Type the token (e.g. {"{Location}"}) into a Rename or Folder template above to use it.
            Its per-job value is entered in the Elements panel on the Disks view, not here.
          </p>
        </div>

        <div className="flex flex-col gap-1">
          <label className="flex items-center gap-2 text-xs">
            <input
              type="checkbox"
              checked={autoLabel.enabled}
              onChange={(e) => {
                setAutoLabelEnabled(e.currentTarget.checked);
                useDisksStore.getState().recomputeAutoLabels();
              }}
            />
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">
              Auto Label new sources
            </span>
          </label>

          {autoLabel.enabled && (
            <>
              <input
                value={autoLabel.template}
                onChange={(e) => {
                  setAutoLabelTemplate(e.currentTarget.value);
                  useDisksStore.getState().recomputeAutoLabels();
                }}
                placeholder="{Source Name}_{Counter}"
                autoComplete="off"
                className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
              />
              <p
                className="truncate rounded border border-neutral-800 bg-neutral-900 px-2 py-1 font-mono text-[10px] text-neutral-400"
                title={autoLabelPreview}
              >
                Preview: {autoLabelPreview}
              </p>
              <div className="flex items-center gap-3 text-xs">
                <label className="flex items-center gap-2">
                  <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                    Start at
                  </span>
                  <input
                    type="number"
                    min={0}
                    value={autoLabel.startCounter}
                    onChange={(e) => {
                      setAutoLabelStartCounter(Number.parseInt(e.currentTarget.value, 10));
                      useDisksStore.getState().recomputeAutoLabels();
                    }}
                    className="w-16 rounded border border-neutral-700 bg-neutral-950 px-2 py-1"
                  />
                </label>
                <label className="flex items-center gap-2">
                  <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                    Padding
                  </span>
                  <input
                    type="number"
                    min={1}
                    max={8}
                    value={autoLabel.counterPadding}
                    onChange={(e) => {
                      setAutoLabelCounterPadding(Number.parseInt(e.currentTarget.value, 10));
                      useDisksStore.getState().recomputeAutoLabels();
                    }}
                    className="w-14 rounded border border-neutral-700 bg-neutral-950 px-2 py-1"
                  />
                </label>
              </div>
            </>
          )}
        </div>
      </section>
    </div>
  );
}
