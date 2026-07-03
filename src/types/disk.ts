export interface DiskInfo {
  id: string;
  name: string;
  mountPoint: string;
  totalBytes: number;
  availableBytes: number;
  isRemovable: boolean;
  fileSystem: string;
}

export interface Endpoint {
  diskId: string;
  label: string;
  path: string;
}
