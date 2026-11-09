import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, BookOpen, BookUp, ChartGantt, Download, Drama, History, ListOrdered, Loader2, MessagesSquare, PenLine, Redo2, Search, Settings2, Sparkles, SpellCheck, Square, Undo2, Users, Wand2, X, ListChecks } from "lucide-react";
import { api, ApiError } from "../../lib/api";
import type { VoiceAge, VoiceGender } from "../../lib/api";
import { activeTake, engineOf, fxOf, newId, VOICE_FX, spliceChars, SPEAKER_TINTS, speechApi, stressDoubts, voiceOf, type Block, type BlockState, type SpeechProject, type QueuedSynth, type StressWord, type BookPreview } from "../../lib/speech";
import { openSettings } from "../../lib/settingsNav";
import { useSpeechProject } from "../../lib/useSpeechProject";
import BookImport from "./BookImport";
import { sentenceAt, wordAt, wordTimes } from "../../lib/readAlong";
import { useLivePlayback } from "../../lib/livePlayback";
import { LAYER_OPENED } from "../../lib/escape";
import { useEngines } from "../../lib/engines";
import { followJob } from "../../lib/followJob";
import { errorText } from "../../lib/voices";
import { named } from "../../lib/a11y";
import EditorMore from "../EditorMore";
import StepBar from "./StepBar";
import LintDialog from "./LintDialog";
import EngineAB from "./EngineAB";
import QualityReport, { QualityButton } from "./QualityReport";
import OwnParams from "./OwnParams";
import { useSimpleMode } from "../../lib/simpleMode";
import { useVoices } from "../../lib/voices";
import { comboOf, matches, typingIn, useShortcuts } from "../../lib/shortcuts";
import DirectorAsk from "./DirectorAsk";
import EnrichDialog, { type EnrichChange, type EnrichMode } from "./EnrichDialog";
import { cloudApi } from "../../lib/cloud";
import Transport, { type TransportBlock } from "./Transport";
import { RATES } from "./rates";
import { useStore } from "../../store";
import { usePublishEditor } from "../../lib/editorBridge";
import ErrorNote from "../ErrorNote";
import RecastDialog from "./RecastDialog";
import { CLOUD_ENGINE_NAMES, askCloud, askDirector } from "../../lib/cloudConsent";
import { JOBS_CHANGED, bridgeError, isMine, useChanged } from "../../lib/mcpBridge";
import { closestCenter, DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type Announcements, type DragEndEvent, type UniqueIdentifier } from "@dnd-kit/core";
import { SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy } from "@dnd-kit/sortable";
import BlockRow, { type BlockActions } from "./BlockRow";
import LazyBlock from "./LazyBlock";

/** Where the settings mend what keeps voicing from starting, by the studio's code for it. */
const NOT_READY_SETTINGS: Record<string, string> = { no_key: "cloud:key", model_missing: "models", no_device: "models" };
import { LAZY_FROM, showBlockRow } from "./blockRows";
import VoicePicker from "./VoicePicker";
import ProjectSettings from "./ProjectSettings";
import ExportDialog from "./ExportDialog";
import DictionaryDialog from "./DictionaryDialog";
import SearchReplace from "./SearchReplace";
import ProjectTimeline, { type Montage, type OverlapChoice } from "./ProjectTimeline";
import MusicDialog from "./MusicDialog";
import EngineSelect from "./EngineSelect";
import HistoryDialog from "./HistoryDialog";
import Hint, { Tip } from "../Hint";

function blockList(p: SpeechProject): Block[] {
  return p.chapters.flatMap((c) => c.blocks);
}

/** Where a block sits: its chapter and index, for edits that move or cut blocks. */
function locate(p: SpeechProject, id: string): [number, number] | null {
  for (let c = 0; c < p.chapters.length; c++) {
    const i = p.chapters[c].blocks.findIndex((b) => b.id === id);
    if (i >= 0) return [c, i];
  }
  return null;
}

const WIDE = "(min-width: 1280px)";
/** Whether the window is wide enough for the block panel to stand beside the list. */
function useWide(): boolean {
  return useSyncExternalStore((change) => {
    const query = window.matchMedia(WIDE);
    query.addEventListener("change", change);
    return () => query.removeEventListener("change", change);
  }, () => window.matchMedia(WIDE).matches);
}

/** A play() that a pause stopped before the sound started: the player is paused as asked, nothing failed. */
function pausedBeforeStart(e: unknown): boolean {
  return e instanceof DOMException && e.name === "AbortError";
}

export default function ProjectEditor({ pid, onBack }: { pid: string; onBack: () => void }) {
  const { t } = useTranslation();
  const { project, error, saving, edit, undo, redo, canUndo, canRedo, flush, reload } = useSpeechProject(pid);
  const [job, setJob] = useState<{ id: string; ids: string[] | null } | null>(null);
  const latestProject = useRef(project);
  const latestJob = useRef(job);
  useEffect(() => { latestProject.current = project; latestJob.current = job; }, [project, job]);
  const [jobError, setJobError] = useState<string | null>(null);
  const [playing, setPlaying] = useState<string | null>(null);
  const [playingTake, setPlayingTake] = useState<number | null>(null);
  const [queue, setQueue] = useState<string[]>([]);
  const [settings, setSettings] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [dictionary, setDictionary] = useState(false);
  const [searching, setSearching] = useState(false);
  const [found, setFound] = useState<{ re: RegExp | null; current: string | null }>({ re: null, current: null });
  const onFound = useCallback((re: RegExp | null, current: string | null) => setFound({ re, current }), []);
  const [timelineOpen, setTimelineOpen] = useState(false);
  const [musicOpen, setMusicOpen] = useState(false);
  const [history, setHistory] = useState(false);
  const [castOpen, setCastOpen] = useState(false);
  const simple = useSimpleMode();
  const keys = useShortcuts();
  const { listing: library } = useVoices();
  // the character's variants (angry, whispering) a block can speak with, from the voice it gets from its speaker
  const variantsFor = (b: Block): { base: string; inherited: string | null; list: { name: string; variant: string }[] } | null => {
    if (!project) return null;
    const inherited = (project.settings.one_voice ? null : project.cast[b.speaker]) || project.settings.voice || null;
    const own = voiceOf(project, b);
    if (!own) return null;
    const base = library?.voices.find((v) => v.name === own)?.card.variant_of || own;
    const list = (library?.voices ?? []).filter((v) => v.card.variant_of === base).map((v) => ({ name: v.name, variant: v.card.variant }));
    return list.length ? { base, inherited, list } : null;
  };
  const [exported, setExported] = useState(false);
  const [linting, setLinting] = useState(false);
  const [quality, setQuality] = useState(false);
  const [comparing, setComparing] = useState<string | null>(null);
  const [casting, setCasting] = useState(false);
  const castCharacters = async () => {
    if (!project) return;
    setCasting(true);
    try {
      if (!(await askDirector())) return;
      flush();
      for (const c of project.chapters) {
        const { job_id } = await speechApi.castChapter(pid, c.id);
        await followJob(job_id, "cast");
      }
    } catch (e) { setJobError(errorText(e)); } finally { setCasting(false); }
  };
  const [enriching, setEnriching] = useState(false);
  const [shownReview, setShownReview] = useState<"all" | "none" | "approved" | "fix">("all");
  const [enriched, setEnriched] = useState<{ changes: EnrichChange[]; refused: number; mode: EnrichMode } | null>(null);
  /** The director's tags for the selected blocks, or every block: shown to keep or drop before anything changes. */
  const enrich = async (ids: string[] | null, mode: "tags" | "punctuation" = "tags") => {
    setEnriching(true);
    try {
      if (!(await askDirector())) return;
      await flush();
      const { job_id } = await speechApi.enrich(pid, ids, mode);
      setEnriched({ ...((await followJob(job_id, "enrich")) as { changes: EnrichChange[]; refused: number }), mode });
    } catch (e) { setJobError(errorText(e)); } finally { setEnriching(false); }
  };
  const [asking, setAsking] = useState<{ mode: "extend" | "rewrite"; error: string | null } | null>(null);
  /** The director asked inside the project: the script's next lines are added after the last block, the chosen blocks
   * worded anew are shown to keep or drop. Either is one edit, so Undo takes it back. */
  const direct = async (text: string, minutes: number) => {
    if (!asking) return;
    const mode = asking.mode;
    setEnriching(true);
    setAsking({ mode, error: null });
    try {
      if (!(await askDirector())) return;
      await flush();
      if (mode === "rewrite") {
        const { job_id } = await speechApi.enrich(pid, selected, "rewrite", text);
        setEnriched({ ...((await followJob(job_id, "enrich")) as { changes: EnrichChange[]; refused: number }), mode });
      } else {
        const { job_id } = await speechApi.extend(pid, minutes, text);
        const { lines } = (await followJob(job_id, "write")) as { lines: { speaker: string; text: string; direction: string }[] };
        edit((p) => {
          const last = p.chapters[p.chapters.length - 1];
          for (const l of lines) last.blocks.push({ id: newId(), speaker: l.speaker, voice: null, text: l.text, direction: l.direction, pause_ms: null, hidden: false, takes: [], active: null, pinned: null, error: null, note: "", marker: "", start: null, end: null });
          return p;
        });
      }
      setAsking(null);
    } catch (e) { setAsking({ mode, error: errorText(e) }); } finally { setEnriching(false); }
  };
  const [selected, setSelected] = useState<string[]>([]);
  const [recast, setRecast] = useState<{ speaker: string; to: string | null } | null>(null);
  const [anchor, setAnchor] = useState<string | null>(null);

  const [synthQueue, setQueueList] = useState<QueuedSynth[]>([]);
  const [queueOpen, setQueueOpen] = useState(false);
  const [notes, setNotes] = useState<Record<string, { text: string; words: StressWord[] }>>({});
  const [stressing, setStressing] = useState<string[] | null>(null);
  const [stressMissing, setStressMissing] = useState(false);
  const audio = useRef<HTMLAudioElement | null>(null);
  const volume = useRef(1);
  const rate = useRef(1);
  const [speed, setSpeed] = useState(1);
  const [paused, setPaused] = useState(false);
  const [panel, setPanel] = useState<string | null>(null);
  const [side, setSide] = useState<HTMLElement | null>(null);
  const wide = useWide();
  const pushActivity = useStore((s) => s.pushActivity);

  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (matches(e, "search")) { e.preventDefault(); setSearching(true); return; }
      if (typingIn(e.target)) return;
      if (matches(e, "undo")) { e.preventDefault(); undo(); }
      else if (matches(e, "redo") || comboOf(e) === "Ctrl+Y") { e.preventDefault(); redo(); }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [undo, redo]);
  useEffect(() => () => { audio.current?.pause(); }, []);
  useLivePlayback(pid);
  const setSpeechEngine = useStore((s) => s.setSpeechEngine);
  const engines = useEngines();
  // the emotion tags the selected blocks' engines read, and one put at the start of each of those blocks
  const emotionsOf = (id: string) => engines?.find((e) => e.id === id)?.tags.find(([c]) => c === "emotion")?.[1] ?? [];
  const emotions = project ? [...new Set(blockList(project).filter((b) => selected.includes(b.id)).flatMap((b) => emotionsOf(engineOf(project, b))))] : [];
  const setEmotion = (value: string) => {
    if (!value || !project) return;
    let done = 0, skipped = 0;
    edit((p) => {
      for (const b of blockList(p).filter((x) => selected.includes(x.id))) {
        const caps = engines?.find((e) => e.id === engineOf(p, b));
        if (!caps || !emotionsOf(caps.id).length) { skipped++; continue; }
        const [head, tail] = caps.tag_format.replace("{category}", "emotion").split("{value}");
        const escape = (x: string) => x.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        const bare = b.text.replace(new RegExp(`^\\s*${escape(head)}[^\\s]+?${escape(tail ?? "")}`), "");
        b.text = value === "-" ? bare : `${head}${value}${tail ?? ""}${bare}`;
        done++;
      }
      return p;
    });
    pushActivity(t("speech.emotionApplied", { n: done, skipped }), "done");
  };
  const projectEngine = project ? engines?.find((e) => e.id === project.settings.engine)?.name ?? project.settings.engine : null;
  useEffect(() => {
    setSpeechEngine(projectEngine);
    return () => setSpeechEngine(null);
  }, [projectEngine, setSpeechEngine]);

  const blocks = useMemo(() => (project ? blockList(project) : []), [project]);
  // id -> flat index, so the block list does not do blocks.indexOf(b) per row (O(n^2) on 5000+ block books)
  const blockIndex = useMemo(() => new Map(blocks.map((b, i) => [b.id, i])), [blocks]);
  const speakers = useMemo(() => [...new Set(blocks.map((b) => b.speaker).filter(Boolean))], [blocks]);
  const tintOf = (s: string) => SPEAKER_TINTS[Math.max(0, speakers.indexOf(s)) % SPEAKER_TINTS.length];
  const stateOf = (b: Block): BlockState => project?.status?.[b.id]?.state ?? "pending";
  const transportBlocks = useMemo<TransportBlock[]>(() => blocks.map((b) => {
    const take = project?.status?.[b.id]?.state === "ready" ? activeTake(b) : null;
    return { id: b.id, speaker: b.speaker, text: b.text, duration: take ? take.duration : null };
  }), [blocks, project]);
  const counts = blocks.reduce<Record<string, number>>((acc, b) => { const s = stateOf(b); acc[s] = (acc[s] ?? 0) + 1; return acc; }, {});
  const dirty = (counts.pending ?? 0) + (counts.stale ?? 0) + (counts.failed ?? 0);
  const paid = project && (project.settings.engine === "fal" || project.settings.engine === "runware") && project.settings.cloud_model ? project.settings.engine : null;
  const paidBlocks = paid && project ? blocks.filter((b) => ["pending", "stale", "failed"].includes(stateOf(b)) && engineOf(project, b) === paid) : [];
  const paidChars = paidBlocks.reduce((n, b) => n + b.text.length, 0);
  const [estimate, setCost] = useState<{ usd: number | null; exact: boolean | null } | null>(null);
  const cost = paid && project && paidBlocks.length ? estimate : null;
  const cloudModel = project?.settings.cloud_model;
  const cloudValues = project?.settings.cloud_values;
  const paidCount = paidBlocks.length;
  useEffect(() => {
    if (!paid || cloudModel == null || !paidCount) return;
    let alive = true;
    const timer = window.setTimeout(() => {
      cloudApi.estimate(paid, cloudModel, cloudValues ?? {}, paidCount, paidChars)
        .then((c) => { if (alive) setCost(c); }, () => { if (alive) setCost(null); });
    }, 400);
    return () => { alive = false; window.clearTimeout(timer); };
  }, [paid, cloudModel, cloudValues, paidCount, paidChars]);
  // the blocks a character's voice reaches: theirs, without a voice of their own
  const castBlocks = (speaker: string) => blocks.filter((b) => b.speaker === speaker && !b.voice && !b.hidden);
  const setCast = (speaker: string, v: string | null) => edit((p) => { if (v) p.cast[speaker] = v; else delete p.cast[speaker]; return p; });
  const castVoice = (speaker: string, v: string | null) => {
    if (castBlocks(speaker).some((b) => b.takes.length > 0)) setRecast({ speaker, to: v });
    else setCast(speaker, v);
  };
  usePublishEditor({
    kind: "speech",
    pid,
    state: () => ({
      name: project?.name ?? null,
      blocks: blocks.map((b) => ({ id: b.id, speaker: b.speaker, text: b.text.slice(0, 160), state: stateOf(b), hidden: b.hidden, takes: b.takes.map((tk) => tk.n), active: activeTake(b)?.n ?? null })),
      selected,
      playing,
      voicing: !!job,
    }),
    select: (id) => {
      if (!blocks.some((b) => b.id === id)) return false;
      setSelected([id]);
      setAnchor(id);
      showBlockRow(id, { smooth: true });
      return true;
    },
    player: {
      state: () => {
        const a = playing ? audio.current : null;
        return { playing, take: playing ? playingTake : null, paused, queued: queue, position: a?.currentTime ?? null, duration: a && Number.isFinite(a.duration) ? a.duration : null, volume: volume.current, rate: rate.current };
      },
      play: ({ id, on, from, take }) => {
        if (from !== undefined) throw bridgeError("speech_from");
        if (!id && playing && paused) return resumeAudio();
        if (!id) throw bridgeError("missing", { command: "player_play", field: "id" });
        const b = blocks.find((x) => x.id === id);
        if (!b) throw bridgeError("no_line", { id });
        if (on) return listenFrom(b.id);
        if (!activeTake(b)) throw bridgeError("no_take", { id });
        if (take === undefined) { playBlock(b, []); return; }
        if (!b.takes.some((tk) => tk.n === take)) throw bridgeError("no_such_take", { id, take });
        playBlock(b, [], take, playing === b.id ? audio.current?.currentTime ?? 0 : 0);
      },
      pause: () => pauseAudio(),
      seek: (seconds) => {
        const a = audio.current;
        if (!a || !playing) throw bridgeError("nothing_plays");
        a.currentTime = seconds;
      },
      set: ({ volume: level, rate, loop }) => {
        if (loop !== undefined) throw bridgeError("bad_value", { command: "player_set", field: "loop", expected: "a Listening recording open (a Speech project plays block by block)" });
        if (rate !== undefined && !RATES.includes(rate)) throw bridgeError("bad_value", { command: "player_set", field: "rate", expected: RATES.join(", ") });
        if (rate !== undefined) changeRate(rate);
        if (level === undefined) return;
        volume.current = level;
        if (audio.current) audio.current.volume = level;
      },
    },
  });

  const stopAudio = useCallback(() => { audio.current?.pause(); setPlaying(null); setPaused(false); setQueue([]); }, []);
  const pauseAudio = () => { if (!playing) return; audio.current?.pause(); setPaused(true); };
  const pauseLatest = useRef(pauseAudio);
  useEffect(() => { pauseLatest.current = pauseAudio; });
  useEffect(() => {
    const pause = () => pauseLatest.current();
    window.addEventListener(LAYER_OPENED, pause);
    return () => window.removeEventListener(LAYER_OPENED, pause);
  }, []);
  const resumeAudio = () => { if (!playing || !audio.current) return; setPaused(false); audio.current.play().catch((e: unknown) => { if (pausedBeforeStart(e)) return; setPlaying(null); setJobError(errorText(e)); }); };
  const editSpoken = () => {
    const b = playing ? blocks.find((x) => x.id === playing) : null;
    if (!b) return;
    const take = b.takes.find((tk) => tk.n === playingTake) ?? activeTake(b);
    const word = take && (!take.text || take.text === b.text) ? wordAt(wordTimes(b.text, take.duration, take.qc?.timed), audio.current?.currentTime ?? 0) : null;
    pauseAudio();
    const field = document.getElementById(`block-${b.id}`)?.querySelector("textarea");
    if (!field) return;
    field.scrollIntoView({ block: "center" });
    field.focus();
    if (word) field.setSelectionRange(...sentenceAt(b.text, word[0]));
  };
  const changeRate = (r: number) => { rate.current = r; setSpeed(r); if (audio.current) audio.current.playbackRate = r; };
  const playBlock = useCallback((b: Block, rest: string[], n?: number, at = 0) => {
    const take = (n == null ? null : b.takes.find((tk) => tk.n === n)) ?? activeTake(b);
    if (!take) return;
    audio.current?.pause();
    const fx = latestProject.current ? fxOf(latestProject.current, b) : null;
    const url = speechApi.takeUrl(pid, take.file, fx, b.mix, take.cut, latestProject.current?.settings.loudness);
    const a = new Audio(`${url}${url.includes("?") ? "&" : "?"}n=${take.n}`);
    a.volume = volume.current;
    a.playbackRate = rate.current;
    const [from, to] = take.cut ?? [0, Infinity];
    const startAt = Math.max(at, from);
    if (startAt > 0) a.addEventListener("loadedmetadata", () => { a.currentTime = Math.min(startAt, Math.max(0, a.duration - 0.05)); }, { once: true });
    if (to < Infinity) a.addEventListener("timeupdate", () => { if (a.currentTime >= to && !a.paused) { a.pause(); if (audio.current === a) setPlaying(null); } });
    audio.current = a;
    setPlaying(b.id);
    setPlayingTake(take.n);
    setPaused(false);
    setQueue(rest);
    a.onended = () => setPlaying(null);
    a.play().catch((e: unknown) => { if (pausedBeforeStart(e)) return; setPlaying(null); setJobError(errorText(e)); });
  }, [pid]);

  useEffect(() => {
    if (!job) return;
    const tick = () => speechApi.queue(pid).then((r) => setQueueList(r.queue), () => undefined);
    const timer = window.setInterval(tick, 2000);
    return () => window.clearInterval(timer);
  }, [job, pid]);
  useEffect(() => {
    if (playing || !queue.length) return;
    const advance = () => {
      const p = latestProject.current;
      if (!p) return;
      const [next, ...rest] = queue;
      const b = blockList(p).find((x) => x.id === next);
      const state = b ? p.status?.[b.id]?.state : undefined;
      if (b && state === "ready" && activeTake(b)) playBlock(b, rest);
      else if (!b || state === "skipped") setQueue(rest);
      else if (!latestJob.current) setQueue([]);
    };
    const timer = window.setInterval(advance, 300);
    return () => window.clearInterval(timer);
  }, [playing, queue, playBlock]);

  const runSynth = async (scope: { ids?: string[]; from?: string; force?: boolean; seed?: number }) => {
    setJobError(null);
    try {
      const provider = project ? CLOUD_ENGINE_NAMES[project.settings.engine] : undefined;
      if (provider && !(await askCloud("speech", provider))) return;
      await flush();
      const r = await speechApi.synth(pid, scope);
      if (r.queued != null) {
        pushActivity(t("speech.queuedAt", { n: r.queued }), "work");
        setQueueList((await speechApi.queue(pid)).queue);
        return;
      }
      const job_id = r.job_id;
      setJob({ id: job_id, ids: scope.ids ?? null });
      await followJob(job_id, "synth");
    } catch (e) {
      setJobError(errorText(e));
      if (e instanceof ApiError && e.code === "no_voice") {
        setCastOpen(true);
        document.getElementById("speech-cast")?.scrollIntoView({ block: "nearest" });
      }
      const mend = e instanceof ApiError ? NOT_READY_SETTINGS[e.code] : undefined;
      if (mend) openSettings(mend);
    } finally { setJob(null); }
  };
  /** A voicing of this project started elsewhere (the agent, another window) is followed here as if started here. */
  const adoptJob = useCallback((id: string) => {
    void speechApi.queue(pid).then((r) => setQueueList(r.queue), () => undefined);
    if (latestJob.current?.id === id) return;
    setJob({ id, ids: null });
    void api.watchJob(id, () => undefined).catch(() => undefined).finally(() => setJob((j) => (j?.id === id ? null : j)));
  }, [pid]);
  useChanged(JOBS_CHANGED, (n) => {
    if (n.pid !== pid || n.kind !== "synth" || isMine(n) || !n.job_id) return;
    adoptJob(n.job_id);
  });
  useEffect(() => {
    api.jobs(pid).then((r) => {
      const running = r.jobs.find((j) => j.kind === "synth" && j.pid === pid && (j.status === "running" || j.status === "queued"));
      if (running) adoptJob(running.id);
    }, () => undefined);
  }, [pid, adoptJob]);
  const stopSynth = async () => {
    if (!job) return;
    try { await api.cancelJob(job.id); } catch (e) { setJobError(errorText(e)); }
  };
  /** A queued voicing taken back for editing: off the queue, its blocks selected to change and queue again. */
  const editQueued = async (q: QueuedSynth) => {
    const order = blocks.map((b) => b.id);
    const ids = q.scope.ids ?? (q.scope.from ? order.slice(Math.max(0, order.indexOf(q.scope.from))) : order);
    await speechApi.unqueue(pid, q.n);
    setQueueList((await speechApi.queue(pid)).queue);
    setQueueOpen(false);
    if (ids[0]) showBlock(ids[0]);
    setSelected(ids);
  };
  const skip = (step: 1 | -1) => {
    const order = blocks.map((b) => b.id);
    const to = order[(playing ? order.indexOf(playing) : -1) + step];
    if (to) listenFrom(to);
  };
  const showBlock = (id: string) => {
    setSelected([id]);
    setAnchor(id);
    showBlockRow(id, { smooth: true });
  };

  const listenFrom = (id: string) => {
    if (!project) return;
    const order = blockList(project).map((b) => b.id);
    const from = order.indexOf(id);
    const rest = order.slice(from);
    const needs = rest.some((x) => { const b = blockList(project).find((y) => y.id === x); return b && ["pending", "stale", "failed"].includes(stateOf(b)); });
    stopAudio();
    setQueue(rest);
    if (needs && !job) void runSynth({ from: id });
  };

  const [splitting, setSplitting] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const splitSpeech = async (ids: string[] | null) => {
    if (!project || splitting) return;
    setSplitting(true);
    setJobError(null);
    setNote(null);
    const cut: Record<string, { from: string; parts: { speech: boolean; text: string }[]; speaker: string }> = {};
    try {
      for (const b of blockList(project).filter((b) => b.text.trim() && (ids === null || ids.includes(b.id)))) {
        const r = await speechApi.directSpeech(b.text);
        if (r.parts.some((x) => x.speech) && b.speaker !== r.speaker) cut[b.id] = { from: b.text, ...r };
      }
    } catch (e) { setJobError(errorText(e)); } finally { setSplitting(false); }
    const n = Object.keys(cut).length;
    setNote(n ? t("speech.splitSpeechDone", { n }) : t("speech.splitSpeechNone"));
    if (!n) return;
    edit((p) => {
      for (const c of p.chapters) {
        c.blocks = c.blocks.flatMap((b) => {
          const x = cut[b.id];
          if (!x || x.from !== b.text) return [b];
          return x.parts.map((part, i) => {
            const speaker = part.speech ? x.speaker : b.speaker;
            const base: Block = i === 0 ? b : { ...b, id: newId(), takes: [], active: null, pinned: null, error: null, note: "", marker: "", start: null, end: null, word_notes: [] };
            return speaker === b.speaker ? { ...base, text: part.text } : { ...base, speaker, text: part.text, voice: null, engine: undefined, params: undefined };
          });
        });
      }
      return p;
    });
  };
  const bookFile = useRef<HTMLInputElement>(null);
  const [bookUpdate, setBookUpdate] = useState<BookPreview | null>(null);
  const openEditedBook = async (f: File) => {
    setJobError(null);
    try { await flush(); setBookUpdate(await speechApi.bookPreview(f)); } catch (e) { setJobError(errorText(e)); }
  };
  const [picking, setPicking] = useState(false);
  const autoCast = async () => {
    if (picking) return;
    setPicking(true);
    setJobError(null);
    setNote(null);
    try {
      await flush();
      const { picks } = await speechApi.autoCast(pid);
      const found = picks.filter((x) => x.voice);
      const none = picks.filter((x) => !x.voice).map((x) => x.speaker);
      if (found.length) edit((p) => { for (const x of found) p.cast[x.speaker] = x.voice!; return p; });
      setNote([
        found.length ? t("speech.autoCastDone", { list: found.map((x) => `${x.speaker} — ${x.voice}`).join(", ") }) : !none.length ? t("speech.autoCastAll") : "",
        none.length ? t("speech.autoCastNone", { list: none.join(", ") }) : "",
      ].filter(Boolean).join(" "));
    } catch (e) { setJobError(errorText(e)); } finally { setPicking(false); }
  };
  const setCharacter = (speaker: string, patch: { gender?: VoiceGender; age?: VoiceAge }) => edit((p) => {
    const all = { ...(p.characters ?? {}) };
    const c = { ...all[speaker], ...patch };
    if (!c.gender) delete c.gender;
    if (!c.age) delete c.age;
    if (Object.keys(c).length) all[speaker] = c; else delete all[speaker];
    p.characters = all;
    return p;
  });
  const stressBlocks = async (ids: string[] | null) => {
    if (!project || stressing) return;
    const todo = blockList(project).filter((b) => b.text.trim() && (ids === null || ids.includes(b.id)));
    setStressing(todo.map((b) => b.id));
    setJobError(null);
    setStressMissing(false);
    const done: Record<string, { from: string; text: string; words: StressWord[] }> = {};
    try {
      for (const b of todo) {
        const r = await speechApi.stress(b.text, project.settings.lang);
        if (r.russian) done[b.id] = { from: b.text, text: r.text, words: stressDoubts(r.words) };
      }
    } catch (e) {
      if (e instanceof ApiError && e.code === "stress-model-missing") setStressMissing(true);
      else setJobError(errorText(e));
    } finally {
      setStressing(null);
    }
    const changed = Object.keys(done);
    if (!changed.length) return;
    edit((p) => {
      for (const b of blockList(p)) {
        const d = done[b.id];
        if (d && b.text === d.from) b.text = d.text;
      }
      return p;
    });
    setNotes((n) => {
      const next = { ...n };
      for (const id of changed) next[id] = { text: done[id].text, words: done[id].words };
      return next;
    });
  };
  const pickVariant = (b: Block, w: StressWord, value: string) => {
    const note = notes[b.id];
    if (!note || note.text !== b.text) return;
    const text = spliceChars(b.text, w.start, w.end, value);
    const shift = Array.from(value).length - (w.end - w.start);
    edit((p) => { const at = locate(p, b.id); if (at) p.chapters[at[0]].blocks[at[1]].text = text; return p; });
    setNotes((n) => ({ ...n, [b.id]: { text, words: note.words.filter((x) => x !== w).map((x) => (x.start > w.start ? { ...x, start: x.start + shift, end: x.end + shift } : x)) } }));
  };

  const selectBlock = (id: string, range: boolean) => {
    const order = blocks.map((x) => x.id);
    if (range && anchor) {
      const [a1, a2] = [order.indexOf(anchor), order.indexOf(id)].sort((x, y) => x - y);
      setSelected([...new Set([...selected, ...order.slice(a1, a2 + 1)])]);
    } else {
      setSelected((cur) => (cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id]));
      setAnchor(id);
    }
  };
  /** Move blocks (in their order) next to `target`: after it when dragged down, before it when dragged up; across chapters too. */
  const moveTo = useCallback((ids: string[], target: string, after: boolean) => {
    if (ids.includes(target)) return;
    edit((p) => {
      const moving: Block[] = [];
      for (const c of p.chapters) {
        c.blocks = c.blocks.filter((b) => { if (ids.includes(b.id)) { moving.push(b); return false; } return true; });
      }
      moving.sort((x, y) => ids.indexOf(x.id) - ids.indexOf(y.id));
      const at = locate(p, target);
      if (!at) return p;
      p.chapters[at[0]].blocks.splice(at[1] + (after ? 1 : 0), 0, ...moving);
      return p;
    });
  }, [edit]);
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const order = blocks.map((x) => x.id);
    const from = order.indexOf(String(active.id));
    const to = order.indexOf(String(over.id));
    const ids = selected.includes(String(active.id)) ? order.filter((id) => selected.includes(id)) : [String(active.id)];
    moveTo(ids, String(over.id), from < to);
  };
  const blockLabel = (id: UniqueIdentifier) => {
    const i = blocks.findIndex((x) => x.id === id);
    return i < 0 ? "" : t("speech.dragBlock", { n: i + 1 });
  };
  const announcements: Announcements = {
    onDragStart: ({ active }) => t("speech.dragPicked", { block: blockLabel(active.id) }),
    onDragOver: ({ active, over }) => (over ? t("speech.dragOver", { block: blockLabel(active.id), target: blockLabel(over.id) }) : undefined),
    onDragEnd: ({ active, over }) => (over ? t("speech.dragDropped", { block: blockLabel(active.id), target: blockLabel(over.id) }) : t("speech.dragCancelled", { block: blockLabel(active.id) })),
    onDragCancel: ({ active }) => t("speech.dragCancelled", { block: blockLabel(active.id) }),
  };


  const actions = (b: Block): BlockActions => ({
    select: (range) => selectBlock(b.id, range),
    panel: (open) => setPanel(open ? b.id : null),
    focus: () => { if (wide && panel && panel !== b.id) setPanel(b.id); },
    ownRef: async (file, whole) => { await flush(); await speechApi.setOwnRef(pid, b.id, file, whole).catch((e: unknown) => { throw new Error(errorText(e)); }); reload(); },
    setText: (text) => edit((p) => { const at = locate(p, b.id); if (at) p.chapters[at[0]].blocks[at[1]].text = text; return p; }),
    patch: (change) => edit((p) => { const at = locate(p, b.id); if (at) Object.assign(p.chapters[at[0]].blocks[at[1]], change); return p; }),
    compare: () => void flush().then(() => setComparing(b.id)),
    split: (pos) => edit((p) => {
      const at = locate(p, b.id);
      if (!at) return p;
      const src = p.chapters[at[0]].blocks[at[1]];
      const head = src.text.slice(0, pos).trim();
      const tail = src.text.slice(pos).trim();
      if (!head || !tail) return p;
      src.text = head;
      p.chapters[at[0]].blocks.splice(at[1] + 1, 0, { ...src, id: newId(), text: tail, takes: [], active: null, pinned: null, error: null, note: "", marker: "", start: null, end: null });
      return p;
    }),
    mergeNext: () => edit((p) => {
      const at = locate(p, b.id);
      if (!at) return p;
      const list = p.chapters[at[0]].blocks;
      const next = list[at[1] + 1];
      if (!next) return p;
      list[at[1]].text = `${list[at[1]].text.trim()} ${next.text.trim()}`.trim();
      list.splice(at[1] + 1, 1);
      return p;
    }),
    insertAfter: () => edit((p) => {
      const at = locate(p, b.id);
      if (at) p.chapters[at[0]].blocks.splice(at[1] + 1, 0, { id: newId(), speaker: b.speaker, voice: null, text: "", direction: "", pause_ms: null, hidden: false, takes: [], active: null, pinned: null, error: null, note: "", marker: "", start: null, end: null });
      return p;
    }),
    move: (dir) => edit((p) => {
      const at = locate(p, b.id);
      if (!at) return p;
      const list = p.chapters[at[0]].blocks;
      const to = at[1] + dir;
      if (to < 0 || to >= list.length) return p;
      [list[at[1]], list[to]] = [list[to], list[at[1]]];
      return p;
    }),
    remove: () => edit((p) => { const at = locate(p, b.id); if (at && blockList(p).length > 1) p.chapters[at[0]].blocks.splice(at[1], 1); return p; }),
    synth: (force) => void runSynth({ ids: [b.id], force }),
    play: () => (playing === b.id ? stopAudio() : playBlock(b, [])),
    ab: (n) => {
      if (playing !== b.id) { playBlock(b, [], n); return; }
      playBlock(b, [], playingTake === n ? activeTake(b)?.n : n, audio.current?.currentTime ?? 0);
    },
    playFrom: () => listenFrom(b.id),
    stress: () => void stressBlocks([b.id]),
    pick: (w, value) => pickVariant(b, w, value),
  });

  if (!project) {
    return (
      <main className="flex-1 grid place-items-center px-4">
        {error ? (
          <div className="max-w-md text-center space-y-3">
            <p role="alert" className="text-[13px] text-[var(--color-warn)] break-words">{error}</p>
            <button onClick={onBack} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]"><ArrowLeft size={13} />{t("speech.allProjects")}</button>
          </div>
        ) : <Loader2 size={20} className="animate-spin text-[var(--color-muted)]" />}
      </main>
    );
  }
  const bar = "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] font-medium hover:border-[var(--color-accent)] disabled:opacity-40 transition-colors";
  return (
    <main className="flex-1 min-h-0 flex flex-col" data-mcp-page={`Speech: ${project.name}`}>
      <div className="flex flex-wrap lg:flex-nowrap items-center gap-x-2 gap-y-2 px-3 md:px-5 py-2.5 border-b border-[var(--color-border)] bg-[var(--color-surface)]/60">
        <button onClick={onBack} className="grid place-items-center w-9 h-9 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t("speech.allProjects"))}><ArrowLeft size={16} /></button>
        <input value={project.name} onChange={(e) => edit((p) => ({ ...p, name: e.target.value }))} aria-label={t("speech.name")}
          className="min-w-0 flex-1 md:flex-initial md:w-56 xl:w-72 lg:min-w-24 bg-transparent text-[15px] font-semibold focus:outline-none rounded px-1 focus:bg-[var(--color-surface-2)]" />
        <span className="hidden sm:inline lg:hidden xl:inline shrink-0 whitespace-nowrap text-[11px] text-[var(--color-muted)] tabnum">{t("speech.readyOf", { ready: counts.ready ?? 0, total: blocks.length - (counts.skipped ?? 0) })}{saving ? ` · ${t("speech.saving")}` : ""}</span>
        <QualityButton pid={pid} version={project} onOpen={() => void flush().then(() => setQuality(true))} />
        <button onClick={undo} disabled={!canUndo} className="hidden sm:grid place-items-center w-8 h-8 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30" {...named(t("speech.undo"))}><Undo2 size={15} /></button>
        <button onClick={redo} disabled={!canRedo} className="hidden sm:grid place-items-center w-8 h-8 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30" {...named(t("speech.redo"))}><Redo2 size={15} /></button>
        <div className="flex items-center gap-1.5 w-full md:w-auto md:shrink-0 max-md:order-last">
          {job && synthQueue.length > 0 && (
            <div className="relative">
              <button onClick={() => setQueueOpen((v) => !v)} aria-expanded={queueOpen} className={bar}><ListOrdered size={13} />{t("speech.queue", { n: synthQueue.length })}</button>
              {queueOpen && (
                <div className="absolute right-0 top-full mt-1 z-30 w-72 rounded-lg glass-panel p-2 space-y-1 text-[12px]">
                  {synthQueue.map((q) => (
                    <div key={q.n} className="flex items-center gap-2">
                      <span className="flex-1 truncate">{q.scope.ids ? t("speech.queueBlocks", { n: q.scope.ids.length }) : q.scope.from ? t("speech.queueFrom") : t("speech.queueAll")}</span>
                      <Tip text={t("speech.queueEditHint")}><button onClick={() => void editQueued(q)} className="px-1.5 rounded text-[11px] text-[var(--color-muted)] hover:text-[var(--color-accent)]">{t("speech.queueEdit")}</button></Tip>
                      <button onClick={() => speechApi.unqueue(pid, q.n).then(() => speechApi.queue(pid)).then((r) => setQueueList(r.queue), () => undefined)} className="p-0.5 rounded text-[var(--color-muted)] hover:text-[var(--color-warn)]" {...named(t("speech.unqueue"))}><X size={12} /></button>
                    </div>
                  ))}
                  <button onClick={() => speechApi.unqueue(pid).then(() => setQueueList([]), () => undefined)} className="w-full mt-1 px-2 py-1 rounded border border-[var(--color-border)] text-[11px] hover:border-[var(--color-warn)]">{t("speech.queueClear")}</button>
                </div>
              )}
            </div>
          )}
          {job ? (
            <button onClick={stopSynth} className={`${bar} flex-1 md:flex-none justify-center border-[var(--color-warn)] text-[var(--color-warn)]`}><Square size={13} />{t("speech.stop")}</button>
          ) : (
            <button onClick={() => void runSynth({})} disabled={!dirty} className={`${bar} flex-1 md:flex-none justify-center whitespace-nowrap bg-[var(--color-accent)] text-[var(--color-on-accent)] border-transparent`}><Sparkles size={13} />{t("speech.voiceDirty", { n: dirty })}{cost?.usd != null && <span className="tabnum font-normal opacity-80">{cost.exact ? "" : "≈ "}${cost.usd < 0.01 ? cost.usd.toFixed(4) : cost.usd.toFixed(2)}</span>}</button>
          )}
          <button onClick={() => setExporting(true)} className={`${bar} max-md:hidden`}><Download size={13} />{t("speech.export")}</button>
        </div>
        <div className="hidden md:flex shrink-0 items-center gap-1.5 ml-auto">
          <button onClick={() => setSettings(true)} className={bar} {...named(t("speech.projectSettings"))}><Settings2 size={13} /><span className="hidden min-[1700px]:inline">{t("speech.projectSettings")}</span></button>
          <button onClick={() => setSearching((v) => !v)} aria-pressed={searching} className={bar} {...named(t("search.title", { keys: keys.search }))}><Search size={13} /><span className="hidden min-[1700px]:inline">{t("search.open")}</span></button>
          {!simple && <>
          <button onClick={() => setTimelineOpen((v) => !v)} aria-pressed={timelineOpen} className={bar} {...named(t("timeline.hint"))}><ChartGantt size={13} /><span className="hidden min-[1700px]:inline">{t("timeline.open")}</span></button>
          <button onClick={() => setDictionary(true)} className={bar} {...named(t("speech.dictionaryHint"))}><BookOpen size={13} /><span className="hidden min-[1700px]:inline">{t("speech.dictionary")}</span></button>
          <button onClick={() => void castCharacters()} disabled={casting} className={bar} {...named(t("speech.castCharactersHint"))}>{casting ? <Loader2 size={13} className="animate-spin" /> : <Drama size={13} />}<span className="hidden min-[1700px]:inline">{t("speech.castCharacters")}</span></button>
          </>}
        </div>
        <EditorMore className="ml-auto md:ml-0" items={[
          { icon: Settings2, label: t("speech.projectSettings"), act: () => setSettings(true), narrowOnly: true },
          { icon: Search, label: t("search.open"), act: () => setSearching((v) => !v), narrowOnly: true },
          { icon: ChartGantt, label: t("timeline.open"), act: () => setTimelineOpen((v) => !v), narrowOnly: !simple },
          { icon: BookOpen, label: t("speech.dictionary"), act: () => setDictionary(true), narrowOnly: !simple },
          { icon: Drama, label: t("speech.castCharacters"), act: () => void castCharacters(), disabled: casting, narrowOnly: !simple },
          { icon: Users, label: t("speech.cast"), act: () => setCastOpen((v) => !v), belowLarge: true },
          { icon: ListChecks, label: t("lint.open"), act: () => void flush().then(() => setLinting(true)) },
          { icon: History, label: t("history.open"), act: () => setHistory(true) },
          { icon: Wand2, label: selected.length ? t("enrich.runSelected", { n: selected.length }) : t("enrich.run"), act: () => void enrich(selected.length ? selected : null), disabled: enriching },
          { icon: Wand2, label: selected.length ? t("enrich.punctSelected", { n: selected.length }) : t("enrich.punct"), act: () => void enrich(selected.length ? selected : null, "punctuation"), disabled: enriching },
          { icon: PenLine, label: t("enrich.extend"), act: () => setAsking({ mode: "extend", error: null }), disabled: enriching || !blocks.length },
          ...(selected.length ? [{ icon: PenLine, label: t("enrich.rewriteSelected", { n: selected.length }), act: () => setAsking({ mode: "rewrite" as const, error: null }), disabled: enriching }] : []),
          { icon: MessagesSquare, label: selected.length ? t("speech.splitSpeechSelected", { n: selected.length }) : t("speech.splitSpeech"), act: () => void splitSpeech(selected.length ? selected : null), disabled: splitting || !blocks.length },
          ...(project.book ? [{ icon: BookUp, label: t("book.rebuildOpen"), act: () => bookFile.current?.click() }] : []),
          { icon: SpellCheck, label: t("speech.stressAll"), act: () => void stressBlocks(null), disabled: !!stressing || !blocks.length },
          { icon: Sparkles, label: t("speech.voiceAll"), act: () => void runSynth({ force: true }), disabled: !!job },
          { icon: Download, label: t("speech.export"), act: () => setExporting(true), narrowOnly: true },
        ]} />
      </div>
      {simple && (
        <StepBar steps={[
          { id: "text", done: blocks.some((b) => b.text.trim()), run: () => { const first = blocks[0]; if (first) showBlockRow(first.id, { focus: true }); } },
          { id: "voices", done: blocks.length > 0 && blocks.every((b) => voiceOf(project, b) !== null || engines?.find((e) => e.id === engineOf(project, b))?.needs_voice === false), run: () => { setCastOpen(true); document.getElementById("speech-cast")?.scrollIntoView({ block: "nearest" }); } },
          { id: "voice", done: (counts.ready ?? 0) > 0 && (counts.ready ?? 0) >= blocks.length - (counts.skipped ?? 0), run: () => { if (!job) void runSynth({}); } },
          { id: "export", done: exported, run: () => setExporting(true) },
        ]} />
      )}

      {selected.length > 0 && (
        <div role="toolbar" aria-label={t("speech.selection")} className="flex flex-wrap items-center gap-2 px-5 py-2 border-b border-[var(--color-border)] bg-[color-mix(in_oklab,var(--color-accent)_8%,var(--color-surface))]">
          <span className="text-[12px] font-medium tabnum">{t("speech.selectedCount", { n: selected.length })}</span>
          {emotions.length > 0 && (
            <select value="" onChange={(e) => setEmotion(e.target.value)} aria-label={t("speech.emotionSelected")} className="bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-1.5 py-1 text-[12px]">
              <option value="">{t("speech.emotionSelected")}</option>
              <option value="-">{t("speech.emotionNone")}</option>
              {emotions.map((v) => <option key={v} value={v}>{t(`tags.${v}` as "tags.anger", { defaultValue: v })}</option>)}
            </select>
          )}
          <button onClick={() => edit((p) => { for (const b of blockList(p)) if (selected.includes(b.id)) b.review = "approved"; return p; })} className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]">{t("review.approveSelected")}</button>
          <button onClick={() => void runSynth({ ids: selected, force: true })} disabled={!!job} className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[12px] font-medium disabled:opacity-40"><Sparkles size={12} />{t("speech.voiceSelected")}</button>
          <button onClick={() => void stressBlocks(selected)} className="px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]">{t("speech.stressBlock")}</button>
          <button onClick={() => edit((p) => { const all = blockList(p).filter((b) => selected.includes(b.id)); const hide = all.some((b) => !b.hidden); all.forEach((b) => { b.hidden = hide; }); return p; })} className="px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]">{t("speech.hideShow")}</button>
          <button onClick={() => { edit((p) => { for (const c of p.chapters) c.blocks = c.blocks.filter((b) => !selected.includes(b.id)); if (!blockList(p).length) p.chapters[0].blocks.push({ id: newId(), speaker: t("speech.narrator"), voice: null, text: "", direction: "", pause_ms: null, hidden: false, takes: [], active: null, pinned: null, error: null, note: "", marker: "", start: null, end: null }); return p; }); setSelected([]); }} className="px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] hover:border-[var(--color-warn)] hover:text-[var(--color-warn)]">{t("speech.deleteSelected")}</button>
          <button onClick={() => setSelected(blocks.map((b) => b.id))} className="px-2.5 py-1 rounded-md text-[12px] text-[var(--color-muted)] hover:text-[var(--color-text)]">{t("speech.selectAll")}</button>
          <button onClick={() => setSelected([])} className="ml-auto p-1 rounded text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t("speech.selectNone"))}><X size={14} /></button>
        </div>
      )}
      {musicOpen && <MusicDialog pid={pid} tracks={(project.tracks ?? []).map((tr) => ({ id: tr.id, name: tr.name }))} flush={flush} onClose={() => setMusicOpen(false)} onDone={() => { setMusicOpen(false); reload(); }} />}
      {searching && (
        <SearchReplace blocks={blocks} onClose={() => { setSearching(false); setFound({ re: null, current: null }); }} onFound={onFound}
          onJump={(id) => showBlockRow(id, { smooth: true })}
          onReplace={(changes) => edit((p) => { for (const b of blockList(p)) if (changes[b.id] !== undefined) b.text = changes[b.id]; return p; })} />
      )}
      <div className="relative flex-1 min-h-0 flex">
        <aside id="speech-cast" aria-label={t("speech.cast")} className={`${castOpen ? "absolute inset-y-0 left-0 z-20 block bg-[var(--color-surface)] shadow-xl" : "hidden"} lg:static lg:block lg:shadow-none w-64 shrink-0 border-r border-[var(--color-border)] overflow-y-auto p-4 space-y-3`}>
          <h2 className="text-[11px] uppercase tracking-[0.14em] text-[var(--color-muted)] flex items-center gap-1.5"><Users size={12} />{t("speech.cast")}</h2>
          <label className="block text-[11px] text-[var(--color-muted)]">{t("speech.projectVoice")} <Hint text={t("hint.projectVoice")} />
            <VoicePicker engine={project.settings.engine} value={project.settings.voice} onChange={(v) => edit((p) => { p.settings.voice = v; return p; })} label={t("speech.projectVoice")}
              className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2 py-1.5 text-[12px]" />
          </label>
          <label className="flex items-start gap-2 text-[12px] cursor-pointer">
            <input type="checkbox" checked={!!project.settings.one_voice} onChange={(e) => edit((p) => { p.settings.one_voice = e.target.checked; return p; })} className="mt-0.5 w-3.5 h-3.5 accent-[var(--color-accent)]" />
            <span>{t("speech.oneVoice")} <Hint text={t("speech.oneVoiceHint")} /></span>
          </label>
          <Tip text={t("speech.autoCastHint")} className="flex">
            <button onClick={() => void autoCast()} disabled={picking || !speakers.length || !!project.settings.one_voice}
              className="flex-1 inline-flex items-center justify-center gap-1.5 px-2 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)] disabled:opacity-40">
              {picking ? <Loader2 size={13} className="animate-spin" /> : <Wand2 size={13} />}{t("speech.autoCast")}
            </button>
          </Tip>
          {speakers.map((s) => (
            <label key={s} className={`block text-[11px] ${project.settings.one_voice ? "opacity-50" : ""}`}>
              <span className="flex items-baseline gap-1.5"><span className="font-semibold" style={{ color: tintOf(s) }}>{s}</span><span className="text-[10px] tabnum text-[var(--color-muted)]">{t("speech.castBlocks", { n: blocks.filter((b) => b.speaker === s && !b.hidden).length })}</span><Hint text={t("hint.castCard")} /></span>
              <VoicePicker engine={project.cast_engines?.[s] || project.settings.engine} value={project.cast[s] ?? null} onChange={(v) => castVoice(s, v)} inherit={t("speech.voiceAsProject")} label={t("speech.voiceFor", { speaker: s })}
                className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2 py-1.5 text-[12px]" />
              <EngineSelect value={project.cast_engines?.[s] ?? null} projectEngine={project.settings.engine} inheritEngine={project.settings.engine} voice={project.cast[s] || project.settings.voice || null} lang={project.settings.lang}
                onChange={(e, v) => edit((p) => { const all = { ...(p.cast_engines ?? {}) }; if (e) all[s] = e; else delete all[s]; p.cast_engines = all; if (v) p.cast[s] = v; else if (v === null) delete p.cast[s]; return p; })}
                inherit={t("speech.engineAsProject")} label={t("speech.engineFor", { speaker: s })}
                className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2 py-1 text-[11px] text-[var(--color-muted)]" />
              <div className="mt-1">
                <OwnParams engine={project.cast_engines?.[s] || project.settings.engine} value={project.cast_params?.[s]} inherited={project.settings.params} label={t("speech.ownParamsSpeaker")}
                  onChange={(v) => edit((p) => { const all = { ...(p.cast_params ?? {}) }; if (v) all[s] = v; else delete all[s]; p.cast_params = all; return p; })} />
              </div>
              <select value={project.cast_fx?.[s] ?? ""} onChange={(e) => edit((p) => { const all = { ...(p.cast_fx ?? {}) }; if (e.target.value) all[s] = e.target.value; else delete all[s]; p.cast_fx = all; return p; })} aria-label={t("speech.voiceFxFor", { speaker: s })}
                className="mt-1 w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-1.5 py-1 text-[11px] text-[var(--color-muted)]">
                <option value="">{t("speech.voiceFx_none")}</option>
                {VOICE_FX.map((f) => <option key={f} value={f}>{t(`speech.voiceFx_${f}`)}</option>)}
              </select>
              <div className="mt-1 grid grid-cols-1 gap-1">
                <select value={project.characters?.[s]?.gender ?? ""} onChange={(e) => setCharacter(s, { gender: e.target.value as VoiceGender })} aria-label={t("speech.characterGender", { speaker: s })}
                  className="min-w-0 bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-1.5 py-1 text-[11px] text-[var(--color-muted)]">
                  {(["", "female", "male"] as const).map((g) => <option key={g} value={g}>{g ? t(`voices.gender_${g}`) : t("speech.genderAny")}</option>)}
                </select>
                <select value={project.characters?.[s]?.age ?? ""} onChange={(e) => setCharacter(s, { age: e.target.value as VoiceAge })} aria-label={t("speech.characterAge", { speaker: s })}
                  className="min-w-0 bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-1.5 py-1 text-[11px] text-[var(--color-muted)]">
                  {(["", "child", "teen", "adult", "senior"] as const).map((a) => <option key={a} value={a}>{a ? t(`voices.age_${a}`) : t("speech.ageAny")}</option>)}
                </select>
              </div>
            </label>
          ))}
        </aside>
        <div data-block-scroll className="flex-1 min-w-0 overflow-y-auto px-4 py-4">
          {error && <ErrorNote text={error} />}
          {jobError && <ErrorNote text={jobError} onClose={() => setJobError(null)} />}
          <input ref={bookFile} type="file" accept=".epub,.fb2,.pdf,.docx,.html,.htm,.md,.txt" hidden onChange={(e) => { const f = e.target.files?.[0]; e.target.value = ""; if (f) void openEditedBook(f); }} />
          {bookUpdate && <BookImport preview={bookUpdate} into={pid} onClose={() => setBookUpdate(null)} onCreated={() => { setBookUpdate(null); reload(); }} />}
          {note && (
            <div role="status" className="mb-3 px-3 py-2 rounded-lg border border-[var(--color-border)] text-[12px] text-[var(--color-muted)] flex items-center gap-2">
              <span className="flex-1 min-w-0">{note}</span>
              <button onClick={() => setNote(null)} className="p-0.5 rounded hover:text-[var(--color-text)]" {...named(t("a11y.close"))}><X size={13} /></button>
            </div>
          )}
          {stressMissing && (
            <div role="alert" className="mb-3 px-3 py-2 rounded-lg border border-[var(--color-warn)]/40 text-[12px] text-[var(--color-warn)] flex flex-wrap items-center gap-2">
              <span className="flex-1 min-w-0">{t("speech.stressMissing")}</span>
              <button onClick={() => { setStressMissing(false); openSettings("models"); }} className="px-2 py-1 rounded-md border border-[var(--color-warn)]/60 hover:bg-[var(--color-warn)]/10">{t("speech.openModels")}</button>
            </div>
          )}
          {blocks.some((b) => b.review) && (
            <div className="max-w-4xl mx-auto mb-2 flex items-center gap-2 text-[12px]">
              <label className="inline-flex items-center gap-1.5 text-[var(--color-muted)]">{t("review.show")}
                <select value={shownReview} onChange={(e) => setShownReview(e.target.value as typeof shownReview)} className="bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-1.5 py-1 text-[12px] text-[var(--color-text)]">
                  {(["all", "none", "approved", "fix"] as const).map((r) => <option key={r} value={r}>{t(`review.filter_${r}`, { n: r === "all" ? blocks.length : blocks.filter((b) => (r === "none" ? !b.review : b.review === r)).length })}</option>)}
                </select>
              </label>
            </div>
          )}
          <datalist id="speech-speakers">{speakers.map((s) => <option key={s} value={s} />)}</datalist>
          <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}
            accessibility={{ announcements, screenReaderInstructions: { draggable: t("speech.dragInstructions") } }}>
          {project.chapters.map((c) => (
            <SortableContext key={c.id} items={c.blocks.map((b) => b.id)} strategy={verticalListSortingStrategy}>
            <section className="max-w-4xl mx-auto">
              {project.chapters.length > 1 && <h2 className="mt-4 mb-2 text-[13px] font-semibold text-[var(--color-muted)]">{c.title || t("speech.chapter")}</h2>}
              <ol className="space-y-0.5">
                {c.blocks.filter((b) => shownReview === "all" || (shownReview === "none" ? !b.review : b.review === shownReview)).map((b) => {
                  const index = blockIndex.get(b.id) ?? 0;
                  const rendering = !!job && (job.ids === null ? ["pending", "stale", "failed"].includes(stateOf(b)) : job.ids.includes(b.id));
                  const note = notes[b.id];
                  const row = () => <BlockRow key={b.id} pid={pid} engine={engineOf(project, b)} projectEngine={project.settings.engine} projectLang={project.settings.lang} speakerEngine={project.cast_engines?.[b.speaker] || null} voice={voiceOf(project, b)} block={b} index={index} state={stateOf(b)} fit={project.status?.[b.id]?.fit} rendering={rendering} playing={playing === b.id} playingTake={playing === b.id ? playingTake : null} playhead={playing === b.id ? audio : undefined} simple={simple} variants={variantsFor(b)} inheritedParams={(project.settings.one_voice ? undefined : project.cast_params?.[b.speaker]) ?? project.settings.params} tint={tintOf(b.speaker)}
                    stressing={!!stressing?.includes(b.id)} doubts={note && note.text === b.text ? note.words : []}
                    selected={selected.includes(b.id)} selecting={selected.length > 0} open={panel === b.id} side={wide ? side : null} a={actions(b)}
                    found={found.re} foundHere={found.current === b.id} />;
                  if (blocks.length < LAZY_FROM) return row();
                  return <LazyBlock key={b.id} id={b.id} chars={b.text.length} keep={panel === b.id || playing === b.id || found.current === b.id} label={t("speech.blockN", { n: index + 1 })}>{row}</LazyBlock>;
                })}
              </ol>
            </section>
            </SortableContext>
          ))}
          </DndContext>
        </div>
        <aside ref={setSide} aria-label={t("speech.blockPanel")} className={`${wide && panel ? "block" : "hidden"} w-80 shrink-0 border-l border-[var(--color-border)] bg-[var(--color-surface)]/60 overflow-y-auto`} />
      </div>
      {timelineOpen && (
        <ProjectTimeline pid={pid} speakers={speakers} tintOf={tintOf} textOf={(id) => blocks.find((b) => b.id === id)?.text ?? ""} refresh={saving ? undefined : project} onClose={() => setTimelineOpen(false)}
          cueTimed={project.settings.timeline}
          tracks={project.tracks ?? []}
          onTrackClip={(trackId, clipId, change) => edit((p) => {
            const clip = p.tracks?.find((tr) => tr.id === trackId)?.clips.find((c) => c.id === clipId);
            if (!clip) return p;
            if (change.start !== undefined) clip.start = Math.max(0, change.start);
            if (change.cut !== undefined) clip.cut = change.cut;
            if (change.mix) clip.mix = change.mix;
            return p;
          })}
          onTrack={(trackId, change) => edit((p) => {
            if (change.remove) { p.tracks = (p.tracks ?? []).filter((tr) => tr.id !== trackId); return p; }
            const track = p.tracks?.find((tr) => tr.id === trackId);
            if (!track) return p;
            if (change.gain_db !== undefined) track.gain_db = change.gain_db;
            if (change.muted !== undefined) track.muted = change.muted;
            return p;
          })}
          onAddFiles={(files, trackId) => { void (async () => {
            try { await flush(); await speechApi.tracksUpload(pid, files, trackId, "", null); reload(); }
            catch (e) { setJobError(errorText(e)); }
          })(); }}
          onMusic={() => setMusicOpen(true)}
          onPlayMedia={(file, [from, to]) => {
            stopAudio();
            const a = new Audio(speechApi.mediaUrl(pid, file));
            a.volume = volume.current;
            a.addEventListener("loadedmetadata", () => { a.currentTime = from; }, { once: true });
            a.addEventListener("timeupdate", () => { if (a.currentTime >= to) a.pause(); });
            audio.current = a;
            a.play().catch((e: unknown) => { if (!pausedBeforeStart(e)) setJobError(errorText(e)); });
          }}
          onPlay={(id) => { const b = blocks.find((x) => x.id === id); if (b) playBlock(b, []); }}
          onMontage={(id, change: Montage) => edit((p) => {
            const at = locate(p, id);
            if (!at) return p;
            const b = p.chapters[at[0]].blocks[at[1]];
            if (change.mix) b.mix = change.mix;
            if (change.cut !== undefined) {
              const take = activeTake(b);
              if (take) take.cut = change.cut;
            }
            if (change.pause) {
              const before = locate(p, change.pause.prev);
              if (before) p.chapters[before[0]].blocks[before[1]].pause_ms = change.pause.ms;
            }
            if (change.shift !== undefined && b.start != null && b.end != null) {
              b.start = Math.max(0, b.start + change.shift);
              b.end = Math.max(b.start, b.end + change.shift);
            }
            return p;
          })}
          onJump={(id) => showBlockRow(id, { smooth: true })}
          onChoose={(id, previous, choice: OverlapChoice) => edit((p) => {
            const at = locate(p, id);
            const before = locate(p, previous);
            if (!at || !before) return p;
            p.chapters[at[0]].blocks[at[1]].cut_previous = choice === "cut";
            if (choice === "join") p.chapters[before[0]].blocks[before[1]].pause_ms = 0;
            return p;
          })} />
      )}
      <Transport blocks={transportBlocks} audio={audio} playing={playing} paused={paused} rate={speed} start={selected[0] ?? blocks[0]?.id ?? null} tint={tintOf}
        onPlay={listenFrom} onPause={pauseAudio} onResume={resumeAudio} onStop={stopAudio} onSkip={skip} onRate={changeRate} onShow={showBlock} onEdit={editSpoken} />
      {asking && <DirectorAsk mode={asking.mode} count={selected.length} busy={enriching} error={asking.error} onRun={(text, minutes) => void direct(text, minutes)} onClose={() => setAsking(null)} />}
      {enriched && (
        <EnrichDialog changes={enriched.changes} refused={enriched.refused} mode={enriched.mode} onClose={() => setEnriched(null)}
          onApply={(kept) => { edit((p) => { for (const b of blockList(p)) { const c = kept.find((x) => x.id === b.id); if (c && b.text === c.before) b.text = c.after; } return p; }); setEnriched(null); }} />
      )}
      {history && <HistoryDialog pid={pid} level={project.settings.loudness} onClose={() => setHistory(false)} onRepeat={(block, seed) => { setHistory(false); void runSynth({ ids: [block], force: true, seed }); }} />}
      {dictionary && <DictionaryDialog pid={pid} project={project} onChange={edit} onClose={() => setDictionary(false)} />}
      {recast && (
        <RecastDialog speaker={recast.speaker} from={project.cast[recast.speaker] ?? null} to={recast.to} blocks={castBlocks(recast.speaker)} engine={project.settings.engine}
          onApply={(revoice) => {
            const ids = castBlocks(recast.speaker).map((b) => b.id);
            setCast(recast.speaker, recast.to);
            setRecast(null);
            if (revoice) void runSynth({ ids, force: true });
          }}
          onClose={() => setRecast(null)} />
      )}
      {settings && <ProjectSettings project={project} onChange={edit} onClose={() => setSettings(false)} />}
      {comparing && blockList(project).some((b) => b.id === comparing) && (
        <EngineAB pid={pid} project={project} block={blockList(project).find((b) => b.id === comparing)!} onClose={() => setComparing(null)} onReload={reload}
          onPick={(change) => edit((p) => { const at = locate(p, comparing); if (at) Object.assign(p.chapters[at[0]].blocks[at[1]], change); return p; })} />
      )}
      {quality && <QualityReport pid={pid} blocks={blocks} onClose={() => setQuality(false)} onChanged={reload} onJump={(id) => { setSelected([id]); showBlockRow(id, { smooth: true }); }} />}
      {linting && <LintDialog pid={pid} blocks={blocks} onClose={() => setLinting(false)} onJump={(id) => { setSelected([id]); showBlockRow(id, { smooth: true }); }} />}
      {exporting && <ExportDialog pid={pid} project={project} onChange={edit} flush={flush} onClose={() => setExporting(false)} onVoiceMissing={(ids) => { setExporting(false); void runSynth({ ids, force: true }); }} onDone={(file) => { setExported(true); pushActivity(t("speech.exported", { file }), "done"); }} />}
    </main>
  );
}
