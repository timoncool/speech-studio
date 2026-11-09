import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, ChevronDown, Combine, Eye, EyeOff, GripVertical, Loader2, Pause, Pin, Play, Plus, RefreshCw, Scale, Scissors, SlidersHorizontal, SpellCheck, Trash2, X } from "lucide-react";
import { activeTake, cueTime, MARKERS, speechApi, VOICE_FX, withCase, type Block, type BlockState, type Marker, type QcCheck, type SpeechParams, type StressWord, type Take } from "../../lib/speech";
import { askCloneTerms } from "../../lib/cloudConsent";
import { named } from "../../lib/a11y";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import VoicePicker from "./VoicePicker";
import OwnParams from "./OwnParams";
import { LANG_CHOICES } from "./langs";
import EngineSelect from "./EngineSelect";
import TagPalette, { type TagPlace } from "./TagPalette";
import { useEngines } from "../../lib/engines";
import PrepareView from "./PrepareView";
import RefTrim from "../voices/RefTrim";
import HeardDiff from "./HeardDiff";
import { cues, spans, withoutCue, type Cue } from "../../lib/prosody";
import { wordAt, wordTimes } from "../../lib/readAlong";
import Hint, { Tip } from "../Hint";

const STATE_DOT: Record<BlockState | "rendering", string> = {
  pending: "bg-[var(--color-muted)]",
  ready: "bg-[var(--color-accent)]",
  stale: "bg-[var(--color-warn)]",
  failed: "bg-[#ff6b6b]",
  skipped: "bg-transparent border border-[var(--color-border)]",
  rendering: "bg-[var(--color-accent)] animate-pulse",
};

export type BlockActions = {
  setText: (text: string) => void;
  patch: (change: Partial<Block>) => void;
  split: (at: number) => void;
  mergeNext: () => void;
  insertAfter: () => void;
  move: (dir: -1 | 1) => void;
  remove: () => void;
  synth: (force: boolean) => void;
  play: () => void;
  /** Plays take n, or switches between it and the active take from the same moment while this block plays. */
  ab: (n: number) => void;
  playFrom: () => void;
  select: (range: boolean) => void;
  stress: () => void;
  pick: (w: StressWord, value: string) => void;
  /** Opens or closes this block's panel; only one is open at a time. */
  panel: (open: boolean) => void;
  /** The user is in this block: an open side panel moves to it. */
  focus: () => void;
  /** Uploads a clip as this block's own reference; its text is recognised. */
  ownRef: (file: File, whole: boolean) => Promise<void>;
  /** Opens the blind comparison of two engines on this block. */
  compare: () => void;
};

/** A/B of two takes: the button switches between the active take and the chosen one from the same moment. */
/** Recognition heard the take otherwise than its text: a clean match says nothing the block's status does not. */
const heardOff = (qc: QcCheck) => qc.text_checked !== false && qc.inserted + qc.deleted + qc.substituted > 0;

function TakeCompare({ block, active, playingTake, onAB }: { block: Block; active: number; playingTake: number | null; onAB: (n: number) => void }) {
  const { t } = useTranslation();
  const others = block.takes.filter((tk) => tk.n !== active).reverse();
  const [chosen, setChosen] = useState<number | null>(null);
  const other = others.find((tk) => tk.n === chosen)?.n ?? others[0].n;
  return (
    <label className="text-[11px] text-[var(--color-muted)]">{t("speech.abWith")} <Hint text={t("hint.abWith")} />
      <div className="mt-1 flex items-center gap-1">
        <select value={other} onChange={(e) => setChosen(Number(e.target.value))} aria-label={t("speech.abWith")}
          className="flex-1 min-w-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]">
          {others.map((tk) => <option key={tk.n} value={tk.n}>#{tk.n} · {tk.duration.toFixed(1)}{t("settings.sec")}{tk.qc ? ` · ${t("speech.takeWer", { wer: Math.round(tk.qc.wer * 100) })}` : ""}</option>)}
        </select>
        <button onClick={() => onAB(other)} aria-pressed={playingTake != null}
          className={`px-2 py-1 rounded-md border text-[11px] font-semibold tabnum ${playingTake != null ? "border-[var(--color-accent)] text-[var(--color-accent)]" : "border-[var(--color-border)] text-[var(--color-text)] hover:border-[var(--color-accent)]"}`}
          {...named(t("speech.abHint", { a: active, b: other }))}>{playingTake == null ? t("speech.ab") : `#${playingTake}`}</button>
      </div>
    </label>
  );
}

/** The word of the block sounding now, while `active`: followed frame by frame, the row redrawn only when it changes. */
function useSpokenWord(audio: React.RefObject<HTMLAudioElement | null> | undefined, text: string, take: Take | null, active: boolean): [number, number] | null {
  const [word, setWord] = useState<[number, number] | null>(null);
  const times = useMemo(() => (take && (!take.text || take.text === text) ? wordTimes(text, take.duration, take.qc?.timed) : []), [text, take]);
  useEffect(() => {
    if (!active || !audio || !times.length) return;
    let frame = 0;
    let last = "";
    const tick = () => {
      const at = audio.current ? wordAt(times, audio.current.currentTime) : null;
      const k = at ? at.join(":") : "";
      if (k !== last) { last = k; setWord(at); }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [active, audio, times]);
  return active && audio && times.length ? word : null;
}

function AutoText({ value, onCommit, label, inputRef, stale, locked, reading, found, foundHere }: { value: string; onCommit: (v: string) => void; label: string; inputRef: React.RefObject<HTMLTextAreaElement | null>; stale: boolean; locked: boolean; reading: [number, number] | null; found: RegExp | null; foundHere: boolean }) {
  const [draft, setDraft] = useState(value);
  const [seen, setSeen] = useState(value);
  if (value !== seen) { setSeen(value); setDraft(value); }
  useLayoutEffect(() => {
    const el = inputRef.current;
    if (!el) return;
    const fit = () => { el.style.height = "auto"; el.style.height = `${el.scrollHeight}px`; };
    fit();
    let width = el.clientWidth;
    const watch = new ResizeObserver(() => { if (el.clientWidth !== width) { width = el.clientWidth; fit(); } });
    watch.observe(el);
    return () => watch.disconnect();
  }, [draft, inputRef]);
  useEffect(() => {
    if (draft === value) return;
    const timer = window.setTimeout(() => onCommit(draft), 700);
    return () => window.clearTimeout(timer);
  }, [draft, value, onCommit]);
  const marks = useMemo(() => [
    ...cues(draft),
    ...(reading && draft === value ? [{ kind: "read" as const, from: reading[0], to: reading[1] }] : []),
    ...(found ? [...draft.matchAll(found)].filter((m) => m[0].length > 0).map((m) => ({ kind: "found" as const, from: m.index ?? 0, to: (m.index ?? 0) + m[0].length })) : []),
  ], [draft, value, reading, found]);
  const area = (
    <textarea ref={inputRef} value={draft} rows={1} aria-label={label} spellCheck readOnly={locked}
      onChange={(e) => setDraft(e.target.value)} onBlur={(e) => { const typed = e.currentTarget.value; if (typed !== value) onCommit(typed); }}
      className={`relative block w-full resize-none overflow-hidden bg-transparent text-[14px] leading-relaxed focus:outline-none placeholder:text-[var(--color-muted)] ${stale ? "underline decoration-dotted decoration-[var(--color-warn)] underline-offset-4" : ""}`} />
  );
  return (
    <div className="relative">
      {marks.length > 0 && <div aria-hidden className="absolute inset-0 pointer-events-none whitespace-pre-wrap [overflow-wrap:break-word] text-[14px] leading-relaxed text-transparent">
        {spans(draft, marks).map((s, k) => {
          if (s.kinds.includes("read")) return <span key={k} className="rounded-sm" style={{ background: "color-mix(in srgb, var(--color-accent) 35%, transparent)" }}>{s.text}</span>;
          if (s.kinds.includes("found")) return <span key={k} className={`rounded-sm ${foundHere ? "bg-[color-mix(in_oklab,var(--color-warn)_45%,transparent)]" : "bg-[color-mix(in_oklab,var(--color-warn)_20%,transparent)]"}`}>{s.text}</span>;
          const kind = s.kinds[s.kinds.length - 1];
          return kind
            ? <span key={k} className="rounded-sm underline decoration-2 underline-offset-[5px]" style={{ textDecorationColor: `var(--color-cue-${kind})`, background: `color-mix(in srgb, var(--color-cue-${kind}) 20%, transparent)` }}>{s.text}</span>
            : s.text;
        })}
        {"​"}
      </div>}
      {area}
    </div>
  );
}

/** The block's prosody cues under its text, coloured as in the text: a click selects what a cue covers, × takes it off. */
function CueList({ text, value, locked, onChange }: { text: React.RefObject<HTMLTextAreaElement | null>; value: string; locked: boolean; onChange: (v: string) => void }) {
  const { t } = useTranslation();
  const list = useMemo(() => cues(value), [value]);
  if (!list.length) return null;
  const excerpt = (c: Cue) => {
    const s = value.slice(c.from, c.to).replace(/\[[^\]]*\]|<[^<>]*>|\*\*/g, "").replace(/\s+/g, " ").trim();
    return s.length > 28 ? `${s.slice(0, 27)}…` : s;
  };
  return (
    <ul aria-label={t("prosody.title")} className="mt-1 flex flex-wrap gap-1 text-[11px]">
      {list.map((c, k) => (
        <li key={`${c.kind}-${c.from}-${k}`} className="inline-flex items-center gap-1 rounded-md border border-[var(--color-border)] pl-1.5 pr-0.5 py-0.5">
          <span className="w-2 h-2 rounded-sm shrink-0" style={{ background: `var(--color-cue-${c.kind})` }} />
          <button onClick={() => { const el = text.current; if (el && el.value === value) { el.focus(); el.setSelectionRange(c.from, c.to); } }} className="text-left hover:text-[var(--color-accent)]">
            {t(`prosody.${c.key}`, { value: c.value })}{c.kind !== "pause" && excerpt(c) ? ` «${excerpt(c)}»` : ""}
          </button>
          {!locked && <button onClick={() => onChange(withoutCue(value, c))} className="p-0.5 rounded text-[var(--color-muted)] hover:text-[var(--color-warn)]" {...named(t("prosody.remove"))}><X size={11} /></button>}
        </li>
      ))}
    </ul>
  );
}

/** A director's turn on a take: what to change, added to the direction the take was made with, makes the next take. */
function RetakeNote({ take, busy, onRun }: { take: Take; busy: boolean; onRun: (note: string) => void }) {
  const { t } = useTranslation();
  const [note, setNote] = useState("");
  const run = () => { const n = note.trim(); if (!n || busy) return; onRun(n); setNote(""); };
  return (
    <div className="text-[11px] text-[var(--color-muted)]">
      <label className="block">{t("speech.retakeNote", { n: take.n })}
        <div className="mt-1 flex items-center gap-1">
          <input value={note} onChange={(e) => setNote(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter") run(); }} placeholder={t("speech.retakeNoteHint")}
            className="flex-1 min-w-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]" />
          <button onClick={run} disabled={!note.trim() || busy} className="px-2 py-1 rounded-md border border-[var(--color-border)] text-[11px] text-[var(--color-text)] hover:border-[var(--color-accent)] disabled:opacity-40">{t("speech.retakeRun")}</button>
        </div>
      </label>
      {take.direction.trim() && <span className="mt-0.5 block italic">{t("speech.retakeChain", { n: take.n, text: take.direction })}</span>}
    </div>
  );
}

/** Where the sentence holding `cursor` starts: after the last sentence end or line break before it, its tags kept. */
function sentenceStart(value: string, cursor: number): number {
  let start = 0;
  for (const m of value.slice(0, cursor).matchAll(/[.!?…]+["»”)]*\s+|\n+/g)) start = (m.index ?? 0) + m[0].length;
  return start;
}

export default function BlockRow({ pid, engine, projectEngine, projectLang, speakerEngine, voice, block, index, state, fit, rendering, playing, playingTake, playhead, simple, variants, inheritedParams, tint, stressing, doubts, selected, selecting, open, side, a, found = null, foundHere = false }: {
  /** The project search's pattern, highlighted in the text; foundHere: the block the search last went to. */
  found?: RegExp | null;
  foundHere?: boolean;
  pid: string;
  /** The engine that voices this block (its own, its speaker's or the project's). */
  engine: string;
  projectEngine: string;
  /** The project's language: a block without its own speaks it. */
  projectLang: string;
  speakerEngine: string | null;
  /** The voice this block speaks with, its own or inherited. */
  voice: string | null;
  selected: boolean;
  selecting: boolean;
  /** Whether this block's panel (voice, engine, takes, tags) is open. */
  open: boolean;
  /** Where the open panel goes on a wide screen; null keeps it under the block. */
  side: HTMLElement | null;
  fit?: "fits" | "tight" | "impossible";
  block: Block;
  stressing: boolean;
  doubts: StressWord[];
  index: number;
  state: BlockState;
  rendering: boolean;
  playing: boolean;
  /** The take sounding now while this block plays. */
  playingTake: number | null;
  /** The player this block sounds in while it plays, to follow the word being read. */
  playhead?: React.RefObject<HTMLAudioElement | null>;
  /** The beginner's mode: the panel keeps what voicing a block needs and leaves out notes, review, tags and own refs. */
  simple: boolean;
  /** The sampling the block gets from its speaker or the project when it has none of its own. */
  inheritedParams: SpeechParams;
  /** The character's variants this block can speak with: its base voice, the voice it gets from its speaker, and each variant. */
  variants: { base: string; inherited: string | null; list: { name: string; variant: string }[] } | null;
  tint: string;
  a: BlockActions;
}) {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({ id: block.id });
  const [howRead, setHowRead] = useState(false);
  const text = useRef<HTMLTextAreaElement>(null);
  const take = activeTake(block);
  const sounding = (playingTake != null ? block.takes.find((tk) => tk.n === playingTake) : null) ?? take;
  const reading = useSpokenWord(playhead, block.text, sounding, playing);
  const insertTag = (tag: string, place: TagPlace) => {
    const el = text.current;
    const value = el?.value ?? block.text;
    const cursor = el?.selectionStart ?? value.length;
    const at = place === "sentence" ? sentenceStart(value, cursor) : cursor;
    const before = value.slice(0, at);
    const after = value.slice(at).trimStart();
    const gap = before && !/\s$/.test(before) ? " " : "";
    a.setText(place === "inline" ? `${before}${gap}${tag} ${after}`.trimEnd() : `${before}${gap}${tag}${after}`);
  };
  const wrapTag = (open: string, close: string) => {
    const el = text.current;
    const value = el?.value ?? block.text;
    const from = el?.selectionStart ?? value.length;
    const to = el?.selectionEnd ?? from;
    a.setText(`${value.slice(0, from)}${open}${value.slice(from, to)}${close}${value.slice(to)}`);
  };
  const rateTag = (tag: string) => {
    const el = text.current;
    if (el && el.selectionEnd > el.selectionStart && tag !== "[rate:1]") wrapTag(tag, "[rate:1]");
    else insertTag(tag, "inline");
  };
  const caps = useEngines()?.find((e) => e.id === engine);
  const shown: BlockState | "rendering" = rendering ? "rendering" : state;
  const icon = "p-1.5 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] hover:bg-[var(--color-surface-2)] disabled:opacity-30";
  const panel = (
    <div data-mcp-row={side ? `panel of block ${index + 1} ${block.speaker || "-"}` : undefined} className={`@container space-y-2 ${side ? "p-3" : "mx-2 mb-2 sm:ml-[10.25rem] rounded-lg bg-[var(--color-surface-2)]/60 border border-[var(--color-border)] p-2.5"}`}>
      {side && (
        <div className="flex items-center gap-2 pb-1 border-b border-[var(--color-border)]">
          <span className="text-[12px] tabnum text-[var(--color-muted)]">{t("speech.blockN", { n: index + 1 })}</span>
          <span className="text-[12px] font-semibold truncate" style={{ color: tint }}>{block.speaker}</span>
          <button onClick={() => a.panel(false)} className={`${icon} ml-auto`} {...named(t("a11y.close"))}><X size={14} /></button>
        </div>
      )}
      <div className="grid gap-2 @md:grid-cols-2 @2xl:grid-cols-4">
        <label className="text-[11px] text-[var(--color-muted)]">{t("speech.blockVoice")}
          <VoicePicker engine={engine} value={block.voice} onChange={(v) => a.patch({ voice: v })} inherit={t("speech.voiceAsCast")} label={t("speech.blockVoice")}
            className="mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]" />
        </label>
        <label className="text-[11px] text-[var(--color-muted)]">{t("speech.blockEngine")}
          <EngineSelect value={block.engine ?? null} projectEngine={projectEngine} inheritEngine={speakerEngine ?? projectEngine} voice={voice} lang={block.lang || projectLang} onChange={(e, v) => a.patch(v === undefined ? { engine: e } : { engine: e, voice: v })}
            inherit={speakerEngine ? t("speech.engineAsSpeaker") : t("speech.engineAsProject")} label={t("speech.blockEngine")}
            className="mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]" />
        </label>
        <label className="text-[11px] text-[var(--color-muted)]">{t("speech.blockLang")} <Hint text={t("hint.blockLang")} />
          <select value={block.lang ?? ""} onChange={(e) => a.patch({ lang: e.target.value })} aria-label={t("speech.blockLang")}
            className="mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]">
            <option value="">{t("speech.blockLangProject")}</option>
            {LANG_CHOICES.filter((l) => l !== "auto").map((l) => <option key={l} value={l}>{l.toUpperCase()}</option>)}
          </select>
        </label>
        <label className="text-[11px] text-[var(--color-muted)]">{t("speech.voiceFx")} <Hint text={t("hint.voiceFx")} />
          <select value={block.fx ?? ""} onChange={(e) => a.patch({ fx: e.target.value })} aria-label={t("speech.voiceFx")}
            className="mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]">
            <option value="">{t("speech.voiceFxAsSpeaker")}</option>
            <option value="none">{t("speech.voiceFx_none")}</option>
            {VOICE_FX.map((f) => <option key={f} value={f}>{t(`speech.voiceFx_${f}`)}</option>)}
          </select>
        </label>
        <label className="text-[11px] text-[var(--color-muted)]">{t("speech.pauseAfter")} <Hint text={t("hint.pauseAfter")} />
          <input type="number" step={50} min={-2000} max={10000} value={block.pause_ms ?? ""} placeholder={t("speech.pauseAuto")}
            onChange={(e) => a.patch({ pause_ms: e.target.value === "" ? null : Math.max(-2000, Math.min(10000, Number(e.target.value))) })}
            className="mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px] mono" />
        </label>
        <label className="text-[11px] text-[var(--color-muted)]">{t("speech.takes")} <Hint text={t("hint.takes")} />
          <div className="mt-1 flex items-center gap-1">
            <select value={block.active ?? ""} onChange={(e) => a.patch({ active: e.target.value ? Number(e.target.value) : null })} disabled={!block.takes.length} aria-label={t("speech.takes")}
              className="flex-1 min-w-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]">
              {!block.takes.length && <option value="">—</option>}
              {[...block.takes].reverse().map((tk) => <option key={tk.n} value={tk.n}>#{tk.n} · {tk.duration.toFixed(1)}{t("settings.sec")}{tk.qc ? ` · ${t("speech.takeWer", { wer: Math.round(tk.qc.wer * 100) })}` : ""}{tk.n === block.pinned ? ` · ${t("speech.pinned")}` : ""}</option>)}
            </select>
            <button onClick={() => a.patch({ pinned: block.pinned === block.active ? null : block.active })} disabled={block.active == null} aria-pressed={block.pinned != null && block.pinned === block.active}
              className={`p-1 rounded-md ${block.pinned != null && block.pinned === block.active ? "text-[var(--color-accent)]" : "text-[var(--color-muted)]"} hover:text-[var(--color-text)] disabled:opacity-30`} {...named(t("speech.pinTake"))}><Pin size={13} /></button>
          </div>
        </label>
        {variants && (
          <div role="radiogroup" aria-label={t("speech.voiceVariant")} className="col-span-full flex flex-wrap items-center gap-1 text-[11px]">
            <span className="text-[var(--color-muted)] mr-1">{t("speech.voiceVariant")} <Hint text={t("hint.voiceVariant")} /></span>
            {[{ name: variants.base, variant: t("speech.variantBase") }, ...variants.list].map((v) => {
              const on = voice === v.name;
              return (
                <button key={v.name} role="radio" aria-checked={on} onClick={() => a.patch({ voice: v.name === variants.inherited ? null : v.name })}
                  className={`px-2 py-0.5 rounded-md border ${on ? "border-[var(--color-accent)] text-[var(--color-text)]" : "border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>{v.variant}</button>
              );
            })}
          </div>
        )}
        {!simple && (
          <div className="col-span-full">
            <OwnParams engine={engine} value={block.params} inherited={inheritedParams} label={t("speech.ownParamsBlock")} onChange={(v) => a.patch({ params: v })} />
          </div>
        )}
        {block.takes.length > 1 && take && <TakeCompare block={block} active={take.n} playingTake={playingTake} onAB={a.ab} />}
        {!simple && (
          <div className="col-span-full">
            <Tip text={t("ab.hint")}><button onClick={a.compare} disabled={!block.text.trim()} className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)] disabled:opacity-40"><Scale size={13} />{t("ab.open")}</button></Tip>
          </div>
        )}
      </div>
      {!simple && <div className="flex flex-wrap items-end gap-2">
        <label className="flex-1 min-w-48 text-[11px] text-[var(--color-muted)]">{t("speech.note")}
          <input value={block.note} onChange={(e) => a.patch({ note: e.target.value })} placeholder={t("speech.noteHint")}
            className="mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]" />
        </label>
        <div role="radiogroup" aria-label={t("speech.marker")} className="flex items-center gap-1 pb-1">
          <button role="radio" aria-checked={!block.marker} onClick={() => a.patch({ marker: "" })} className={`w-4 h-4 rounded-sm border ${!block.marker ? "border-[var(--color-text)]" : "border-[var(--color-border)]"}`} {...named(t("speech.markerNone"))} />
          {(Object.keys(MARKERS) as Marker[]).map((m) => (
            <button key={m} role="radio" aria-checked={block.marker === m} onClick={() => a.patch({ marker: m })} {...named(t(`speech.marker_${m}`))}
              className={`w-4 h-4 rounded-sm border-2 ${block.marker === m ? "border-[var(--color-text)]" : "border-transparent"}`} style={{ background: MARKERS[m] }} />
          ))}
        </div>
      </div>}
      <label className="block text-[11px] text-[var(--color-muted)]">{t("speech.direction")} <Hint text={t("hint.direction")} />
        <input value={block.direction} onChange={(e) => a.patch({ direction: e.target.value })} placeholder={t("speech.directionHint")}
          className="mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]" />
        {caps && block.direction.trim() && !caps.direction && <span className="mt-0.5 block text-[var(--color-warn)]">{t("speech.directionIgnored", { engine: caps.name })}</span>}
        {caps && block.direction.trim() && caps.direction && caps.direction_presets_only && voice !== null && !caps.presets.some((p) => p.id === voice) && <span className="mt-0.5 block text-[var(--color-warn)]">{t("speech.directionPresetsOnly", { engine: caps.name })}</span>}
      </label>
      {caps?.direction && take && <RetakeNote take={take} busy={rendering} onRun={(note) => { a.patch({ direction: [take.direction.trim(), note].filter(Boolean).join("; ") }); a.synth(true); }} />}
      {!simple && <>
      <div role="radiogroup" aria-label={t("review.title")} className="flex flex-wrap items-center gap-1 text-[11px]">
        <span className="text-[var(--color-muted)] mr-1">{t("review.title")} <Hint text={t("hint.review")} /></span>
        {(["", "approved", "fix"] as const).map((r) => (
          <button key={r || "none"} role="radio" aria-checked={(block.review ?? "") === r} onClick={() => a.patch({ review: r })}
            className={`px-2 py-0.5 rounded-md border ${(block.review ?? "") === r ? "border-[var(--color-accent)] text-[var(--color-text)]" : "border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>{t(r ? `review.${r}` : "review.none")}</button>
        ))}
      </div>
      <WordNoteField block={block} a={a} text={text} />
      <OwnRefField block={block} a={a} />
      {take && (
        <details className="rounded-md border border-[var(--color-border)] px-2 py-1.5">
          <summary className="cursor-pointer text-[11px] text-[var(--color-muted)]">{take.cut ? t("speech.takeEdgesSet", { n: take.n, a: take.cut[0].toFixed(1), z: take.cut[1].toFixed(1) }) : t("speech.takeEdges", { n: take.n })} <Hint text={t("hint.takeEdges")} /></summary>
          <div className="mt-2">
            <RefTrim url={speechApi.takeUrl(pid, take.file)} initial={take.cut ?? null} busy={false}
              onApply={(a0, z) => a.patch({ takes: block.takes.map((tk) => (tk.n === take.n ? { ...tk, cut: [Math.round(a0 * 100) / 100, Math.round(z * 100) / 100] as [number, number] } : tk)) })}
              onReset={() => a.patch({ takes: block.takes.map((tk) => (tk.n === take.n ? { ...tk, cut: null } : tk)) })} />
          </div>
        </details>
      )}
      <details open={howRead} onToggle={(e) => setHowRead(e.currentTarget.open)} className="rounded-md border border-[var(--color-border)] px-2 py-1.5">
        <summary className="cursor-pointer text-[11px] text-[var(--color-muted)]">{t("speech.howRead")} <Hint text={t("hint.howRead")} /></summary>
        {howRead && <div className="mt-2"><PrepareView pid={pid} block={block.id} text={block.text} /></div>}
      </details>
      <details className="rounded-md border border-[var(--color-border)] px-2 py-1.5">
        <summary className="cursor-pointer text-[11px] text-[var(--color-muted)]">{t("speech.tagsTitle")} <Hint text={t("hint.tags")} /></summary>
        <div className="mt-2"><TagPalette engine={engine} onInsert={insertTag} onWrap={wrapTag} onRate={rateTag} /></div>
      </details>
      </>}
      <div className="flex flex-wrap gap-1">
        <button onClick={() => a.split(text.current?.selectionStart ?? block.text.length)} className={icon + " inline-flex items-center gap-1 text-[11px]"}><Scissors size={12} />{t("speech.split")}</button>
        <button onClick={a.mergeNext} className={icon + " inline-flex items-center gap-1 text-[11px]"}><Combine size={12} />{t("speech.mergeNext")}</button>
        <button onClick={a.insertAfter} className={icon + " inline-flex items-center gap-1 text-[11px]"}><Plus size={12} />{t("speech.insertAfter")}</button>
        <Tip text={t("speech.stressAllHint")}><button onClick={a.stress} disabled={stressing} className={icon + " inline-flex items-center gap-1 text-[11px]"}>
          {stressing ? <Loader2 size={12} className="animate-spin" /> : <SpellCheck size={12} />}{t("speech.stressBlock")}
        </button></Tip>
        <button onClick={() => a.move(-1)} className={icon} {...named(t("speech.moveUp"))}><ArrowUp size={12} /></button>
        <button onClick={() => a.move(1)} className={icon} {...named(t("speech.moveDown"))}><ArrowDown size={12} /></button>
        <button onClick={() => a.patch({ hidden: !block.hidden })} className={icon + " inline-flex items-center gap-1 text-[11px]"}>{block.hidden ? <Eye size={12} /> : <EyeOff size={12} />}{block.hidden ? t("speech.show") : t("speech.hide")}</button>
        <button onClick={a.playFrom} disabled={!take} className={icon + " inline-flex items-center gap-1 text-[11px]"}><Play size={12} />{t("speech.playFromHere")}</button>
        <button onClick={a.remove} className={icon + " ml-auto inline-flex items-center gap-1 text-[11px] hover:text-[var(--color-warn)]"}><Trash2 size={12} />{t("speech.deleteBlock")}</button>
      </div>
    </div>
  );
  return (
    <li id={`block-${block.id}`} ref={setNodeRef} onFocus={a.focus} onClick={a.focus} data-mcp-row={`block ${index + 1} ${block.speaker || "-"} ${shown}${take ? ` ${take.duration.toFixed(1)}s` : ""}${block.hidden ? " hidden" : ""}: ${block.text.slice(0, 60)}`} style={{ transform: CSS.Transform.toString(transform), transition, contentVisibility: "auto", containIntrinsicSize: "auto 96px" }}
      className={`group rounded-xl border transition-colors ${playing ? "border-[var(--color-accent)]" : selected ? "border-[var(--color-accent)]/60 bg-[color-mix(in_oklab,var(--color-accent)_6%,transparent)]" : "border-transparent hover:border-[var(--color-border)]"} ${block.hidden ? "opacity-50" : ""} ${isDragging ? "relative z-10 bg-[var(--color-surface)] shadow-lg" : ""}`}>
      <div className="grid grid-cols-[auto_minmax(0,1fr)_auto] gap-x-3 gap-y-1 px-2 py-2 sm:flex sm:gap-3">
        <div className="flex flex-col items-center gap-1 pt-1 -mr-1">
          <button type="button" ref={setActivatorNodeRef} {...attributes} {...listeners} aria-label={t("speech.drag")} title={t("speech.drag")}
            className="cursor-grab touch-none rounded text-[var(--color-muted)] pointer-fine:opacity-0 pointer-fine:group-hover:opacity-100 focus-visible:opacity-100"><GripVertical size={13} /></button>
          <input type="checkbox" checked={selected} onChange={() => undefined} onClick={(e) => a.select(e.shiftKey)} {...named(t("speech.selectBlock"))}
            className={`w-3.5 h-3.5 accent-[var(--color-accent)] ${selecting || selected ? "" : "pointer-fine:opacity-0 pointer-fine:group-hover:opacity-100 focus-visible:opacity-100"}`} />
        </div>
        <div className="min-w-0 pt-0.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 sm:block sm:w-36 sm:shrink-0 sm:space-y-1">
          <div className="flex items-baseline gap-1.5 min-w-0">
            <span aria-hidden className="w-7 shrink-0 text-right text-[12px] mono tabnum text-[var(--color-muted)]">{index + 1}</span>
            <input list="speech-speakers" value={block.speaker} onChange={(e) => a.patch({ speaker: e.target.value })} aria-label={t("speech.speaker")}
              className="min-w-0 w-32 sm:w-full bg-transparent text-[12px] font-semibold truncate focus:outline-none rounded px-1 -mx-1 focus:bg-[var(--color-surface-2)]" style={{ color: tint }} />
          </div>
          <div className="flex flex-wrap items-center gap-x-1.5 gap-y-0.5 sm:pl-[2.125rem] text-[10px] text-[var(--color-muted)]">
            <span className={`w-2 h-2 rounded-full shrink-0 ${STATE_DOT[shown]}`} title={t(`speech.state_${shown}`)} />
            <span>{t(`speech.state_${shown}`)}</span>
            {block.review && <span className={`px-1 rounded ${block.review === "approved" ? "text-[var(--color-accent)]" : "text-[var(--color-warn)]"}`} title={t(`review.${block.review}Hint`)}>{t(`review.${block.review}`)}</span>}
            {take && <span className="mono tabnum ml-auto">{take.duration.toFixed(1)}{t("settings.sec")}</span>}
          </div>
          {block.start != null && block.end != null && (
            <Tip text={t("speech.cueHint")} className="flex"><div className="flex items-center gap-1 sm:pl-[2.125rem] text-[10px] mono tabnum text-[var(--color-muted)]">
              <span>{cueTime(block.start)}–{cueTime(block.end)}</span>
              {fit && fit !== "fits" && <span className={`px-1 rounded ${fit === "tight" ? "text-[var(--color-warn)]" : "text-[#ff8f8f]"}`} title={t(`speech.fit_${fit}`)}>{fit === "tight" ? "≈" : "!"}</span>}
            </div></Tip>
          )}
          {(block.marker || block.note) && (
            <div className="flex items-center gap-1 sm:pl-[2.125rem] text-[10px] text-[var(--color-muted)]" title={block.note || undefined}>
              {block.marker && <span className="w-2 h-2 rounded-sm shrink-0" style={{ background: MARKERS[block.marker] }} />}
              {block.note && <span className="truncate italic">{block.note}</span>}
            </div>
          )}
        </div>
        <div className="col-span-3 order-last min-w-0 sm:order-none sm:flex-1">
          <AutoText value={block.text} onCommit={a.setText} label={t("speech.blockText", { n: index + 1 })} inputRef={text} stale={state === "stale"} locked={block.review === "approved"} reading={reading} found={found} foundHere={foundHere} />
          <CueList text={text} value={block.text} locked={block.review === "approved"} onChange={a.setText} />
          {doubts.length > 0 && (
            <div className="mt-1 flex flex-wrap items-center gap-1 text-[11px]">
              <span className="text-[var(--color-muted)]">{t("speech.stressCheck")}</span>
              {doubts.map((w) => (
                <span key={`${w.start}-${w.word}`} title={w.doubt ? t("speech.stressDoubt") : undefined}
                  className={`inline-flex items-center gap-0.5 rounded-md border px-1 py-0.5 ${w.doubt || w.yo_choice ? "border-[var(--color-warn)]/60" : "border-[var(--color-border)]"}`}>
                  {w.yo_choice ? (
                    <>
                      <button onClick={() => a.pick(w, w.word)} className="px-1 rounded hover:text-[var(--color-accent)]" title={t("speech.stressKeep")}>{w.word}</button>
                      <span className="text-[var(--color-muted)]">/</span>
                      <button onClick={() => a.pick(w, w.yo_choice!)} className="px-1 rounded hover:text-[var(--color-accent)]">{w.yo_choice}</button>
                    </>
                  ) : (w.variants ?? []).map(([v, p], k) => {
                    const shown = withCase(w.word, v);
                    return (
                      <span key={v} className="inline-flex items-center">
                        {k > 0 && <span className="text-[var(--color-muted)] px-0.5">/</span>}
                        <button onClick={() => a.pick(w, shown)} aria-pressed={shown === w.word}
                          className={`px-1 rounded mono hover:text-[var(--color-accent)] ${shown === w.word ? "text-[var(--color-accent)]" : ""}`}>{shown} <span className="text-[var(--color-muted)] tabnum">{Math.round(p * 100)}%</span></button>
                      </span>
                    );
                  })}
                </span>
              ))}
            </div>
          )}
          {take && take.text && take.text !== block.text && (
            <p className="mt-0.5 flex flex-wrap items-center gap-1.5 text-[11px] text-[var(--color-muted)]">
              <span className="truncate max-w-[60ch]" title={take.text}>{t("speech.takeText", { n: take.n, text: take.text })}</span>
              <button onClick={() => a.patch({ text: take.text, direction: take.direction })} className="px-1.5 rounded border border-[var(--color-border)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)]">{t("speech.takeTextRestore")}</button>
            </p>
          )}
          {take && (take.fit_stretch > 1.001 || take.fit_lead > 0.001 || take.fit_over) && (
            <p className={`mt-0.5 text-[11px] ${take.fit_over ? "text-[#ff8f8f]" : "text-[var(--color-muted)]"}`}>
              {[take.fit_lead > 0.001 && t("speech.fitLead", { s: take.fit_lead.toFixed(1) }), take.fit_stretch > 1.001 && t("speech.fitStretch", { k: take.fit_stretch.toFixed(2) }), take.fit_over && t("speech.fitOver")].filter(Boolean).join(" · ")}
            </p>
          )}
          {take?.qc && (heardOff(take.qc) || take.qc.similarity != null || (take.qc.issues ?? []).length > 0) && (
            <p className={`mt-0.5 text-[11px] ${take.qc.passed && take.qc.similar !== false && !(take.qc.issues ?? []).length ? "text-[var(--color-muted)]" : "text-[var(--color-warn)]"}`} title={take.qc.heard || undefined}>
              {[heardOff(take.qc) && t("speech.qcLine", { wer: Math.round(take.qc.wer * 100), ins: take.qc.inserted, del: take.qc.deleted, sub: take.qc.substituted }),
                take.qc.similarity != null && t("speech.qcSimilarity", { s: take.qc.similarity.toFixed(2) }), ...(take.qc.issues ?? []).map((i) => t(`speech.qcIssue_${i}`))].filter(Boolean).join(" · ")}
            </p>
          )}
          {take?.qc?.notes?.length ? <p className="mt-0.5 text-[11px] text-[var(--color-muted)]">{t("speech.qcNotes", { list: take.qc.notes.map((n) => t(`speech.qcNote_${n}`)).join(", ") })}</p> : null}
          {take?.qc && take.qc.text_checked !== false && <HeardDiff off={take.qc.off} className="mt-0.5" />}
          {(block.word_notes?.length ?? 0) > 0 && (
            <ul aria-label={t("speech.wordNotes")} className="mt-1 flex flex-wrap gap-1 text-[11px]">
              {block.word_notes!.map((w, k) => (
                <li key={k} className="inline-flex items-center gap-1 rounded-md border border-[var(--color-border)] pl-1.5 pr-0.5 py-0.5">
                  {w.marker && <span className="w-2 h-2 rounded-sm shrink-0" style={{ background: MARKERS[w.marker] }} />}
                  <button onClick={() => { const el = text.current; const at = el ? el.value.indexOf(w.quote) : -1; if (el && at >= 0) { el.focus(); el.setSelectionRange(at, at + w.quote.length); } }}
                    className="text-left hover:text-[var(--color-accent)]">«{w.quote}» {w.note}</button>
                  <button onClick={() => a.patch({ word_notes: block.word_notes!.filter((_, i) => i !== k) })} className="p-0.5 rounded text-[var(--color-muted)] hover:text-[var(--color-warn)]" {...named(t("speech.wordNoteRemove"))}><X size={11} /></button>
                </li>
              ))}
            </ul>
          )}
          {block.error && state === "failed" && <p role="alert" className="mt-1 text-[11px] text-[#ff8f8f] break-words">{block.error.text}</p>}
          {block.direction && !open && <p className="mt-0.5 text-[11px] text-[var(--color-muted)] italic truncate">{t("speech.directionShort", { text: block.direction })}</p>}
        </div>
        <div className="flex items-start gap-0.5 shrink-0 opacity-70 group-hover:opacity-100 focus-within:opacity-100">
          <button onClick={a.play} disabled={!take} className={icon} {...named(playing ? t("speech.stopPlaying") : t("speech.play"))}>{playing ? <Pause size={14} /> : <Play size={14} />}</button>
          <button onClick={() => a.synth(state === "ready")} disabled={rendering || state === "skipped"} className={icon} {...named(state === "ready" ? t("speech.newTake") : t("speech.voiceBlock"))}>
            {rendering ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
          </button>
          <button onClick={() => a.panel(!open)} aria-expanded={open} className={icon} {...named(t("speech.blockPanel"))}>{open ? <ChevronDown size={14} className="rotate-180" /> : <SlidersHorizontal size={14} />}</button>
        </div>
      </div>
      {open && !side && panel}
      {open && side && createPortal(panel, side)}
    </li>
  );
}

/** The block's own reference instead of a voice: a clip with its text; "whole" continues the clip's speech. */
function OwnRefField({ block, a }: { block: Block; a: BlockActions }) {
  const { t } = useTranslation();
  const input = useRef<HTMLInputElement>(null);
  const [whole, setWhole] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const r = block.own_ref;
  const upload = async (f: File) => {
    if (!(await askCloneTerms())) return;
    setBusy(true);
    setError(null);
    try { await a.ownRef(f, whole); } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setBusy(false); }
  };
  const field = "mt-1 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]";
  return (
    <fieldset className="rounded-md border border-[var(--color-border)] px-2 py-1.5 space-y-1.5">
      <legend className="px-1 text-[11px] text-[var(--color-muted)]">{t("speech.ownRef")} <Hint text={t("speech.ownRefHint")} /></legend>
      {r ? (
        <>
          <div className="flex items-center gap-2 text-[12px]">
            <span className="mono truncate flex-1" title={r.file}>{r.file}</span>
            <button onClick={() => a.patch({ own_ref: null })} className="px-1.5 rounded border border-[var(--color-border)] text-[11px] hover:border-[var(--color-warn)] hover:text-[var(--color-warn)]">{t("speech.ownRefClear")}</button>
          </div>
          <label className="block text-[11px] text-[var(--color-muted)]">{t("speech.ownRefText")}
            <textarea value={r.text} rows={2} onChange={(e) => a.patch({ own_ref: { ...r, text: e.target.value } })} className={`${field} resize-y`} />
          </label>
          <label className="flex items-center gap-1.5 text-[11px]">
            <input type="checkbox" checked={r.whole} onChange={(e) => a.patch({ own_ref: { ...r, whole: e.target.checked } })} className="accent-[var(--color-accent)]" />{t("speech.ownRefWhole")} <Hint text={t("hint.ownRefWhole")} />
          </label>
        </>
      ) : (
        <div className="flex flex-wrap items-center gap-2">
          <button onClick={() => input.current?.click()} disabled={busy} className="inline-flex items-center gap-1 px-2 py-1 rounded-md border border-[var(--color-border)] text-[11px] hover:border-[var(--color-accent)] disabled:opacity-40">
            {busy ? <Loader2 size={12} className="animate-spin" /> : <Plus size={12} />}{t("speech.ownRefUpload")}
          </button>
          <label className="flex items-center gap-1.5 text-[11px]">
            <input type="checkbox" checked={whole} onChange={(e) => setWhole(e.target.checked)} className="accent-[var(--color-accent)]" />{t("speech.ownRefWhole")} <Hint text={t("hint.ownRefWhole")} />
          </label>
          <input ref={input} type="file" accept="audio/*,video/*" hidden onChange={(e) => { const f = e.target.files?.[0]; e.target.value = ""; if (f) void upload(f); }} />
        </div>
      )}
      {error && <p role="alert" className="text-[11px] text-[var(--color-warn)] break-words">{error}</p>}
    </fieldset>
  );
}

/** A note on the words selected in the block's text, with a marker colour. */
function WordNoteField({ block, a, text }: { block: Block; a: BlockActions; text: React.RefObject<HTMLTextAreaElement | null> }) {
  const { t } = useTranslation();
  const [quote, setQuote] = useState("");
  const [note, setNote] = useState("");
  const [marker, setMarker] = useState<Marker | "">("");
  const take = () => { const el = text.current; if (el) setQuote(el.value.slice(el.selectionStart, el.selectionEnd).trim()); };
  const add = () => {
    if (!quote) return;
    a.patch({ word_notes: [...(block.word_notes ?? []), { quote, note: note.trim(), marker }] });
    setQuote("");
    setNote("");
    setMarker("");
  };
  const field = "bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]";
  return (
    <fieldset className="rounded-md border border-[var(--color-border)] px-2 py-1.5 space-y-1.5">
      <legend className="px-1 text-[11px] text-[var(--color-muted)]">{t("speech.wordNotes")} <Hint text={t("hint.wordNotes")} /></legend>
      <div className="flex flex-wrap items-center gap-1.5">
        <button onMouseDown={(e) => e.preventDefault()} onClick={take} className="px-2 py-1 rounded-md border border-[var(--color-border)] text-[11px] hover:border-[var(--color-accent)]">{t("speech.wordNoteTake")}</button>
        <input value={quote} onChange={(e) => setQuote(e.target.value)} placeholder={t("speech.wordNoteNone")} aria-label={t("speech.wordNoteQuote")} className={`${field} flex-1 min-w-40`} />
      </div>
      <div className="flex flex-wrap items-center gap-1.5">
        <input value={note} onChange={(e) => setNote(e.target.value)} placeholder={t("speech.wordNotePlaceholder")} aria-label={t("speech.wordNotes")} className={`${field} flex-1 min-w-40`} />
        <div role="radiogroup" aria-label={t("speech.wordNoteMarker")} className="flex items-center gap-1">
          <button role="radio" aria-checked={!marker} onClick={() => setMarker("")} className={`w-4 h-4 rounded-sm border ${!marker ? "border-[var(--color-text)]" : "border-[var(--color-border)]"}`} {...named(`${t("speech.wordNoteMarker")}: ${t("speech.markerNone")}`)} />
          {(Object.keys(MARKERS) as Marker[]).map((m) => (
            <button key={m} role="radio" aria-checked={marker === m} onClick={() => setMarker(m)} {...named(`${t("speech.wordNoteMarker")}: ${t(`speech.marker_${m}`)}`)}
              className={`w-4 h-4 rounded-sm border-2 ${marker === m ? "border-[var(--color-text)]" : "border-transparent"}`} style={{ background: MARKERS[m] }} />
          ))}
        </div>
        <button onClick={add} disabled={!quote} className="px-2 py-1 rounded-md bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[11px] font-semibold disabled:opacity-40">{t("speech.wordNoteAdd")}</button>
      </div>
    </fieldset>
  );
}
