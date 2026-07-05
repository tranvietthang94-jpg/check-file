import { useSettingsStore } from "../../state/settingsStore";
import type { ChecksumAlgorithm, VerificationMode } from "../../types/job";
import type { QueueMode } from "../../types/queue";

const VERIFICATION_MODES: { value: VerificationMode; label: string; hint: string }[] = [
  { value: "transfer", label: "Transfer", hint: "Size check only, fastest" },
  { value: "source", label: "Source", hint: "Hash source while copying" },
  {
    value: "sourceAndDestination",
    label: "Source & Destination",
    hint: "Hash both, compare (safest)",
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
  { value: "off", label: "Off", hint: "Every transfer starts immediately" },
  {
    value: "singleSource",
    label: "Single Source",
    hint: "One source's destinations at a time; next source auto-starts",
  },
  {
    value: "singleDestination",
    label: "Single Destination",
    hint: "One destination at a time per source",
  },
  { value: "singleTransfer", label: "Single Transfer", hint: "One job at a time, app-wide" },
];

export function TransfersPreferences() {
  const verificationMode = useSettingsStore((s) => s.verificationMode);
  const checksumAlgorithm = useSettingsStore((s) => s.checksumAlgorithm);
  const queueMode = useSettingsStore((s) => s.queueMode);
  const moveSameVolume = useSettingsStore((s) => s.moveSameVolume);
  const legacyChecksumEnabled = useSettingsStore((s) => s.legacyChecksumEnabled);
  const legacyChecksumAlgorithm = useSettingsStore((s) => s.legacyChecksumAlgorithm);
  const setVerificationMode = useSettingsStore((s) => s.setVerificationMode);
  const setChecksumAlgorithm = useSettingsStore((s) => s.setChecksumAlgorithm);
  const setQueueMode = useSettingsStore((s) => s.setQueueMode);
  const setMoveSameVolume = useSettingsStore((s) => s.setMoveSameVolume);
  const setLegacyChecksumEnabled = useSettingsStore((s) => s.setLegacyChecksumEnabled);
  const setLegacyChecksumAlgorithm = useSettingsStore((s) => s.setLegacyChecksumAlgorithm);

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Transfer
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
              Don't copy but move data when a Source and Destination are on the same volume
            </span>
            <span className="text-neutral-500">
              Relocates the file instantly instead of duplicating it, when possible.
            </span>
          </span>
        </label>
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Verification
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
            Checksum algorithm
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
          <span>Also generate legacy checksums:</span>
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
          Queuing
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
    </div>
  );
}
