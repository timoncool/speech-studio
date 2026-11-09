import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, Download, Loader2, X } from "lucide-react";
import { listenApi, TRANSCRIPT_FORMATS, type SubtitleRules, type TranscriptFormat } from "../../lib/listen";
import { errorText } from "../../lib/voices";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import FileActions from "../speech/FileActions";
import Hint from "../Hint";

/** The transcript as a file: SRT, VTT, ASS, TXT or JSON, with or without speakers and times; a preview and a copy. */
export default function ListenExport({ pid, onClose }: { pid: string; onClose: () => void }) {
  const { t } = useTranslation();
  useEscapeLayer(onClose);
  const [format, setFormat] = useState<TranscriptFormat>("srt");
  const [speakers, setSpeakers] = useState(true);
  const [times, setTimes] = useState(true);
  const [made, setMade] = useState<{ file: string; text: string; too_fast: number | null; digits_unknown: number | null } | null>(null);
  const [digits, setDigits] = useState(false);
  const [rules, setRules] = useState("");
  const [profiles, setProfiles] = useState<SubtitleRules[]>([]);
  const subtitles = format === "srt" || format === "vtt" || format === "ass";
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { listenApi.subtitleRules().then((r) => setProfiles(r.rules), (e: unknown) => setError(errorText(e))); }, []);
  const [copied, setCopied] = useState(false);
  const changed = <T,>(set: (v: T) => void) => (v: T) => { set(v); setMade(null); setCopied(false); };
  const make = async () => {
    setBusy(true);
    setError(null);
    try { setMade(await listenApi.export(pid, format, speakers, times, subtitles ? rules : "", format !== "json" && digits)); } catch (e) { setError(errorText(e)); } finally { setBusy(false); }
  };
  const copy = async () => {
    if (!made) return;
    try { await navigator.clipboard.writeText(made.text); setCopied(true); } catch (e) { setError(errorText(e)); }
  };
  const toggle = (on: boolean) => `px-2.5 py-1 rounded-md border text-[12px] ${on ? "border-[var(--color-accent)] text-[var(--color-accent)]" : "border-[var(--color-border)] text-[var(--color-muted)]"}`;
  return (
    <div className="fixed inset-0 z-[60] grid place-items-center glass-scrim anim-fade" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-labelledby="le-title" className="w-[min(94vw,640px)] max-h-[90vh] flex flex-col rounded-xl glass-panel anim-pop p-5 gap-3" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <h2 id="le-title" className="font-semibold">{t("listen.exportTitle")}</h2>
          <button onClick={onClose} className="text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t("a11y.close"))}><X size={16} /></button>
        </div>
        <div role="radiogroup" aria-label={t("listen.format")} className="flex flex-wrap gap-1.5">
          {TRANSCRIPT_FORMATS.map((f) => <button key={f} role="radio" aria-checked={format === f} onClick={() => changed(setFormat)(f)} className={`${toggle(format === f)} uppercase mono`}>{f}</button>)}
        </div>
        <p className="text-[11px] text-[var(--color-muted)]">{t(`listen.format_${format}`)}</p>
        <div className="flex flex-wrap gap-3 text-[13px]">
          {format !== "json" && <label className="flex items-center gap-2 cursor-pointer"><input type="checkbox" checked={speakers} onChange={(e) => changed(setSpeakers)(e.target.checked)} className="w-4 h-4 accent-[var(--color-accent)]" />{t("listen.withSpeakers")} <Hint text={t("hint.withSpeakers")} /></label>}
          {format !== "json" && <label className="flex items-center gap-2 cursor-pointer"><input type="checkbox" checked={digits} onChange={(e) => changed(setDigits)(e.target.checked)} className="w-4 h-4 accent-[var(--color-accent)]" />{t("listen.digits")} <Hint text={t("listen.digitsHint")} /></label>}
          {format === "txt" && <label className="flex items-center gap-2 cursor-pointer"><input type="checkbox" checked={times} onChange={(e) => changed(setTimes)(e.target.checked)} className="w-4 h-4 accent-[var(--color-accent)]" />{t("listen.withTimes")} <Hint text={t("hint.withTimes")} /></label>}
        </div>
        {subtitles && (
          <label className="block space-y-0.5 text-[12px]">
            <span>{t("listen.rules")} <Hint text={t(rules ? "listen.rulesHint" : "listen.rulesNoneHint")} /></span>
            <select value={rules} onChange={(e) => changed(setRules)(e.target.value)} className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]">
              <option value="">{t("listen.rulesNone")}</option>
              {profiles.map((p) => <option key={p.name} value={p.name}>{t("listen.rulesOne", { name: p.name, chars: p.max_line_len, lines: p.max_lines, cps: p.max_cps })}</option>)}
            </select>
          </label>
        )}
        {made?.digits_unknown ? <p className="text-[11px] text-[var(--color-warn)]">{t("listen.digitsUnknown", { count: made.digits_unknown })}</p> : null}
        {made?.too_fast ? <p className="text-[11px] text-[var(--color-warn)]">{t("listen.tooFast", { count: made.too_fast })}</p> : null}
        {made && <pre className="flex-1 min-h-24 max-h-72 overflow-auto rounded-md bg-[var(--color-surface-2)] border border-[var(--color-border)] p-2 text-[11px] mono whitespace-pre-wrap">{made.text.slice(0, 6000)}</pre>}
        {error && <p role="alert" className="text-[12px] text-[var(--color-warn)] break-words">{error}</p>}
        <div className="flex flex-wrap items-center gap-2">
          {made ? (
            <>
              <FileActions pid={pid} file={made.file} primary />
              <button onClick={() => void copy()} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]"><Copy size={13} />{copied ? t("listen.copied") : t("listen.copy")}</button>
            </>
          ) : (
            <button onClick={() => void make()} disabled={busy} className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40">
              {busy ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />}{t("listen.make")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
