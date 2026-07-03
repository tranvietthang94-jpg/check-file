import { create } from "zustand";
import { listTransferLogs } from "../lib/tauri";
import type { TransferLogEntry } from "../types/transferLog";

interface TransferLogState {
  logs: TransferLogEntry[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export const useTransferLogStore = create<TransferLogState>((set) => ({
  logs: [],
  loading: false,
  error: null,

  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const logs = await listTransferLogs();
      set({ logs, loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },
}));
