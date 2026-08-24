import { Info } from "lucide-react";
import { Modal } from "./ui/Modal";
import { Button } from "./ui/Button";

interface AboutDialogProps {
  open: boolean;
  onClose: () => void;
}

export function AboutDialog({ open, onClose }: AboutDialogProps) {
  return (
    <Modal open={open} onClose={onClose} title="Giới thiệu OffloadKit">
      <div className="flex flex-col items-center gap-4 py-4 text-center">
        <Info className="h-10 w-10 text-blue-400" />
        <div>
          <h2 className="text-lg font-semibold">OffloadKit</h2>
          <p className="mt-1 text-sm text-neutral-400">Công cụ offload media local-first.</p>
          <p className="mt-2 text-xs text-neutral-500">Phiên bản 0.1.1 · Không cloud · Không telemetry</p>
        </div>
        <Button variant="primary" onClick={onClose}>Đóng</Button>
      </div>
    </Modal>
  );
}
