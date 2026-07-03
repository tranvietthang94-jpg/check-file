import { create } from "zustand";
import { setPreventSleepEnabled } from "../lib/tauri";
import type { ChecksumAlgorithm, VerificationMode } from "../types/job";

interface SettingsState {
  verificationMode: VerificationMode;
  checksumAlgorithm: ChecksumAlgorithm;
  preventSleep: boolean;
  desktopNotifications: boolean;
  setVerificationMode: (mode: VerificationMode) => void;
  setChecksumAlgorithm: (algorithm: ChecksumAlgorithm) => void;
  setPreventSleep: (enabled: boolean) => void;
  setDesktopNotifications: (enabled: boolean) => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  verificationMode: "sourceAndDestination",
  checksumAlgorithm: "xxh64",
  preventSleep: true,
  desktopNotifications: true,
  setVerificationMode: (verificationMode) => set({ verificationMode }),
  setChecksumAlgorithm: (checksumAlgorithm) => set({ checksumAlgorithm }),
  setPreventSleep: (preventSleep) => {
    set({ preventSleep });
    setPreventSleepEnabled(preventSleep).catch(console.error);
  },
  setDesktopNotifications: (desktopNotifications) => set({ desktopNotifications }),
}));
