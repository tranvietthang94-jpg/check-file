export type MediaKind = "video" | "audio" | "photo" | "other";

export interface MediaMetadata {
  codec: string | null;
  width: number | null;
  height: number | null;
  frameRate: number | null;
  durationSecs: number | null;
  timecode: string | null;
  sampleRate: number | null;
  channels: number | null;
  cameraModel: string | null;
  lens: string | null;
  iso: string | null;
  aperture: string | null;
  shutterSpeed: string | null;
  focalLength: string | null;
}

export interface MediaEntry {
  path: string;
  size: number;
  kind: MediaKind;
  metadata: MediaMetadata | null;
  thumbnailBase64: string | null;
}

export interface MediaScanItemPayload {
  scanId: string;
  entry: MediaEntry;
}

export interface MediaScanCompletePayload {
  scanId: string;
  total: number;
}
