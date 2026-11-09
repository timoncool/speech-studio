import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Trash2, X } from "lucide-react";
import { DENOISE_LEVELS, VAD_DEFAULTS, type ListenEngine, type ListenProject, type VadConfig } from "../../lib/listen";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import OpenRouterModelSelect from "../OpenRouterModelSelect";
import CloudModelPicker from "../cloud/CloudModelPicker";
import { LANG_CHOICES } from "../speech/langs";
import { api, slot } from "../../lib/api";
import Hint from "../Hint";

import { ENGINES } from "./engines";

const SPEAKERS = [0, 1, 2, 3, 4, 5, 6, 7, 8];

/** How the recording is recognised: engine, language, speakers, noise removal, voice separation, hallucination filter, a
 * hint for Whisper (the only engine that takes one), how Parakeet's pieces are cut at pauses, corrections. */
export default function ListenSettings({ project, onChange, onClose }: { project: ListenProject; onChange: (p: ListenProject) => void; onClose: () => void }) {
  const { t } = useTranslation();
  useEscapeLayer(onClose);
  const s = project.settings;
  const set = (patch: Partial<ListenProject["settings"]>) => onChange({ ...project, settings: { ...s, ...patch } });
  const [studioAsr, setStudioAsr] = useState<string | undefined>();
  useEffect(() => {
    api.capabilities().then((c) => setStudioAsr(slot(c.selection, "or_asr_on") === "1" ? "openrouter" : slot(c.selection, "asr_engine") ?? "parakeet"), () => undefined);
  }, []);
  const effective = s.engine || studioAsr;
  const whisper = effective === "whisper";
  const local = effective === "parakeet" || effective === "nemotron";
  const vad = { ...VAD_DEFAULTS, ...s.vad };
  const setVad = (patch: Partial<VadConfig>) => set({ vad: { ...vad, ...patch } });
  const vadFields: { key: keyof VadConfig; step: number; min: number; max: number }[] = [
    { key: "threshold", step: 0.05, min: 0.05, max: 0.95 },
    { key: "hysteresis", step: 0.05, min: 0, max: 0.5 },
    { key: "min_speech_ms", step: 50, min: 0, max: 5000 },
    { key: "min_silence_ms", step: 50, min: 0, max: 5000 },
    { key: "pad_ms", step: 10, min: 0, max: 1000 },
  ];
  const fixes = project.fixes;
  const setFixes = (next: ListenProject["fixes"]) => onChange({ ...project, fixes: next });
  const field = "w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1.5 text-[13px] focus:border-[var(--color-accent)] focus:outline-none";
  const label = "block space-y-1 text-[12px] text-[var(--color-muted)]";
  const check = "flex items-start gap-2 text-[13px] cursor-pointer";
  return (
    <div className="fixed inset-0 z-[60] grid place-items-center glass-scrim anim-fade" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-labelledby="ls-title" className="w-[min(94vw,560px)] max-h-[90vh] overflow-y-auto rounded-xl glass-panel anim-pop p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <h2 id="ls-title" className="font-semibold">{t("listen.settingsTitle")}</h2>
          <button onClick={onClose} className="text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t("a11y.close"))}><X size={16} /></button>
        </div>
        <label className={label}>{t("listen.engine")} <Hint text={t("hint.listenEngine")} />
          <select value={s.engine} onChange={(e) => set({ engine: e.target.value as ListenEngine })} className={field}>
            {ENGINES.map((e) => <option key={e} value={e}>{t(`listen.engine_${e || "default"}`)}</option>)}
          </select>
          <span className="block text-[11px] leading-relaxed">{t(`listen.engineHint_${s.engine || "default"}`)}</span>
        </label>
        {s.engine === "fal" && <CloudModelPicker task="transcribe" provider="fal" value={s.cloud_model} onChange={(id) => set({ cloud_model: id })} />}
        {s.engine === "openrouter" && (
          <OpenRouterModelSelect kind="asr" value={s.cloud_model} onChange={(id) => set({ cloud_model: id })} placeholder={t("listen.cloudModelDefault")} />
        )}
        <div className="grid grid-cols-2 gap-3">
          <label className={label}>{t("speech.lang")} <Hint text={t("hint.listenLang")} />
            <select value={s.lang} onChange={(e) => set({ lang: e.target.value })} className={field}>
              {LANG_CHOICES.map((l) => <option key={l} value={l}>{l === "auto" ? t("speech.langAuto") : l.toUpperCase()}</option>)}
            </select>
          </label>
          <label className={label}>{t("listen.speakers")} <Hint text={t("hint.listenSpeakers")} />
            <select value={s.speakers} onChange={(e) => set({ speakers: Number(e.target.value) })} className={field}>
              {SPEAKERS.map((n) => <option key={n} value={n}>{n === 0 ? t("listen.speakersAuto") : n === 1 ? t("listen.speakersOne") : n}</option>)}
            </select>
          </label>
        </div>
        {whisper && (
          <label className={label}>{t("listen.prompt")} <Hint text={t("listen.promptHint")} />
            <textarea value={s.prompt ?? ""} onChange={(e) => set({ prompt: e.target.value })} rows={2} placeholder={t("listen.promptPlaceholder")} className={`${field} resize-y`} />
          </label>
        )}
        <div className="flex items-start gap-2">
          <label className={`${check} flex-1`}>
            <input type="checkbox" checked={!!s.denoise} onChange={(e) => set({ denoise: e.target.checked })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
            <span>{t("listen.denoise")} <Hint text={t("listen.denoiseHint")} /></span>
          </label>
          {s.denoise && (
            <select value={s.denoise_db ?? 18} onChange={(e) => set({ denoise_db: Number(e.target.value) })} aria-label={t("denoise.strength")} className="shrink-0 bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px] focus:border-[var(--color-accent)] focus:outline-none">
              {DENOISE_LEVELS.map((db) => <option key={db} value={db}>{db ? t("denoise.db", { db }) : t("denoise.full")}</option>)}
            </select>
          )}
        </div>
        <label className={check}>
          <input type="checkbox" checked={s.separate} onChange={(e) => set({ separate: e.target.checked })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("listen.separate")} <Hint text={t("listen.separateHint")} /></span>
        </label>
        <label className={check}>
          <input type="checkbox" checked={s.filter} onChange={(e) => set({ filter: e.target.checked })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("listen.filter")} <Hint text={t("listen.filterHint")} /></span>
        </label>
        {local && (
          <details className="rounded-lg border border-[var(--color-border)] px-3 py-2">
            <summary className="cursor-pointer text-[12px] font-semibold">{t("listen.vad")} <Hint text={t("listen.vadHint")} /></summary>
            <div className="mt-2 grid grid-cols-2 gap-2">
              {vadFields.map((f) => (
                <label key={f.key} className={label}>{t(`listen.vad_${f.key}`)} <Hint text={t(`hint.vad_${f.key}`)} />
                  <input type="number" value={vad[f.key]} step={f.step} min={f.min} max={f.max}
                    onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) setVad({ [f.key]: Math.min(f.max, Math.max(f.min, v)) }); }} className={field} />
                </label>
              ))}
            </div>
            <button onClick={() => set({ vad: VAD_DEFAULTS })} className="mt-2 text-[11px] text-[var(--color-muted)] hover:text-[var(--color-accent)]">{t("listen.vadReset")}</button>
          </details>
        )}
        <fieldset className="space-y-2">
          <legend className="text-[11px] uppercase tracking-[0.14em] text-[var(--color-muted)] mb-1">{t("listen.fixes")} <Hint text={t("listen.fixesHint")} /></legend>
          {fixes.map((f, i) => (
            <div key={i} className="flex items-center gap-2">
              <input value={f.heard} onChange={(e) => setFixes(fixes.map((x, j) => (j === i ? { ...x, heard: e.target.value } : x)))} placeholder={t("listen.fixHeard")} aria-label={t("listen.fixHeard")} className={field} />
              <span className="text-[var(--color-muted)]">→</span>
              <input value={f.say} onChange={(e) => setFixes(fixes.map((x, j) => (j === i ? { ...x, say: e.target.value } : x)))} placeholder={t("listen.fixSay")} aria-label={t("listen.fixSay")} className={field} />
              <button onClick={() => setFixes(fixes.filter((_, j) => j !== i))} className="p-1 rounded text-[var(--color-muted)] hover:text-[var(--color-warn)]" {...named(t("listen.fixRemove"))}><Trash2 size={13} /></button>
            </div>
          ))}
          <button onClick={() => setFixes([...fixes, { heard: "", say: "" }])} className="inline-flex items-center gap-1 px-2 py-1 rounded-md border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]"><Plus size={12} />{t("listen.fixAdd")}</button>
        </fieldset>
        <p className="text-[11px] text-[var(--color-muted)]">{t("listen.settingsApply")}</p>
      </div>
    </div>
  );
}
