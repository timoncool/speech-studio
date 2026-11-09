import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Ear, FileAudio, FolderOpen, Link2, Loader2, Mic, Sparkles, Square, Trash2 } from "lucide-react";
import { PERSON_ONLY, byPerson } from "../../lib/person";
import { api } from "../../lib/api";
import { listenApi, clock, MEDIA, prepare, TRANSCRIPT_FORMATS, type ListenCard, type ListenEngine, type ListenProject, type Start, type TranscriptFormat } from "../../lib/listen";
import QuickListen from "./QuickListen";
import WatchFolder from "./WatchFolder";
import BatchList, { type BatchItem } from "../BatchList";
import { ENGINES } from "./engines";
import { LANG_CHOICES } from "../speech/langs";
import { speechApi } from "../../lib/speech";
import { errorText } from "../../lib/voices";
import { PROJECTS_CHANGED, useChanged } from "../../lib/mcpBridge";
import { named } from "../../lib/a11y";
import UrlImport from "../UrlImport";
import Hint, { Tip } from "../Hint";
import UndoToast from "../UndoToast";
import { useAfterDownload } from "../../lib/useAfterDownload";

const START = "listen.start";

function savedStart(): Start {
  try {
    const s = JSON.parse(localStorage.getItem(START) ?? "{}") as Partial<Start>;
    return { engine: ENGINES.includes(s.engine as ListenEngine) ? (s.engine as ListenEngine) : "", lang: (LANG_CHOICES as readonly string[]).includes(s.lang ?? "") ? s.lang! : "auto" };
  } catch { return { engine: "", lang: "auto" }; }
}

/** A new recording takes the engine and language picked on the start screen; one recognised before keeps its own. */
/** Listening's start: a recording from a file (several at once go to recognition one after another), a link or the
 * microphone, and the recordings opened before. */
type Source = "file" | "link" | "mic" | "folder";
const SOURCES: { id: Source; icon: typeof FileAudio }[] = [
  { id: "file", icon: FileAudio },
  { id: "link", icon: Link2 },
  { id: "mic", icon: Mic },
  { id: "folder", icon: FolderOpen },
];
const SOURCE = "speech.listen.source";

function savedSource(): Source {
  try {
    const v = localStorage.getItem(SOURCE);
    return SOURCES.some((s) => s.id === v) ? (v as Source) : "file";
  } catch {
    return "file";
  }
}

export default function ListeningHome({ onOpen }: { onOpen: (pid: string) => void }) {
  const { t, i18n } = useTranslation();
  const [projects, setProjects] = useState<ListenCard[] | null>(null);
  const [quick, setQuick] = useState(false);
  const [find, setFind] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [drag, setDrag] = useState(false);
  const [removed, setRemoved] = useState<ListenCard | null>(null);
  const remove = async (card: ListenCard) => {
    try { await speechApi.remove(card.pid); setRemoved(card); load(); } catch (e) { setError(errorText(e)); }
  };
  const [batch, setBatch] = useState<BatchItem[] | null>(null);
  const [batchRunning, setBatchRunning] = useState(false);
  const [formats, setFormats] = useState<TranscriptFormat[]>(["srt", "txt"]);
  const file = useRef<HTMLInputElement>(null);
  const [start, setStart] = useState<Start>(savedStart);
  const [source, setSource] = useState<Source>(savedSource);
  const pickSource = (next: Source) => {
    setSource(next);
    try { localStorage.setItem(SOURCE, next); } catch { /* the pick holds for this screen only */ }
  };
  const pickStart = (next: Start) => {
    setStart(next);
    try { localStorage.setItem(START, JSON.stringify(next)); } catch { /* the pick holds for this screen only */ }
  };
  const load = () => {
    speechApi.list().then((r) => setProjects(r.projects.filter((p): p is ListenCard => p.kind === "listening") as ListenCard[]), (e: unknown) => setError(errorText(e)));
  };
  useEffect(load, []);
  useChanged(PROJECTS_CHANGED, load);

  const upload = async (files: File[]) => {
    if (!files.length) return;
    setBusy(true);
    setError(null);
    try {
      if (files.length === 1) {
        const r = await listenApi.upload(files[0]);
        await prepare(r.pid, r.project, start);
        onOpen(r.pid);
        return;
      }
      setBusy(false);
      setBatch(files.map((f) => ({ name: f.name, status: "queued", pct: 0, pid: null, files: [] })));
      setBatchRunning(true);
      for (const [k, f] of files.entries()) {
        const upd = (patch: Partial<BatchItem>) => setBatch((xs) => xs && xs.map((x, j) => (j === k ? { ...x, ...patch } : x)));
        try {
          upd({ status: "working", pct: 0, detail: t("listen.uploading") });
          const r = await listenApi.upload(f);
          upd({ pid: r.pid });
          await prepare(r.pid, r.project, start);
          const { job_id } = await listenApi.transcribe(r.pid);
          await api.watchJob(job_id, (e) => { if (e.type === "progress") upd({ pct: e.pct ?? 0, detail: e.msg || undefined }); });
          upd({ status: "done", pct: 100, detail: undefined });
        } catch (e) { upd({ status: "error", error: errorText(e) }); }
        load();
      }
      setBatchRunning(false);
    } catch (e) { setError(errorText(e)); } finally { setBusy(false); }
  };
  /** Writes every recognised recording of the batch in the chosen formats and copies them into one folder. */
  const saveBatch = async () => {
    let folder: string | undefined;
    let copied = 0;
    for (const it of batch ?? []) {
      if (it.status !== "done" || !it.pid) continue;
      const files: string[] = [];
      for (const f of formats) files.push((await listenApi.export(it.pid, f, true, true)).file);
      const out = await speechApi.exportsToFolder(it.pid, files, folder);
      if (!out.folder) return null;
      folder = out.folder;
      copied += out.copied ?? 0;
    }
    return folder ? { folder, copied } : null;
  };
  const when = (s: number) => new Date(s * 1000).toLocaleString(i18n.language, { dateStyle: "medium", timeStyle: "short" });

  return (
    <main className="flex-1 min-h-0 overflow-y-auto">
      {quick && <QuickListen start={start} onClose={() => setQuick(false)} onOpen={(pid) => { setQuick(false); onOpen(pid); }} />}
      <div className="max-w-5xl mx-auto px-6 py-8 grid gap-8 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <section aria-labelledby="listen-new" className="space-y-4">
          <div className="flex items-center gap-2">
            <h1 id="listen-new" className="text-xl font-bold tracking-tight flex items-center gap-2"><Ear size={20} className="text-[var(--color-accent)]" />{t("listen.newTitle")}</h1>
            <Tip text={t("listen.quickHint")} className="ml-auto inline-flex"><button onClick={() => setQuick(true)} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]"><Sparkles size={13} className="text-[var(--color-accent)]" />{t("listen.quickTitle")}</button></Tip>
          </div>
          <p className="text-[13px] text-[var(--color-muted)] leading-relaxed">{t("listen.newHint")}</p>
          <div className="grid gap-2 grid-cols-2">
            <label className="text-[11px] text-[var(--color-muted)] space-y-1">
              <span className="block">{t("listen.engine")} <Hint text={t("hint.listenEngine")} /></span>
              <select value={start.engine} onChange={(e) => pickStart({ ...start, engine: e.target.value as ListenEngine })}
                className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2 py-1.5 text-[12px] text-[var(--color-text)]">
                {ENGINES.map((e) => <option key={e} value={e}>{t(`listen.engine_${e || "default"}`)}</option>)}
              </select>
            </label>
            <label className="text-[11px] text-[var(--color-muted)] space-y-1">
              <span className="block">{t("speech.lang")} <Hint text={t("hint.listenLang")} /></span>
              <select value={start.lang} onChange={(e) => pickStart({ ...start, lang: e.target.value })}
                className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2 py-1.5 text-[12px] text-[var(--color-text)]">
                {LANG_CHOICES.map((l) => <option key={l} value={l}>{l === "auto" ? t("speech.langAuto") : l.toUpperCase()}</option>)}
              </select>
            </label>
          </div>
          <div role="tablist" aria-label={t("listen.sources")} className="flex gap-1 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-1">
            {SOURCES.map(({ id, icon: Icon }) => (
              <button key={id} role="tab" id={`listen-source-${id}`} aria-selected={source === id} aria-controls="listen-source-panel" onClick={() => pickSource(id)}
                className={`flex-1 inline-flex items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 text-[12px] font-medium transition-colors ${source === id ? "bg-[var(--color-accent)] text-[var(--color-on-accent)]" : "text-[var(--color-muted)] hover:text-[var(--color-text)] hover:bg-[var(--color-surface-2)]"}`}>
                <Icon size={14} />{t(`listen.source_${id}`)}
              </button>
            ))}
          </div>
          <div role="tabpanel" id="listen-source-panel" aria-labelledby={`listen-source-${source}`}>
            {source === "file" && (
            <div
              onDragOver={(e) => { e.preventDefault(); setDrag(true); }}
              onDragLeave={() => setDrag(false)}
              onDrop={(e) => { e.preventDefault(); setDrag(false); void upload([...e.dataTransfer.files]); }}
              className={`rounded-xl border-2 border-dashed px-5 py-8 grid place-items-center text-center gap-2 transition-colors ${drag ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_8%,transparent)]" : "border-[var(--color-border)]"}`}
            >
              {busy ? <Loader2 size={22} className="animate-spin text-[var(--color-accent)]" /> : <FileAudio size={22} className="text-[var(--color-accent)]" />}
              <p className="text-[13px]">{busy ? t("listen.uploading") : t("listen.drop")}</p>
              <button onClick={() => file.current?.click()} disabled={busy} className="px-3 py-1.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40">{t("listen.pickFile")}</button>
              <input ref={file} type="file" accept={MEDIA} multiple hidden onChange={(e) => { const all = [...(e.target.files ?? [])]; e.target.value = ""; void upload(all); }} />
              <p className="text-[11px] text-[var(--color-muted)]">{t("listen.formats")}</p>
            </div>
            )}
            {source === "link" && <UrlImport onReady={(f) => onOpen(f.pid)} />}
            {source === "mic" && <MicRecorder lang={start.lang} onDone={(pid, project) => { void prepare(pid, project, start).then(() => onOpen(pid), (e: unknown) => setError(errorText(e))); }} onError={setError} />}
            {source === "folder" && <WatchFolder start={start} onOpen={onOpen} />}
          </div>
          {batch && (
            <BatchList title={t("batch.listenTitle")} items={batch} running={batchRunning} onOpen={onOpen} onSaveAll={saveBatch} onClose={() => setBatch(null)}
              options={
                <fieldset className="flex flex-wrap items-center gap-2 text-[12px]">
                  <legend className="sr-only">{t("batch.formats")}</legend>
                  <span className="text-[var(--color-muted)]">{t("batch.formats")}</span>
                  {TRANSCRIPT_FORMATS.map((f) => (
                    <label key={f} className="inline-flex items-center gap-1">
                      <input type="checkbox" checked={formats.includes(f)} onChange={(e) => setFormats((all) => (e.target.checked ? [...all, f] : all.filter((x) => x !== f)))} className="accent-[var(--color-accent)]" />
                      {f.toUpperCase()}
                    </label>
                  ))}
                </fieldset>
              } />
          )}
          {error && <p role="alert" className="text-[12px] text-[var(--color-warn)] break-words">{error}</p>}
        </section>
        <section aria-labelledby="listen-list" className="space-y-3">
          <h2 id="listen-list" className="text-[13px] font-semibold">{t("listen.projects")}</h2>
          {(projects?.length ?? 0) > 5 && (
            <input value={find} onChange={(e) => setFind(e.target.value)} placeholder={t("speech.findProject")} aria-label={t("speech.findProject")}
              className="w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-3 py-1.5 text-[12px] focus:border-[var(--color-accent)] focus:outline-none" />
          )}
          {projects === null ? <Loader2 size={16} className="animate-spin text-[var(--color-muted)]" /> : projects.length === 0 ? (
            <p className="text-[13px] text-[var(--color-muted)]">{t("listen.noProjects")}</p>
          ) : (
            <ul className="space-y-1.5">
              {projects.filter((p) => !find.trim() || p.name.toLowerCase().includes(find.trim().toLowerCase())).map((p) => (
                <li key={p.pid} className="group flex items-center gap-2 rounded-lg border border-[var(--color-border)] hover:border-[var(--color-accent)] transition-colors">
                  <button onClick={() => onOpen(p.pid)} className="flex-1 min-w-0 text-left px-3 py-2">
                    <span className="block text-[13px] font-medium truncate">{p.name}</span>
                    <span className="block text-[11px] text-[var(--color-muted)] tabnum">
                      {[clock(p.duration), p.transcribed ? t("listen.cardLines", { n: p.lines, speakers: p.speakers }) : t("listen.cardNew"), when(p.created || p.mtime)].join(" · ")}
                    </span>
                  </button>
                  <button onClick={() => void remove(p)} className="p-2 mr-1 rounded text-[var(--color-muted)] pointer-fine:opacity-0 pointer-fine:group-hover:opacity-100 focus:opacity-100 hover:text-[var(--color-warn)]" {...named(t("listen.remove", { name: p.name }))}><Trash2 size={14} /></button>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
      {removed && <UndoToast key={removed.pid} text={t("undo.deleted", { name: removed.name || t("speech.untitled") })} onUndo={async () => { await speechApi.restore(removed.pid); load(); }} onClose={() => setRemoved(null)} />}
    </main>
  );
}

/** A recording from a microphone straight into a new project: device, level, time. */
function nowMs(): number {
  return performance.now();
}

function MicRecorder({ lang, onDone, onError }: { lang: string; onDone: (pid: string, project: ListenProject) => void; onError: (e: string) => void }) {
  const { t } = useTranslation();
  const [devices, setDevices] = useState<string[] | null>(null);
  const [device, setDevice] = useState("");
  const [recording, setRecording] = useState(false);
  const [level, setLevel] = useState(0);
  const [seconds, setSeconds] = useState(0);
  const [live, setLive] = useState(true);
  const [heard, setHeard] = useState<{ start: number; text: string }[]>([]);
  const [partial, setPartial] = useState<{ start: number; text: string } | null>(null);
  const timer = useRef<number | null>(null);
  const polls = useRef(0);
  useEffect(() => {
    api.recordDevices().then((d) => { setDevices(d.devices); setDevice(d.devices[0] ?? ""); }, () => setDevices([]));
    return () => { if (timer.current) window.clearInterval(timer.current); };
  }, []);
  const { waiting: fetching, caught } = useAfterDownload(onError);
  const start = async () => {
    try {
      await listenApi.recordStart(device || undefined, live, lang);
      setRecording(true);
      setHeard([]);
      setPartial(null);
      const began = nowMs();
      timer.current = window.setInterval(() => {
        setSeconds(Math.floor((nowMs() - began) / 1000));
        api.recordLevel().then((l) => setLevel(l.level), () => undefined);
        polls.current += 1;
        if (live && polls.current % 8 === 0) {
          listenApi.recordLive().then((s) => { setHeard(s.lines); setPartial(s.partial); if (s.error) onError(s.error); }, (e: unknown) => onError(errorText(e)));
        }
      }, 120);
    } catch (e) { if (!caught(e, () => { void start(); })) onError(errorText(e)); }
  };
  const stop = async () => {
    if (timer.current) window.clearInterval(timer.current);
    timer.current = null;
    setRecording(false);
    setLevel(0);
    try { const r = await listenApi.recordStop(); onDone(r.pid, r.project); } catch (e) { onError(errorText(e)); }
  };
  return (
    <div className="rounded-xl border border-[var(--color-border)] px-4 py-3 space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        <Mic size={15} className="text-[var(--color-accent)]" />
        <span className="text-[13px] font-medium">{t("listen.mic")}</span> <Hint text={t("hint.mic")} />
        <select value={device} onChange={(e) => setDevice(e.target.value)} disabled={recording || !devices?.length} aria-label={t("voices.device")}
          className="ml-auto max-w-[55%] bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]">
          {devices === null && <option>…</option>}
          {devices?.length === 0 && <option value="">{t("voices.noDevices")}</option>}
          {devices?.map((d) => <option key={d} value={d}>{d}</option>)}
        </select>
      </div>
      <div className="flex items-center gap-3">
        <div className="flex-1 h-1.5 rounded-full bg-[var(--color-surface-2)] overflow-hidden" role="meter" aria-label={t("voices.level")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(Math.min(1, level * 1.4) * 100)}>
          <div className="h-full bg-[var(--color-accent)] transition-[width] duration-100" style={{ width: `${Math.min(100, level * 140)}%` }} />
        </div>
        <label className="inline-flex items-center gap-1.5 text-[12px] cursor-pointer">
          <input type="checkbox" checked={live} onChange={(e) => setLive(e.target.checked)} disabled={recording} className="w-3.5 h-3.5 accent-[var(--color-accent)]" />{t("listen.live")} <Hint text={t("listen.liveHint")} />
        </label>
        <span className="mono text-[12px] tabnum w-12 text-right text-[var(--color-muted)]">{clock(seconds)}</span>
        {recording ? (
          <button onClick={() => void stop()} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--color-warn)] text-[var(--color-on-accent)] text-[12px] font-semibold"><Square size={12} />{t("listen.micStop")}</button>
        ) : (
          <button onClick={(e) => { if (byPerson(e)) void start(); }} {...PERSON_ONLY} disabled={!devices?.length} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)] disabled:opacity-40"><Mic size={12} />{t("listen.micStart")}</button>
        )}
      </div>
      {fetching && <p role="status" className="text-[12px] text-[var(--color-muted)]">{fetching}</p>}
      {recording && live && (
        <div aria-live="polite" className="max-h-40 overflow-y-auto rounded-md bg-[var(--color-surface-2)] px-2 py-1.5 text-[12px] space-y-1">
          {heard.map((h) => <p key={h.start}><span className="mono text-[10px] text-[var(--color-muted)] mr-2">{clock(h.start)}</span>{h.text}</p>)}
          {partial && <p className="text-[var(--color-muted)]"><span className="mono text-[10px] mr-2">{clock(partial.start)}</span>{partial.text}…</p>}
          {!heard.length && !partial && <p className="text-[var(--color-muted)]">{t("listen.liveWaiting")}</p>}
        </div>
      )}
    </div>
  );
}
