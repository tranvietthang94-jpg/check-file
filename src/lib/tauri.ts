import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DiskInfo } from "../types/disk";

export function listDisks(): Promise<DiskInfo[]> {
  return invoke<DiskInfo[]>("list_disks");
}

export function onDisksChanged(
  callback: (disks: DiskInfo[]) => void,
): Promise<UnlistenFn> {
  return listen<DiskInfo[]>("disks-changed", (event) => callback(event.payload));
}
