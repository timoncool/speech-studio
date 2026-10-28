import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Archive, Check, Cloud, FileAudio, FolderInput, Loader2, Mic, Package, Pause, Play, ScanText, Sparkles, Star, Wand2, X } from "lucide-react";
import { api, ApiError, slot, type Selection, type VoiceEntry } from "../../lib/api";
import { askCloneTerms } from "../../lib/cloudConsent";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import { openSettings } from "../../lib/settingsNav";
import { SETTINGS_CHANGED, useChanged } from "../../lib/mcpBridge";
import { errorText, langName, prettyVoice, useVoices, voiceLabel } from "../../lib/voices";
import { applyVoiceFilter, NO_FILTER, type VoiceFacts, type VoiceFilter } from "../../lib/voiceFilter";
import VoiceFilterBar from "./VoiceFilterBar";
import { followJob } from "../../lib/followJob";
import { useStore } from "../../store";
import VoiceAvatar from "./VoiceAvatar";
import VoiceEditor from "./VoiceEditor";
import VoiceRecorder from "./VoiceRecorder";
import VoiceCatalog from "./VoiceCatalog";
import VoiceDesigner from "./VoiceDesigner";
import Hint, { Tip } from "../Hint";

const AUDIO_ACCEPT = "audio/*,.wav,.mp3,.flac,.m4a,.ogg";
const chip = "inline-flex items-center px-1.5 py-0.5 rounded text-[10px] uppercase tracking-wide bg-[var(--color-surface)] border border-[var(--color-border)] text-[var(--color-muted)]";
const tool = "inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] text-[var(--color-text)] hover:border-[var(--color-accent)] disabled:opacity-40 transition-colors";

const SOURCES = ["pack", "file", "mic", "russia_voices", "speaker"] as const;


/** The voice library: every voice of the studio as a persona card, with search, filters and every way to add voices. */
export default function VoiceLibrary({ onClose, onPick }: { onClose: () => void; onPick?: (name: string) => void }) {
  const { t } = useTranslation();
  const { listing, error: loadError, reload, patch } = useVoices();
  const [filter, setFilter] = useState<VoiceFilter>(NO_FILTER);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [editing, setEditing] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const [catalog, setCatalog] = useState(false);
  const [designing, setDesigning] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [cleanBlocked, setCleanBlocked] = useState<File[] | null>(null);
  const [playing, setPlaying] = useState<string | null>(null);
  const [sel, setSel] = useState<Selection | null>(null);
  const audio = useRef<HTMLAudioElement | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const zipInput = useRef<HTMLInputElement>(null);
  const pushActivity = useStore((s) => s.pushActivity);

  const loadSel = () => { api.capabilities().then((c) => setSel(c.selection ?? {}), (e: unknown) => setError(errorText(e))); };
  useEffect(loadSel, []);
  useChanged(SETTINGS_CHANGED, loadSel);
  useEffect(() => () => { audio.current?.pause(); }, []);
  useEscapeLayer(onClose, !editing && !recording && !catalog && !designing && !cleanBlocked);
  const cleanOn = (slot(sel ?? undefined, "voice_clean") ?? "1") !== "0";
  const setCleanOn = (on: boolean) => { api.setSelection("voice_clean", on ? "1" : "0").then(setSel, (e: unknown) => setError(errorText(e))); };

  const voices = useMemo(() => listing?.voices ?? [], [listing]);
  const langs = useMemo(() => [...new Set(voices.map((v) => v.card.lang).filter(Boolean))].sort((a, b) => langName(a).localeCompare(langName(b))), [voices]);
  const sources = useMemo(() => [...new Set(voices.map((v) => v.card.source).filter(Boolean))].sort(), [voices]);
  const facts = (v: VoiceEntry): VoiceFacts => ({
    label: voiceLabel(v), gender: v.card.gender, age: v.card.age, lang: v.card.lang, description: v.card.description ?? "",
    tags: [v.name, ...v.card.tags], source: v.card.source, favorite: v.card.favorite, duration: v.card.duration_s ?? null, born: v.card.birth_year ?? null,
  });
  // favourites stay on top in every order
  const shown = applyVoiceFilter(voices, facts, filter).sort((a, b) => Number(b.card.favorite) - Number(a.card.favorite));
  const sourceLabel = (s: string) => (SOURCES.includes(s as (typeof SOURCES)[number]) ? t(`voices.src_${s as (typeof SOURCES)[number]}`) : s);
  const editingVoice = editing ? voices.find((v) => v.name === editing) ?? null : null;
  const names = useMemo(() => new Set(voices.map((v) => v.name)), [voices]);

  const play = (v: VoiceEntry) => {
    audio.current?.pause();
    if (playing === v.name) { setPlaying(null); return; }
    const a = new Audio(api.voiceSampleUrl(v.name, false, v.size));
    audio.current = a;
    setPlaying(v.name);
    a.onended = () => setPlaying(null);
    a.play().catch((e: unknown) => { setPlaying(null); setError(errorText(e)); });
  };
  const toggle = (n: string) => setSelected((s) => { const x = new Set(s); if (x.has(n)) x.delete(n); else x.add(n); return x; });

  const importFiles = async (files: File[], clean: boolean) => {
    if (!(await askCloneTerms())) return;
    setBusy("import");
    setError(null);
    try {
      const r = await api.voiceImport(files, { clean, transcribe: true });
      pushActivity(t("voices.imported", { names: r.names.map(prettyVoice).join(", ") }), "done");
      reload();
      for (const j of r.jobs) {
        if ("job_id" in j) await followJob(j.job_id, clean ? "voice_clean" : "voice_transcript").catch(() => undefined);
        else setError(errorText(new ApiError("job", JSON.stringify(j.error))));
      }
      reload();
    } catch (e) {
      if (e instanceof ApiError && e.code === "clean_missing") setCleanBlocked(files);
      else setError(errorText(e));
    } finally { setBusy(null); }
  };
  const untranscribed = voices.filter((v) => !v.transcript);
  const transcribeAll = async () => {
    setBusy("transcribeAll");
    setError(null);
    try {
      for (const v of untranscribed) {
        const { job_id } = await api.voiceTranscribe(v.name);
        await followJob(job_id, "voice_transcript").catch(() => undefined);
      }
      reload();
    } catch (e) { setError(errorText(e)); } finally { setBusy(null); }
  };
  const pack = async () => {
    setBusy("pack");
    setError(null);
    try {
      const { job_id } = await api.voicesDownloadPack();
      await followJob(job_id, "voices_pack");
      reload();
    } catch (e) { setError(errorText(e)); } finally { setBusy(null); }
  };
  const exportZip = async () => {
    setBusy("export");
    setError(null);
    try {
      const blob = await api.voicesExport([...selected]);
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "voices.zip";
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) { setError(errorText(e)); } finally { setBusy(null); }
  };
  const importZip = async (f: File) => {
    setBusy("zip");
    setError(null);
    try {
      const r = await api.voicesImportZip(f);
      pushActivity(t("voices.zipImported", { added: r.added.length, already: r.already.length }), "done");
      reload();
    } catch (e) { setError(errorText(e)); } finally { setBusy(null); }
  };

  return (
    <div className="fixed inset-0 z-[55] grid place-items-center glass-scrim anim-fade" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-labelledby="voices-title" className="w-[min(96vw,1100px)] h-[min(92vh,860px)] flex flex-col rounded-xl glass-panel anim-pop overflow-hidden" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-3 px-5 py-3.5 border-b border-[var(--color-border)]">
          <h2 id="voices-title" className="font-semibold whitespace-nowrap">{t("voices.title")}</h2>
          <div className="ml-auto flex flex-wrap items-center justify-end gap-1.5">
            <button onClick={() => setRecording(true)} className={tool}><Mic size={13} />{t("voices.record")}</button>
            <button onClick={() => fileInput.current?.click()} disabled={!!busy} className={tool}>{busy === "import" ? <Loader2 size={13} className="animate-spin" /> : <FileAudio size={13} />}{t("voices.importFiles")}</button>
            <Tip text={t("hint.voiceCatalog")}><button onClick={() => setCatalog(true)} className={tool}><Cloud size={13} />{t("voices.catalog")}</button></Tip>
            <Tip text={t("voices.designHint")}><button onClick={() => setDesigning(true)} className={tool}><Wand2 size={13} />{t("voices.design")}</button></Tip>
            <Tip text={t("hint.voicePack")}><button onClick={pack} disabled={!!busy} className={tool}>{busy === "pack" ? <Loader2 size={13} className="animate-spin" /> : <Package size={13} />}{t("voices.pack")}</button></Tip>
            <Tip text={t("hint.voiceZipIn")}><button onClick={() => zipInput.current?.click()} disabled={!!busy} className={tool}>{busy === "zip" ? <Loader2 size={13} className="animate-spin" /> : <FolderInput size={13} />}{t("voices.importZip")}</button></Tip>
            <Tip text={t("hint.voiceZipOut")}><button onClick={exportZip} disabled={!selected.size || !!busy} className={tool}>{busy === "export" ? <Loader2 size={13} className="animate-spin" /> : <Archive size={13} />}{t("voices.exportPicked", { n: selected.size })}</button></Tip>
            <button onClick={onClose} {...named(t("a11y.close"))} className="ml-1 p-1.5 text-[var(--color-muted)] hover:text-[var(--color-text)]"><X size={16} /></button>
          </div>
          <input ref={fileInput} type="file" multiple accept={AUDIO_ACCEPT} className="hidden" onChange={(e) => { const f = [...(e.target.files ?? [])]; e.target.value = ""; if (f.length) importFiles(f, cleanOn); }} />
          <input ref={zipInput} type="file" accept=".zip,application/zip" className="hidden" onChange={(e) => { const f = e.target.files?.[0]; e.target.value = ""; if (f) importZip(f); }} />
        </div>

        <div className="flex flex-wrap items-center gap-2 px-5 py-2.5 border-b border-[var(--color-border)]">
          <div className="flex-1 min-w-[320px]">
            <VoiceFilterBar filter={filter} onChange={setFilter} languages={langs} langLabel={langName} sources={sources} sourceLabel={sourceLabel}
              favorites shown={shown.length} total={voices.length} t={t} />
          </div>
          {untranscribed.length > 0 && (
            <Tip text={t("voices.transcribeAllHint")}><button onClick={transcribeAll} disabled={!!busy} className={tool}>
              {busy === "transcribeAll" ? <Loader2 size={13} className="animate-spin" /> : <ScanText size={13} />}{t("voices.transcribeAll", { n: untranscribed.length })}
            </button></Tip>
          )}
          <label className="ml-auto inline-flex items-center gap-2 text-[12px] cursor-pointer">
            <input type="checkbox" checked={cleanOn} onChange={(e) => setCleanOn(e.target.checked)} className="w-3.5 h-3.5 accent-[var(--color-accent)]" />
            <Sparkles size={13} className="text-[var(--color-accent)]" />{t("voices.cleanToggle")} <Hint text={t("hint.cleanToggle")} />
          </label>
        </div>

        {(error || loadError) && <div role="alert" className="mx-5 mt-3 px-3 py-2 rounded-lg border border-[var(--color-warn)]/40 text-[12px] text-[var(--color-warn)] break-words">{error ?? loadError}</div>}
        {listing && listing.broken.length > 0 && (
          <div role="alert" className="mx-5 mt-3 px-3 py-2 rounded-lg border border-[var(--color-warn)]/40 text-[12px] text-[var(--color-warn)] break-words">
            {t("voices.broken")} {listing.broken.map((b) => `${b.name}: ${b.error}`).join("; ")}
          </div>
        )}
        {cleanBlocked && (
          <div role="alert" className="mx-5 mt-3 px-3 py-2.5 rounded-lg border border-[var(--color-accent)]/40 bg-[color-mix(in_oklab,var(--color-accent)_8%,transparent)] text-[12px] space-y-2">
            <div>{t("voices.cleanMissing")}</div>
            <div className="flex flex-wrap gap-2">
              <button onClick={() => { setCleanBlocked(null); openSettings("models"); }} className={tool}>{t("voices.cleanDownload")}</button>
              <button onClick={() => { const f = cleanBlocked; setCleanBlocked(null); importFiles(f, false); }} className={tool}>{t("voices.useWithoutCleaning")}</button>
              <button onClick={() => setCleanBlocked(null)} className={tool}>{t("voices.cancel")}</button>
            </div>
          </div>
        )}

        <div className="flex-1 overflow-y-auto p-5">
          {listing === null && !loadError && <div className="grid place-items-center py-16"><Loader2 size={20} className="animate-spin text-[var(--color-muted)]" /></div>}
          {listing && voices.length === 0 && (
            <div className="max-w-md mx-auto text-center py-12 space-y-3">
              <p className="text-[14px] font-medium">{t("voices.emptyTitle")}</p>
              <p className="text-[12px] text-[var(--color-muted)] leading-relaxed">{t("voices.emptyHint")}</p>
              <div className="flex flex-wrap justify-center gap-2">
                <button onClick={pack} disabled={!!busy} className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40"><Package size={14} />{t("voices.pack")}</button>
                <button onClick={() => setRecording(true)} className={tool}><Mic size={13} />{t("voices.record")}</button>
              </div>
            </div>
          )}
          {listing && voices.length > 0 && shown.length === 0 && (
            <div className="text-center py-12 space-y-2">
              <p className="text-[13px] text-[var(--color-muted)]">{t("voices.nothingFound")}</p>
              <button type="button" onClick={() => setFilter({ ...NO_FILTER, sort: filter.sort })} className="text-[12px] text-[var(--color-accent)] hover:underline">{t("voices.resetFilters")}</button>
            </div>
          )}
          <ul className="grid gap-2.5 [grid-template-columns:repeat(auto-fill,minmax(240px,1fr))]">
            {shown.map((v) => (
              <li key={v.name} data-mcp-row={`voice ${voiceLabel(v)}${v.transcript ? "" : " (no reference text)"}`} className={`group relative rounded-xl border p-3 transition-colors ${selected.has(v.name) ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_7%,var(--color-surface))]" : "border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[color-mix(in_oklab,var(--color-accent)_45%,var(--color-border))]"}`}>
                <div className="flex items-start gap-2.5">
                  <button onClick={() => setEditing(v.name)} className="rounded-full" {...named(t("voices.edit"))}><VoiceAvatar voice={v} /></button>
                  <div className="min-w-0 flex-1">
                    <button onClick={() => setEditing(v.name)} className="block w-full text-left text-[13px] font-semibold truncate hover:text-[var(--color-accent)]" title={v.name}>{voiceLabel(v)}</button>
                    <div className="mt-1 flex flex-wrap gap-1">
                      {v.card.gender && <span className={chip}>{t(`voices.gender_${v.card.gender}`)}</span>}
                      {v.card.age && <span className={chip}>{t(`voices.age_${v.card.age}`)}</span>}
                      {v.card.lang && <span className={chip}>{v.card.lang}</span>}
                      {typeof v.card.duration_s === "number" && <span className={chip}>{t("voices.seconds", { s: Math.round(v.card.duration_s) })}</span>}
                      {typeof v.card.birth_year === "number" && <span className={chip}>{t("voices.born", { year: v.card.birth_year })}</span>}
                      {v.card.cleaned &&<span className={`${chip} text-[var(--color-accent)]`}>{t("voices.cleanedBadge")}</span>}
                      {v.card.variant_of && <span className={chip}>{t("voices.variantBadge", { variant: v.card.variant, of: prettyVoice(v.card.variant_of) })}</span>}
                    </div>
                    {v.card.description && <p className="mt-1 text-[11px] text-[var(--color-muted)] line-clamp-2" title={v.card.description}>{v.card.description}</p>}
                  </div>
                  <input type="checkbox" checked={selected.has(v.name)} onChange={() => toggle(v.name)} aria-label={t("voices.select", { name: voiceLabel(v) })}
                    className="mt-0.5 w-3.5 h-3.5 accent-[var(--color-accent)] opacity-60 group-hover:opacity-100 focus:opacity-100" />
                </div>
                {v.card.tags.length > 0 && <div className="mt-2 text-[11px] text-[var(--color-muted)] truncate">#{v.card.tags.join(" #")}</div>}
                <div className="mt-2.5 flex items-center gap-1.5">
                  <button onClick={() => play(v)} {...named(playing === v.name ? t("voices.stop") : t("voice.preview"))}
                    className="p-1.5 rounded-md bg-[var(--color-surface-2)] text-[var(--color-text)] hover:text-[var(--color-accent)]">{playing === v.name ? <Pause size={13} /> : <Play size={13} />}</button>
                  <span className={`text-[11px] truncate flex-1 ${v.transcript ? "text-[var(--color-muted)]" : "text-[var(--color-warn)]"}`} title={v.transcript ?? ""}>
                    {v.transcript ? <><Check size={11} className="inline mr-0.5 -mt-0.5" />{t("voices.transcriptReady")}</> : t("voices.transcriptNone")}
                  </span>
                  <button onClick={() => api.voiceSetCard(v.name, { favorite: !v.card.favorite }).then((r) => patch(r.voice), (e: unknown) => setError(errorText(e)))}
                    aria-pressed={v.card.favorite} {...named(t("voices.favorite"))}
                    className={`p-1 rounded-md ${v.card.favorite ? "text-[var(--color-warn)]" : "text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}><Star size={14} fill={v.card.favorite ? "currentColor" : "none"} /></button>
                  {onPick && <button onClick={() => onPick(v.name)} className="px-2 py-1 rounded-md bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[11px] font-semibold">{t("voices.pick")}</button>}
                </div>
              </li>
            ))}
          </ul>
        </div>
      </div>

      {editingVoice && (
        <VoiceEditor key={editingVoice.name} voice={editingVoice} all={voices} onClose={() => setEditing(null)} onChanged={patch}
          onRenamed={(to) => { setEditing(to); reload(); }} onDeleted={() => { setEditing(null); reload(); }} />
      )}
      {recording && <VoiceRecorder clean={cleanOn} onClose={() => setRecording(false)} onRecorded={() => reload()} />}
      {catalog && <VoiceCatalog have={names} onClose={() => setCatalog(false)} onGot={reload} />}
      {designing && <VoiceDesigner onClose={() => setDesigning(false)} onKept={(name) => { setDesigning(false); reload(); setEditing(name); }} />}
    </div>
  );
}
