import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { useAllCommands } from "../../lib/commands";
import { useEscapeLayer } from "../../lib/escape";

/** Ctrl+K: every action of the window by its name, picked with the arrows and Enter. */
export default function CommandPalette({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const all = useAllCommands();
  const [query, setQuery] = useState("");
  const [at, setAt] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  useEscapeLayer(onClose);
  useEffect(() => { input.current?.focus(); }, []);
  const shown = useMemo(() => {
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    return all.filter((c) => !c.disabled && words.every((w) => `${c.label} ${c.group}`.toLowerCase().includes(w)));
  }, [all, query]);
  const run = (k: number) => {
    const c = shown[k];
    if (!c) return;
    onClose();
    c.run();
  };
  const keys = useRef({ shown, at, run });
  useEffect(() => {
    keys.current = { shown, at, run };
  });
  // On the window, not the field: an agent's typing leaves the field, and Enter still has to run the pick.
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      const { shown: list, at: k, run: go } = keys.current;
      if (e.key === "ArrowDown") { e.preventDefault(); setAt(Math.min(list.length - 1, k + 1)); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setAt(Math.max(0, k - 1)); }
      else if (e.key === "Enter") { e.preventDefault(); go(k); }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);
  return (
    <div className="fixed inset-0 z-[70] flex items-start justify-center bg-black/40 px-4 pt-[12vh]" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-label={t("palette.title")} onClick={(e) => e.stopPropagation()}
        className="w-[min(94vw,560px)] rounded-xl glass-panel anim-pop overflow-hidden">
        <div className="flex items-center gap-2 px-3 py-2.5 border-b border-[var(--color-border)]">
          <Search size={15} className="text-[var(--color-muted)]" />
          <input ref={input} value={query} onChange={(e) => { setQuery(e.target.value); setAt(0); }} placeholder={t("palette.placeholder")} aria-label={t("palette.title")}
            className="flex-1 bg-transparent text-[14px] focus:outline-none" />
        </div>
        <ul role="listbox" aria-label={t("palette.title")} className="max-h-[50vh] overflow-y-auto p-1.5">
          {shown.length === 0 && <li className="px-3 py-2 text-[13px] text-[var(--color-muted)]">{t("palette.none")}</li>}
          {shown.map((c, k) => (
            <li key={`${c.group}:${c.id}`} role="option" aria-selected={k === at} onMouseEnter={() => setAt(k)} onClick={() => run(k)}
              className={`flex items-center gap-3 px-3 py-2 rounded-md cursor-pointer text-[13px] ${k === at ? "bg-[var(--color-surface-2)]" : ""}`}>
              <span className="flex-1 truncate">{c.label}</span>
              <span className="text-[11px] text-[var(--color-muted)]">{c.group}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
