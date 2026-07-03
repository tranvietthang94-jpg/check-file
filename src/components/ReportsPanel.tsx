import { useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { useTransferLogStore } from "../state/transferLogStore";
import { generateReport } from "../lib/tauri";

function formatTimestamp(iso: string): string {
  const parsed = new Date(iso);
  return Number.isNaN(parsed.getTime()) ? iso : parsed.toLocaleString();
}

function readAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

export function ReportsPanel() {
  const logs = useTransferLogStore((s) => s.logs);

  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [title, setTitle] = useState("");
  const [notes, setNotes] = useState("");
  const [logoDataUrl, setLogoDataUrl] = useState<string | null>(null);
  const [includeThumbnails, setIncludeThumbnails] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  function toggleSelected(jobId: string) {
    setSelectedIds((prev) =>
      prev.includes(jobId) ? prev.filter((id) => id !== jobId) : [...prev, jobId],
    );
  }

  async function handleLogoChange(file: File | undefined) {
    if (!file) {
      setLogoDataUrl(null);
      return;
    }
    try {
      setLogoDataUrl(await readAsDataUrl(file));
    } catch {
      setLogoDataUrl(null);
    }
  }

  async function handleGenerate() {
    if (selectedIds.length === 0) return;
    setBusy(true);
    setError(null);
    setStatus(null);
    try {
      const path = await generateReport({
        jobIds: selectedIds,
        title,
        notes,
        logoDataUrl,
        includeThumbnails,
      });
      setStatus(path);
      await openPath(path);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="col-span-full flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">Reports</h2>

      {logs.length === 0 ? (
        <p className="text-xs text-neutral-500">
          No completed transfers yet -- Reports summarize one or more Transfer Log entries.
        </p>
      ) : (
        <>
          <div className="flex flex-col gap-1">
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">
              Include in report
            </span>
            <ul className="grid grid-cols-1 gap-1 sm:grid-cols-2 lg:grid-cols-3">
              {logs.map((entry) => (
                <li key={entry.jobId}>
                  <label className="flex items-start gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1 text-xs">
                    <input
                      type="checkbox"
                      checked={selectedIds.includes(entry.jobId)}
                      onChange={() => toggleSelected(entry.jobId)}
                      className="mt-0.5"
                    />
                    <span className="flex flex-col">
                      <span className="font-medium">{entry.sourceName}</span>
                      <span className="text-neutral-500">{formatTimestamp(entry.finishedAt)}</span>
                    </span>
                  </label>
                </li>
              ))}
            </ul>
          </div>

          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <label className="flex flex-col gap-1 text-xs">
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">Title</span>
              <input
                value={title}
                onChange={(e) => setTitle(e.currentTarget.value)}
                placeholder="Transfer Report"
                autoComplete="off"
                className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
              />
            </label>

            <label className="flex flex-col gap-1 text-xs">
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">Logo</span>
              <input
                type="file"
                accept="image/*"
                title="Report logo image"
                onChange={(e) => handleLogoChange(e.currentTarget.files?.[0])}
                className="text-xs"
              />
            </label>
          </div>

          <label className="flex flex-col gap-1 text-xs">
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">Notes</span>
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.currentTarget.value)}
              placeholder="Optional notes for this report…"
              rows={2}
              className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
            />
          </label>

          <label className="flex items-center gap-2 text-xs">
            <input
              type="checkbox"
              checked={includeThumbnails}
              onChange={(e) => setIncludeThumbnails(e.currentTarget.checked)}
            />
            Include clip thumbnails (slower -- re-reads files at their destination)
          </label>

          <button
            type="button"
            disabled={selectedIds.length === 0 || busy}
            onClick={handleGenerate}
            className="w-fit rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
          >
            {busy ? "Generating…" : "Generate Report"}
          </button>

          {status && (
            <p className="text-[10px] text-neutral-500">
              Saved to {status} -- opened in your browser. Use Print &rarr; Save as PDF for a PDF copy.
            </p>
          )}
          {error && <p className="text-[10px] text-red-400">{error}</p>}
        </>
      )}
    </section>
  );
}
