import { useSettingsStore } from "../../state/settingsStore";
import { SectionHeading } from "../ui/SectionHeading";
import { Checkbox, Radio } from "../ui/Checkbox";
import type { ChecksumAlgorithm, VerificationMode } from "../../types/job";
import type { QueueMode } from "../../types/queue";

const CARD_LABEL = "rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5";

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
  { value: "xxh3", label: "XXH3" },
  { value: "xxh128", label: "XXH128" },
  { value: "md5", label: "MD5" },
  { value: "sha1", label: "SHA-1" },
  { value: "c4", label: "C4" },
];

const LEGACY_ALGORITHMS: { value: ChecksumAlgorithm; label: string }[] = [
  { value: "sha1", label: "SHA-1" },
  { value: "md5", label: "MD5" },
  { value: "c4", label: "C4" },
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
        <SectionHeading as="h3">Truyền tải</SectionHeading>
        <Checkbox
          align="start"
          className={CARD_LABEL}
          checked={moveSameVolume}
          onChange={(e) => setMoveSameVolume(e.currentTarget.checked)}
          label={
            <span className="flex flex-col">
              <span className="font-medium">
                Di chuyển thay vì sao chép khi Nguồn và Đích cùng ổ đĩa
              </span>
              <span className="text-neutral-500">
                Di chuyển tệp ngay lập tức thay vì nhân bản, khi có thể.
              </span>
            </span>
          }
        />
      </section>

      <section className="flex flex-col gap-2">
        <SectionHeading as="h3">Xác minh</SectionHeading>
        <div className="flex flex-col gap-1">
          {VERIFICATION_MODES.map((m) => (
            <Radio
              key={m.value}
              align="start"
              className={CARD_LABEL}
              name="verification-mode"
              checked={verificationMode === m.value}
              onChange={() => setVerificationMode(m.value)}
              label={
                <span className="flex flex-col">
                  <span className="font-medium">{m.label}</span>
                  <span className="text-neutral-500">{m.hint}</span>
                </span>
              }
            />
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

        <Checkbox
          className={CARD_LABEL}
          checked={legacyChecksumEnabled}
          disabled={verificationMode === "transfer"}
          onChange={(e) => setLegacyChecksumEnabled(e.currentTarget.checked)}
          label={
            <span className="flex items-center gap-2">
              Tạo thêm mã băm cũ:
              <select
                title="Thuật toán mã băm cũ"
                value={legacyChecksumAlgorithm}
                onChange={(e) =>
                  setLegacyChecksumAlgorithm(e.currentTarget.value as ChecksumAlgorithm)
                }
                disabled={!legacyChecksumEnabled || verificationMode === "transfer"}
                className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 disabled:opacity-40"
              >
                {LEGACY_ALGORITHMS.map((a) => (
                  <option key={a.value} value={a.value}>
                    {a.label}
                  </option>
                ))}
              </select>
            </span>
          }
        />
      </section>

      <section className="flex flex-col gap-2">
        <SectionHeading as="h3">Hàng đợi</SectionHeading>
        <div className="flex flex-col gap-1">
          {QUEUE_MODES.map((m) => (
            <Radio
              key={m.value}
              align="start"
              className={CARD_LABEL}
              name="queue-mode"
              checked={queueMode === m.value}
              onChange={() => setQueueMode(m.value)}
              label={
                <span className="flex flex-col">
                  <span className="font-medium">{m.label}</span>
                  <span className="text-neutral-500">{m.hint}</span>
                </span>
              }
            />
          ))}
        </div>
      </section>

      <section className="flex flex-col gap-2">
        <SectionHeading as="h3">Hồ sơ lưu trữ</SectionHeading>
        <Checkbox
          align="start"
          className={CARD_LABEL}
          checked={saveLogToDestination}
          onChange={(e) => setSaveLogToDestination(e.currentTarget.checked)}
          label={
            <span className="flex flex-col">
              <span className="font-medium">Lưu thêm Nhật ký truyền tải ở Đích</span>
              <span className="text-neutral-500">Nhật ký truyền tải luôn được lưu cục bộ.</span>
            </span>
          }
        />
        <Checkbox
          align="start"
          className={CARD_LABEL}
          checked={createPerFileMhl}
          onChange={(e) => setCreatePerFileMhl(e.currentTarget.checked)}
          label={
            <span className="flex flex-col">
              <span className="font-medium">Tạo thêm MHL riêng cho từng tệp</span>
              <span className="text-neutral-500">
                Một MHL đi kèm mỗi tệp, bên cạnh MHL gộp chung.
              </span>
            </span>
          }
        />
      </section>
    </div>
  );
}
