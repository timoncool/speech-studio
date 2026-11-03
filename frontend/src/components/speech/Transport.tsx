import { useEffect, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { Pause, PencilLine, Play, RotateCcw, RotateCw, SkipBack, SkipForward, Square, Volume2, VolumeX } from "lucide-react";
import { named } from "../../lib/a11y";
import { masterVolume, setMasterVolume, MASTER_VOLUME_EVENT } from "../../lib/masterVolume";

export const RATES = [0.75, 1, 1.25, 1.5, 2];

export type TransportBlock = { id: string; speaker: string; text: string; duration: number | null };

function clock(s: number): string {
  const m = Math.floor(s / 60);
  return `${m}:${(s - m * 60).toFixed(1).padStart(4, "0")}`;
}

/** The editor's player bar: what plays, where in it, and the controls to move through the project. */
export default function Transport({ blocks, audio, playing, paused, rate, start, tint, onPlay, onPause, onResume, onStop, onSkip, onRate, onShow, onEdit }: {
  /** Every block in reading order; duration is null for a block without a take. */
  blocks: TransportBlock[];
  audio: RefObject<HTMLAudioElement | null>;
  playing: string | null;
  paused: boolean;
  rate: number;
  /** Where ▶ starts when nothing plays: the selected block or the first. */
  start: string | null;
  tint: (speaker: string) => string;
  onPlay: (from: string) => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onSkip: (step: 1 | -1) => void;
  onRate: (rate: number) => void;
  onShow: (id: string) => void;
  /** Stops on the sentence being read and opens it for editing in its block. */
  onEdit: () => void;
}) {
  const { t } = useTranslation();
  const [time, setTime] = useState({ at: 0, of: 0 });
  useEffect(() => {
    if (!playing) { setTime({ at: 0, of: 0 }); return; }
    let frame = 0;
    const tick = () => {
      const a = audio.current;
      if (a) setTime({ at: a.currentTime, of: Number.isFinite(a.duration) ? a.duration : 0 });
      if (!paused) frame = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(frame);
  }, [playing, paused, audio]);

  const [vol, setVol] = useState(masterVolume());
  const beforeMute = useRef(masterVolume() || 1);
  useEffect(() => {
    const onVolume = (e: Event) => setVol((e as CustomEvent<number>).detail);
    window.addEventListener(MASTER_VOLUME_EVENT, onVolume);
    return () => window.removeEventListener(MASTER_VOLUME_EVENT, onVolume);
  }, []);
  const toggleMute = () => {
    if (vol > 0) { beforeMute.current = vol; setMasterVolume(0); }
    else { setMasterVolume(beforeMute.current || 1); }
  };

  const index = playing ? blocks.findIndex((b) => b.id === playing) : -1;
  const current = index >= 0 ? blocks[index] : null;
  const total = blocks.reduce((s, b) => s + (b.duration ?? 0), 0);
  const voiced = blocks.filter((b) => b.duration != null).length;
  const seek = (to: number) => { const a = audio.current; if (a && playing) a.currentTime = Math.max(0, Math.min(to, time.of || to)); };
  const round = "grid place-items-center w-8 h-8 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] hover:bg-[var(--color-surface-2)] disabled:opacity-30";
  const fromSelected = start && blocks[0] && start !== blocks[0].id;

  return (
    <div role="region" aria-label={t("transport.label")} className="shrink-0 border-t border-[var(--color-border)] bg-[var(--color-surface)]/80 px-3 md:px-5 py-2 flex items-center gap-2 md:gap-3">
      <button onClick={() => onSkip(-1)} disabled={index <= 0} className={round} {...named(t("transport.prev"))}><SkipBack size={15} /></button>
      <button onClick={() => (!playing ? start && onPlay(start) : paused ? onResume() : onPause())} disabled={!playing && !start}
        className="grid place-items-center w-9 h-9 rounded-full bg-[var(--color-accent)] text-[var(--color-on-accent)] disabled:opacity-40 shrink-0"
        {...named(!playing ? (fromSelected ? t("transport.fromSelected") : t("transport.fromStart")) : paused ? t("transport.resume") : t("transport.pause"))}>
        {playing && !paused ? <Pause size={15} /> : <Play size={15} />}
      </button>
      <button onClick={() => onSkip(1)} disabled={!playing || index >= blocks.length - 1} className={round} {...named(t("transport.next"))}><SkipForward size={15} /></button>
      <button onClick={() => seek(time.at - 5)} disabled={!playing} className={`${round} max-md:hidden`} {...named(t("transport.back5"))}><RotateCcw size={14} /></button>
      <button onClick={() => seek(time.at + 5)} disabled={!playing} className={`${round} max-md:hidden`} {...named(t("transport.forward5"))}><RotateCw size={14} /></button>
      {current ? (
        <>
          <button onClick={() => onShow(current.id)} className="min-w-0 flex-1 md:flex-none md:w-64 lg:w-80 text-left text-[12px] truncate rounded px-1 hover:bg-[var(--color-surface-2)]" title={current.text}>
            <span className="tabnum text-[var(--color-muted)]">{index + 1}/{blocks.length}</span>{" "}
            <span className="font-semibold" style={{ color: tint(current.speaker) }}>{current.speaker}</span>{" "}
            <span className="text-[var(--color-muted)]">{current.text}</span>
          </button>
          <span className="mono text-[11px] tabnum text-[var(--color-muted)] shrink-0 max-sm:hidden">{clock(time.at)} / {clock(time.of)}</span>
          <input type="range" min={0} max={time.of || 1} step={0.05} value={time.at} onChange={(e) => seek(Number(e.target.value))} aria-label={t("transport.position")}
            className="flex-1 min-w-0 accent-[var(--color-accent)] max-sm:hidden" />
        </>
      ) : (
        <span className="min-w-0 flex-1 text-[12px] text-[var(--color-muted)] truncate tabnum">{t("transport.idle", { voiced, total: blocks.length, length: clock(total) })}</span>
      )}
      <button onClick={toggleMute} className={`${round} shrink-0`} {...named(vol > 0 ? t("transport.mute") : t("transport.unmute"))}>
        {vol > 0 ? <Volume2 size={15} /> : <VolumeX size={15} />}
      </button>
      <input type="range" min={0} max={1} step={0.01} value={vol} onChange={(e) => setMasterVolume(Number(e.target.value))}
        aria-label={t("transport.volume")} className="shrink-0 w-20 accent-[var(--color-accent)] max-md:hidden" />
      <select value={rate} onChange={(e) => onRate(Number(e.target.value))} aria-label={t("transport.rate")}
        className="shrink-0 bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-1.5 py-1 text-[12px] tabnum">
        {RATES.map((r) => <option key={r} value={r}>{r}×</option>)}
      </select>
      {playing && <button onClick={onEdit} className={round} {...named(t("transport.edit"))}><PencilLine size={14} /></button>}
      {playing && <button onClick={onStop} className={round} {...named(t("transport.stop"))}><Square size={14} /></button>}
    </div>
  );
}
