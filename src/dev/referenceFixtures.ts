import type { DiskInfo, Endpoint } from "../types/disk";

export const referenceDisks: DiskInfo[] = [
  { id: "D:", name: "KHANH VAN", mountPoint: "D:\\", totalBytes: 5_550_000_000_000, availableBytes: 500_000_000_000, isRemovable: true, fileSystem: "NTFS" },
  { id: "G:", name: "Local Disk I", mountPoint: "G:\\", totalBytes: 5_790_000_000_000, availableBytes: 520_000_000_000, isRemovable: false, fileSystem: "NTFS" },
  { id: "F:", name: "TEMP SSD", mountPoint: "F:\\", totalBytes: 725_000_000_000, availableBytes: 100_000_000_000, isRemovable: true, fileSystem: "NTFS" },
  { id: "C:", name: "Local Disk", mountPoint: "C:\\", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: false, fileSystem: "NTFS" },
  { id: "onedrive", name: "OneDrive - Personal", mountPoint: "C:\\Users\\Rin\\OneDrive", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: false, fileSystem: "Cloud" },
  { id: "phone", name: "S24 Ultra RinOP", mountPoint: "Phone:\\", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: true, fileSystem: "MTP" },
  { id: "tablet", name: "Tab S8+ by RinOP", mountPoint: "Tablet:\\", totalBytes: 690_000_000_000, availableBytes: 47_000_000_000, isRemovable: true, fileSystem: "MTP" },
  { id: "E:", name: "DATA", mountPoint: "E:\\", totalBytes: 1_000_000_000_000, availableBytes: 276_000_000_000, isRemovable: false, fileSystem: "NTFS" },
];

export const referenceSources: Endpoint[] = [];
export const referenceDestinations: Endpoint[] = [
  { diskId: "E:", label: "", path: "E:\\", isAutoLabel: false },
];

export function isReferenceFixtureEnabled(): boolean {
  return import.meta.env.DEV && new URLSearchParams(window.location.search).get("referenceFixture") === "disks";
}
