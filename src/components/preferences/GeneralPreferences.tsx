import { useEffect, useState } from "react";
import { useSettingsStore } from "../../state/settingsStore";
import { useOrganizeStore } from "../../state/organizeStore";
import { SectionHeading } from "../ui/SectionHeading";
import { Checkbox, Radio } from "../ui/Checkbox";
import { Button } from "../ui/Button";
import type { DateOverrideMode } from "../../types/organize";
import {
  explorerIntegrationStatus,
  installExplorerIntegration,
  uninstallExplorerIntegration,
} from "../../lib/tauri";

export function GeneralPreferences() {
  const preventSleep = useSettingsStore((s) => s.preventSleep);
  const desktopNotifications = useSettingsStore((s) => s.desktopNotifications);
  const setPreventSleep = useSettingsStore((s) => s.setPreventSleep);
  const setDesktopNotifications = useSettingsStore((s) => s.setDesktopNotifications);
  const dateOverride = useOrganizeStore((s) => s.dateOverride);
  const setDateOverrideMode = useOrganizeStore((s) => s.setDateOverrideMode);
  const setManualDate = useOrganizeStore((s) => s.setManualDate);
  const setRolloverAt4am = useOrganizeStore((s) => s.setRolloverAt4am);
  const isWindows = navigator.userAgent.includes("Windows");
  const [explorerEnabled, setExplorerEnabled] = useState(false);
  const [explorerLoading, setExplorerLoading] = useState(false);
  const [explorerError, setExplorerError] = useState<string | null>(null);

  useEffect(() => {
    if (!isWindows) return;
    explorerIntegrationStatus()
      .then((status) => setExplorerEnabled(status.healthy))
      .catch((error) => setExplorerError(String(error)));
  }, [isWindows]);

  async function toggleExplorer(enabled: boolean) {
    setExplorerLoading(true);
    setExplorerError(null);
    try {
      const changed = enabled
        ? await installExplorerIntegration()
        : await uninstallExplorerIntegration();
      const readBack = await explorerIntegrationStatus();
      if (enabled ? !changed.healthy || !readBack.healthy : readBack.installed) {
        throw new Error(readBack.message ?? "Không thể xác nhận trạng thái Windows Explorer Integration.");
      }
      setExplorerEnabled(enabled);
    } catch (error) {
      setExplorerError(String(error));
    } finally {
      setExplorerLoading(false);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <SectionHeading as="h3">Thông báo</SectionHeading>
        <Checkbox label="Thông báo trên desktop" checked={desktopNotifications} onChange={(e) => setDesktopNotifications(e.currentTarget.checked)} />
        <Checkbox label="Ngăn máy vào chế độ ngủ khi đang truyền" checked={preventSleep} onChange={(e) => setPreventSleep(e.currentTarget.checked)} />
      </section>

      {isWindows && (
        <section className="flex flex-col gap-2">
          <SectionHeading as="h3">Windows Explorer Integration</SectionHeading>
          <Checkbox
            label="Bật menu chuột phải OffloadKit"
            checked={explorerEnabled}
            disabled={explorerLoading}
            onChange={(e) => void toggleExplorer(e.currentTarget.checked)}
          />
          {explorerError && <p role="alert" className="text-xs text-red-400">{explorerError}</p>}
        </section>
      )}

      <section className="flex flex-col gap-1">
        <SectionHeading as="h3">Ngày</SectionHeading>
        <div className="flex gap-3 text-xs">
          {(["automatic", "manual"] as DateOverrideMode[]).map((m) => (
            <Radio key={m} name="date-override-mode" checked={dateOverride.mode === m} onChange={() => setDateOverrideMode(m)} label={m === "automatic" ? "Theo đồng hồ hệ thống" : "Đặt thủ công"} />
          ))}
        </div>
        {dateOverride.mode === "automatic" ? (
          <Checkbox label="Chuyển ngày lúc 4 giờ sáng (buổi quay đêm vẫn giữ ngày hôm trước)" checked={dateOverride.rolloverAt4am} onChange={(e) => setRolloverAt4am(e.currentTarget.checked)} />
        ) : (
          <div className="flex items-center gap-2">
            <input type="date" title="Ngày quay" value={dateOverride.manualDate ?? ""} onChange={(e) => setManualDate(e.currentTarget.value || null)} className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs" />
            <Button variant="ghost" className="uppercase" onClick={() => { setDateOverrideMode("automatic"); setManualDate(null); }}>Bây giờ</Button>
          </div>
        )}
      </section>
    </div>
  );
}
