import { create } from "zustand";
import type { TransferGroup, TransferGroupMode } from "../types/transferGroup";

interface GroupsState {
  groups: Record<string, TransferGroup>;
  /**
   * Upsert group metadata. The backend command that creates a group emits
   * per-job "job added" events synchronously, before its own return value
   * (the group id) reaches the frontend over the IPC event channel -- so
   * `addJobToGroup` can legitimately arrive before this does. Both are
   * upserts that preserve whatever the other already contributed, so either
   * order converges to the same state.
   */
  setGroupMeta: (
    id: string,
    mode: TransferGroupMode,
    sourceLabel: string,
    destinationLabels: string[],
  ) => void;
  addJobToGroup: (groupId: string, jobId: string) => void;
}

function emptyGroup(id: string): TransferGroup {
  return { id, mode: "parallel", sourceLabel: "", destinationLabels: [], jobIds: [] };
}

export const useGroupsStore = create<GroupsState>((set) => ({
  groups: {},

  setGroupMeta: (id, mode, sourceLabel, destinationLabels) =>
    set((state) => {
      const existing = state.groups[id] ?? emptyGroup(id);
      return {
        groups: {
          ...state.groups,
          [id]: { ...existing, mode, sourceLabel, destinationLabels },
        },
      };
    }),

  addJobToGroup: (groupId, jobId) =>
    set((state) => {
      const existing = state.groups[groupId] ?? emptyGroup(groupId);
      if (existing.jobIds.includes(jobId)) return state;
      return {
        groups: {
          ...state.groups,
          [groupId]: { ...existing, jobIds: [...existing.jobIds, jobId] },
        },
      };
    }),
}));
