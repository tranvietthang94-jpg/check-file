import { useEffect, useState } from "react";
import { GeneralPreferences } from "./preferences/GeneralPreferences";
import { DisksPreferences } from "./preferences/DisksPreferences";
import { OrganizePreferences } from "./preferences/OrganizePreferences";
import { TransfersPreferences } from "./preferences/TransfersPreferences";
import { Modal } from "./ui/Modal";

type Tab = "general" | "disks" | "organize" | "transfers";

const TABS: { id: Tab; label: string }[] = [
  { id: "general", label: "Chung" },
  { id: "disks", label: "Ổ đĩa" },
  { id: "organize", label: "Tổ chức" },
  { id: "transfers", label: "Truyền tải" },
];

interface PreferencesModalProps {
  open: boolean;
  onClose: () => void;
  initialTab?: Tab;
}

export function PreferencesModal({ open, onClose, initialTab = "general" }: PreferencesModalProps) {
  const [tab, setTab] = useState<Tab>(initialTab);

  useEffect(() => {
    if (open) setTab(initialTab);
  }, [open, initialTab]);

  return (
    <Modal open={open} onClose={onClose} title="Cài đặt">
      <div className="-mx-4 -mt-4 mb-4 flex gap-4 border-b border-neutral-800 px-4">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => setTab(t.id)}
            className={`border-b-2 px-1 py-2.5 text-sm transition-colors ${
              tab === t.id
                ? "border-green-500 text-neutral-100"
                : "border-transparent text-neutral-500 hover:text-neutral-300"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === "general" && <GeneralPreferences />}
      {tab === "disks" && <DisksPreferences />}
      {tab === "organize" && <OrganizePreferences />}
      {tab === "transfers" && <TransfersPreferences />}
    </Modal>
  );
}
