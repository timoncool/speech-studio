import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, X } from "lucide-react";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";

const MINUTES = [1, 2, 3, 5, 10];

/** What the director is asked inside an open project: where the script goes next, or how to word the chosen blocks anew. */
export default function DirectorAsk({ mode, count, busy, error, onRun, onClose }: { mode: "extend" | "rewrite"; count: number; busy: boolean; error: string | null; onRun: (text: string, minutes: number) => void; onClose: () => void }) {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const [minutes, setMinutes] = useState(3);
  useEscapeLayer(() => { if (!busy) onClose(); });
  const title = mode === "extend" ? t("enrich.extendTitle") : t("enrich.rewriteAskTitle", { n: count });
  const ready = mode === "extend" || text.trim().length > 0;
  const box = "bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1.5 text-[13px] focus:border-[var(--color-accent)] focus:outline-none";
  return (
    <div className="fixed inset-0 z-[60] grid place-items-center glass-scrim anim-fade" onClick={() => { if (!busy) onClose(); }}>
      <form role="dialog" aria-modal="true" aria-label={title} className="w-[min(94vw,560px)] rounded-xl glass-panel anim-pop p-5 space-y-3" onClick={(e) => e.stopPropagation()}
        onSubmit={(e) => { e.preventDefault(); if (ready && !busy) onRun(text.trim(), minutes); }}>
        <div className="flex items-center gap-2">
          <h2 className="font-semibold">{title}</h2>
          <button type="button" onClick={onClose} disabled={busy} className="ml-auto p-1.5 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-40" {...named(t("a11y.close"))}><X size={16} /></button>
        </div>
        <p className="text-[12px] text-[var(--color-muted)]">{t(mode === "extend" ? "enrich.extendHint" : "enrich.rewriteAskHint")}</p>
        <label className="block space-y-1 text-[12px]">
          <span className="text-[var(--color-muted)]">{t(mode === "extend" ? "enrich.extendDirection" : "enrich.rewriteHow")}</span>
          <textarea autoFocus value={text} onChange={(e) => setText(e.target.value)} rows={3} disabled={busy} placeholder={t(mode === "extend" ? "enrich.extendPlaceholder" : "enrich.rewritePlaceholder")} className={`w-full resize-y ${box}`} />
        </label>
        {mode === "extend" && (
          <label className="flex items-center gap-2 text-[12px]">
            <span className="text-[var(--color-muted)]">{t("enrich.extendMinutes")}</span>
            <select value={minutes} onChange={(e) => setMinutes(Number(e.target.value))} disabled={busy} className={box}>
              {MINUTES.map((m) => <option key={m} value={m}>{m}</option>)}
            </select>
          </label>
        )}
        {error && <p role="alert" className="text-[12px] text-[var(--color-warn)]">{error}</p>}
        <div className="flex justify-end">
          <button type="submit" disabled={!ready || busy} className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40">
            {busy && <Loader2 size={14} className="animate-spin" />}{t(mode === "extend" ? "enrich.extendRun" : "enrich.rewriteRun")}
          </button>
        </div>
      </form>
    </div>
  );
}
