import { useState } from "react";
import { GeneralPreferences } from "./preferences/GeneralPreferences";
import { DisksPreferences } from "./preferences/DisksPreferences";
import { OrganizePreferences } from "./preferences/OrganizePreferences";
import { TransfersPreferences } from "./preferences/TransfersPreferences";

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
}

export function PreferencesModal({ open, onClose }: PreferencesModalProps) {
  const [tab, setTab] = useState<Tab>("general");

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6"
      onClick={onClose}
    >
      <div
        className="flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden rounded border border-neutral-800 bg-neutral-950"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
          <h2 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
            Cài đặt
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="rounded border border-neutral-700 px-2 py-1 text-xs"
          >
            Đóng
          </button>
        </div>

        <div className="flex gap-1 border-b border-neutral-800 px-4 py-2">
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => setTab(t.id)}
              className={`rounded px-3 py-1.5 text-xs uppercase tracking-wide ${
                tab === t.id
                  ? "bg-neutral-800 text-neutral-100"
                  : "text-neutral-500 hover:text-neutral-300"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div className="overflow-y-auto p-4">
          {tab === "general" && <GeneralPreferences />}
          {tab === "disks" && <DisksPreferences />}
          {tab === "organize" && <OrganizePreferences />}
          {tab === "transfers" && <TransfersPreferences />}
        </div>
      </div>
    </div>
  );
}
