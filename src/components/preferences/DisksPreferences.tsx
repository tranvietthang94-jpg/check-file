import { useDisksStore } from "../../state/disksStore";
import { SectionHeading } from "../ui/SectionHeading";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { Eye, EyeOff } from "../icons";

export function DisksPreferences() {
  const disks = useDisksStore((s) => s.disks);
  const hiddenDiskIds = useDisksStore((s) => s.hiddenDiskIds);
  const unhideDisk = useDisksStore((s) => s.unhideDisk);

  const hiddenDisks = disks.filter((d) => hiddenDiskIds.includes(d.id));

  return (
    <div className="flex flex-col gap-2">
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
    </div>
  );
}
