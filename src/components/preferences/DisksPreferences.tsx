import { useDisksStore } from "../../state/disksStore";
import { useSettingsStore } from "../../state/settingsStore";
import { SectionHeading } from "../ui/SectionHeading";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { Checkbox } from "../ui/Checkbox";
import { Eye, EyeOff } from "../icons";

export function DisksPreferences() {
  const disks = useDisksStore((s) => s.disks);
  const hiddenDiskIds = useDisksStore((s) => s.hiddenDiskIds);
  const unhideDisk = useDisksStore((s) => s.unhideDisk);
  const autoSourceEnabled = useSettingsStore((s) => s.autoSourceEnabled);
  const autoSourcePattern = useSettingsStore((s) => s.autoSourcePattern);
  const autoEjectEnabled = useSettingsStore((s) => s.autoEjectEnabled);
  const setAutoSourceEnabled = useSettingsStore((s) => s.setAutoSourceEnabled);
  const setAutoSourcePattern = useSettingsStore((s) => s.setAutoSourcePattern);
  const setAutoEjectEnabled = useSettingsStore((s) => s.setAutoEjectEnabled);

  const hiddenDisks = disks.filter((d) => hiddenDiskIds.includes(d.id));

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <SectionHeading as="h3">Tự động hóa</SectionHeading>
        <Checkbox
          label="Tự động thêm ổ khớp mẫu làm Nguồn"
          checked={autoSourceEnabled}
          onChange={(e) => setAutoSourceEnabled(e.currentTarget.checked)}
        />
        <label className="flex flex-col gap-1 text-xs text-neutral-400">
          Mẫu tên ổ Nguồn
          <input
            aria-label="Mẫu tên ổ Nguồn"
            value={autoSourcePattern}
            disabled={!autoSourceEnabled}
            onChange={(e) => setAutoSourcePattern(e.currentTarget.value)}
            placeholder="CARD_*"
            className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs text-neutral-200 disabled:opacity-50"
          />
        </label>
        <Checkbox
          label="Tự động tháo ổ Nguồn sau khi truyền thành công"
          checked={autoEjectEnabled}
          onChange={(e) => setAutoEjectEnabled(e.currentTarget.checked)}
        />
      </section>

      <section className="flex flex-col gap-2">
      <SectionHeading as="h3">Ổ đĩa đã ẩn</SectionHeading>
      {hiddenDisks.length === 0 ? (
        <EmptyState icon={<EyeOff className="h-5 w-5" />}>
          Không có ổ đĩa nào bị ẩn. Dùng "Ẩn" trên một ổ đĩa ở màn Ổ đĩa để ẩn nó khỏi danh sách.
        </EmptyState>
      ) : (
        <ul className="flex flex-col gap-1">
          {hiddenDisks.map((disk) => (
            <li
              key={disk.id}
              className="flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900 px-3 py-1.5 text-xs"
            >
              <span className="truncate text-neutral-400">
                {disk.name} · {disk.mountPoint}
              </span>
              <Button
                variant="secondary"
                icon={<Eye className="h-3.5 w-3.5" />}
                onClick={() => unhideDisk(disk.id)}
              >
                Bỏ ẩn
              </Button>
            </li>
          ))}
        </ul>
      )}
      </section>
    </div>
  );
}
