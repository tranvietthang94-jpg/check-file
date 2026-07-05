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
  const [logoFileName, setLogoFileName] = useState<string | null>(null);
  const [logoError, setLogoError] = useState<string | null>(null);
  const [includeThumbnails, setIncludeThumbnails] = useState(false);
  const [logoInputKey, setLogoInputKey] = useState(0);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  function toggleSelected(jobId: string) {
    setSelectedIds((prev) =>
      prev.includes(jobId) ? prev.filter((id) => id !== jobId) : [...prev, jobId],
    );
  }

  const MAX_LOGO_BYTES = 5 * 1024 * 1024;

  async function handleLogoChange(file: File | undefined) {
    if (!file) {
      return;
    }
    if (file.size > MAX_LOGO_BYTES) {
      setLogoError("Ảnh logo quá lớn (tối đa 5 MB) -- chọn tệp nhỏ hơn.");
      setLogoInputKey((k) => k + 1);
      return;
    }
    try {
      setLogoDataUrl(await readAsDataUrl(file));
      setLogoFileName(file.name);
      setLogoError(null);
    } catch {
      setLogoDataUrl(null);
      setLogoFileName(null);
      setLogoError("Không đọc được ảnh đó -- thử tệp khác.");
    }
  }

  function clearLogo() {
    setLogoDataUrl(null);
    setLogoFileName(null);
    setLogoError(null);
    setLogoInputKey((k) => k + 1);
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
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">Báo cáo</h2>

      {logs.length === 0 ? (
        <p className="text-xs text-neutral-500">
          Chưa có lượt truyền nào hoàn tất -- Báo cáo tổng hợp một hoặc nhiều mục trong Nhật ký
          truyền tải.
        </p>
      ) : (
        <>
          <div className="flex flex-col gap-1">
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">
              Đưa vào báo cáo
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
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">Tiêu đề</span>
              <input
                value={title}
                onChange={(e) => setTitle(e.currentTarget.value)}
                placeholder="Báo cáo truyền tải"
                autoComplete="off"
                className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
              />
            </label>

            <div className="flex flex-col gap-1 text-xs">
              <label className="flex flex-col gap-1">
                <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                  Logo (tùy chọn)
                </span>
                <div className="flex items-center gap-2">
                  <input
                    key={logoInputKey}
                    type="file"
                    accept="image/*"
                    title="Ảnh logo báo cáo"
                    onChange={(e) => handleLogoChange(e.currentTarget.files?.[0])}
                    className="text-xs"
                  />
                  {logoDataUrl && (
                    <button
                      type="button"
                      onClick={clearLogo}
                      className="shrink-0 rounded border border-neutral-700 px-1.5 py-0.5 text-[10px] text-neutral-400 hover:text-neutral-200"
                    >
                      Xóa
                    </button>
                  )}
                </div>
              </label>
              {logoDataUrl && (
                <div className="flex items-center gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1">
                  <img
                    src={logoDataUrl}
                    alt="Xem trước logo"
                    className="h-8 max-w-[120px] object-contain"
                  />
                  <span className="truncate text-[10px] text-neutral-500">{logoFileName}</span>
                </div>
              )}
              {logoError && <span className="text-[10px] text-red-400">{logoError}</span>}
            </div>
          </div>

          <label className="flex flex-col gap-1 text-xs">
            <span className="text-[10px] uppercase tracking-wide text-neutral-500">Ghi chú</span>
            <textarea
              value={notes}
              onChange={(e) => setNotes(e.currentTarget.value)}
              placeholder="Ghi chú tùy chọn cho báo cáo này…"
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
            Kèm ảnh thu nhỏ clip (chậm hơn -- đọc lại tệp tại đích)
          </label>

          <button
            type="button"
            disabled={selectedIds.length === 0 || busy}
            onClick={handleGenerate}
            className="w-fit rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
          >
            {busy ? "Đang tạo…" : "Tạo báo cáo"}
          </button>

          {status && (
            <p className="text-[10px] text-neutral-500">
              Đã lưu tại {status} -- đã mở trong trình duyệt. Dùng In &rarr; Lưu dưới dạng PDF để có
              bản PDF.
            </p>
          )}
          {error && <p className="text-[10px] text-red-400">{error}</p>}
        </>
      )}
    </section>
  );
}
