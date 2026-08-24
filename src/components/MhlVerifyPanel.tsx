import { useEffect } from "react";
import { useMhlVerifyStore } from "../state/mhlVerifyStore";
import { useTransferLogStore } from "../state/transferLogStore";
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

function ReportCard({
  report,
  busy,
  onRepair,
}: {
  report: MhlVerifyReport;
  busy: boolean;
  onRepair: (relativePath: string) => void;
}) {
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
            <span className="flex items-center gap-1">
              <Badge tone={STATUS_TONE[r.status]}>{STATUS_LABEL[r.status]}</Badge>
              {r.status !== "verified" && r.status !== "noChecksumRecorded" && (
                <Button
                  variant="secondary"
                  disabled={busy}
                  onClick={() => onRepair(r.relativePath)}
                  className="w-fit px-1.5 py-0.5 text-[10px]"
                >
                  Lập kế hoạch sửa
                </Button>
              )}
            </span>
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
  const repairPlan = useMhlVerifyStore((s) => s.repairPlan);
  const selectedCandidateRoot = useMhlVerifyStore((s) => s.selectedCandidateRoot);
  const manualCandidateRoot = useMhlVerifyStore((s) => s.manualCandidateRoot);
  const planRepair = useMhlVerifyStore((s) => s.planRepair);
  const closeRepairPlan = useMhlVerifyStore((s) => s.closeRepairPlan);
  const setManualCandidateRoot = useMhlVerifyStore((s) => s.setManualCandidateRoot);
  const selectCandidateRoot = useMhlVerifyStore((s) => s.selectCandidateRoot);
  const repairSelected = useMhlVerifyStore((s) => s.repairSelected);

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    (window as Window & { __OFFLOADKIT_MHL_TEST__?: (report: MhlVerifyReport) => void })
      .__OFFLOADKIT_MHL_TEST__ = (report) => useMhlVerifyStore.setState({ reports: [report] });
    return () => {
      delete (window as Window & { __OFFLOADKIT_MHL_TEST__?: unknown }).__OFFLOADKIT_MHL_TEST__;
    };
  }, []);

  const requestRepair = (report: MhlVerifyReport, relativePath: string) => {
    const logs = useTransferLogStore.getState().logs;
    const roots = logs
      .filter((log) => log.mhlPath === report.mhlPath || report.mhlPath.startsWith(log.destination))
      .flatMap((log) => [log.source, log.destination]);
    void planRepair(report.mhlPath, relativePath, roots);
  };

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

      {repairPlan && (
        <Panel className="flex flex-col gap-2 p-3" data-testid="repair-plan">
          <div className="flex items-center justify-between gap-2">
            <SectionHeading as="h3">Kế hoạch sửa: {repairPlan.relativePath}</SectionHeading>
            <Button variant="ghost" onClick={closeRepairPlan}>Đóng</Button>
          </div>
          <p className="text-[10px] text-neutral-500">
            Chỉ các bản có checksum {repairPlan.algorithm.toUpperCase()} khớp MHL mới được hiển thị.
          </p>
          {repairPlan.candidates.length === 0 ? (
            <EmptyState icon={<ShieldCheck className="h-5 w-5" />}>Không tìm thấy bản đã xác minh. Thêm một thư mục ứng viên rồi lập lại kế hoạch.</EmptyState>
          ) : (
            <div className="flex flex-col gap-1">
              {repairPlan.candidates.map((candidate) => (
                <label key={candidate.root} className="flex items-start gap-2 rounded border border-neutral-800 p-2 text-xs">
                  <input
                    type="radio"
                    name="repair-candidate"
                    checked={selectedCandidateRoot === candidate.root}
                    onChange={() => selectCandidateRoot(candidate.root)}
                  />
                  <span className="min-w-0">
                    <span className="block truncate font-mono">{candidate.root}</span>
                    <span className="block truncate text-[10px] text-green-500">{candidate.checksum}</span>
                  </span>
                </label>
              ))}
            </div>
          )}
          <div className="flex items-center gap-2">
            <input
              aria-label="Thư mục ứng viên sửa"
              value={manualCandidateRoot}
              onChange={(e) => setManualCandidateRoot(e.currentTarget.value)}
              placeholder="Thêm thư mục Source hoặc Destination khác…"
              className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
            />
            <Button
              variant="secondary"
              disabled={!manualCandidateRoot.trim() || busy}
              onClick={() => planRepair(repairPlan.mhlPath, repairPlan.relativePath, [manualCandidateRoot.trim()])}
            >
              Tìm lại
            </Button>
          </div>
          <Button
            disabled={!selectedCandidateRoot || busy}
            onClick={() => {
              if (window.confirm(`Thay ${repairPlan.relativePath} và giữ bản lỗi làm evidence?`)) {
                void repairSelected();
              }
            }}
          >
            Xác nhận sửa
          </Button>
        </Panel>
      )}

      {reports && (
        <div className="flex flex-col gap-2">
          {reports.length === 0 ? (
            <EmptyState icon={<ShieldCheck className="h-5 w-5" />}>
              Không tìm thấy tệp .mhl nào ở đó.
            </EmptyState>
          ) : (
            reports.map((report) => (
              <ReportCard
                key={report.mhlPath}
                report={report}
                busy={busy}
                onRepair={(relativePath) => requestRepair(report, relativePath)}
              />
            ))
          )}
        </div>
      )}
    </section>
  );
}
