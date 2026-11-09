import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { History, Loader2, Pause, Play, RotateCcw, X } from "lucide-react";
import { speechApi, type HistoryTake } from "../../lib/speech";
import { errorText } from "../../lib/voices";
import { useEngines, voiceName } from "../../lib/engines";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import { Tip } from "../Hint";

type Sort = "created" | "duration" | "rtf";

/** Every take of the project with the parameters it was made with; play one or voice its block again with the same seed. */
export default function HistoryDialog({ pid, level, onRepeat, onClose }: { pid: string; /** The project's loudness preset: takes play as loud as the export. */ level: string; onRepeat: (block: string, seed: number) => void; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  useEscapeLayer(onClose);
  const engines = useEngines();
  const [takes, setTakes] = useState<HistoryTake[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sort, setSort] = useState<Sort>("created");
  const [playing, setPlaying] = useState<string | null>(null);
  const audio = useRef<HTMLAudioElement | null>(null);
  useEffect(() => {
    speechApi.history(pid).then((r) => setTakes(r.takes), (e: unknown) => setError(errorText(e)));
    return () => audio.current?.pause();
  }, [pid]);
  const rows = useMemo(() => [...(takes ?? [])].sort((a, b) => b[sort] - a[sort]), [takes, sort]);
  const play = (r: HistoryTake) => {
    const key = `${r.block}-${r.n}`;
    audio.current?.pause();
    if (playing === key) { setPlaying(null); return; }
    const url = speechApi.takeUrl(pid, r.file, null, undefined, null, level);
    const a = new Audio(`${url}${url.includes("?") ? "&" : "?"}n=${r.n}`);
    audio.current = a;
    a.onended = () => setPlaying(null);
    a.play().then(() => setPlaying(key), (e: unknown) => setError(errorText(e)));
  };
  const head = (key: Sort, label: string) => (
    <th className="px-2 py-1 font-medium text-right">
      <button onClick={() => setSort(key)} className={sort === key ? "text-[var(--color-accent)]" : "hover:text-[var(--color-text)]"}>{label}</button>
    </th>
  );
  return (
    <div className="fixed inset-0 z-[60] grid place-items-center glass-scrim anim-fade" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-labelledby="hist-title" className="w-[min(96vw,1040px)] h-[min(88vh,720px)] flex flex-col rounded-xl glass-panel anim-pop" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-3 px-5 pt-4 pb-3 border-b border-[var(--color-border)]">
          <History size={16} className="text-[var(--color-accent)]" />
          <h2 id="hist-title" className="font-semibold">{t("history.title")}</h2>
          <span className="text-[12px] text-[var(--color-muted)] tabnum">{takes ? t("history.count", { n: takes.length }) : ""}</span>
          <button onClick={onClose} {...named(t("a11y.close"))} className="ml-auto text-[var(--color-muted)] hover:text-[var(--color-text)]"><X size={16} /></button>
        </div>
        {error && <p role="alert" className="mx-5 mt-2 text-[12px] text-[var(--color-warn)]">{error}</p>}
        <div className="flex-1 min-h-0 overflow-auto px-3 pb-3">
          {takes === null ? <Loader2 size={16} className="m-4 animate-spin text-[var(--color-muted)]" /> : !rows.length ? <p className="p-6 text-center text-[13px] text-[var(--color-muted)]">{t("history.empty")}</p> : (
            <table className="w-full text-[12px]">
              <thead className="sticky top-0 bg-[var(--color-surface)] text-[11px] text-[var(--color-muted)]">
                <tr>
                  <th className="px-2 py-1" />
                  <th className="px-2 py-1 font-medium text-left">{t("history.text")}</th>
                  <th className="px-2 py-1 font-medium text-left">{t("history.voice")}</th>
                  <th className="px-2 py-1 font-medium text-left">{t("history.params")}</th>
                  {head("duration", t("history.duration"))}
                  {head("rtf", t("history.rtf"))}
                  <th className="px-2 py-1 font-medium text-right"><Tip text={t("history.firstSoundHint")} className="inline">{t("history.firstSound")}</Tip></th>
                  {head("created", t("history.when"))}
                  <th className="px-2 py-1" />
                </tr>
              </thead>
              <tbody>
                {rows.map((r) => {
                  const key = `${r.block}-${r.n}`;
                  return (
                    <tr key={key} className={`border-t border-[var(--color-border)] ${r.active ? "" : "opacity-80"}`}>
                      <td className="px-2 py-1"><button onClick={() => play(r)} className="p-1 rounded text-[var(--color-muted)] hover:text-[var(--color-text)]" {...named(playing === key ? t("speech.stopPlaying") : t("speech.play"))}>{playing === key ? <Pause size={13} /> : <Play size={13} />}</button></td>
                      <td className="px-2 py-1 max-w-[28ch] truncate" title={r.text}><span className="text-[var(--color-muted)]">#{r.n}{r.active ? " ★" : ""} </span>{r.text}</td>
                      <td className="px-2 py-1 truncate max-w-[16ch]" title={r.voice}>{r.voice ? voiceName(r.voice, engines, r.engine) : t("speech.baseVoice")}</td>
                      <td className="px-2 py-1 mono text-[11px] text-[var(--color-muted)]">{t("history.paramLine", { seed: r.seed, t: r.temperature.toFixed(2), k: r.top_k, p: r.top_p.toFixed(2) })}{r.wer != null ? ` · ${t("speech.takeWer", { wer: Math.round(r.wer * 100) })}` : ""}</td>
                      <td className="px-2 py-1 text-right tabnum">{r.duration.toFixed(1)}{t("settings.sec")}</td>
                      <td className="px-2 py-1 text-right tabnum"><Tip text={t("history.rtfHint")} className="inline">×{r.rtf > 0 ? (1 / r.rtf).toFixed(1) : "—"}</Tip></td>
                      <td className="px-2 py-1 text-right tabnum">{r.first_ms != null ? t("history.ms", { ms: r.first_ms }) : "—"}</td>
                      <td className="px-2 py-1 text-right tabnum text-[var(--color-muted)]">{new Date(r.created * 1000).toLocaleString(i18n.language, { dateStyle: "short", timeStyle: "short" })}</td>
                      <td className="px-2 py-1"><button onClick={() => onRepeat(r.block, r.seed)} className="p-1 rounded text-[var(--color-muted)] hover:text-[var(--color-accent)]" {...named(t("history.repeat"))}><RotateCcw size={13} /></button></td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
