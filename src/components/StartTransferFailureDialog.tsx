import { AlertTriangle } from "lucide-react";
import { Modal } from "./ui/Modal";
import { Button } from "./ui/Button";

interface StartTransferFailureDialogProps {
  message: string | null;
  onClose: () => void;
}

export function StartTransferFailureDialog({ message, onClose }: StartTransferFailureDialogProps) {
  return (
    <Modal open={message !== null} onClose={onClose} title="Không thể bắt đầu lượt truyền">
      <div className="flex flex-col gap-4">
        <div className="flex items-start gap-3 rounded border border-red-900 bg-red-950/30 p-3 text-sm text-red-200">
          <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-red-400" />
          <p className="whitespace-pre-wrap break-words">{message}</p>
        </div>
        <div className="flex justify-end">
          <Button variant="primary" onClick={onClose}>Đóng</Button>
        </div>
      </div>
    </Modal>
  );
}
