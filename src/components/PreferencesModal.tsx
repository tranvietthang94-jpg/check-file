import { useState, type ReactNode } from "react";
import { GeneralPreferences } from "./preferences/GeneralPreferences";
import { DisksPreferences } from "./preferences/DisksPreferences";
import { OrganizePreferences } from "./preferences/OrganizePreferences";
import { TransfersPreferences } from "./preferences/TransfersPreferences";
import { Button } from "./ui/Button";
import { IconButton } from "./ui/IconButton";
import { Panel } from "./ui/Panel";
import { SectionHeading } from "./ui/SectionHeading";
import { ArrowLeftRight, FolderTree, HardDrive, Settings, X } from "./icons";

type Tab = "general" | "disks" | "organize" | "transfers";

const TABS: { id: Tab; label: string; icon: ReactNode }[] = [
  { id: "general", label: "Chung", icon: <Settings className="h-3.5 w-3.5" /> },
  { id: "disks", label: "Ổ đĩa", icon: <HardDrive className="h-3.5 w-3.5" /> },
  { id: "organize", label: "Tổ chức", icon: <FolderTree className="h-3.5 w-3.5" /> },
  { id: "transfers", label: "Truyền tải", icon: <ArrowLeftRight className="h-3.5 w-3.5" /> },
];

interface PreferencesModalProps {
  open: boolean;
  onClose: () => void;
}

export function PreferencesModal({ open, onClose }: PreferencesModalProps) {
  const [tab, setTab] = useState<Tab>("general");

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6"
      onClick={onClose}
    >
      <Panel
        className="flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
          <SectionHeading>Cài đặt</SectionHeading>
          <IconButton onClick={onClose} aria-label="Đóng" icon={<X className="h-3.5 w-3.5" />} />
        </div>

        <div className="flex gap-1 border-b border-neutral-800 px-4 py-2">
          {TABS.map((t) => (
            <Button
              key={t.id}
              variant="ghost"
              active={tab === t.id}
              icon={t.icon}
              onClick={() => setTab(t.id)}
              className="uppercase"
            >
              {t.label}
            </Button>
          ))}
        </div>

        <div className="overflow-y-auto p-4">
          {tab === "general" && <GeneralPreferences />}
          {tab === "disks" && <DisksPreferences />}
          {tab === "organize" && <OrganizePreferences />}
          {tab === "transfers" && <TransfersPreferences />}
        </div>
      </Panel>
    </div>
  );
}
