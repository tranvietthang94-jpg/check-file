import { useSettingsStore } from "../../state/settingsStore";
import { useOrganizeStore } from "../../state/organizeStore";
import type { DateOverrideMode } from "../../types/organize";

export function GeneralPreferences() {
  const preventSleep = useSettingsStore((s) => s.preventSleep);
  const desktopNotifications = useSettingsStore((s) => s.desktopNotifications);
  const setPreventSleep = useSettingsStore((s) => s.setPreventSleep);
  const setDesktopNotifications = useSettingsStore((s) => s.setDesktopNotifications);

  const dateOverride = useOrganizeStore((s) => s.dateOverride);
  const setDateOverrideMode = useOrganizeStore((s) => s.setDateOverrideMode);
  const setManualDate = useOrganizeStore((s) => s.setManualDate);
  const setRolloverAt4am = useOrganizeStore((s) => s.setRolloverAt4am);

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">
          Thông báo
        </h3>
        <label className="flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            checked={desktopNotifications}
            onChange={(e) => setDesktopNotifications(e.currentTarget.checked)}
          />
          Thông báo trên desktop
        </label>
        <label className="flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            checked={preventSleep}
            onChange={(e) => setPreventSleep(e.currentTarget.checked)}
          />
          Ngăn máy vào chế độ ngủ khi đang truyền
        </label>
      </section>

      <section className="flex flex-col gap-1">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-neutral-400">Ngày</h3>
        <div className="flex gap-3 text-xs">
          {(["automatic", "manual"] as DateOverrideMode[]).map((m) => (
            <label key={m} className="flex items-center gap-1">
              <input
                type="radio"
                name="date-override-mode"
                checked={dateOverride.mode === m}
                onChange={() => setDateOverrideMode(m)}
              />
              {m === "automatic" ? "Theo đồng hồ hệ thống" : "Đặt thủ công"}
            </label>
          ))}
        </div>

        {dateOverride.mode === "automatic" ? (
          <label className="flex items-center gap-2 text-xs">
            <input
              type="checkbox"
              checked={dateOverride.rolloverAt4am}
              onChange={(e) => setRolloverAt4am(e.currentTarget.checked)}
            />
            Chuyển ngày lúc 4 giờ sáng (buổi quay đêm vẫn giữ ngày hôm trước)
          </label>
        ) : (
          <div className="flex items-center gap-2">
            <input
              type="date"
              title="Ngày quay"
              value={dateOverride.manualDate ?? ""}
              onChange={(e) => setManualDate(e.currentTarget.value || null)}
              className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
            />
            <button
              type="button"
              onClick={() => {
                setDateOverrideMode("automatic");
                setManualDate(null);
              }}
              className="rounded border border-neutral-700 px-2 py-1 text-[10px] uppercase tracking-wide text-neutral-400 hover:bg-neutral-800"
            >
              Bây giờ
            </button>
          </div>
        )}
      </section>
    </div>
  );
}
