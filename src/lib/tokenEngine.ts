import type { OrganizeSettings } from "../types/organize";

/**
 * Display-only mirror of src-tauri/src/organize.rs's token engine, used for
 * the live rename/folder preview. The actual destination path used during a
 * real transfer is always computed in Rust -- this never touches disk.
 */
export interface TokenPreviewContext {
  sourceName: string;
  counter: number;
  counterPadding: number;
  fileStem: string;
  fileExtension: string;
  /** Stands in for job-start, per-file, and content-oldest timestamps alike. */
  now: Date;
}

function pad(n: number, width: number): string {
  return String(Math.max(0, Math.trunc(n))).padStart(width, "0");
}

function dateTokens(prefix: string, d: Date): [string, string][] {
  return [
    [`{${prefix}YYYY}`, String(d.getFullYear()).padStart(4, "0")],
    [`{${prefix}YY}`, String(d.getFullYear() % 100).padStart(2, "0")],
    [`{${prefix}MM}`, String(d.getMonth() + 1).padStart(2, "0")],
    [`{${prefix}DD}`, String(d.getDate()).padStart(2, "0")],
    [`{${prefix}hh}`, String(d.getHours()).padStart(2, "0")],
    [`{${prefix}mm}`, String(d.getMinutes()).padStart(2, "0")],
    [`{${prefix}ss}`, String(d.getSeconds()).padStart(2, "0")],
  ];
}

export function renderTemplate(template: string, ctx: TokenPreviewContext): string {
  const tokens: [string, string][] = [
    ["{Source Name}", ctx.sourceName],
    ["{Counter}", pad(ctx.counter, ctx.counterPadding)],
    ["{Filename}", ctx.fileStem],
    ["{File Counter}", pad(ctx.counter, 5)],
    ["{File Extension}", ctx.fileExtension],
    ...dateTokens("", ctx.now),
    ...dateTokens("File ", ctx.now),
    ...dateTokens("Content ", ctx.now),
  ];

  let rendered = template;
  for (const [token, value] of tokens) {
    rendered = rendered.split(token).join(value);
  }
  return rendered;
}

function buildFileName(
  template: string | null,
  ctx: TokenPreviewContext,
): string {
  const original = ctx.fileExtension ? `${ctx.fileStem}.${ctx.fileExtension}` : ctx.fileStem;
  if (!template) return original;
  const rendered = renderTemplate(template, ctx);
  if (!ctx.fileExtension || template.includes("{File Extension}")) return rendered;
  return `${rendered}.${ctx.fileExtension}`;
}

/** Renders a sample destination path for the Organize panel's live preview. */
export function previewDestinationPath(settings: OrganizeSettings, ctx: TokenPreviewContext): string {
  const fileName = buildFileName(settings.renameTemplate, ctx);

  let folder = "";
  if (settings.flatten) {
    folder = "";
  } else if (settings.folderTemplate) {
    folder = renderTemplate(settings.folderTemplate, ctx);
  } else {
    folder = "CLIP"; // illustrative stand-in for "original subfolder structure"
  }

  return folder ? `${folder}/${fileName}` : fileName;
}
