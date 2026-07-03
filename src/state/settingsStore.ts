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
  setVerificationMode: (mode: VerificationMode) => void;
  setChecksumAlgorithm: (algorithm: ChecksumAlgorithm) => void;
  setPreventSleep: (enabled: boolean) => void;
  setDesktopNotifications: (enabled: boolean) => void;
  setQueueMode: (mode: QueueMode) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  verificationMode: "sourceAndDestination",
  checksumAlgorithm: "xxh64",
  preventSleep: true,
  desktopNotifications: true,
  queueMode: "off",
  setVerificationMode: (verificationMode) => set({ verificationMode }),
  setChecksumAlgorithm: (checksumAlgorithm) => set({ checksumAlgorithm }),
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
