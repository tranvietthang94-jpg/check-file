import { useMhlVerifyStore } from "../state/mhlVerifyStore";
import { Panel } from "./ui/Panel";
import { SectionHeading } from "./ui/SectionHeading";
import { EmptyState } from "./ui/EmptyState";
import { Button } from "./ui/Button";
import { Radio } from "./ui/Checkbox";
import { Badge, type BadgeTone } from "./ui/Badge";
import { ShieldCheck } from "./icons";
import type { MhlEntryStatus, MhlVerifyReport } from "../types/mhl";

const STATUS_LABEL: Record<MhlEntryStatus, string> = {
  verified: "Đã xác minh",
  mismatch: "Sai lệch mã băm",
  missing: "Bị thiếu",
  sizeMismatch: "Sai lệch kích thước",
  noChecksumRecorded: "Không có mã băm ghi nhận",
};

const STATUS_TONE: Record<MhlEntryStatus, BadgeTone> = {
  verified: "green",
  mismatch: "red",
  missing: "red",
  sizeMismatch: "orange",
  noChecksumRecorded: "neutral",
};

function ReportCard({ report }: { report: MhlVerifyReport }) {
  const problems = report.results.filter((r) => r.status !== "verified" && r.status !== "noChecksumRecorded");
  return (
    <Panel className="px-2 py-1.5 text-xs">
      <div className="flex items-center justify-between gap-2">
        <span className="truncate font-mono" title={report.mhlPath}>
          {report.mhlPath}
        </span>
        <Badge tone={problems.length === 0 ? "green" : "red"}>
          {problems.length === 0
            ? `${report.results.length} hợp lệ`
            : `${problems.length} vấn đề`}
        </Badge>
      </div>
      <ul className="mt-1 flex flex-col gap-0.5">
        {report.results.map((r) => (
          <li key={r.relativePath} className="flex items-center justify-between gap-2">
            <span className="truncate text-neutral-400">{r.relativePath}</span>
            <Badge tone={STATUS_TONE[r.status]}>{STATUS_LABEL[r.status]}</Badge>
          </li>
        ))}
      </ul>
    </Panel>
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
      <SectionHeading>Xác minh MHL</SectionHeading>
      <p className="text-xs text-neutral-500">
        Kiểm tra lại mã băm đã ghi trong một tệp .mhl có sẵn so với các tệp thật trên đĩa -- không
        cần chạy lượt truyền nào.
      </p>

      <div className="flex flex-wrap items-center gap-3">
        <Radio
          name="mhl-verify-mode"
          checked={mode === "file"}
          onChange={() => setMode("file")}
          label="Một tệp .mhl"
        />
        <Radio
          name="mhl-verify-mode"
          checked={mode === "folder"}
          onChange={() => setMode("folder")}
          label="Tất cả tệp .mhl trong một thư mục"
        />
      </div>

      <div className="flex items-center gap-2">
        <input
          value={path}
          onChange={(e) => setPath(e.currentTarget.value)}
          placeholder={mode === "file" ? "Đường dẫn đầy đủ tới tệp .mhl…" : "Đường dẫn thư mục đầy đủ…"}
          autoComplete="off"
          className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
        />
        <Button
          variant="secondary"
          icon={<ShieldCheck className="h-3.5 w-3.5" />}
          disabled={!path.trim() || busy}
          onClick={() => runVerify()}
          className="w-fit shrink-0"
        >
          {busy ? "Đang xác minh…" : "Xác minh"}
        </Button>
      </div>

      {error && <p className="text-[10px] text-red-400">{error}</p>}

      {reports && (
        <div className="flex flex-col gap-2">
          {reports.length === 0 ? (
            <EmptyState icon={<ShieldCheck className="h-5 w-5" />}>
              Không tìm thấy tệp .mhl nào ở đó.
            </EmptyState>
          ) : (
            reports.map((report) => <ReportCard key={report.mhlPath} report={report} />)
          )}
        </div>
      )}
    </section>
  );
}
