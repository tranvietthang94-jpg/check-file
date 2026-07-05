import { useSettingsStore } from "../../state/settingsStore";
import type { ChecksumAlgorithm, VerificationMode } from "../../types/job";
import type { QueueMode } from "../../types/queue";

const VERIFICATION_MODES: { value: VerificationMode; label: string; hint: string }[] = [
  { value: "transfer", label: "Chỉ Truyền", hint: "Chỉ kiểm tra kích thước, nhanh nhất" },
  { value: "source", label: "Nguồn", hint: "Băm nguồn trong khi sao chép" },
  {
    value: "sourceAndDestination",
    label: "Nguồn & Đích",
    hint: "Băm cả hai, so sánh (an toàn nhất)",
  },
];

const ALGORITHMS: { value: ChecksumAlgorithm; label: string }[] = [
  { value: "xxh64", label: "XXH64" },
  { value: "md5", label: "MD5" },
  { value: "sha1", label: "SHA-1" },
];

const LEGACY_ALGORITHMS: { value: ChecksumAlgorithm; label: string }[] = [
  { value: "sha1", label: "SHA-1" },
  { value: "md5", label: "MD5" },
];

const QUEUE_MODES: { value: QueueMode; label: string; hint: string }[] = [
  { value: "off", label: "Tắt", hint: "Mọi lượt truyền bắt đầu ngay lập tức" },
  {
    value: "singleSource",
    label: "Từng Nguồn",
    hint: "Xử lý các đích của một nguồn tại một thời điểm; nguồn kế tiếp tự động bắt đầu",
  },
  {
    value: "singleDestination",
    label: "Từng Đích",
    hint: "Mỗi nguồn xử lý từng đích một",
  },
  { value: "singleTransfer", label: "Từng Lượt truyền", hint: "Một công việc tại một thời điểm, toàn ứng dụng" },
];

export function TransfersPreferences() {
  const verificationMode = useSettingsStore((s) => s.verificationMode);
  const checksumAlgorithm = useSettingsStore((s) => s.checksumAlgorithm);
  const queueMode = useSettingsStore((s) => s.queueMode);
  const moveSameVolume = useSettingsStore((s) => s.moveSameVolume);
  const legacyChecksumEnabled = useSettingsStore((s) => s.legacyChecksumEnabled);
  const legacyChecksumAlgorithm = useSettingsStore((s) => s.legacyChecksumAlgorithm);
  const saveLogToDestination = useSettingsStore((s) => s.saveLogToDestination);
  const createPerFileMhl = useSettingsStore((s) => s.createPerFileMhl);
  const setVerificationMode = useSettingsStore((s) => s.setVerificationMode);
  const setChecksumAlgorithm = useSettingsStore((s) => s.setChecksumAlgorithm);
  const setQueueMode = useSettingsStore((s) => s.setQueueMode);
  const setMoveSameVolume = useSettingsStore((s) => s.setMoveSameVolume);
  const setLegacyChecksumEnabled = useSettingsStore((s) => s.setLegacyChecksumEnabled);
  const setLegacyChecksumAlgorithm = useSettingsStore((s) => s.setLegacyChecksumAlgorithm);
  const setSaveLogToDestination = useSettingsStore((s) => s.setSaveLogToDestination);
  const setCreatePerFileMhl = useSettingsStore((s) => s.setCreatePerFileMhl);

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Truyền tải
        </h3>
        <label className="flex cursor-pointer items-start gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs">
          <input
            type="checkbox"
            checked={moveSameVolume}
            onChange={(e) => setMoveSameVolume(e.currentTarget.checked)}
            className="mt-0.5"
          />
          <span className="flex flex-col">
            <span className="font-medium">
              Di chuyển thay vì sao chép khi Nguồn và Đích cùng ổ đĩa
            </span>
            <span className="text-neutral-500">
              Di chuyển tệp ngay lập tức thay vì nhân bản, khi có thể.
            </span>
          </span>
        </label>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Xác minh
        </h3>
        <div className="flex flex-col gap-1">
          {VERIFICATION_MODES.map((m) => (
            <label
              key={m.value}
              className="flex cursor-pointer items-start gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs"
            >
              <input
                type="radio"
                name="verification-mode"
                checked={verificationMode === m.value}
                onChange={() => setVerificationMode(m.value)}
                className="mt-0.5"
              />
              <span className="flex flex-col">
                <span className="font-medium">{m.label}</span>
                <span className="text-neutral-500">{m.hint}</span>
              </span>
            </label>
          ))}
        </div>

        <label className="flex flex-col gap-1 text-xs">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Thuật toán mã băm
          </span>
          <select
            value={checksumAlgorithm}
            onChange={(e) => setChecksumAlgorithm(e.currentTarget.value as ChecksumAlgorithm)}
            disabled={verificationMode === "transfer"}
            className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 disabled:opacity-40"
          >
            {ALGORITHMS.map((a) => (
              <option key={a.value} value={a.value}>
                {a.label}
              </option>
            ))}
          </select>
        </label>

        <label className="flex cursor-pointer items-center gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs">
          <input
            type="checkbox"
            checked={legacyChecksumEnabled}
            disabled={verificationMode === "transfer"}
            onChange={(e) => setLegacyChecksumEnabled(e.currentTarget.checked)}
          />
          <span>Tạo thêm mã băm cũ:</span>
          <select
            value={legacyChecksumAlgorithm}
            onChange={(e) => setLegacyChecksumAlgorithm(e.currentTarget.value as ChecksumAlgorithm)}
            disabled={!legacyChecksumEnabled || verificationMode === "transfer"}
            className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 disabled:opacity-40"
          >
            {LEGACY_ALGORITHMS.map((a) => (
              <option key={a.value} value={a.value}>
                {a.label}
              </option>
            ))}
          </select>
        </label>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Hàng đợi
        </h3>
        <div className="flex flex-col gap-1">
          {QUEUE_MODES.map((m) => (
            <label
              key={m.value}
              className="flex cursor-pointer items-start gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs"
            >
              <input
                type="radio"
                name="queue-mode"
                checked={queueMode === m.value}
                onChange={() => setQueueMode(m.value)}
                className="mt-0.5"
              />
              <span className="flex flex-col">
                <span className="font-medium">{m.label}</span>
                <span className="text-neutral-500">{m.hint}</span>
              </span>
            </label>
          ))}
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Hồ sơ lưu trữ
        </h3>
        <label className="flex cursor-pointer items-start gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs">
          <input
            type="checkbox"
            checked={saveLogToDestination}
            onChange={(e) => setSaveLogToDestination(e.currentTarget.checked)}
            className="mt-0.5"
          />
          <span className="flex flex-col">
            <span className="font-medium">Lưu thêm Nhật ký truyền tải ở Đích</span>
            <span className="text-neutral-500">
              Nhật ký truyền tải luôn được lưu cục bộ.
            </span>
          </span>
        </label>
        <label className="flex cursor-pointer items-start gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs">
          <input
            type="checkbox"
            checked={createPerFileMhl}
            onChange={(e) => setCreatePerFileMhl(e.currentTarget.checked)}
            className="mt-0.5"
          />
          <span className="flex flex-col">
            <span className="font-medium">Tạo thêm MHL riêng cho từng tệp</span>
            <span className="text-neutral-500">
              Một MHL đi kèm mỗi tệp, bên cạnh MHL gộp chung.
            </span>
          </span>
        </label>
      </section>
    </div>
  );
}
