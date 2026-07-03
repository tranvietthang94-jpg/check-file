import { useEffect, useState } from "react";
import { usePresetsStore } from "../state/presetsStore";
import { useSettingsStore } from "../state/settingsStore";
import { useOrganizeStore } from "../state/organizeStore";

export function PresetsPanel() {
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
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Presets
      </h2>

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
