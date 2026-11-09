import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { Dices, X } from "lucide-react";
import { CLOUD_ENGINES, useEngines } from "../../lib/engines";
import { COSTLY_SOUND_CHECKS, SOUND_CHECKS, type Loudness, type SeedMode, type SpeechProject } from "../../lib/speech";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import { LANG_CHOICES } from "./langs";
import VoicePicker from "./VoicePicker";
import { PARAM_LABEL } from "./paramLabel";
import OpenRouterModelSelect from "../OpenRouterModelSelect";
import CloudModelPicker from "../cloud/CloudModelPicker";
import CloudForm from "../cloud/CloudForm";
import GoogleTts from "../GoogleTts";
import { api } from "../../lib/api";
import { openSettings } from "../../lib/settingsNav";
import Hint from "../Hint";
import VoiceLibrary from "../voices/VoiceLibrary";
import { defaultLibraryVoice, useVoices } from "../../lib/voices";

const RU_MARK = "RU";

const LOUDNESS: Loudness[] = ["video", "podcast", "acx", "off"];
const field = "mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1.5 text-[13px] mono";
const label = "block text-[12px] text-[var(--color-muted)]";

/** The project's synthesis: language, sampling, seed, pauses between lines and the loudness of the export. */
const NORMALIZE_GROUPS = ["numbers", "dates", "time", "money", "phones", "web", "codes", "roman"] as const;

/** How much checking a voicing gets: a fast draft, every take heard once, or several takes with the best kept. */
const QUALITY_MODES = {
  draft: { asr_check: false, takes: 1, similarity: false, strictness: "normal" },
  normal: { asr_check: true, takes: 1, similarity: false, strictness: "normal" },
  best: { asr_check: true, takes: 3, similarity: true, strictness: "strict" },
} as const;

export default function ProjectSettings({ project, onChange, onClose }: { project: SpeechProject; onChange: (f: (p: SpeechProject) => SpeechProject) => void; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  useEscapeLayer(onClose);
  const s = project.settings;
  const engines = useEngines();
  const engine = engines?.find((e) => e.id === s.engine) ?? null;
  const local = !CLOUD_ENGINES.includes(s.engine);
  const blocks = project.chapters.flatMap((c) => c.blocks).filter((b) => !b.hidden && b.text.trim());
  const catalogEngine = s.engine === "fal" || s.engine === "runware" ? s.engine : null;
  const continuable = engine?.continuation ?? false;
  const [cloudVoices, setCloudVoices] = useState<{ name: string; gender: string; ru: boolean }[]>([]);
  const [hasKey, setHasKey] = useState<boolean | null>(null);
  const { listing } = useVoices();
  const [library, setLibrary] = useState(false);
  const set = (f: (p: SpeechProject) => void) => onChange((p) => { f(p); return p; });
  const [studioGoogle, setStudioGoogle] = useState<{ model: string; mode: string }>({ model: "", mode: "standard" });
  const voiceModel = s.engine === "google" ? s.cloud_model || studioGoogle.model : s.cloud_model;
  useEffect(() => {
    if (s.engine === "openrouter") api.openrouterSettings().then((r) => setHasKey(r.configured), () => setHasKey(null));
    if (s.engine !== "google") return;
    api.googleSettings().then((r) => setHasKey(r.configured), () => setHasKey(null));
    api.capabilities().then((c) => setStudioGoogle({ model: String(c.selection?.google_tts_model ?? ""), mode: String(c.selection?.google_tts_mode || "standard") }), () => undefined);
  }, [s.engine]);
  useEffect(() => {
    if ((s.engine === "openrouter" || s.engine === "google") && voiceModel) api.openrouterVoices(voiceModel).then((r) => setCloudVoices(r.voices), () => setCloudVoices([]));
  }, [s.engine, voiceModel]);
  const setGoogle = (key: string, value: string) => {
    if (key === "google_tts_model") set((p) => { p.settings.cloud_model = value; p.settings.cloud_voice = ""; });
    else void api.setSelection(key, value).then(() => setStudioGoogle((g) => ({ ...g, mode: value })));
  };
  const voiceField = (
    <label className={label}>{t("speech.cloudVoice")}
      <input list="cloud-voices" value={s.cloud_voice} onChange={(e) => set((p) => { p.settings.cloud_voice = e.target.value; })} placeholder={t(s.engine === "google" ? "speech.googleVoicePick" : "speech.cloudVoiceDefault")} className={field} />
      <datalist id="cloud-voices">
        {cloudVoices.map((v) => <option key={v.name} value={v.name}>{[v.gender, v.ru ? RU_MARK : ""].filter(Boolean).join(" · ")}</option>)}
      </datalist>
    </label>
  );
  const num = (v: string, lo: number, hi: number, fallback: number) => { const n = Number(v); return Number.isFinite(n) ? Math.min(hi, Math.max(lo, n)) : fallback; };
  return (
    <div className="fixed inset-0 z-[60] grid place-items-center glass-scrim anim-fade" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-labelledby="ps-title" className="w-[min(94vw,560px)] max-h-[88vh] overflow-y-auto rounded-xl glass-panel anim-pop p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <h2 id="ps-title" className="font-semibold">{t("speech.settingsTitle")}</h2>
          <button onClick={onClose} {...named(t("a11y.close"))} className="text-[var(--color-muted)] hover:text-[var(--color-text)]"><X size={16} /></button>
        </div>
        <div className="space-y-2">
          <label className={label}>{t("speech.engine")} <Hint text={t("hint.engine")} />
            <select value={s.engine} onChange={(e) => set((p) => {
              p.settings.engine = e.target.value;
              p.settings.cloud_model = "";
              p.settings.cloud_voice = "";
              p.settings.cloud_values = {};
              const next = engines?.find((en) => en.id === e.target.value);
              for (const spec of next?.params ?? []) p.settings.params[spec.key] = spec.default;
              const before = new Set((engine?.presets ?? []).map((v) => v.id));
              const own = next?.presets ?? [];
              const stale = (v: string | null | undefined) => !!v && before.has(v) && !own.some((o) => o.id === v);
              if (stale(p.settings.voice)) p.settings.voice = null;
              for (const speaker of Object.keys(p.cast)) if (stale(p.cast[speaker])) delete p.cast[speaker];
              for (const b of p.chapters.flatMap((c) => c.blocks)) if (stale(b.voice)) b.voice = null;
              for (const l of Object.keys(p.settings.lang_voices ?? {})) if (stale(p.settings.lang_voices?.[l])) p.settings.lang_voices![l] = "";
              if (own.length && (!p.settings.voice || (next?.clone === "none" && !own.some((v) => v.id === p.settings.voice)))) p.settings.voice = own[0].id;
              if (next?.needs_voice && !own.length && !p.settings.voice) p.settings.voice = defaultLibraryVoice(listing, p.settings.lang, next.ref_text_required);
            })} className={field}>
              {(engines ?? []).map((en) => <option key={en.id} value={en.id} disabled={!!en.unavailable && en.id !== s.engine}>{en.name}{en.unavailable ? ` — ${t("speech.engineUnavailable")}` : ""}</option>)}
            </select>
          </label>
          {engine?.unavailable && <p role="alert" className="text-[11px] text-[var(--color-warn)]">{engine.unavailable}</p>}
          {!engine?.unavailable && engine?.needs_voice && !engine.presets.length && !s.voice && (
            <p className="text-[11px] text-[var(--color-warn)]">{t("speech.engineNeedsLibraryVoice")}{" "}
              <button onClick={() => setLibrary(true)} className="underline hover:text-[var(--color-text)]">{t("voice.packTitle")}</button>
            </p>
          )}
          {library && createPortal(
            <div className="relative z-[70]">
              <VoiceLibrary onClose={() => setLibrary(false)} onPick={(name) => { set((p) => { p.settings.voice = name; }); setLibrary(false); }} />
            </div>, document.body)}
          {!engine?.unavailable && engines?.some((en) => en.unavailable) && (
            <p className="text-[11px] text-[var(--color-muted)]">{engines.filter((en) => en.unavailable).map((en) => `${en.name}: ${en.unavailable}`).join(" ")}</p>
          )}
          {catalogEngine && (
            <div className="space-y-2 rounded-lg border border-[var(--color-border)] p-2.5">
              <CloudModelPicker task="speech" provider={catalogEngine} value={s.cloud_model} onChange={(id) => set((p) => { p.settings.cloud_model = id; p.settings.cloud_values = {}; })} />
              {s.cloud_model && (
                <CloudForm provider={catalogEngine} id={s.cloud_model} values={s.cloud_values ?? {}} onChange={(v) => set((p) => { p.settings.cloud_values = v; })}
                  runs={blocks.length} chars={blocks.reduce((n, b) => n + b.text.length, 0)} />
              )}
              <p className="text-[11px] text-[var(--color-muted)]">{t("speech.cloudHint")}</p>
            </div>
          )}
          {s.engine === "openrouter" && (
            <div className="space-y-2 rounded-lg border border-[var(--color-border)] p-2.5">
              {hasKey === false && (
                <p className="text-[11px] text-[var(--color-warn)]">{t("speech.cloudNoKey")} <button onClick={() => openSettings("cloud:key")} className="underline">{t("speech.cloudOpenKey")}</button></p>
              )}
              <OpenRouterModelSelect kind="tts" value={s.cloud_model} onChange={(id) => set((p) => { p.settings.cloud_model = id; p.settings.cloud_voice = ""; })} placeholder={t("speech.cloudModelDefault")} />
              {voiceField}
              <p className="text-[11px] text-[var(--color-muted)]">{t("speech.cloudHint")}</p>
            </div>
          )}
          {s.engine === "google" && (
            <div className="space-y-2 rounded-lg border border-[var(--color-border)] p-2.5">
              {hasKey === false && (
                <p className="text-[11px] text-[var(--color-warn)]">{t("speech.googleNoKey")} <button onClick={() => openSettings("cloud")} className="underline">{t("speech.cloudOpenKey")}</button></p>
              )}
              <GoogleTts model={s.cloud_model || studioGoogle.model} mode={studioGoogle.mode} onChange={setGoogle} />
              {voiceField}
              <p className="text-[11px] text-[var(--color-muted)]">{t("speech.googleHint")}</p>
            </div>
          )}
        </div>
        <label className={label}>{t("speech.lang")} <Hint text={t("hint.lang")} />
          <select value={s.lang} onChange={(e) => set((p) => { p.settings.lang = e.target.value; })} className={field}>
            {LANG_CHOICES.map((l) => {
              const off = !!engine && engine.languages.length > 0 && l !== "auto" && !engine.languages.includes(l);
              return <option key={l} value={l} disabled={off}>{l === "auto" ? t("speech.langAuto") : l.toUpperCase()}{off ? ` — ${t("speech.langNotByEngine", { engine: engine.name })}` : ""}</option>;
            })}
          </select>
        </label>
        <fieldset className="space-y-1.5">
          <legend className={label}>{t("speech.langVoices")} <Hint text={t("speech.langVoicesHint")} /></legend>
          {Object.entries(s.lang_voices ?? {}).map(([lang, voice]) => (
            <div key={lang} className="flex items-center gap-2">
              <span className="w-10 text-[12px] font-medium uppercase">{lang}</span>
              <VoicePicker engine={s.engine} value={voice || null} onChange={(v) => set((p) => { p.settings.lang_voices = { ...(p.settings.lang_voices ?? {}), [lang]: v ?? "" }; })} label={t("speech.langVoiceOf", { lang: lang.toUpperCase() })} className={`${field} flex-1`} />
              <button onClick={() => set((p) => { const next = { ...(p.settings.lang_voices ?? {}) }; delete next[lang]; p.settings.lang_voices = next; })} className="p-1 rounded text-[var(--color-muted)] hover:text-[var(--color-warn)]" {...named(t("speech.langVoiceRemove", { lang: lang.toUpperCase() }))}><X size={13} /></button>
            </div>
          ))}
          <select value="" onChange={(e) => { const lang = e.target.value; if (lang) set((p) => { p.settings.lang_voices = { ...(p.settings.lang_voices ?? {}), [lang]: "" }; }); }} aria-label={t("speech.langVoiceAdd")} className={`${field} w-auto`}>
            <option value="">{t("speech.langVoiceAdd")}</option>
            {LANG_CHOICES.filter((l) => l !== "auto" && l !== s.lang && !(l in (s.lang_voices ?? {}))).map((l) => <option key={l} value={l}>{l.toUpperCase()}</option>)}
          </select>
        </fieldset>
        <label className="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="checkbox" checked={s.normalize} onChange={(e) => set((p) => { p.settings.normalize = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("speech.normalize")} <Hint text={t("speech.normalizeHint")} /></span>
        </label>
        {s.normalize && (
          <fieldset className="ml-6 -mt-1">
            <legend className="text-[11px] text-[var(--color-muted)] mb-1">{t("speech.normalizeGroups")} <Hint text={t("hint.normalizeGroups")} /></legend>
            <div className="flex flex-wrap gap-1">
              {NORMALIZE_GROUPS.map((g) => {
                const on = !(s.normalize_off ?? []).includes(g);
                return (
                  <button key={g} type="button" aria-pressed={on} onClick={() => set((p) => { const off = new Set(p.settings.normalize_off ?? []); if (on) off.add(g); else off.delete(g); p.settings.normalize_off = NORMALIZE_GROUPS.filter((x) => off.has(x)); })}
                    className={`px-2 py-0.5 rounded-md border text-[11px] ${on ? "border-[var(--color-accent)] text-[var(--color-text)]" : "border-[var(--color-border)] text-[var(--color-muted)] line-through"}`}>{t(`speech.normalizeGroup_${g}`)}</button>
                );
              })}
            </div>
          </fieldset>
        )}
        <label className="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="checkbox" checked={s.auto_stress} onChange={(e) => set((p) => { p.settings.auto_stress = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("speech.autoStress")} <Hint text={t("speech.autoStressHint")} /></span>
        </label>
        {local && <>
{continuable && (
        <label className="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="checkbox" checked={s.continuation} onChange={(e) => set((p) => { p.settings.continuation = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("speech.continuation")} <Hint text={t("speech.continuationHint")} /></span>
        </label>
        )}
        <fieldset className="space-y-2">
          <legend className="text-[11px] uppercase tracking-[0.14em] text-[var(--color-muted)] mb-1">{t("speech.sampling")} <Hint text={t("hint.sampling")} /></legend>
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
            {(engine?.params ?? []).map((spec) => {
              const meta = PARAM_LABEL[spec.key];
              return (
                <label key={spec.key} className={label}>{meta.label.startsWith("speech.") ? t(meta.label as "speech.temperature") : meta.label} <Hint text={t(meta.hint as "speech.topKHint")} />
                  <input type="number" step={spec.step} min={spec.min} max={spec.max} value={s.params[spec.key]}
                    onChange={(e) => set((p) => { const v = num(e.target.value, spec.min, spec.max, spec.default); p.settings.params[spec.key] = spec.kind === "int" ? Math.round(v) : v; })} className={field} />
                </label>
              );
            })}
          </div>
          <p className="text-[11px] text-[var(--color-muted)]">{t("speech.samplingDefaults", { values: (engine?.params ?? []).filter((spec) => spec.default > 0).map((spec) => {
            const meta = PARAM_LABEL[spec.key];
            const name = meta.label.startsWith("speech.") ? t(meta.label as "speech.temperature").toLowerCase() : meta.label;
            return `${name} ${spec.kind === "int" ? spec.default : spec.default.toLocaleString(i18n.language, { minimumFractionDigits: 2 })}`;
          }).join(", ") })}</p>
          <div className="flex items-end gap-2">
            <label className={label + " flex-1"}>{t("speech.seed")} <Hint text={t("hint.seed")} />
              <select value={s.params.seed_mode} onChange={(e) => set((p) => { p.settings.params.seed_mode = e.target.value as SeedMode; })} className={field}>
                {(engine?.seed_modes ?? ["random", "fixed"]).map((m) => <option key={m} value={m}>{t(`speech.seed_${m}`)}</option>)}
              </select>
            </label>
            {s.params.seed_mode !== "random" && (
              <>
                <label className={label + " w-36"}>{t("speech.seedValue")}
                  <input type="number" min={0} max={2147483647} value={s.params.seed} onChange={(e) => set((p) => { p.settings.params.seed = Math.round(num(e.target.value, 0, 2147483647, 42)); })} className={field} />
                </label>
                <button onClick={() => set((p) => { p.settings.params.seed = Math.floor(Math.random() * 2147483647); })} className="mb-0.5 p-2 rounded-md border border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t("speech.seedNew"))}><Dices size={14} /></button>
              </>
            )}
          </div>
        </fieldset>
        </>}
        <fieldset className="grid grid-cols-2 gap-2">
          <legend className="col-span-2 text-[11px] uppercase tracking-[0.14em] text-[var(--color-muted)] mb-1">{t("speech.pauses")} <Hint text={t("speech.pausesHint")} /></legend>
          <label className={label}>{t("speech.pauseSame")}
            <input type="number" step={50} min={-2000} max={10000} value={s.pause_same_ms} onChange={(e) => set((p) => { p.settings.pause_same_ms = Math.round(num(e.target.value, -2000, 10000, 300)); })} className={field} />
          </label>
          <label className={label}>{t("speech.pauseSwitch")}
            <input type="number" step={50} min={-2000} max={10000} value={s.pause_switch_ms} onChange={(e) => set((p) => { p.settings.pause_switch_ms = Math.round(num(e.target.value, -2000, 10000, 600)); })} className={field} />
          </label>
        </fieldset>
        <fieldset className="space-y-2">
          <legend className="text-[11px] uppercase tracking-[0.14em] text-[var(--color-muted)] mb-1">{t("speech.quality")} <Hint text={t("hint.qualityMode")} /></legend>
          <div role="radiogroup" aria-label={t("speech.qualityMode")} className="flex flex-wrap items-center gap-1">
            {(Object.keys(QUALITY_MODES) as (keyof typeof QUALITY_MODES)[]).map((m) => {
              const q = QUALITY_MODES[m];
              const on = s.qc.asr_check === q.asr_check && s.qc.takes === q.takes && s.qc.similarity === q.similarity && s.qc.strictness === q.strictness;
              return (
                <button key={m} type="button" role="radio" aria-checked={on} onClick={() => set((p) => { p.settings.qc = { ...p.settings.qc, ...q }; })} title={t(`speech.qualityMode_${m}Hint`)}
                  className={`px-2.5 py-1 rounded-md border text-[12px] ${on ? "border-[var(--color-accent)] text-[var(--color-text)]" : "border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>{t(`speech.qualityMode_${m}`)}</button>
              );
            })}
          </div>
          <label className="flex items-center gap-2 text-[13px] cursor-pointer">
            <input type="checkbox" checked={s.qc.asr_check} onChange={(e) => set((p) => { p.settings.qc.asr_check = e.target.checked; })} className="w-4 h-4 accent-[var(--color-accent)]" />
            {t("speech.asrCheck")} <Hint text={t("speech.asrCheckHint")} />
          </label>
          {engine?.clone !== "none" && <>
            <label className="flex items-center gap-2 text-[13px] cursor-pointer">
              <input type="checkbox" checked={s.qc.similarity} onChange={(e) => set((p) => { p.settings.qc.similarity = e.target.checked; })} className="w-4 h-4 accent-[var(--color-accent)]" />
              {t("speech.similarityCheck")} <Hint text={t("speech.similarityHint")} />
            </label>
          </>}
          <div className="grid grid-cols-2 gap-2">
            <label className={label}>{t("speech.strictness")} <Hint text={t("hint.strictness")} />
              <select value={s.qc.strictness} onChange={(e) => set((p) => { p.settings.qc.strictness = e.target.value as "strict" | "normal" | "lenient"; })} className={field}>
                {(["strict", "normal", "lenient"] as const).map((k) => <option key={k} value={k}>{t(`speech.strictness_${k}`)}</option>)}
              </select>
            </label>
            <label className={label}>{t("speech.takesPerBlock")} <Hint text={t("speech.takesHint")} />
              <input type="number" min={1} max={20} value={s.qc.takes} onChange={(e) => set((p) => { p.settings.qc.takes = Math.round(num(e.target.value, 1, 20, 1)); })} className={field} />
            </label>
          </div>
          <div className="pt-1 space-y-1">
            <p className="text-[12px]">{t("speech.soundChecks")} <Hint text={t("speech.soundChecksHint")} /></p>
            {SOUND_CHECKS.map((k) => {
              const on = (s.qc.heuristics ?? []).includes(k);
              return (
                <label key={k} className="flex items-start gap-2 text-[12px] cursor-pointer">
                  <input type="checkbox" checked={on} onChange={(e) => set((p) => { const all = new Set(p.settings.qc.heuristics ?? []); if (e.target.checked) all.add(k); else all.delete(k); p.settings.qc.heuristics = SOUND_CHECKS.filter((x) => all.has(x)); })} className="mt-0.5 w-3.5 h-3.5 accent-[var(--color-accent)]" />
                  <span>{t(`speech.soundCheck_${k}`)}{COSTLY_SOUND_CHECKS.includes(k) && <span className="ml-1.5 text-[10px] text-[var(--color-warn)]">{t("speech.soundCheckCostly")}</span>}</span>
                </label>
              );
            })}
          </div>
        </fieldset>
        {project.chapters.some((c) => c.blocks.some((b) => b.start != null)) && (
          <fieldset className="space-y-2">
            <legend className="text-[11px] uppercase tracking-[0.14em] text-[var(--color-muted)] mb-1">{t("speech.timelineTitle")}</legend>
            <label className="flex items-start gap-2 text-[13px] cursor-pointer">
              <input type="checkbox" checked={s.timeline} onChange={(e) => set((p) => { p.settings.timeline = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
              <span>{t("speech.timeline")} <Hint text={t("speech.timelineHint")} /></span>
            </label>
            <div className="grid grid-cols-2 gap-2 items-end">
              <label className={label}>{t("speech.maxStretch")} <Hint text={t("speech.maxStretchHint")} />
                <input type="number" step={0.05} min={1} max={2} value={s.max_stretch} disabled={!s.timeline} onChange={(e) => set((p) => { p.settings.max_stretch = num(e.target.value, 1, 2, 1.25); })} className={field} />
              </label>
              <label className="flex items-center gap-2 text-[12px] cursor-pointer pb-1.5">
                <input type="checkbox" checked={s.lead_into_silence} disabled={!s.timeline} onChange={(e) => set((p) => { p.settings.lead_into_silence = e.target.checked; })} className="w-4 h-4 accent-[var(--color-accent)]" />{t("speech.leadIntoSilence")} <Hint text={t("hint.leadIntoSilence")} />
              </label>
            </div>
          </fieldset>
        )}
        <label className="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="checkbox" checked={s.tidy_pauses} onChange={(e) => set((p) => { p.settings.tidy_pauses = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("speech.tidyPauses")} <Hint text={t("speech.tidyPausesHint")} /></span>
        </label>
        <label className="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="checkbox" checked={s.level_pieces} onChange={(e) => set((p) => { p.settings.level_pieces = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("speech.levelPieces")} <Hint text={t("speech.levelPiecesHint")} /></span>
        </label>
        <label className="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="checkbox" checked={s.room_tone || s.loudness === "acx"} disabled={s.loudness === "acx"} onChange={(e) => set((p) => { p.settings.room_tone = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("speech.roomTone")} <Hint text={t("speech.roomToneHint")} /></span>
        </label>
        <label className="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="checkbox" checked={s.breaths} onChange={(e) => set((p) => { p.settings.breaths = e.target.checked; })} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
          <span>{t("speech.breaths")} <Hint text={t("speech.breathsHint")} /></span>
        </label>
        <label className={label}>{t("speech.loudness")} <Hint text={t("hint.loudness")} />
          <select value={s.loudness} onChange={(e) => set((p) => { p.settings.loudness = e.target.value as Loudness; })} className={field}>
            {LOUDNESS.map((l) => <option key={l} value={l}>{t(`speech.loudness_${l}`)}</option>)}
          </select>
        </label>
      </div>
    </div>
  );
}
