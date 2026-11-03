import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AudioLines, Camera, Crop, Loader2, Pause, Play, RotateCcw, ScanText, Scissors, Sparkles, Star, Trash2, X } from "lucide-react";
import { api, slot, type RefCandidate, type VoiceAge, type VoiceCard, type VoiceConsent, type VoiceEntry, type VoiceGender } from "../../lib/api";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import { errorText, voiceLabel } from "../../lib/voices";
import { followJob } from "../../lib/followJob";
import ConfirmDialog from "../ConfirmDialog";
import VoiceAvatar from "./VoiceAvatar";
import RefNeeds from "./RefNeeds";
import RefTrim from "./RefTrim";
import { DENOISE_LEVELS } from "../../lib/listen";
import Hint, { Tip } from "../Hint";

const GENDERS: VoiceGender[] = ["", "female", "male"];
const AGES: VoiceAge[] = ["", "child", "teen", "adult", "senior"];
const CONSENTS: VoiceConsent[] = ["", "own", "permitted", "public"];

const field = "w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2.5 py-1.5 text-[13px] text-[var(--color-text)] focus:border-[var(--color-accent)] focus:outline-none";
const label = "block text-[11px] uppercase tracking-[0.12em] text-[var(--color-muted)] mb-1";

/** One voice's card: name, photo, who it is, where it comes from and the text of its reference. */
export default function VoiceEditor({ voice, all, onClose, onChanged, onRenamed, onDeleted }: {
  voice: VoiceEntry;
  /** The library, for the voices this one can be a variant of. */
  all: VoiceEntry[];
  onClose: () => void;
  onChanged: (v: VoiceEntry) => void;
  onRenamed: (to: string) => void;
  onDeleted: () => void;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState(voice.name);
  const [tags, setTags] = useState(voice.card.tags.join(", "));
  const [source, setSource] = useState(voice.card.source);
  const [lang, setLang] = useState(voice.card.lang);
  const [variant, setVariant] = useState(voice.card.variant);
  const [license, setLicense] = useState(voice.card.license);
  useEffect(() => { setVariant(voice.card.variant); }, [voice.card.variant]);
  const [transcript, setTranscript] = useState(voice.transcript ?? "");
  const [busy, setBusy] = useState<string | null>(null);
  const [denoiseDb, setDenoiseDb] = useState(18);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [photoRev, setPhotoRev] = useState(0);
  const photoInput = useRef<HTMLInputElement>(null);
  const [pieces, setPieces] = useState<RefCandidate[] | null>(null);
  const [trimming, setTrimming] = useState(false);
  const [refLimit, setRefLimit] = useState(12);
  useEffect(() => { api.capabilities().then((c) => setRefLimit(Number(slot(c.selection, "higgs_ref_secs") || "12")), (e: unknown) => setError(errorText(e))); }, []);
  const [playingPiece, setPlayingPiece] = useState<number | null>(null);
  const piecePlayer = useRef<HTMLAudioElement | null>(null);
  useEffect(() => () => { piecePlayer.current?.pause(); }, []);
  useEscapeLayer(onClose, !confirmDelete);

  const run = async (what: string, action: () => Promise<void>) => {
    setBusy(what);
    setError(null);
    try { await action(); } catch (e) { setError(errorText(e)); } finally { setBusy(null); }
  };
  const setCard = (patch: Partial<VoiceCard>) => run("card", async () => { onChanged((await api.voiceSetCard(voice.name, patch)).voice); });
  const saveText = () => run("text", async () => {
    const cleanTags = tags.split(",").map((x) => x.trim()).filter(Boolean);
    const card: Partial<VoiceCard> = {};
    if (cleanTags.join(",") !== voice.card.tags.join(",")) card.tags = cleanTags;
    if (source.trim() !== voice.card.source) card.source = source.trim();
    if (lang.trim().toLowerCase() !== voice.card.lang) card.lang = lang.trim().toLowerCase();
    if (Object.keys(card).length) onChanged((await api.voiceSetCard(voice.name, card)).voice);
    if (transcript.trim() !== (voice.transcript ?? "")) onChanged((await api.voiceSetTranscript(voice.name, transcript)).voice);
    if (name.trim() && name.trim() !== voice.name) {
      await api.voicesRename(voice.name, name.trim());
      onRenamed(name.trim());
    }
  });
  const transcribe = () => run("transcribe", async () => {
    const { job_id } = await api.voiceTranscribe(voice.name);
    const r = (await followJob(job_id, "voice_transcript")) as { transcript?: string };
    if (r?.transcript) setTranscript(r.transcript);
  });
  const clean = () => run("clean", async () => {
    const { job_id } = await api.voiceClean(voice.name);
    await followJob(job_id, "voice_clean");
  });
  const denoise = () => run("denoise", async () => {
    const { job_id } = await api.voiceDenoise(voice.name, denoiseDb);
    await followJob(job_id, "voice_clean");
  });
  const findPieces = () => run("pieces", async () => { setPieces((await api.voiceRefCandidates(voice.name)).candidates); });
  const playPiece = (i: number, c: RefCandidate) => {
    piecePlayer.current?.pause();
    if (playingPiece === i) { setPlayingPiece(null); return; }
    const a = new Audio(api.voiceSampleUrl(voice.name, false, voice.size));
    piecePlayer.current = a;
    a.currentTime = c.start;
    a.ontimeupdate = () => { if (a.currentTime >= c.end) { a.pause(); setPlayingPiece(null); } };
    a.onended = () => setPlayingPiece(null);
    setPlayingPiece(i);
    a.play().catch((e: unknown) => { setPlayingPiece(null); setError(errorText(e)); });
  };
  const applyPiece = (c: RefCandidate) => run("apply", async () => {
    piecePlayer.current?.pause();
    const r = await api.voiceRefApply(voice.name, c.start, c.end);
    onChanged(r.voice);
    setTranscript(r.voice.transcript ?? "");
    setPieces(null);
  });
  const trim = (start: number, end: number) => run("apply", async () => {
    const r = await api.voiceRefApply(voice.name, start, end);
    onChanged(r.voice);
    setTranscript(r.voice.transcript ?? "");
    setTrimming(false);
  });
  const restore = () => run("restore", async () => { const r = await api.voiceRestore(voice.name); onChanged(r.voice); setTranscript(r.voice.transcript ?? ""); setPieces(null); });
  const photo = (f: File) => run("photo", async () => { onChanged((await api.voicePhoto(voice.name, f)).voice); setPhotoRev((n) => n + 1); });
  const dirty = name.trim() !== voice.name || tags !== voice.card.tags.join(", ") || source !== voice.card.source || lang !== voice.card.lang || transcript.trim() !== (voice.transcript ?? "");

  return (
    <div className="fixed inset-0 z-[65] flex justify-end glass-scrim anim-fade" onClick={onClose}>
      <aside role="dialog" aria-modal="true" aria-labelledby="voice-edit-title" className="w-[min(100vw,440px)] h-full overflow-y-auto glass-panel border-l border-[var(--color-border)] p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-3">
          <button type="button" onClick={() => photoInput.current?.click()} className="relative group rounded-full" {...named(t("voices.photo"))}>
            <VoiceAvatar voice={voice} size={56} rev={photoRev} />
            <span className="absolute inset-0 rounded-full grid place-items-center bg-black/50 opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100 transition-opacity">
              {busy === "photo" ? <Loader2 size={16} className="animate-spin" /> : <Camera size={16} />}
            </span>
          </button>
          <input ref={photoInput} type="file" accept="image/jpeg,image/png,image/webp" className="hidden" onChange={(e) => { const f = e.target.files?.[0]; if (f) photo(f); e.target.value = ""; }} />
          <div className="min-w-0 flex-1">
            <div id="voice-edit-title" className="font-semibold truncate">{voiceLabel(voice)}</div>
            <div className="text-[11px] text-[var(--color-muted)]">{voice.card.cleaned ? t("voices.cleanedBadge") : t("voices.notCleaned")} · {voice.audio.toUpperCase()}</div>
          </div>
          <button onClick={() => setCard({ favorite: !voice.card.favorite })} aria-pressed={voice.card.favorite} {...named(t("voices.favorite"))}
            className={`p-1.5 rounded-md ${voice.card.favorite ? "text-[var(--color-warn)]" : "text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}><Star size={17} fill={voice.card.favorite ? "currentColor" : "none"} /></button>
          <button onClick={onClose} {...named(t("a11y.close"))} className="p-1.5 text-[var(--color-muted)] hover:text-[var(--color-text)]"><X size={16} /></button>
        </div>

        <audio controls preload="none" src={api.voiceSampleUrl(voice.name, false, voice.size)} className="w-full h-9" aria-label={t("voice.preview")} />

        <div>
          <label className={label} htmlFor="ve-name">{t("voices.name")}</label>
          <input id="ve-name" value={name} onChange={(e) => setName(e.target.value)} maxLength={48} className={field} />
        </div>
        <div className="grid grid-cols-3 gap-2">
          <div>
            <label className={label} htmlFor="ve-gender">{t("voices.gender")} <Hint text={t("hint.voiceGender")} /></label>
            <select id="ve-gender" value={voice.card.gender} onChange={(e) => setCard({ gender: e.target.value as VoiceGender })} className={field}>
              {GENDERS.map((g) => <option key={g} value={g}>{t(`voices.gender_${g || "unknown"}`)}</option>)}
            </select>
          </div>
          <div>
            <label className={label} htmlFor="ve-age">{t("voices.age")}</label>
            <select id="ve-age" value={voice.card.age} onChange={(e) => setCard({ age: e.target.value as VoiceAge })} className={field}>
              {AGES.map((a) => <option key={a} value={a}>{t(`voices.age_${a || "unknown"}`)}</option>)}
            </select>
          </div>
          <div>
            <label className={label} htmlFor="ve-lang">{t("voices.lang")} <Hint text={t("hint.voiceLang")} /></label>
            <input id="ve-lang" value={lang} onChange={(e) => setLang(e.target.value)} maxLength={3} className={`${field} mono`} />
          </div>
        </div>
        <div>
          <label className={label} htmlFor="ve-tags">{t("voices.tags")} <Hint text={t("hint.voiceTags")} /></label>
          <input id="ve-tags" value={tags} onChange={(e) => setTags(e.target.value)} placeholder={t("voices.tagsHint")} className={field} />
        </div>
        <div className="grid grid-cols-2 gap-2">
          <div>
            <label className={label} htmlFor="ve-source">{t("voices.source")} <Hint text={t("hint.voiceSource")} /></label>
            <input id="ve-source" value={source} onChange={(e) => setSource(e.target.value)} placeholder={t("voices.sourceHint")} className={field} />
          </div>
          <div>
            <label className={label} htmlFor="ve-consent">{t("voices.consent")} <Hint text={t("hint.voiceConsent")} /></label>
            <select id="ve-consent" value={voice.card.consent} onChange={(e) => setCard({ consent: e.target.value as VoiceConsent })} className={field}>
              {CONSENTS.map((c) => <option key={c} value={c}>{t(`voices.consent_${c || "unknown"}`)}</option>)}
            </select>
          </div>
        </div>
        <div>
          <label className={label} htmlFor="ve-license">{t("voices.license")} <Hint text={t("hint.voiceLicense")} /></label>
          <input id="ve-license" list="ve-licenses" value={license} onChange={(e) => setLicense(e.target.value)} onBlur={() => { if (license.trim() !== voice.card.license) void setCard({ license: license.trim() }); }} placeholder={t("voices.licenseHint")} className={field} />
          <datalist id="ve-licenses">{["CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0", "CC-BY-NC-4.0", "CC-BY-NC-SA-4.0"].map((l) => <option key={l} value={l} />)}</datalist>
        </div>
        {!all.some((v) => v.card.variant_of === voice.name) && (
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className={label} htmlFor="ve-variant-of">{t("voices.variantOf")} <Hint text={t("hint.voiceVariantOf")} /></label>
              <select id="ve-variant-of" value={voice.card.variant_of} onChange={(e) => setCard(e.target.value ? { variant_of: e.target.value, variant: variant.trim() || t("voices.variantDefault") } : { variant_of: "", variant: "" })} className={field}>
                <option value="">{t("voices.variantNone")}</option>
                {all.filter((v) => v.name !== voice.name && !v.card.variant_of).map((v) => <option key={v.name} value={v.name}>{voiceLabel(v)}</option>)}
              </select>
            </div>
            {voice.card.variant_of && (
              <div>
                <label className={label} htmlFor="ve-variant">{t("voices.variantName")}</label>
                <input id="ve-variant" value={variant} onChange={(e) => setVariant(e.target.value)} onBlur={() => { if (variant.trim() && variant.trim() !== voice.card.variant) void setCard({ variant: variant.trim() }); }} placeholder={t("voices.variantHint")} className={field} />
              </div>
            )}
          </div>
        )}
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className={label + " mb-0"} htmlFor="ve-text">{t("voices.transcript")} <Hint text={t("voices.transcriptHint")} /></label>
            <span className="text-[10px] text-[var(--color-muted)]">{voice.transcript ? (voice.card.transcript_auto ? t("voices.transcriptAuto") : t("voices.transcriptManual")) : t("voices.transcriptNoneShort")}</span>
          </div>
          <textarea id="ve-text" value={transcript} onChange={(e) => setTranscript(e.target.value)} rows={4} placeholder={t("voices.transcriptHint")} className={`${field} resize-y leading-relaxed`} />
          <button onClick={transcribe} disabled={!!busy} className="mt-1.5 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] text-[var(--color-accent-2)] hover:border-[var(--color-accent)] disabled:opacity-40">
            {busy === "transcribe" ? <Loader2 size={12} className="animate-spin" /> : <ScanText size={12} />}{t("voices.transcribe")}
          </button>
        </div>
        <RefNeeds hasText={!!voice.transcript?.trim()} />
        <div>
          <div className="flex items-center justify-between mb-1">
            <span className={label + " mb-0"}>{t("voices.pieces")} <Hint text={t("voices.piecesHint")} /></span>
            <button onClick={findPieces} disabled={!!busy || !voice.transcript} title={voice.transcript ? undefined : t("voices.piecesNeedText")}
              className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] text-[var(--color-accent-2)] hover:border-[var(--color-accent)] disabled:opacity-40">
              {busy === "pieces" ? <Loader2 size={12} className="animate-spin" /> : <Crop size={12} />}{t("voices.findPieces")}
            </button>
            <button onClick={() => setTrimming((v) => !v)} aria-pressed={trimming}
              className="ml-1.5 inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] text-[var(--color-accent-2)] hover:border-[var(--color-accent)]">
              <Scissors size={12} />{t("voices.trimOpen")}
            </button>
          </div>
          <label className="mb-2 flex items-start gap-2 text-[12px] cursor-pointer">
            <input type="checkbox" checked={voice.card.rotate} onChange={(e) => void setCard({ rotate: e.target.checked })} className="mt-0.5 w-3.5 h-3.5 accent-[var(--color-accent)]" />
            <span>{t("voices.rotate")} <Hint text={t("voices.rotateHint", { s: refLimit })} /></span>
          </label>
          {trimming && <div className="mb-2"><RefTrim url={api.voiceSampleUrl(voice.name, false, voice.size)} limit={refLimit} busy={busy === "apply"} onApply={trim} /></div>}
          {pieces && pieces.length === 0 && <p className="text-[12px] text-[var(--color-muted)]">{t("voices.noPieces")}</p>}
          {pieces && pieces.length > 0 && (
            <ul className="space-y-1.5">
              {pieces.map((c, i) => (
                <li key={`${c.start}-${c.end}`} className={`rounded-lg border p-2 ${i === 0 ? "border-[var(--color-accent)]" : "border-[var(--color-border)]"} bg-[var(--color-surface-2)]`}>
                  <div className="flex items-center gap-2">
                    <button onClick={() => playPiece(i, c)} {...named(t("voice.preview"))} className="p-1 rounded-md text-[var(--color-text)] hover:text-[var(--color-accent)]">{playingPiece === i ? <Pause size={13} /> : <Play size={13} />}</button>
                    <span className="mono text-[11px] tabnum">{c.duration.toFixed(1)} {t("settings.sec")}</span>
                    <span className="mono text-[11px] text-[var(--color-muted)] tabnum">{t("voices.snr", { db: Math.round(c.snr_db) })}</span>
                    <span className="mono text-[11px] text-[var(--color-muted)] tabnum">{t("voices.pause", { s: c.max_pause.toFixed(1) })}</span>
                    {i === 0 && <span className="text-[10px] uppercase tracking-wide text-[var(--color-accent)]">{t("voices.best")}</span>}
                    <button onClick={() => applyPiece(c)} disabled={!!busy} className="ml-auto px-2 py-0.5 rounded-md bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[11px] font-semibold disabled:opacity-40">{t("voices.usePiece")}</button>
                  </div>
                  <div className="mt-1 text-[11px] leading-snug text-[var(--color-muted)] line-clamp-2">{c.text}</div>
                  {c.issues.length > 0 && <div className="mt-1 flex flex-wrap gap-1">{c.issues.map((k) => <span key={k} className="text-[10px] px-1.5 py-0.5 rounded bg-[color-mix(in_oklab,var(--color-warn)_14%,transparent)] text-[var(--color-warn)]">{t(`voices.issue_${k}`)}</span>)}</div>}
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          {voice.original ? (
            <button onClick={restore} disabled={!!busy} className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)] disabled:opacity-40">
              {busy === "restore" ? <Loader2 size={12} className="animate-spin" /> : <RotateCcw size={12} />}{t("voices.restore")}
            </button>
          ) : null}
          <Tip text={t("voices.cleanHint")}><button onClick={clean} disabled={!!busy} className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)] disabled:opacity-40">
            {busy === "clean" ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}{voice.card.cleaned ? t("voices.cleanAgain") : t("voices.clean")}
          </button></Tip>
          <Tip text={t("voices.denoiseHint")}><span className="inline-flex items-stretch rounded-lg border border-[var(--color-border)] focus-within:border-[var(--color-accent)]">
            <button onClick={denoise} disabled={!!busy} className="inline-flex items-center gap-1.5 px-2.5 py-1.5 text-[12px] hover:text-[var(--color-accent)] disabled:opacity-40">
              {busy === "denoise" ? <Loader2 size={12} className="animate-spin" /> : <AudioLines size={12} />}{t("voices.denoise")}
            </button>
            <select value={denoiseDb} onChange={(e) => setDenoiseDb(Number(e.target.value))} disabled={!!busy} aria-label={t("denoise.strength")} className="bg-transparent border-l border-[var(--color-border)] px-1.5 text-[11px] text-[var(--color-muted)] focus:outline-none">
              {DENOISE_LEVELS.map((db) => <option key={db} value={db}>{db ? t("denoise.db", { db }) : t("denoise.full")}</option>)}
            </select>
          </span></Tip>
          <button onClick={() => setConfirmDelete(true)} disabled={!!busy} className="ml-auto inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] text-[var(--color-muted)] hover:text-[var(--color-warn)] hover:border-[var(--color-warn)] disabled:opacity-40">
            <Trash2 size={12} />{t("voices.delete")}
          </button>
        </div>
        {error && <p role="alert" className="text-[12px] text-[var(--color-warn)] break-words">{error}</p>}
        <div className="sticky bottom-0 pt-2 flex justify-end gap-2">
          <button onClick={onClose} className="px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[13px] hover:border-[var(--color-accent)]">{t("voices.close")}</button>
          <button onClick={saveText} disabled={!dirty || !!busy} className="px-4 py-1.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40">
            {busy === "text" ? <Loader2 size={13} className="animate-spin" /> : t("voices.save")}
          </button>
        </div>
      </aside>
      {confirmDelete && (
        <ConfirmDialog title={t("voices.deleteTitle")} message={t("voices.deleteMessage", { name: voiceLabel(voice) })} confirmLabel={t("voices.delete")} danger
          onCancel={() => setConfirmDelete(false)}
          onConfirm={async () => { await api.voicesDelete(voice.name); setConfirmDelete(false); onDeleted(); }} />
      )}
    </div>
  );
}
