import { useEffect, useRef, useState } from "react";

export interface TemplateToken {
  name: string;
  group: string;
}

type Segment = { type: "token"; name: string } | { type: "text"; value: string };

const TOKEN_DRAG_MIME = "application/x-offloadkit-template-token";

/** Splits a `{Token}`-style template string into an alternating list of
 * token and literal-text segments, so each can be rendered as its own chip
 * or inline text input. Any `{...}` is treated as a token chip regardless
 * of whether it's a recognized built-in or a user-defined Element -- this
 * builder never needs to validate token names, only display them. */
function parseTemplate(template: string): Segment[] {
  const segments: Segment[] = [];
  const regex = /\{[^}]+\}/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = regex.exec(template))) {
    if (match.index > lastIndex) {
      segments.push({ type: "text", value: template.slice(lastIndex, match.index) });
    }
    segments.push({ type: "token", name: match[0].slice(1, -1) });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < template.length) {
    segments.push({ type: "text", value: template.slice(lastIndex) });
  }
  if (segments.length === 0 || segments[segments.length - 1].type === "token") {
    segments.push({ type: "text", value: "" });
  }
  return segments;
}

function serializeTemplate(segments: Segment[]): string {
  return segments.map((s) => (s.type === "token" ? `{${s.name}}` : s.value)).join("");
}

/** Finds where in the segment list a dropped/clicked token should land, by
 * comparing the drop's x position against each rendered segment's midpoint. */
function computeDropIndex(container: HTMLElement, clientX: number): number {
  const els = Array.from(container.querySelectorAll<HTMLElement>("[data-segment-index]"));
  for (const el of els) {
    const rect = el.getBoundingClientRect();
    if (clientX < rect.left + rect.width / 2) {
      return Number(el.dataset.segmentIndex);
    }
  }
  return els.length;
}

interface TemplateBuilderProps {
  value: string;
  onChange: (value: string) => void;
  tokens: TemplateToken[];
  disabled?: boolean;
  placeholder?: string;
}

export function TemplateBuilder({
  value,
  onChange,
  tokens,
  disabled,
  placeholder,
}: TemplateBuilderProps) {
  const rowRef = useRef<HTMLDivElement>(null);
  const segments = parseTemplate(value);
  const [menuIndex, setMenuIndex] = useState<number | null>(null);

  useEffect(() => {
    if (menuIndex === null) return;
    const close = () => setMenuIndex(null);
    document.addEventListener("click", close);
    return () => document.removeEventListener("click", close);
  }, [menuIndex]);

  function commit(next: Segment[]) {
    onChange(serializeTemplate(next));
  }

  function updateText(index: number, text: string) {
    const next = [...segments];
    next[index] = { type: "text", value: text };
    commit(next);
  }

  function removeToken(index: number) {
    commit(segments.filter((_, i) => i !== index));
  }

  function insertTokenAt(name: string, index: number) {
    const next = [...segments];
    next.splice(index, 0, { type: "token", name });
    commit(next);
  }

  function appendToken(name: string) {
    insertTokenAt(name, segments.length);
  }

  const groups = Array.from(new Set(tokens.map((t) => t.group)));

  return (
    <div className="flex flex-col gap-2">
      <div
        ref={rowRef}
        onDragOver={(e) => {
          if (disabled) return;
          e.preventDefault();
        }}
        onDrop={(e) => {
          if (disabled) return;
          e.preventDefault();
          const name = e.dataTransfer.getData(TOKEN_DRAG_MIME);
          if (!name || !rowRef.current) return;
          insertTokenAt(name, computeDropIndex(rowRef.current, e.clientX));
        }}
        className={`flex min-h-[34px] flex-wrap items-center gap-1 rounded border border-neutral-700 bg-neutral-950 px-2 py-1.5 ${
          disabled ? "opacity-40" : ""
        }`}
      >
        {segments.length === 1 && segments[0].type === "text" && segments[0].value === "" && (
          <span className="pointer-events-none absolute font-mono text-xs text-neutral-600">
            {placeholder}
          </span>
        )}
        {segments.map((seg, i) =>
          seg.type === "token" ? (
            <span
              key={i}
              data-segment-index={i}
              className="relative flex items-center gap-1 rounded bg-green-500/15 px-2 py-0.5 font-mono text-[11px] text-green-400"
            >
              {seg.name}
              {!disabled && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setMenuIndex(menuIndex === i ? null : i);
                  }}
                  className="text-green-400/70 hover:text-green-200"
                  aria-label={`Tùy chọn cho {${seg.name}}`}
                >
                  ⌄
                </button>
              )}
              {menuIndex === i && (
                <div className="absolute left-0 top-full z-10 mt-1 rounded border border-neutral-700 bg-neutral-900 py-0.5 shadow-lg">
                  <button
                    type="button"
                    onClick={() => {
                      removeToken(i);
                      setMenuIndex(null);
                    }}
                    className="whitespace-nowrap px-2 py-1 text-left font-mono text-[11px] text-neutral-200 hover:bg-neutral-800"
                  >
                    Xóa
                  </button>
                </div>
              )}
            </span>
          ) : (
            <input
              key={i}
              data-segment-index={i}
              value={seg.value}
              disabled={disabled}
              onChange={(e) => updateText(i, e.currentTarget.value)}
              autoComplete="off"
              style={{ width: `${Math.max(seg.value.length, 2)}ch` }}
              className="bg-transparent font-mono text-xs text-neutral-100 outline-none disabled:cursor-not-allowed"
            />
          ),
        )}
      </div>

      {!disabled && (
        <div className="flex flex-col gap-1">
          {groups.map((group) => (
            <div key={group} className="flex flex-wrap items-center gap-1">
              <span className="mr-1 text-[10px] uppercase tracking-wide text-neutral-600">
                {group}
              </span>
              {tokens
                .filter((t) => t.group === group)
                .map((t) => (
                  <button
                    key={t.name}
                    type="button"
                    draggable
                    onDragStart={(e) => e.dataTransfer.setData(TOKEN_DRAG_MIME, t.name)}
                    onClick={() => appendToken(t.name)}
                    title={`Thêm {${t.name}}`}
                    className="rounded border border-neutral-700 bg-neutral-900 px-1.5 py-0.5 font-mono text-[10px] text-neutral-400 hover:border-green-500 hover:text-green-400"
                  >
                    {t.name}
                  </button>
                ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
