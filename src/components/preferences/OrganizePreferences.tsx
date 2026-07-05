import { useEffect, useState } from "react";
import { useOrganizeStore } from "../../state/organizeStore";
import { useDisksStore } from "../../state/disksStore";
import { usePresetsStore } from "../../state/presetsStore";
import { useSettingsStore } from "../../state/settingsStore";
import { effectiveJobDate, previewDestinationPath, renderTemplate } from "../../lib/tokenEngine";
import { TemplateBuilder, type TemplateToken } from "../organize/TemplateBuilder";
import { SectionHeading } from "../ui/SectionHeading";
import { Checkbox, Radio } from "../ui/Checkbox";
import { Button } from "../ui/Button";
import { IconButton } from "../ui/IconButton";
import { EmptyState } from "../ui/EmptyState";
import { Plus, Save, Trash2, X } from "../icons";
import type { SelectiveCopyMode } from "../../types/organize";

const DATE_SUB_TOKENS = ["YYYY", "YY", "MM", "DD", "hh", "mm", "ss"];

// Token *names* (the part inside `{...}`) are functional identifiers parsed
// by both this preview and src-tauri/src/organize.rs's real renderer --
// left in English so a saved template's tokens keep working. Only `group`
// (the palette's section headers) is pure display text, safe to localize.
const BUILTIN_TOKENS: TemplateToken[] = [
  { name: "Source Name", group: "Chung" },
  { name: "Counter", group: "Chung" },
  { name: "Filename", group: "Chung" },
  { name: "File Counter", group: "Chung" },
  { name: "File Extension", group: "Chung" },
  ...DATE_SUB_TOKENS.map((t) => ({ name: t, group: "Ngày quay" })),
  ...DATE_SUB_TOKENS.map((t) => ({ name: `File ${t}`, group: "Ngày tệp" })),
  ...DATE_SUB_TOKENS.map((t) => ({ name: `Content ${t}`, group: "Ngày nội dung" })),
];

const NOW = new Date();

const PREVIEW_CONTEXT = {
  sourceName: "A-Cam",
  counter: 1,
  fileStem: "C0001",
  fileExtension: "MP4",
  now: NOW,
};

function parseList(value: string): string[] {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function PresetsSection() {
  const presets = usePresetsStore((s) => s.presets);
  const loading = usePresetsStore((s) => s.loading);
  const error = usePresetsStore((s) => s.error);
  const refresh = usePresetsStore((s) => s.refresh);
  const save = usePresetsStore((s) => s.save);
  const remove = usePresetsStore((s) => s.remove);

  const verificationMode = useSettingsStore((s) => s.verificationMode);
  const checksumAlgorithm = useSettingsStore((s) => s.checksumAlgorithm);
  const setVerificationMode = useSettingsStore((s) => s.setVerificationMode);
  const setChecksumAlgorithm = useSettingsStore((s) => s.setChecksumAlgorithm);

  const renameTemplate = useOrganizeStore((s) => s.renameTemplate);
  const folderTemplate = useOrganizeStore((s) => s.folderTemplate);
  const counterPadding = useOrganizeStore((s) => s.counterPadding);
  const selectiveCopy = useOrganizeStore((s) => s.selectiveCopy);
  const bundleIgnore = useOrganizeStore((s) => s.bundleIgnore);
  const ignoreEmptyFolders = useOrganizeStore((s) => s.ignoreEmptyFolders);
  const flatten = useOrganizeStore((s) => s.flatten);
  const contentDateExcludedExtensions = useOrganizeStore((s) => s.contentDateExcludedExtensions);
  const dateOverride = useOrganizeStore((s) => s.dateOverride);
  const elements = useOrganizeStore((s) => s.elements);
  const autoLabel = useOrganizeStore((s) => s.autoLabel);
  const skipModificationDateCheck = useOrganizeStore((s) => s.skipModificationDateCheck);
  const autoContinueOnBrokenMedia = useOrganizeStore((s) => s.autoContinueOnBrokenMedia);
  const loadOrganizeSettings = useOrganizeStore((s) => s.loadSettings);

  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleSave() {
    if (newName.trim() === "") return;
    setBusy(true);
    try {
      await save({
        name: newName.trim(),
        verificationMode,
        checksumAlgorithm,
        organize: {
          renameTemplate,
          folderTemplate,
          counterPadding,
          selectiveCopy,
          bundleIgnore,
          ignoreEmptyFolders,
          flatten,
          contentDateExcludedExtensions,
          dateOverride,
          elements,
          autoLabel,
          skipModificationDateCheck,
          autoContinueOnBrokenMedia,
        },
      });
      setNewName("");
    } finally {
      setBusy(false);
    }
  }

  function handleLoad(name: string) {
    const preset = presets.find((p) => p.name === name);
    if (!preset) return;
    setVerificationMode(preset.verificationMode);
    setChecksumAlgorithm(preset.checksumAlgorithm);
    loadOrganizeSettings(preset.organize);
  }

  async function handleDelete(name: string) {
    setBusy(true);
    try {
      await remove(name);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="flex flex-col gap-2">
      <SectionHeading as="h3">Mẫu cấu hình</SectionHeading>

      {loading && <p className="text-xs text-neutral-500">Đang tải…</p>}
      {error && <p className="text-xs text-red-400">{error}</p>}
      {!loading && presets.length === 0 && (
        <EmptyState icon={<Save className="h-5 w-5" />}>Chưa lưu mẫu cấu hình nào.</EmptyState>
      )}

      <ul className="flex flex-col gap-1">
        {presets.map((preset) => (
          <li
            key={preset.name}
            className="flex items-center justify-between gap-2 rounded border border-neutral-800 bg-neutral-900 px-2 py-1.5 text-xs"
          >
            <span className="truncate font-medium" title={preset.name}>
              {preset.name}
            </span>
            <span className="flex shrink-0 gap-1">
              <Button variant="secondary" disabled={busy} onClick={() => handleLoad(preset.name)}>
                Tải
              </Button>
              <Button
                variant="danger"
                icon={<Trash2 className="h-3.5 w-3.5" />}
                disabled={busy}
                onClick={() => handleDelete(preset.name)}
              >
                Xóa
              </Button>
            </span>
          </li>
        ))}
      </ul>

      <div className="flex gap-2">
        <input
          value={newName}
          onChange={(e) => setNewName(e.currentTarget.value)}
          placeholder="Tên mẫu cấu hình…"
          autoComplete="off"
          className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
        />
        <Button
          variant="secondary"
          icon={<Save className="h-3.5 w-3.5" />}
          disabled={busy || newName.trim() === ""}
          onClick={handleSave}
        >
          Lưu cấu hình hiện tại
        </Button>
      </div>
    </section>
  );
}

export function OrganizePreferences() {
  const renameTemplate = useOrganizeStore((s) => s.renameTemplate);
  const folderTemplate = useOrganizeStore((s) => s.folderTemplate);
  const counterPadding = useOrganizeStore((s) => s.counterPadding);
  const selectiveCopy = useOrganizeStore((s) => s.selectiveCopy);
  const bundleIgnore = useOrganizeStore((s) => s.bundleIgnore);
  const ignoreEmptyFolders = useOrganizeStore((s) => s.ignoreEmptyFolders);
  const flatten = useOrganizeStore((s) => s.flatten);
  const contentDateExcludedExtensions = useOrganizeStore((s) => s.contentDateExcludedExtensions);
  const dateOverride = useOrganizeStore((s) => s.dateOverride);
  const elements = useOrganizeStore((s) => s.elements);
  const autoLabel = useOrganizeStore((s) => s.autoLabel);
  const skipModificationDateCheck = useOrganizeStore((s) => s.skipModificationDateCheck);
  const autoContinueOnBrokenMedia = useOrganizeStore((s) => s.autoContinueOnBrokenMedia);

  const setRenameTemplate = useOrganizeStore((s) => s.setRenameTemplate);
  const setFolderTemplate = useOrganizeStore((s) => s.setFolderTemplate);
  const setCounterPadding = useOrganizeStore((s) => s.setCounterPadding);
  const setSelectiveCopyMode = useOrganizeStore((s) => s.setSelectiveCopyMode);
  const setSelectiveCopyPatterns = useOrganizeStore((s) => s.setSelectiveCopyPatterns);
  const setBundleIgnore = useOrganizeStore((s) => s.setBundleIgnore);
  const setIgnoreEmptyFolders = useOrganizeStore((s) => s.setIgnoreEmptyFolders);
  const setFlatten = useOrganizeStore((s) => s.setFlatten);
  const setContentDateExcludedExtensions = useOrganizeStore((s) => s.setContentDateExcludedExtensions);
  const addElement = useOrganizeStore((s) => s.addElement);
  const removeElement = useOrganizeStore((s) => s.removeElement);
  const setAutoLabelEnabled = useOrganizeStore((s) => s.setAutoLabelEnabled);
  const setAutoLabelTemplate = useOrganizeStore((s) => s.setAutoLabelTemplate);
  const setAutoLabelStartCounter = useOrganizeStore((s) => s.setAutoLabelStartCounter);
  const setAutoLabelCounterPadding = useOrganizeStore((s) => s.setAutoLabelCounterPadding);
  const setSkipModificationDateCheck = useOrganizeStore((s) => s.setSkipModificationDateCheck);
  const setAutoContinueOnBrokenMedia = useOrganizeStore((s) => s.setAutoContinueOnBrokenMedia);

  const [patternsText, setPatternsText] = useState(selectiveCopy.patterns.join(", "));
  const [excludedExtText, setExcludedExtText] = useState(
    contentDateExcludedExtensions.join(", "),
  );
  const [bundleEnabled, setBundleEnabled] = useState(!!bundleIgnore);
  const [bundleName, setBundleName] = useState(bundleIgnore?.name ?? "");
  const [bundleMaxMb, setBundleMaxMb] = useState(
    bundleIgnore ? (bundleIgnore.maxSizeBytes / (1024 * 1024)).toString() : "50",
  );
  const [newElementName, setNewElementName] = useState("");

  function commitNewElement() {
    if (newElementName.trim() === "") return;
    addElement(newElementName);
    setNewElementName("");
  }

  function commitBundle(enabled: boolean, name: string, maxMb: string) {
    if (!enabled || name.trim() === "") {
      setBundleIgnore(null);
      return;
    }
    const mb = Number.parseFloat(maxMb);
    setBundleIgnore({
      name: name.trim(),
      maxSizeBytes: Math.max(0, Number.isFinite(mb) ? mb : 0) * 1024 * 1024,
    });
  }

  const autoLabelPreview = renderTemplate(autoLabel.template, {
    ...PREVIEW_CONTEXT,
    counter: autoLabel.startCounter,
    counterPadding: autoLabel.counterPadding,
    jobStarted: effectiveJobDate(NOW, dateOverride),
    elements,
  });

  const preview = previewDestinationPath(
    {
      renameTemplate,
      folderTemplate,
      counterPadding,
      selectiveCopy,
      bundleIgnore,
      ignoreEmptyFolders,
      flatten,
      contentDateExcludedExtensions,
      dateOverride,
      elements,
      autoLabel,
      skipModificationDateCheck,
      autoContinueOnBrokenMedia,
    },
    {
      ...PREVIEW_CONTEXT,
      counterPadding,
      jobStarted: effectiveJobDate(NOW, dateOverride),
      elements,
    },
  );

  const allTokens: TemplateToken[] = [
    ...BUILTIN_TOKENS,
    ...elements
      .filter((e) => e.name.trim() !== "")
      .map((e) => ({ name: e.name.trim(), group: "Thành phần" })),
  ];

  return (
    <div className="flex flex-col gap-6">
      <PresetsSection />

      <section className="flex flex-col gap-2">
        <SectionHeading as="h3">Tổ chức</SectionHeading>

        <div className="flex flex-col gap-1 text-xs">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Mẫu đổi tên
          </span>
          <TemplateBuilder
            value={renameTemplate ?? ""}
            onChange={setRenameTemplate}
            tokens={allTokens}
            placeholder="Giữ nguyên tên tệp gốc…"
          />
        </div>

        <div className="flex flex-col gap-1 text-xs">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Mẫu thư mục
          </span>
          <TemplateBuilder
            value={folderTemplate ?? ""}
            onChange={setFolderTemplate}
            tokens={allTokens}
            disabled={flatten}
            placeholder="Giữ nguyên cấu trúc thư mục gốc…"
          />
        </div>

        <p
          className="truncate rounded border border-neutral-800 bg-neutral-900 px-2 py-1 font-mono text-[10px] text-neutral-400"
          title={preview}
        >
          Xem trước: {preview}
        </p>

        <label className="flex items-center gap-2 text-xs">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Số chữ số {"{Counter}"}
          </span>
          <input
            type="number"
            min={1}
            max={8}
            value={counterPadding}
            onChange={(e) => setCounterPadding(Number.parseInt(e.currentTarget.value, 10))}
            className="w-14 rounded border border-neutral-700 bg-neutral-950 px-2 py-1"
          />
        </label>

        <div className="flex flex-col gap-1">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Sao chép có chọn lọc
          </span>
          <div className="flex gap-3 text-xs">
            {(["exclude", "include"] as SelectiveCopyMode[]).map((m) => (
              <Radio
                key={m}
                name="selective-copy-mode"
                checked={selectiveCopy.mode === m}
                onChange={() => setSelectiveCopyMode(m)}
                label={m === "exclude" ? "Không sao chép" : "Chỉ sao chép"}
              />
            ))}
          </div>
          <input
            value={patternsText}
            onChange={(e) => {
              setPatternsText(e.currentTarget.value);
              setSelectiveCopyPatterns(parseList(e.currentTarget.value));
            }}
            placeholder=".xml, proxy, .tmp"
            autoComplete="off"
            className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
          />
        </div>

        <Checkbox
          align="start"
          checked={skipModificationDateCheck}
          onChange={(e) => setSkipModificationDateCheck(e.currentTarget.checked)}
          label={
            <span>
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                Bỏ qua kiểm tra ngày sửa đổi
              </span>
              <span className="block text-neutral-500">
                Phát hiện trùng lặp chỉ so khớp tên + kích thước -- dùng cho quy trình mà thời gian
                sửa đổi của tệp không đáng tin cậy.
              </span>
            </span>
          }
        />

        <Checkbox
          align="start"
          checked={autoContinueOnBrokenMedia}
          onChange={(e) => setAutoContinueOnBrokenMedia(e.currentTarget.checked)}
          label={
            <span>
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                Tự động tiếp tục khi Media hỏng
              </span>
              <span className="block text-neutral-500">
                Bỏ qua cảnh báo khi phát hiện tệp 0 byte ở nguồn và vẫn sao chép. Mặc định tắt, để
                thẻ nhớ bị lỗi được cảnh báo trước khi sao chép bất kỳ thứ gì.
              </span>
            </span>
          }
        />

        <div className="flex flex-col gap-1">
          <Checkbox
            checked={bundleEnabled}
            onChange={(e) => {
              setBundleEnabled(e.currentTarget.checked);
              commitBundle(e.currentTarget.checked, bundleName, bundleMaxMb);
            }}
            label={
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                Bỏ qua thư mục gói (bundle)
              </span>
            }
          />
          {bundleEnabled && (
            <div className="flex gap-2">
              <input
                value={bundleName}
                onChange={(e) => {
                  setBundleName(e.currentTarget.value);
                  commitBundle(bundleEnabled, e.currentTarget.value, bundleMaxMb);
                }}
                placeholder="Tên thư mục (vd: PRIVATE)"
                autoComplete="off"
                className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
              />
              <input
                type="number"
                min={0}
                title="Kích thước tối đa (MB)"
                value={bundleMaxMb}
                onChange={(e) => {
                  setBundleMaxMb(e.currentTarget.value);
                  commitBundle(bundleEnabled, bundleName, e.currentTarget.value);
                }}
                className="w-16 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 text-xs"
              />
              <span className="self-center text-[10px] text-neutral-500">MB tối đa</span>
            </div>
          )}
        </div>

        <Checkbox
          checked={flatten}
          onChange={(e) => setFlatten(e.currentTarget.checked)}
          label="Làm phẳng (bỏ thư mục con gốc)"
        />

        <Checkbox
          checked={ignoreEmptyFolders}
          onChange={(e) => setIgnoreEmptyFolders(e.currentTarget.checked)}
          disabled={flatten}
          label="Bỏ qua thư mục rỗng"
        />

        <label className="flex flex-col gap-1 text-xs">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            {"{Content *}"} loại trừ đuôi tệp
          </span>
          <input
            value={excludedExtText}
            onChange={(e) => {
              setExcludedExtText(e.currentTarget.value);
              setContentDateExcludedExtensions(parseList(e.currentTarget.value));
            }}
            placeholder=".xml"
            autoComplete="off"
            className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
          />
        </label>

        <div className="flex flex-col gap-1">
          <span className="text-[10px] uppercase tracking-wide text-neutral-500">
            Thành phần (token tùy chỉnh)
          </span>

          {elements.map((element) => (
            <div key={element.name} className="flex items-center gap-2">
              <span
                className="min-w-0 flex-1 truncate font-mono text-xs text-neutral-400"
                title={`{${element.name}}`}
              >
                {`{${element.name}}`}
              </span>
              <IconButton
                tone="neutral"
                onClick={() => removeElement(element.name)}
                title={`Xóa {${element.name}}`}
                aria-label={`Xóa {${element.name}}`}
                icon={<X className="h-3.5 w-3.5" />}
              />
            </div>
          ))}

          <div className="flex gap-2">
            <input
              value={newElementName}
              onChange={(e) => setNewElementName(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") commitNewElement();
              }}
              placeholder="Tên thành phần mới (vd: Location)…"
              autoComplete="off"
              className="min-w-0 flex-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
            />
            <Button variant="secondary" icon={<Plus className="h-3.5 w-3.5" />} onClick={commitNewElement}>
              Thêm
            </Button>
          </div>
          <p className="text-[10px] leading-relaxed text-neutral-600">
            Gõ token (vd {"{Location}"}) vào Mẫu đổi tên hoặc Mẫu thư mục ở trên để dùng nó. Giá trị
            theo từng lượt truyền được nhập ở bảng Thành phần trên màn Ổ đĩa, không phải ở đây.
          </p>
        </div>

        <div className="flex flex-col gap-1">
          <Checkbox
            checked={autoLabel.enabled}
            onChange={(e) => {
              setAutoLabelEnabled(e.currentTarget.checked);
              useDisksStore.getState().recomputeAutoLabels();
            }}
            label={
              <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                Tự động gắn nhãn nguồn mới
              </span>
            }
          />

          {autoLabel.enabled && (
            <>
              <input
                value={autoLabel.template}
                onChange={(e) => {
                  setAutoLabelTemplate(e.currentTarget.value);
                  useDisksStore.getState().recomputeAutoLabels();
                }}
                placeholder="{Source Name}_{Counter}"
                autoComplete="off"
                className="rounded border border-neutral-700 bg-neutral-950 px-2 py-1 font-mono text-xs"
              />
              <p
                className="truncate rounded border border-neutral-800 bg-neutral-900 px-2 py-1 font-mono text-[10px] text-neutral-400"
                title={autoLabelPreview}
              >
                Xem trước: {autoLabelPreview}
              </p>
              <div className="flex items-center gap-3 text-xs">
                <label className="flex items-center gap-2">
                  <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                    Bắt đầu từ
                  </span>
                  <input
                    type="number"
                    min={0}
                    value={autoLabel.startCounter}
                    onChange={(e) => {
                      setAutoLabelStartCounter(Number.parseInt(e.currentTarget.value, 10));
                      useDisksStore.getState().recomputeAutoLabels();
                    }}
                    className="w-16 rounded border border-neutral-700 bg-neutral-950 px-2 py-1"
                  />
                </label>
                <label className="flex items-center gap-2">
                  <span className="text-[10px] uppercase tracking-wide text-neutral-500">
                    Số chữ số
                  </span>
                  <input
                    type="number"
                    min={1}
                    max={8}
                    value={autoLabel.counterPadding}
                    onChange={(e) => {
                      setAutoLabelCounterPadding(Number.parseInt(e.currentTarget.value, 10));
                      useDisksStore.getState().recomputeAutoLabels();
                    }}
                    className="w-14 rounded border border-neutral-700 bg-neutral-950 px-2 py-1"
                  />
                </label>
              </div>
            </>
          )}
        </div>
      </section>
    </div>
  );
}
