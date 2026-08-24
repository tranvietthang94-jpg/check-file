import type { ReactNode } from "react";
import { Panel } from "./Panel";
import { SectionHeading } from "./SectionHeading";
import { IconButton } from "./IconButton";
import { X } from "../icons";

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  children: ReactNode;
  /** Extra classes for the panel itself, e.g. to widen/narrow it per use case. */
  panelClassName?: string;
}

/** Shared centered-dialog shell (backdrop + close-X header row) -- extracted
 * from `PreferencesModal`'s original inline markup so `TransferLogPanel` and
 * `ReportsPanel` (moved behind the app menu, matching OffShoot's own
 * menu-driven Transfer Logs/Reports windows) can reuse the exact same shell. */
export function Modal({ open, onClose, title, children, panelClassName }: ModalProps) {
  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6"
      role="dialog"
      aria-modal="true"
      aria-label={String(title)}
      onClick={onClose}
    >
      <Panel
        className={`flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden ${panelClassName ?? ""}`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-neutral-800 px-4 py-3">
          <SectionHeading>{title}</SectionHeading>
          <IconButton onClick={onClose} aria-label="Đóng" icon={<X className="h-3.5 w-3.5" />} />
        </div>
        <div className="overflow-y-auto p-4">{children}</div>
      </Panel>
    </div>
  );
}
