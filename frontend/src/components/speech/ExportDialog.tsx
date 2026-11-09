import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, Loader2, Package, Sparkles, X } from "lucide-react";
import { EXPORT_FORMATS, speechApi, type AcxReport, type ExportFormat, type ExportProblem, type SpeechProject } from "../../lib/speech";
import { followJob } from "../../lib/followJob";
import { errorText } from "../../lib/voices";
import { named } from "../../lib/a11y";
import FileActions from "./FileActions";
import { useEscapeLayer } from "../../lib/escape";
import Hint from "../Hint";
import { clock } from "../../lib/listen";

/** Export: what keeps it back (blocks not voiced, out of date, with a lost or changed take file, or a take too long for
 * its cue, the one problem that can be let through), the format and the finished file to save. */
// -1: a file per chapter
const PART_MINUTES = [0, -1, 60, 120, 300, 600] as const;

export default function ExportDialog({ pid, project, onChange, flush, onClose, onVoiceMissing, onDone }: {
  pid: string;
  project: SpeechProject;
  onChange: (f: (p: SpeechProject) => SpeechProject) => void;
  flush: () => Promise<unknown>;
  onClose: () => void;
  onVoiceMissing: (ids: string[]) => void;
  onDone: (file: string) => void;
}) {
  const { t } = useTranslation();
  const [problems, setProblems] = useState<ExportProblem[] | null>(null);
  const [format, setFormat] = useState<ExportFormat>(project.settings.loudness === "acx" ? "mp3" : "wav");
  const [busy, setBusy] = useState(false);
  const [file, setFile] = useState<string | null>(null);
  const [subs, setSubs] = useState<string[]>([]);
  const [chapterList, setChapterList] = useState<string | null>(null);
  const [acx, setAcx] = useState<AcxReport | null>(null);
  const [files, setFiles] = useState<{ file: string; bytes: number; mtime: number }[] | null>(null);
  const [perBlock, setPerBlock] = useState(false);
  const [partMinutes, setPartMinutes] = useState(0);
  const [allowDuration, setAllowDuration] = useState(false);
  const [allowUnvoiced, setAllowUnvoiced] = useState(false);
  const [withStems, setWithStems] = useState(false);
  const [stems, setStems] = useState<string | null>(null);
  const [table, setTable] = useState<string | null>(null);
  const [manifest, setManifest] = useState<string | null>(null);
  type Part = { file: string; subtitles: string; timings: string; chapters: string | null; duration: number };
  const [parts, setParts] = useState<Part[] | null>(null);
  const [timings, setTimings] = useState<string | null>(null);
  const [player, setPlayer] = useState<string | null>(null);
  const [recipe, setRecipe] = useState<string | null>(null);
  const [clips, setClips] = useState<string[] | null>(null);
  const [clipsSaved, setClipsSaved] = useState<string | null>(null);
  /** An error shows beside what failed: the export (and the check before it), the package or the list of files. */
  const [error, setFailure] = useState<{ at: "export" | "pack" | "files"; text: string } | null>(null);
  const setError = (text: string | null, at: "export" | "pack" | "files" = "export") => setFailure(text ? { at, text } : null);
  const shown = (at: "export" | "pack" | "files") => error?.at === at && <p role="alert" className="text-[12px] text-[var(--color-warn)] break-words">{error.text}</p>;
  const [withAudio, setWithAudio] = useState(true);
  const [rate, setRate] = useState(0);
  const [bits, setBits] = useState(0);
  const [kbps, setKbps] = useState(0);
  const pickClass = "bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-1.5 py-1 text-[12px]";
  const [pack, setPack] = useState<{ file: string; bytes: number } | null>(null);
  const [packing, setPacking] = useState(false);
  const runPack = async () => {
    setPacking(true);
    setError(null);
    try { setPack(await speechApi.pack(pid, withAudio)); } catch (e) { setError(errorText(e), "pack"); } finally { setPacking(false); }
  };
  useEscapeLayer(() => { if (!busy) onClose(); });
  useEffect(() => { speechApi.exportCheck(pid).then((r) => setProblems(r.problems), (e: unknown) => setError(errorText(e))); }, [pid]);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      await flush();
      const { job_id } = await speechApi.export(pid, format, undefined, perBlock, perBlock ? 0 : Math.max(partMinutes, 0), allowDuration, { sample_rate: rate || undefined, bits: bits || undefined, kbps: kbps || undefined }, !perBlock && partMinutes === -1, !perBlock && withStems, allowUnvoiced);
      const r = (await followJob(job_id, "export")) as { file: string; files?: string[]; subtitles?: string; subtitle_files?: string[]; timings?: string; player?: string; recipe?: string; chapters?: string | null; parts?: Part[] | null; acx?: AcxReport | null; stems?: string | null; table?: string; manifest?: string };
      if (r.files) { setClips(r.files); setRecipe(r.recipe ?? null); return; }
      setAcx(r.acx ?? null);
      setFile(r.file);
      setSubs(r.subtitle_files ?? (r.subtitles ? [r.subtitles] : []));
      setChapterList(r.chapters ?? null);
      setTimings(r.timings ?? null);
      setPlayer(r.player ?? null);
      setRecipe(r.recipe ?? null);
      setParts(r.parts ?? null);
      setStems(r.stems ?? null);
      setTable(r.table ?? null);
      setManifest(r.manifest ?? null);
      onDone(r.file);
    } catch (e) { setError(errorText(e)); } finally { setBusy(false); }
  };
  const blocking = (problems ?? []).filter((p) => !(allowDuration && p.state === "duration") && !(allowUnvoiced && p.state !== "duration"));
  const unvoiced = (problems ?? []).filter((p) => p.state !== "duration").map((p) => p.block);
  const tooLong = (problems ?? []).some((p) => p.state === "duration");
  const counts = (problems ?? []).reduce<Record<string, number>>((a, p) => { a[p.state] = (a[p.state] ?? 0) + 1; return a; }, {});
  return (
    <div className="fixed inset-0 z-[60] grid place-items-center glass-scrim anim-fade" onClick={() => { if (!busy) onClose(); }}>
      <div role="dialog" aria-modal="true" aria-labelledby="ex-title" className="w-[min(94vw,460px)] rounded-xl glass-panel anim-pop p-5 space-y-4" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <h2 id="ex-title" className="font-semibold">{t("speech.exportTitle")}</h2>
          <button onClick={onClose} disabled={busy} {...named(t("a11y.close"))} className="text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30"><X size={16} /></button>
        </div>
        {problems === null && (error ? shown("export") : <Loader2 size={16} className="animate-spin text-[var(--color-muted)]" />)}
        {problems && problems.length > 0 && (
          <div role="alert" className="rounded-lg border border-[var(--color-warn)]/40 bg-[color-mix(in_oklab,var(--color-warn)_8%,transparent)] p-3 space-y-2">
            <p className="text-[13px]">{blocking.length ? t("speech.exportBlocked", { n: blocking.length }) : t("speech.exportDurationAllowed")}</p>
            <ul className="text-[12px] text-[var(--color-muted)] space-y-0.5">
              {Object.entries(counts).map(([state, n]) => <li key={state}>{t(`speech.state_${state as ExportProblem["state"]}`)}: {n}</li>)}
            </ul>
            {unvoiced.length > 0 && <button onClick={() => onVoiceMissing(unvoiced)} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[12px] font-semibold"><Sparkles size={13} />{t("speech.voiceTheMissing")}</button>}
            {counts.failed > 0 && <p className="text-[12px] text-[var(--color-muted)]">{t("speech.exportApproveHint")}</p>}
            {tooLong && (
              <label className="flex items-center gap-2 text-[12px] cursor-pointer">
                <input type="checkbox" checked={allowDuration} onChange={(e) => setAllowDuration(e.target.checked)} className="w-4 h-4 accent-[var(--color-accent)]" />{t("speech.allowDuration")} <Hint text={t("hint.allowDuration")} />
              </label>
            )}
            {unvoiced.length > 0 && (
              <label className="flex items-center gap-2 text-[12px] cursor-pointer">
                <input type="checkbox" checked={allowUnvoiced} onChange={(e) => setAllowUnvoiced(e.target.checked)} className="w-4 h-4 accent-[var(--color-accent)]" />{t("speech.allowUnvoiced", { n: unvoiced.length })} <Hint text={t("hint.allowUnvoiced")} />
              </label>
            )}
          </div>
        )}
        {problems && blocking.length === 0 && (
          <>
            <div className="flex flex-wrap items-center gap-1.5" role="radiogroup" aria-label={t("speech.format")}>
              {EXPORT_FORMATS.map((f) => (
                <button key={f} role="radio" aria-checked={format === f} onClick={() => { setFormat(f); setRate(0); setBits(0); setKbps(0); setFile(null); setSubs([]); setChapterList(null); setAcx(null); setClips(null); }}
                  className={`px-3 py-1.5 rounded-lg border text-[12px] font-semibold uppercase ${format === f ? "border-[var(--color-accent)] text-[var(--color-accent)]" : "border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>{f}</button>
              ))}
              <Hint text={t("hint.exportFormat")} />
            </div>
            <fieldset className="flex flex-wrap items-center gap-2 text-[12px]">
              <legend className="sr-only">{t("speech.quality")}</legend>
              <label className="inline-flex items-center gap-1.5">{t("speech.rate")}
                <select value={rate} onChange={(e) => setRate(Number(e.target.value))} className={pickClass}>
                  <option value={0}>{t("speech.qualityAuto")}</option>
                  {(format === "mp3" ? [32000, 44100, 48000] : [16000, 22050, 24000, 32000, 44100, 48000]).map((r) => <option key={r} value={r}>{t("speech.rateKhz", { v: (r / 1000).toLocaleString() })}</option>)}
                </select>
              </label>
              {(format === "wav" || format === "flac") && (
                <label className="inline-flex items-center gap-1.5">{t("speech.bits")}
                  <select value={bits} onChange={(e) => setBits(Number(e.target.value))} className={pickClass}>
                    <option value={0}>{t("speech.qualityAuto")}</option>
                    {(format === "wav" ? [16, 24, 32] : [16, 24]).map((b) => <option key={b} value={b}>{b === 32 ? t("speech.bitsFloat") : b}</option>)}
                  </select>
                </label>
              )}
              {format !== "wav" && format !== "flac" && (
                <label className="inline-flex items-center gap-1.5">{t("speech.kbps")}
                  <select value={kbps} onChange={(e) => setKbps(Number(e.target.value))} className={pickClass}>
                    <option value={0}>{t("speech.qualityAuto")}</option>
                    {[96, 128, 160, 192, 256, 320].map((k) => <option key={k} value={k}>{k}</option>)}
                  </select>
                </label>
              )}
              <Hint text={t("hint.exportQuality")} />
            </fieldset>
            <label className="flex items-center gap-2 text-[12px] cursor-pointer">
              <input type="checkbox" checked={perBlock} onChange={(e) => { setPerBlock(e.target.checked); setFile(null); setClips(null); }} className="w-4 h-4 accent-[var(--color-accent)]" />{t("speech.perBlock")} <Hint text={t("hint.perBlock")} />
            </label>
            {!perBlock && (
              <label className="flex flex-wrap items-center gap-2 text-[12px]">{t("speech.parts")}
                <select value={partMinutes} onChange={(e) => { setPartMinutes(Number(e.target.value)); setFile(null); setParts(null); }} className="bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]">
                  {PART_MINUTES.map((m) => <option key={m} value={m}>{m === -1 ? t("speech.partsChapters") : m ? t("speech.partsEvery", { h: m / 60 }) : t("speech.partsNone")}</option>)}
                </select>
                <Hint text={t("speech.partsHint")} />
              </label>
            )}
            <label className="block text-[12px] space-y-1">
              <span>{t(perBlock ? "speech.clipName" : "speech.exportName")} <Hint text={t(perBlock ? "speech.clipNameHint" : "speech.exportNameHint")} /></span>
              <input value={perBlock ? project.settings.clip_name ?? "" : project.settings.export_name ?? ""} spellCheck={false}
                onChange={(e) => { const v = e.target.value; setFile(null); setClips(null); onChange((p) => ({ ...p, settings: { ...p.settings, [perBlock ? "clip_name" : "export_name"]: v } })); }}
                placeholder={t(perBlock ? "speech.clipNamePlaceholder" : "speech.exportNamePlaceholder")}
                className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px] mono focus:border-[var(--color-accent)] outline-none" />
            </label>
            {!perBlock && (
              <label className="flex items-start gap-2 text-[12px] cursor-pointer">
                <input type="checkbox" checked={withStems} onChange={(e) => { setWithStems(e.target.checked); setFile(null); setStems(null); }} className="mt-0.5 w-4 h-4 accent-[var(--color-accent)]" />
                <span>{t("speech.stems")} <Hint text={t("speech.stemsHint")} /></span>
              </label>
            )}
            {clips && (
              <div className="space-y-1.5">
                <p className="text-[12px]">{t("speech.clipsDone", { n: clips.length })}</p>
                <button onClick={() => speechApi.exportsToFolder(pid, recipe ? [...clips, recipe] : clips).then((r) => setClipsSaved(r.folder), (e: unknown) => setError(errorText(e)))} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[12px] font-semibold">{t("speech.clipsToFolder")}</button>
                {clipsSaved && <p className="text-[11px] text-[var(--color-muted)] break-all">{t("files.savedTo", { path: clipsSaved })}</p>}
              </div>
            )}
            <p className="text-[12px] text-[var(--color-muted)]">{t("speech.exportLoudness", { preset: t(`speech.loudness_${project.settings.loudness}`) })}</p>
            {file ? (
              <div className="space-y-1.5">
                {parts ? parts.map((part, k) => (
                  <div key={part.file} className="space-y-1">
                    <p className="text-[11px] text-[var(--color-muted)]">{t("speech.partN", { n: k + 1, time: clock(part.duration) })}</p>
                    <FileActions pid={pid} file={part.file} primary={k === 0} />
                  </div>
                )) : <FileActions pid={pid} file={file} primary />}
                {!parts && chapterList && <FileActions pid={pid} file={chapterList} label={t("speech.saveChapters")} />}
                {!parts && subs.map((f) => <FileActions key={f} pid={pid} file={f} label={t("speech.saveSubs", { ext: f.slice(f.lastIndexOf(".")) })} />)}
                {!parts && table && <FileActions pid={pid} file={table} label={t("speech.saveTable")} />}
                {!parts && player && <FileActions pid={pid} file={player} label={t("speech.savePlayer")} />}
                {!parts && timings && <FileActions pid={pid} file={timings} label={t("speech.saveTimings")} />}
                {!parts && recipe && <FileActions pid={pid} file={recipe} label={t("speech.saveRecipe")} />}
                {stems && <FileActions pid={pid} file={stems} label={t("speech.saveStems")} />}
                {manifest && <FileActions pid={pid} file={manifest} label={t("speech.saveManifest")} />}
              </div>
            ) : (
              <button onClick={run} disabled={busy} className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40">
                {busy ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />}{t("speech.exportRun")}
              </button>
            )}
            {shown("export")}
          </>
        )}
        {acx && (
          <div role="status" className={`rounded-lg border p-3 space-y-1 ${acx.passed ? "border-[var(--color-accent)]/40" : "border-[var(--color-warn)]/40"}`}>
            <p className="text-[12px] font-semibold">{acx.passed ? t("speech.acxPassed") : t("speech.acxFailed")}</p>
            <ul className="text-[11px] space-y-0.5">
              {acx.checks.map((c) => (
                <li key={c.name} className={c.passed ? "text-[var(--color-muted)]" : "text-[var(--color-warn)]"}>
                  {c.passed ? "✓" : "✗"} {t(`speech.acx_${c.name}`, { v: ["rms", "peak", "noise_floor"].includes(c.name) ? c.value.toFixed(1) : c.name === "length" ? (c.value / 60).toFixed(1) : c.value.toFixed(2) })}
                </li>
              ))}
            </ul>
          </div>
        )}
        <div className="pt-3 border-t border-[var(--color-border)] space-y-2">
          <h3 className="text-[12px] font-semibold">{t("speech.packTitle")} <Hint text={t("speech.packHint")} /></h3>
          <label className="flex items-center gap-2 text-[12px] cursor-pointer">
            <input type="checkbox" checked={withAudio} onChange={(e) => { setWithAudio(e.target.checked); setPack(null); }} className="w-4 h-4 accent-[var(--color-accent)]" />{t("speech.packAudio")} <Hint text={t("hint.packAudio")} />
          </label>
          {pack ? (
            <FileActions pid={pid} file={pack.file} />
          ) : (
            <button onClick={() => void runPack()} disabled={packing} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] font-medium hover:border-[var(--color-accent)] disabled:opacity-40">
              {packing ? <Loader2 size={13} className="animate-spin" /> : <Package size={13} />}{t("speech.packRun")}
            </button>
          )}
          {shown("pack")}
        </div>
        <details onToggle={(e) => { if (e.currentTarget.open) speechApi.exports(pid).then((r) => setFiles(r.files), (er: unknown) => setError(errorText(er), "files")); }} className="pt-3 border-t border-[var(--color-border)]">
          <summary className="cursor-pointer text-[12px] font-semibold">{t("files.title")}</summary>
          <ul className="mt-2 space-y-1.5 max-h-48 overflow-y-auto">
            {files?.length === 0 && <li className="text-[11px] text-[var(--color-muted)]">{t("files.none")}</li>}
            {files?.map((f) => <li key={f.file}><FileActions pid={pid} file={f.file} label={f.file} /></li>)}
          </ul>
          {shown("files")}
        </details>
      </div>
    </div>
  );
}
