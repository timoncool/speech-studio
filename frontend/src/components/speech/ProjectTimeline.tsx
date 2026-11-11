import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { FileAudio, FolderOpen, Minus, Music, Play, Plus, SlidersHorizontal, Trash2, Volume2, VolumeX, X } from "lucide-react";
import { FLAT_MIX, MIX_GAIN_DB, speechApi, type Mix, type TimelineBlock, type Track } from "../../lib/speech";
import { errorText } from "../../lib/voices";
import { named } from "../../lib/a11y";
import { Tip } from "../Hint";
import { pageZoom } from "../../lib/uiScale";

export type OverlapChoice = "cut" | "join" | "keep";
/** What the track editor changed on a clip: the part of the take kept, its gain and fades, or where it sounds — the
 * pause after the block before it (`prev`), or with cue times a shift of its cue. */
export type Montage = { cut?: [number, number] | null; mix?: Mix; pause?: { prev: string; ms: number }; shift?: number };

const ZOOMS = [4, 8, 16, 32, 64, 128, 256] as const;
const LANE = 28;
const LANE_EDIT = 60;
const RULER = 16;
/** Edges of a clip stick to the end of the reply before within this many seconds (Dub Studio's timeline). */
const SNAP = 0.15;
const MIN_CLIP = 0.1;
/** The pause after a block the project takes, ms (`speech_project` validates it). */
const PAUSE_MS = [-2000, 10_000] as const;
/** Dragging the gain line moves it a decibel every this many pixels, whatever the clip's height. */
const PX_PER_DB = 3;
/** Where the gain line sits in a clip `height` px high: 0 dB in the middle, following the pointer, held inside the clip. */
const lineY = (gain: number, height: number) => Math.max(6, Math.min(94, 50 - (gain * PX_PER_DB * 100) / height));
const [GAIN_MIN, GAIN_MAX] = MIX_GAIN_DB;
const round = (x: number, step: number) => Number((Math.round(x / step) * step).toFixed(3));

/** A clip on the scale: a block's take, or with `track` a sound of the book's track (`file` in the project's media). */
type Row = TimelineBlock & { track?: string; file?: string; label?: string };
/** What the editor changed on a clip of a book's track. */
export type ClipChange = { start?: number; cut?: [number, number] | null; mix?: Mix };
const trackLane = (id: string) => `\u0000track:${id}`;
const peakKey = (r: Row) => (r.track ? (r.file ? `media:${r.file}` : null) : r.take);
const TRACK_TINT = "var(--color-accent-2)";

type Drag = { block: string; kind: "move" | "left" | "right" | "fin" | "fout" | "gain"; x0: number; y0: number; moved: boolean; from: Live };
type Live = { start: number; cut: [number, number]; mix: Mix };

/** The project on one time scale, as the export lays it out: a lane per speaker, takes as bars (an estimate while a block
 * is not voiced), an interruption striped where it covers the previous reply, with the choice to cut that reply off, join
 * the two back to back or keep both. «Montage» turns it into a track editor: the clip's waveform, its edges trimmed, its
 * fades and gain dragged, the clip moved along the time, everything kept in the project and the take left as it is. */
export default function ProjectTimeline({ pid, speakers, tintOf, textOf, refresh, cueTimed, tracks, onChoose, onJump, onMontage, onPlay, onTrackClip, onTrack, onAddFiles, onMusic, onPlayMedia, onClose }: {
  pid: string;
  /** A block's text, shown inside its clip as far as it fits. */
  textOf: (block: string) => string;
  speakers: string[];
  tintOf: (speaker: string) => string;
  /** Changes when the saved project changes; `undefined` while a save is on its way. */
  refresh: unknown;
  /** Blocks sound at their cue times (a project from subtitles) rather than one after another. */
  cueTimed: boolean;
  /** `previous` is the block the interruption covers: joining zeroes its pause after. */
  onChoose: (block: string, previous: string, choice: OverlapChoice) => void;
  onJump: (block: string) => void;
  onMontage: (block: string, change: Montage) => void;
  onPlay: (block: string) => void;
  /** The book's or the play's own tracks, under the voices. */
  tracks: Track[];
  onTrackClip: (track: string, clip: string, change: ClipChange) => void;
  onTrack: (track: string, change: { gain_db?: number; muted?: boolean; remove?: boolean }) => void;
  /** Sound files onto a track, or a new one when `track` is null. */
  onAddFiles: (files: File[], track: string | null) => void;
  onMusic: () => void;
  onPlayMedia: (file: string, cut: [number, number]) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const [rows, setRows] = useState<TimelineBlock[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [zoom, setZoom] = useState(2);
  const [open, setOpen] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [peaks, setPeaks] = useState<Record<string, { per_second: number; peaks: number[] }>>({});
  const [drag, setDrag] = useState<Drag | null>(null);
  const [live, setLive] = useState<{ block: string; value: Live } | null>(null);
  const scroller = useRef<HTMLDivElement | null>(null);
  const asked = useRef(new Set<string>());
  const filesInput = useRef<HTMLInputElement | null>(null);
  const folderInput = useRef<HTMLInputElement | null>(null);
  const [target, setTarget] = useState<string | null>(null);
  const trackRows = useMemo<Row[]>(() => tracks.flatMap((track) => track.clips.map((c) => {
    const [a, z] = c.cut ?? [0, c.duration];
    return { block: c.id, speaker: trackLane(track.id), start: c.start, end: c.start + (z - a), voiced: true, overlap: null, cut_previous: false, prev: null, take: null, take_secs: c.duration, cut: c.cut, mix: c.mix ?? FLAT_MIX, track: track.id, file: c.file, label: c.label };
  })), [tracks]);
  const all = useMemo<Row[]>(() => [...(rows ?? []), ...trackRows], [rows, trackRows]);
  const pick = (folder: boolean, track: string | null) => { setTarget(track); (folder ? folderInput : filesInput).current?.click(); };
  const picked = (list: FileList | null) => {
    const files = [...(list ?? [])].filter((f) => f.type.startsWith("audio/") || /\.(wav|mp3|flac|ogg|opus|m4a|aac|wma|aiff?)$/i.test(f.name));
    if (files.length) onAddFiles(files, target);
  };
  useEffect(() => {
    if (refresh === undefined) return;
    let alive = true;
    speechApi.timeline(pid).then((r) => { if (alive) { setRows(r.blocks); setError(null); setLive(null); } }, (e: unknown) => { if (alive) setError(errorText(e)); });
    return () => { alive = false; };
  }, [pid, refresh]);
  useEffect(() => {
    if (!editing) return;
    let alive = true;
    const wanted = [...new Set(all.map(peakKey).filter((f): f is string => !!f && !asked.current.has(f)))];
    wanted.forEach((f) => asked.current.add(f));
    void (async () => {
      for (const file of wanted) {
        try {
          const got = file.startsWith("media:") ? await speechApi.mediaPeaks(pid, file.slice(6)) : await speechApi.takePeaks(pid, file);
          setPeaks((p) => ({ ...p, [file]: { per_second: got.per_second, peaks: got.peaks } }));
        } catch (e) { asked.current.delete(file); if (alive) setError(errorText(e)); return; }
      }
    })();
    return () => { alive = false; };
  }, [editing, all, pid]);
  const lanes = useMemo(() => {
    const seen = [...speakers];
    for (const r of rows ?? []) if (!seen.includes(r.speaker)) seen.push(r.speaker);
    return [...seen.filter((s) => rows?.some((r) => r.speaker === s)), ...tracks.map((tr) => trackLane(tr.id))];
  }, [rows, speakers, tracks]);
  const pps = ZOOMS[zoom];
  const lane = editing ? LANE_EDIT : LANE;
  const total = Math.max(1, ...all.map((r) => r.end));
  const overlaps = (rows ?? []).filter((r) => r.overlap);
  const chosen = rows?.find((r) => r.block === open);
  const clock = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
  const ticks = Math.max(1, [1, 5, 10, 30, 60, 300].find((step) => step * pps >= 60) ?? 600);

  const valueOf = (r: Row): Live => live?.block === r.block ? live.value : { start: r.start, cut: r.cut ?? [0, r.take_secs ?? r.end - r.start], mix: r.mix ?? FLAT_MIX };
  const commit = (r: Row, v: Live, kind: Drag["kind"]) => {
    if (r.track) {
      const whole = r.take_secs ?? v.cut[1];
      const cut: [number, number] = [round(v.cut[0], 0.01), round(v.cut[1], 0.01)];
      if (kind === "move") onTrackClip(r.track, r.block, { start: round(v.start, 0.01) });
      else if (kind === "left" || kind === "right") onTrackClip(r.track, r.block, { start: round(v.start, 0.01), cut: cut[0] <= 0.005 && cut[1] >= whole - 0.005 ? null : cut });
      else onTrackClip(r.track, r.block, { mix: v.mix });
      return;
    }
    if (kind === "move") {
      const shift = round(v.start - r.start, 0.01);
      if (Math.abs(shift) < 0.005) return;
      if (cueTimed) { onMontage(r.block, { shift }); return; }
      const before = rows?.find((x) => x.block === r.prev);
      if (!before) return;
      onMontage(r.block, { pause: { prev: before.block, ms: Math.max(PAUSE_MS[0], Math.min(PAUSE_MS[1], Math.round((v.start - before.end) * 1000))) } });
      return;
    }
    if (kind === "left" || kind === "right") {
      const whole = r.take_secs ?? v.cut[1];
      const cut: [number, number] = [round(v.cut[0], 0.01), round(v.cut[1], 0.01)];
      onMontage(r.block, { cut: cut[0] <= 0.005 && cut[1] >= whole - 0.005 ? null : cut });
      return;
    }
    const was = r.mix ?? FLAT_MIX;
    if (v.mix.gain_db !== was.gain_db || v.mix.fade_in !== was.fade_in || v.mix.fade_out !== was.fade_out) onMontage(r.block, { mix: v.mix });
  };
  const change = (r: Row, kind: Drag["kind"], dx: number, dy: number, from: Live): Live => {
    const dt = dx / pps;
    const [a, z] = from.cut;
    const length = z - a;
    const whole = r.take_secs ?? z;
    const mix = from.mix;
    switch (kind) {
      case "move": {
        let start = Math.max(0, from.start + dt);
        const before = rows?.find((x) => x.block === r.prev);
        if (before && Math.abs(start - before.end) < SNAP) start = before.end;
        return { ...from, start };
      }
      case "left": {
        const na = Math.min(z - MIN_CLIP, Math.max(0, a + dt));
        return { ...from, cut: [na, z], start: r.track ? from.start + (na - a) : from.start, mix: { ...mix, fade_in: Math.min(mix.fade_in, z - na), fade_out: Math.min(mix.fade_out, z - na) } };
      }
      case "right": {
        const nz = Math.max(a + MIN_CLIP, Math.min(whole, z + dt));
        return { ...from, cut: [a, nz], mix: { ...mix, fade_in: Math.min(mix.fade_in, nz - a), fade_out: Math.min(mix.fade_out, nz - a) } };
      }
      case "fin": return { ...from, mix: { ...mix, fade_in: round(Math.max(0, Math.min(length - mix.fade_out, mix.fade_in + dt)), 0.01) } };
      case "fout": return { ...from, mix: { ...mix, fade_out: round(Math.max(0, Math.min(length - mix.fade_in, mix.fade_out - dt)), 0.01) } };
      case "gain": return { ...from, mix: { ...mix, gain_db: round(Math.max(GAIN_MIN, Math.min(GAIN_MAX, mix.gain_db - dy / PX_PER_DB)), 0.5) } };
    }
  };
  const begin = (e: React.PointerEvent, r: Row, kind: Drag["kind"]) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const from = valueOf(r);
    setDrag({ block: r.block, kind, x0: e.clientX, y0: e.clientY, moved: false, from });
    setLive({ block: r.block, value: from });
  };
  const move = (e: React.PointerEvent, r: Row) => {
    if (!drag || drag.block !== r.block) return;
    const dx = (e.clientX - drag.x0) / pageZoom();
    const dy = (e.clientY - drag.y0) / pageZoom();
    if (!drag.moved && Math.abs(dx) < 3 && Math.abs(dy) < 3) return;
    if (!drag.moved) setDrag({ ...drag, moved: true });
    setLive({ block: r.block, value: change(r, drag.kind, dx, dy, drag.from) });
  };
  const end = (e: React.PointerEvent, r: Row) => {
    if (!drag || drag.block !== r.block) return;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    const kind = drag.kind;
    setDrag(null);
    if (!drag.moved) { setLive(null); if (kind === "move" && !r.track) onJump(r.block); return; }
    if (live) commit(r, live.value, kind);
  };
  const nudge = (e: React.KeyboardEvent, r: Row, kind: Drag["kind"]) => {
    const step = kind === "gain" ? { ArrowUp: 1, ArrowDown: -1 }[e.key as "ArrowUp" | "ArrowDown"] : { ArrowLeft: -1, ArrowRight: 1 }[e.key as "ArrowLeft" | "ArrowRight"];
    if (step === undefined) return;
    e.preventDefault();
    e.stopPropagation();
    const from = valueOf(r);
    const next = kind === "gain"
      ? { ...from, mix: { ...from.mix, gain_db: Math.max(GAIN_MIN, Math.min(GAIN_MAX, from.mix.gain_db + step * 0.5)) } }
      : change(r, kind, step * 0.05 * pps, 0, from);
    setLive({ block: r.block, value: next });
    commit(r, next, kind);
  };
  const reset = (r: Row, kind: "fin" | "fout" | "gain" | "trim") => {
    const v = valueOf(r);
    if (kind === "trim") {
      if (r.track) onTrackClip(r.track, r.block, { start: round(v.start - v.cut[0], 0.01), cut: null });
      else onMontage(r.block, { cut: null });
      return;
    }
    const mix = { ...v.mix, ...(kind === "fin" ? { fade_in: 0 } : kind === "fout" ? { fade_out: 0 } : { gain_db: 0 }) };
    setLive({ block: r.block, value: { ...v, mix } });
    if (r.track) onTrackClip(r.track, r.block, { mix });
    else onMontage(r.block, { mix });
  };
  const wheel = (e: React.WheelEvent) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    e.preventDefault();
    setZoom((z) => Math.max(0, Math.min(ZOOMS.length - 1, z + (e.deltaY < 0 ? 1 : -1))));
  };
  const readout = drag?.moved && live ? (() => {
    const v = live.value;
    switch (drag.kind) {
      case "fin": return t("timeline.fadeIn", { secs: v.mix.fade_in.toFixed(2) });
      case "fout": return t("timeline.fadeOut", { secs: v.mix.fade_out.toFixed(2) });
      case "gain": return t("timeline.gain", { db: `${v.mix.gain_db > 0 ? "+" : ""}${v.mix.gain_db.toFixed(1)}` });
      case "left": case "right": return t("timeline.length", { secs: (v.cut[1] - v.cut[0]).toFixed(2) });
      case "move": return clock(v.start) + `.${String(Math.round((v.start % 1) * 100)).padStart(2, "0")}`;
    }
  })() : null;

  return (
    <section aria-label={t("timeline.title")} className="border-b border-[var(--color-border)] bg-[var(--color-surface)]/60 px-5 py-2 space-y-1.5">
      <div className="flex items-center gap-2 text-[12px]">
        <span className="font-semibold">{t("timeline.title")}</span>
        <span className="text-[11px] text-[var(--color-muted)] tabnum">{clock(total)}{overlaps.length ? ` · ${t("timeline.overlaps", { n: overlaps.length })}` : ""}</span>
        {readout && <span className="text-[11px] tabnum text-[var(--color-accent)]">{readout}</span>}
        <div className="ml-auto flex items-center gap-1">
          {editing && (
            <>
              <Tip text={t("tracks.addFilesHint")}><button onClick={() => pick(false, null)} className="inline-flex items-center gap-1 px-2 py-1 rounded-md border border-[var(--color-border)] text-[11px] text-[var(--color-muted)] hover:text-[var(--color-text)]"><FileAudio size={12} />{t("tracks.addFiles")}</button></Tip>
              <Tip text={t("tracks.addFolderHint")}><button onClick={() => pick(true, null)} className="inline-flex items-center gap-1 px-2 py-1 rounded-md border border-[var(--color-border)] text-[11px] text-[var(--color-muted)] hover:text-[var(--color-text)]"><FolderOpen size={12} />{t("tracks.addFolder")}</button></Tip>
              <Tip text={t("tracks.musicHint")}><button onClick={onMusic} className="inline-flex items-center gap-1 px-2 py-1 rounded-md border border-[var(--color-border)] text-[11px] text-[var(--color-muted)] hover:text-[var(--color-text)]"><Music size={12} />{t("tracks.music")}</button></Tip>
              <input ref={filesInput} type="file" multiple accept="audio/*" hidden onChange={(e) => { picked(e.target.files); e.target.value = ""; }} />
              <input ref={folderInput} type="file" multiple hidden {...{ webkitdirectory: "" }} onChange={(e) => { picked(e.target.files); e.target.value = ""; }} />
            </>
          )}
          <Tip text={t("timeline.montageHint")}><button onClick={() => setEditing((on) => !on)} aria-pressed={editing}
            className={`inline-flex items-center gap-1 px-2 py-1 rounded-md border text-[11px] ${editing ? "border-[var(--color-accent)] text-[var(--color-accent)]" : "border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>
            <SlidersHorizontal size={12} />{t("timeline.montage")}
          </button></Tip>
          <button onClick={() => setZoom((z) => Math.max(0, z - 1))} disabled={zoom === 0} className="p-1 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30" {...named(t("timeline.zoomOut"))}><Minus size={13} /></button>
          <button onClick={() => setZoom((z) => Math.min(ZOOMS.length - 1, z + 1))} disabled={zoom === ZOOMS.length - 1} className="p-1 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30" {...named(t("timeline.zoomIn"))}><Plus size={13} /></button>
          <button onClick={onClose} className="p-1 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t("a11y.close"))}><X size={13} /></button>
        </div>
      </div>
      {editing && <p className="text-[11px] text-[var(--color-muted)]">{t("timeline.montageHelp")}</p>}
      {error && <p role="alert" className="text-[12px] text-[var(--color-warn)]">{error}</p>}
      {rows && (
        <div className="flex">
          <div className={`shrink-0 ${editing ? "w-36" : "w-24"}`} style={{ paddingTop: RULER }}>
            {lanes.map((s) => {
              const track = tracks.find((tr) => trackLane(tr.id) === s);
              if (!track) return <div key={s} style={{ height: lane, lineHeight: `${lane}px`, color: tintOf(s) }} className="text-[11px] font-semibold truncate pr-2" title={s}>{s}</div>;
              return (
                <div key={s} style={{ height: lane }} className={`pr-2 flex flex-col justify-center gap-0.5 ${track.muted ? "opacity-50" : ""}`}>
                  <span className="text-[11px] font-semibold truncate" style={{ color: TRACK_TINT }} title={track.name}>{track.name}</span>
                  {editing && (
                    <span className="flex items-center gap-1">
                      <button onClick={() => onTrack(track.id, { muted: !track.muted })} aria-pressed={track.muted} className="p-0.5 rounded text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t(track.muted ? "tracks.unmute" : "tracks.mute"))}>{track.muted ? <VolumeX size={12} /> : <Volume2 size={12} />}</button>
                      <Tip text={t("tracks.gainHint")}><input type="number" value={track.gain_db} step={0.5} min={GAIN_MIN} max={GAIN_MAX} onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) onTrack(track.id, { gain_db: Math.max(GAIN_MIN, Math.min(GAIN_MAX, v)) }); }}
                        className="w-12 bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded px-1 text-[10px] tabnum" aria-label={t("tracks.gain")} /></Tip>
                      <button onClick={() => pick(false, track.id)} className="p-0.5 rounded text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(t("tracks.addHere"))}><Plus size={12} /></button>
                      <button onClick={() => onTrack(track.id, { remove: true })} className="p-0.5 rounded text-[var(--color-muted)] hover:text-[var(--color-warn)]" {...named(t("tracks.remove"))}><Trash2 size={12} /></button>
                    </span>
                  )}
                </div>
              );
            })}
          </div>
          <div ref={scroller} onWheel={wheel} className="relative flex-1 min-w-0 overflow-x-auto pb-1">
            <div className="relative" style={{ width: total * pps + 40, height: RULER + lanes.length * lane }}>
              {Array.from({ length: Math.floor(total / ticks) + 1 }, (_, k) => (
                <span key={k} className="absolute top-0 text-[9px] tabnum text-[var(--color-muted)] border-l border-[var(--color-border)] pl-0.5 h-full" style={{ left: k * ticks * pps }}>{clock(k * ticks)}</span>
              ))}
              {all.map((r) => {
                const top = RULER + lanes.indexOf(r.speaker) * lane + 4;
                const v = valueOf(r);
                const length = editing && r.voiced ? v.cut[1] - v.cut[0] : r.end - r.start;
                const left = (editing ? v.start : r.start) * pps;
                const width = Math.max(2, length * pps);
                const height = lane - 8;
                const tint = r.track ? TRACK_TINT : tintOf(r.speaker);
                const key = peakKey(r);
                const wave = key ? peaks[key] : undefined;
                const owner = r.track ? tracks.find((tr) => tr.id === r.track) : undefined;
                const who = owner ? owner.name : r.speaker;
                return (
                  <div key={r.block}>
                    {editing && r.voiced ? (
                      <div role="group" aria-label={t("timeline.block", { speaker: who, at: clock(v.start) })}
                        onPointerDown={(e) => begin(e, r, "move")} onPointerMove={(e) => move(e, r)} onPointerUp={(e) => end(e, r)}
                        className={`absolute rounded-[4px] border overflow-hidden select-none touch-none ${owner?.muted ? "opacity-40" : ""} ${drag?.block === r.block && drag.kind === "move" ? "cursor-grabbing z-10" : "cursor-grab"}`}
                        style={{ left, width, top, height, borderColor: tint, background: `color-mix(in oklab, ${tint} 22%, transparent)` }}>
                        {wave && (
                          <svg aria-hidden className="absolute inset-0 w-full h-full pointer-events-none" viewBox={`0 0 ${Math.max(1, Math.round(length * wave.per_second))} 2`} preserveAspectRatio="none">
                            {wave.peaks.slice(Math.floor(v.cut[0] * wave.per_second), Math.ceil(v.cut[1] * wave.per_second)).map((p, i) => {
                              const h = Math.min(1, p * 10 ** (v.mix.gain_db / 20));
                              return <rect key={i} x={i} y={1 - h} width={0.8} height={Math.max(0.02, 2 * h)} fill={tint} opacity={0.75} />;
                            })}
                          </svg>
                        )}
                        <svg aria-hidden className="absolute inset-0 w-full h-full pointer-events-none" viewBox="0 0 1000 100" preserveAspectRatio="none">
                          {v.mix.fade_in > 0 && <path d={`M0 0 L${(v.mix.fade_in / length) * 1000} 0 L0 100 Z`} fill="rgba(0,0,0,0.5)" />}
                          {v.mix.fade_out > 0 && <path d={`M${1000 - (v.mix.fade_out / length) * 1000} 0 L1000 0 L1000 100 Z`} fill="rgba(0,0,0,0.5)" />}
                        </svg>
                        <span aria-hidden className="absolute left-2.5 right-2.5 top-2 text-[10px] leading-[14px] text-[var(--color-text)] truncate pointer-events-none">{r.track ? r.label : textOf(r.block)}</span>
                        <button onPointerDown={(e) => e.stopPropagation()} onClick={(e) => { e.stopPropagation(); if (r.track && r.file) onPlayMedia(r.file, v.cut); else onPlay(r.block); }}
                          className="absolute left-2.5 bottom-1 p-0.5 rounded bg-[var(--color-accent)] text-[var(--color-on-accent)]" {...named(t("timeline.listen"))}><Play size={9} /></button>
                        <div onPointerDown={(e) => begin(e, r, "gain")} onPointerMove={(e) => move(e, r)} onPointerUp={(e) => end(e, r)}
                          onDoubleClick={(e) => { e.stopPropagation(); reset(r, "gain"); }} onKeyDown={(e) => nudge(e, r, "gain")}
                          role="slider" tabIndex={0} aria-label={t("timeline.gainHandle")} aria-valuemin={GAIN_MIN} aria-valuemax={GAIN_MAX} aria-valuenow={v.mix.gain_db} aria-valuetext={`${v.mix.gain_db} dB`}
                          title={t("timeline.gainHandleHint")} className="absolute left-3 right-3 h-[7px] -translate-y-1/2 cursor-ns-resize flex items-center focus-visible:outline focus-visible:outline-1 focus-visible:outline-[var(--color-accent)]"
                          style={{ top: `${lineY(v.mix.gain_db, height)}%` }}>
                          <span className="block w-full h-px" style={{ background: v.mix.gain_db === 0 ? "rgba(255,255,255,0.45)" : "var(--color-accent)" }} />
                        </div>
                        {(["fin", "fout"] as const).map((k) => (
                          <button key={k} onPointerDown={(e) => begin(e, r, k)} onPointerMove={(e) => move(e, r)} onPointerUp={(e) => end(e, r)}
                            onDoubleClick={(e) => { e.stopPropagation(); reset(r, k); }} onKeyDown={(e) => nudge(e, r, k)}
                            className="absolute top-0 h-2.5 w-2.5 -translate-x-1/2 rounded-[2px] border border-slate-950/70 bg-white cursor-ew-resize touch-none"
                            style={{ left: `${k === "fin" ? (v.mix.fade_in / length) * 100 : 100 - (v.mix.fade_out / length) * 100}%` }}
                            title={t(k === "fin" ? "timeline.fadeInHandleHint" : "timeline.fadeOutHandleHint")} aria-label={t(k === "fin" ? "timeline.fadeInHandle" : "timeline.fadeOutHandle")} />
                        ))}
                        {(["left", "right"] as const).map((k) => (
                          <button key={k} onPointerDown={(e) => begin(e, r, k)} onPointerMove={(e) => move(e, r)} onPointerUp={(e) => end(e, r)}
                            onDoubleClick={(e) => { e.stopPropagation(); reset(r, "trim"); }} onKeyDown={(e) => nudge(e, r, k)}
                            className={`absolute top-3 bottom-0 w-1.5 cursor-col-resize bg-white/15 hover:bg-[var(--color-accent)] ${k === "left" ? "left-0" : "right-0"}`}
                            title={t("timeline.trimHint")} aria-label={t(k === "left" ? "timeline.trimStart" : "timeline.trimEnd")} />
                        ))}
                      </div>
                    ) : (
                      <button onClick={() => { if (!r.track) onJump(r.block); }} title={`${who} · ${clock(r.start)}–${clock(r.end)}${r.voiced ? "" : ` · ${t("timeline.estimate")}`}`}
                        className={`absolute rounded-[4px] border ${r.voiced ? "" : "border-dashed"} ${owner?.muted ? "opacity-40" : ""} hover:brightness-125`}
                        style={{ left, width, top, height, borderColor: tint, background: r.voiced ? `color-mix(in oklab, ${tint} 35%, transparent)` : "transparent" }}
                        aria-label={t("timeline.block", { speaker: who, at: clock(r.start) })}>
                        <span aria-hidden className="block px-1 text-[10px] leading-[20px] text-left text-[var(--color-text)] truncate pointer-events-none">{r.track ? r.label : textOf(r.block)}</span>
                      </button>
                    )}
                    {r.overlap && !editing && (
                      <button onClick={() => setOpen(open === r.block ? null : r.block)} aria-expanded={open === r.block} aria-controls="timeline-overlap"
                          className={`absolute rounded-[4px] border z-10 ${open === r.block ? "border-[var(--color-text)]" : "border-[var(--color-warn)]"}`}
                          style={{ left: r.start * pps, width: Math.max(4, r.overlap.secs * pps), top: top - 2, height: lane - 4, background: "repeating-linear-gradient(135deg, color-mix(in oklab, var(--color-warn) 45%, transparent) 0 4px, transparent 4px 8px)" }}
                          aria-label={t("timeline.overlap", { secs: r.overlap.secs.toFixed(2) })} title={t("timeline.overlap", { secs: r.overlap.secs.toFixed(2) })} />
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
      {chosen?.overlap && !editing && (
        <div id="timeline-overlap" role="group" aria-label={t("timeline.overlap", { secs: chosen.overlap.secs.toFixed(2) })} className="flex flex-wrap items-center gap-1.5 text-[12px]">
          <span className="text-[var(--color-warn)] tabnum">{t("timeline.overlap", { secs: chosen.overlap.secs.toFixed(2) })}</span>
          {(["cut", "join", "keep"] as const).map((c) => {
            const current = c === "cut" ? chosen.cut_previous : c === "keep" ? !chosen.cut_previous : false;
            return (
              <button key={c} onClick={() => { if (chosen.overlap) onChoose(chosen.block, chosen.overlap.with, c); setOpen(null); }} aria-pressed={current} title={t(`timeline.choiceHint_${c}`)}
                className={`px-2.5 py-1 rounded-md border text-[12px] ${current ? "border-[var(--color-accent)] text-[var(--color-accent)]" : "border-[var(--color-border)] hover:border-[var(--color-accent)]"}`}>
                {t(`timeline.choice_${c}`)}
              </button>
            );
          })}
        </div>
      )}
    </section>
  );
}
