import type { MediaEntry, MediaKind } from "../types/media";
import { formatBytes, formatDuration, formatFrameRate } from "../lib/format";
import { Panel } from "./ui/Panel";
import { SectionHeading } from "./ui/SectionHeading";
import { EmptyState } from "./ui/EmptyState";
import { IconButton } from "./ui/IconButton";
import { File, ImageIcon, Inbox, Music, Video, X } from "./icons";

const KIND_LABEL: Record<MediaKind, string> = {
  video: "video",
  audio: "âm thanh",
  photo: "ảnh",
  other: "khác",
};

const KIND_ICON: Record<MediaKind, typeof Video> = {
  video: Video,
  audio: Music,
  photo: ImageIcon,
  other: File,
};

interface MediaBrowserProps {
  folder: string;
  entries: MediaEntry[];
  status: "scanning" | "complete";
  total: number;
  onClose: () => void;
}

function metadataLine(entry: MediaEntry): string {
  const m = entry.metadata;
  if (!m) return "";
  const parts: string[] = [];
  if (entry.kind === "video") {
    if (m.width && m.height) parts.push(`${m.width}×${m.height}`);
    if (m.codec) parts.push(m.codec.toUpperCase());
    if (m.frameRate) parts.push(formatFrameRate(m.frameRate));
    if (m.durationSecs) parts.push(formatDuration(m.durationSecs));
    if (m.timecode) parts.push(`TC ${m.timecode}`);
  } else if (entry.kind === "audio") {
    if (m.codec) parts.push(m.codec.toUpperCase());
    if (m.sampleRate) parts.push(`${(m.sampleRate / 1000).toFixed(1)}kHz`);
    if (m.channels) parts.push(`${m.channels}ch`);
    if (m.durationSecs) parts.push(formatDuration(m.durationSecs));
  } else if (entry.kind === "photo") {
    if (m.cameraModel) parts.push(m.cameraModel);
    if (m.lens) parts.push(m.lens);
    if (m.focalLength) parts.push(m.focalLength);
    if (m.aperture) parts.push(m.aperture);
    if (m.shutterSpeed) parts.push(m.shutterSpeed);
    if (m.iso) parts.push(`ISO ${m.iso}`);
  }
  return parts.join(" · ");
}

export function MediaBrowser({ folder, entries, status, total, onClose }: MediaBrowserProps) {
  return (
    <Panel as="section" className="col-span-full flex flex-col gap-3 p-4">
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <SectionHeading>Duyệt Nguồn</SectionHeading>
          <p className="truncate text-xs text-neutral-500">{folder}</p>
        </div>
        <div className="flex shrink-0 items-center gap-3">
          <span className="text-xs text-neutral-500">
            {status === "scanning" ? `Đang quét… ${entries.length}` : `${total} tệp`}
          </span>
          <IconButton onClick={onClose} aria-label="Đóng" icon={<X className="h-3.5 w-3.5" />} />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6">
        {entries.map((entry) => {
          const line = metadataLine(entry);
          const KindIcon = KIND_ICON[entry.kind];
          return (
            <Panel key={entry.path} className="flex flex-col gap-1 overflow-hidden">
              <div className="flex aspect-video items-center justify-center bg-neutral-950">
                {entry.thumbnailBase64 ? (
                  <img
                    src={`data:image/jpeg;base64,${entry.thumbnailBase64}`}
                    alt={entry.path}
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <KindIcon className="h-6 w-6 text-neutral-600" aria-label={KIND_LABEL[entry.kind]} />
                )}
              </div>
              <div className="flex flex-col gap-0.5 px-2 pb-2">
                <span className="truncate text-xs font-medium" title={entry.path}>
                  {entry.path}
                </span>
                <span className="text-[10px] text-neutral-500">{formatBytes(entry.size)}</span>
                {line && (
                  <span className="truncate text-[10px] text-neutral-500" title={line}>
                    {line}
                  </span>
                )}
              </div>
            </Panel>
          );
        })}
      </div>

      {entries.length === 0 && status === "complete" && (
        <EmptyState icon={<Inbox className="h-5 w-5" />}>Không tìm thấy tệp nào.</EmptyState>
      )}
    </Panel>
  );
}
