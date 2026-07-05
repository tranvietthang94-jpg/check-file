import { create } from "zustand";
import { setPreventSleepEnabled, setQueueMode as setQueueModeBackend } from "../lib/tauri";
import type { ChecksumAlgorithm, VerificationMode } from "../types/job";
import type { QueueMode } from "../types/queue";

interface SettingsState {
  verificationMode: VerificationMode;
  checksumAlgorithm: ChecksumAlgorithm;
  preventSleep: boolean;
  desktopNotifications: boolean;
  queueMode: QueueMode;
  /** OffShoot's "Don't copy but move data when a Source and Destination are
   * located on the same volume" -- an `fs::rename` fast path, distinct from
   * the per-transfer Move checkbox which always fully copies then deletes
   * (needed across different volumes). */
  moveSameVolume: boolean;
  /** OffShoot's "Also generate legacy checksums" -- a second hash computed
   * alongside the primary `checksumAlgorithm`, for interop with tooling
   * that expects an older algorithm (e.g. plain MD5/SHA-1 MHL readers). */
  legacyChecksumEnabled: boolean;
  legacyChecksumAlgorithm: ChecksumAlgorithm;
  setVerificationMode: (mode: VerificationMode) => void;
  setChecksumAlgorithm: (algorithm: ChecksumAlgorithm) => void;
  setPreventSleep: (enabled: boolean) => void;
  setDesktopNotifications: (enabled: boolean) => void;
  setQueueMode: (mode: QueueMode) => void;
  setMoveSameVolume: (enabled: boolean) => void;
  setLegacyChecksumEnabled: (enabled: boolean) => void;
  setLegacyChecksumAlgorithm: (algorithm: ChecksumAlgorithm) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  verificationMode: "sourceAndDestination",
  checksumAlgorithm: "xxh64",
  preventSleep: true,
  desktopNotifications: true,
  queueMode: "off",
  moveSameVolume: false,
  legacyChecksumEnabled: false,
  legacyChecksumAlgorithm: "sha1",
  setVerificationMode: (verificationMode) => set({ verificationMode }),
  setChecksumAlgorithm: (checksumAlgorithm) => set({ checksumAlgorithm }),
  setMoveSameVolume: (moveSameVolume) => set({ moveSameVolume }),
  setLegacyChecksumEnabled: (legacyChecksumEnabled) => set({ legacyChecksumEnabled }),
  setLegacyChecksumAlgorithm: (legacyChecksumAlgorithm) => set({ legacyChecksumAlgorithm }),
  setPreventSleep: (preventSleep) => {
    set({ preventSleep });
    setPreventSleepEnabled(preventSleep).catch(console.error);
  },
  setDesktopNotifications: (desktopNotifications) => set({ desktopNotifications }),
  setQueueMode: (queueMode) => {
    set({ queueMode });
    setQueueModeBackend(queueMode).catch(console.error);
  },
}));
