import { useMhlVerifyStore } from "../state/mhlVerifyStore";
import type { MhlEntryStatus, MhlVerifyReport } from "../types/mhl";

const STATUS_LABEL: Record<MhlEntryStatus, string> = {
  verified: "Đã xác minh",
  mismatch: "Sai lệch mã băm",
  missing: "Bị thiếu",
  sizeMismatch: "Sai lệch kích thước",
  noChecksumRecorded: "Không có mã băm ghi nhận",
};

const STATUS_CLASS: Record<MhlEntryStatus, string> = {
  verified: "text-green-400",
  mismatch: "text-red-400",
  missing: "text-red-400",
  sizeMismatch: "text-orange-400",
  noChecksumRecorded: "text-neutral-400",
};

function ReportCard({ report }: { report: MhlVerifyReport }) {
  const problems = report.results.filter((r) => r.status !== "verified" && r.status !== "noChecksumRecorded");
  return (
    <div className="rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs">
      <div className="flex items-center justify-between gap-2">
        <span className="truncate font-mono" title={report.mhlPath}>
          {report.mhlPath}
        </span>
        <span className={problems.length === 0 ? "text-green-400" : "text-red-400"}>
          {problems.length === 0
            ? `${report.results.length} hợp lệ`
            : `${problems.length} vấn đề`}
        </span>
      </div>
      <ul className="mt-1 flex flex-col gap-0.5">
        {report.results.map((r) => (
          <li key={r.relativePath} className="flex items-center justify-between gap-2">
            <span className="truncate text-neutral-400">{r.relativePath}</span>
            <span className={STATUS_CLASS[r.status]}>{STATUS_LABEL[r.status]}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function MhlVerifyPanel() {
  const path = useMhlVerifyStore((s) => s.path);
  const mode = useMhlVerifyStore((s) => s.mode);
  const busy = useMhlVerifyStore((s) => s.busy);
  const reports = useMhlVerifyStore((s) => s.reports);
  const error = useMhlVerifyStore((s) => s.error);
  const setPath = useMhlVerifyStore((s) => s.setPath);
  const setMode = useMhlVerifyStore((s) => s.setMode);
  const runVerify = useMhlVerifyStore((s) => s.runVerify);

  return (
    <section className="col-span-full flex flex-col gap-2">
      <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
        Xác minh MHL
      </h2>
      <p className="text-xs text-neutral-500">
        Kiểm tra lại mã băm đã ghi trong một tệp .mhl có sẵn so với các tệp thật trên đĩa -- không
        cần chạy lượt truyền nào.
      </p>

      <div className="flex flex-wrap items-center gap-2">
        <label className="flex items-center gap-1 text-xs">
          <input
            type="radio"
            name="mhl-verify-mode"
            checked={mode === "file"}
            onChange={() => setMode("file")}
          />
          Một tệp .mhl
        </label>
        <label className="flex items-center gap-1 text-xs">
          <input
            type="radio"
            name="mhl-verify-mode"
            checked={mode === "folder"}
            onChange={() => setMode("folder")}
          />
          Tất cả tệp .mhl trong một thư mục
        </label>
      </div>

      <div className="flex items-center gap-2">
        <input
          value={path}
          onChange={(e) => setPath(e.currentTarget.value)}
          placeholder={mode === "file" ? "Đường dẫn đầy đủ tới tệp .mhl…" : "Đường dẫn thư mục đầy đủ…"}
          autoComplete="off"
          className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
        />
        <button
          type="button"
          disabled={!path.trim() || busy}
          onClick={() => runVerify()}
          className="w-fit shrink-0 rounded border border-neutral-700 px-2 py-1 text-xs disabled:opacity-40"
        >
          {busy ? "Đang xác minh…" : "Xác minh"}
        </button>
      </div>

      {error && <p className="text-[10px] text-red-400">{error}</p>}

      {reports && (
        <div className="flex flex-col gap-2">
          {reports.length === 0 ? (
            <p className="text-xs text-neutral-500">Không tìm thấy tệp .mhl nào ở đó.</p>
          ) : (
            reports.map((report) => <ReportCard key={report.mhlPath} report={report} />)
          )}
        </div>
      )}
    </section>
  );
}
