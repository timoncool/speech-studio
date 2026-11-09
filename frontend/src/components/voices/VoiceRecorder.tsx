import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Mic, Square, X } from "lucide-react";
import { PERSON_ONLY, byPerson } from "../../lib/person";
import { api } from "../../lib/api";
import { askCloneTerms } from "../../lib/cloudConsent";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import { errorText } from "../../lib/voices";
import { followJob } from "../../lib/followJob";

/** Record a voice from a microphone: device, level, name; on stop the voice gets cleaned (if chosen) and its text recognised. */
export default function VoiceRecorder({ clean, onClose, onRecorded }: { clean: boolean; onClose: () => void; onRecorded: (name: string) => void }) {
  const { t } = useTranslation();
  const [devices, setDevices] = useState<string[] | null>(null);
  const [device, setDevice] = useState("");
  const [name, setName] = useState("");
  const [recording, setRecording] = useState(false);
  const [level, setLevel] = useState(0);
  const [seconds, setSeconds] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | null>(null);
  const started = useRef(0);

  useEffect(() => {
    api.recordDevices().then((d) => { setDevices(d.devices); setDevice(d.devices[0] ?? ""); }, (e: unknown) => { setDevices([]); setError(errorText(e)); });
    return () => { if (timer.current) window.clearInterval(timer.current); };
  }, []);

  const stopTimer = () => { if (timer.current) window.clearInterval(timer.current); timer.current = null; };
  const start = async () => {
    setError(null);
    if (!(await askCloneTerms())) return;
    try {
      const r = await api.recordStart(name.trim(), device || undefined);
      if (!r.ok) { setError(r.error ?? t("voices.recordFailed")); return; }
      setRecording(true);
      started.current = Date.now();
      timer.current = window.setInterval(() => {
        setSeconds(Math.floor((Date.now() - started.current) / 1000));
        api.recordLevel().then((l) => setLevel(l.level), (e: unknown) => setError(errorText(e)));
      }, 120);
    } catch (e) { setError(errorText(e)); }
  };
  const stop = async () => {
    stopTimer();
    setRecording(false);
    setLevel(0);
    try {
      const r = await api.recordStop({ clean, transcribe: true });
      onRecorded(r.name);
      if (r.level_problem) setError(r.level_problem);
      else onClose();
      if (r.job_id) await followJob(r.job_id, clean ? "voice_clean" : "voice_transcript").catch(() => undefined);
    } catch (e) { setError(errorText(e)); }
  };
  useEscapeLayer(() => { if (!recording) onClose(); });

  return (
    <div className="fixed inset-0 z-[70] grid place-items-center glass-scrim anim-fade" onClick={() => { if (!recording) onClose(); }}>
      <div role="dialog" aria-modal="true" aria-labelledby="rec-title" className="w-[min(92vw,440px)] rounded-xl glass-panel anim-pop p-5 space-y-3" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <span id="rec-title" className="font-semibold flex items-center gap-2"><Mic size={16} className="text-[var(--color-accent)]" />{t("voices.recordTitle")}</span>
          <button onClick={onClose} disabled={recording} {...named(t("a11y.close"))} className="text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30"><X size={16} /></button>
        </div>
        <p className="text-[12px] leading-relaxed text-[var(--color-muted)]">{t("voices.recordHint")}</p>
        <label className="block text-[12px] text-[var(--color-muted)]">{t("voices.device")}
          <select value={device} onChange={(e) => setDevice(e.target.value)} disabled={recording || !devices?.length}
            className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2.5 py-1.5 text-[13px] text-[var(--color-text)]">
            {devices === null && <option>…</option>}
            {devices?.length === 0 && <option value="">{t("voices.noDevices")}</option>}
            {devices?.map((d) => <option key={d} value={d}>{d}</option>)}
          </select>
        </label>
        <label className="block text-[12px] text-[var(--color-muted)]">{t("voices.name")}
          <input value={name} onChange={(e) => setName(e.target.value)} disabled={recording} placeholder={t("voices.namePlaceholder")} maxLength={48}
            className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2.5 py-1.5 text-[13px] text-[var(--color-text)] focus:border-[var(--color-accent)] focus:outline-none" />
        </label>
        <div className="flex items-center gap-3">
          <div className="flex-1 h-2 rounded-full bg-[var(--color-surface-2)] overflow-hidden" role="meter" aria-label={t("voices.level")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(Math.min(1, level * 1.4) * 100)}>
            <div className="h-full bg-[var(--color-accent)] transition-[width] duration-100" style={{ width: `${Math.min(100, level * 140)}%` }} />
          </div>
          <span className="mono text-[12px] tabnum w-12 text-right text-[var(--color-muted)]">{Math.floor(seconds / 60)}:{String(seconds % 60).padStart(2, "0")}</span>
        </div>
        {error && <p role="alert" className="text-[12px] text-[var(--color-warn)] break-words">{error}</p>}
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-[var(--color-muted)]">{clean ? t("voices.willClean") : t("voices.willNotClean")}</span>
          {recording ? (
            <button onClick={stop} className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg bg-[var(--color-warn)] text-[var(--color-on-accent)] text-[13px] font-semibold"><Square size={13} />{t("voices.recordStop")}</button>
          ) : (
            <button onClick={(e) => { if (byPerson(e)) void start(); }} {...PERSON_ONLY} disabled={!devices?.length} className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40"><Mic size={13} />{t("voices.recordStart")}</button>
          )}
        </div>
      </div>
    </div>
  );
}
