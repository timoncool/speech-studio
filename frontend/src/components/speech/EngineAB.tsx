import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, Pause, Play, Scale, X } from "lucide-react";
import { engineOf, speechApi, voiceOf, type AbSide, type AbVote, type Block, type SpeechProject } from "../../lib/speech";
import { useEngines } from "../../lib/engines";
import { CLOUD_ENGINE_NAMES } from "../../lib/cloudConsent";
import { followJob } from "../../lib/followJob";
import { errorText } from "../../lib/voices";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import VoicePicker from "./VoicePicker";

type Side = { engine: string; voice: string | null };

/** Which take sounds first in the blind comparison: either, by chance. */
function blindOrder(): number[] {
  return Math.random() < 0.5 ? [0, 1] : [1, 0];
}

/** Two engines on one block, heard without their names: a take by each, played as variant 1 and 2 in an order drawn
 * at random; the one picked becomes the block's take, engine and voice, and the names are shown after the choice. */
export default function EngineAB({ pid, project, block, onPick, onReload, onClose }: {
  pid: string; project: SpeechProject; block: Block;
  onPick: (change: Partial<Block>) => void; onReload: () => Promise<void> | void; onClose: () => void;
}) {
  const { t } = useTranslation();
  const engines = (useEngines() ?? []).filter((e) => !(e.id in CLOUD_ENGINE_NAMES) && !e.unavailable);
  const current = engineOf(project, block);
  const [sides, setSides] = useState<Side[] | null>(null);
  const [made, setMade] = useState<AbSide[] | null>(null);
  const [order, setOrder] = useState<number[]>([0, 1]);
  const [vote, setVote] = useState<AbVote | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [playing, setPlaying] = useState<number | null>(null);
  const audio = useRef<HTMLAudioElement | null>(null);
  const stop = () => { audio.current?.pause(); audio.current = null; setPlaying(null); };
  const close = () => { stop(); onClose(); };
  useEscapeLayer(() => { if (!busy) close(); });
  const name = (id: string) => engines.find((e) => e.id === id)?.name ?? id;
  const voiceName = (s: Side) => s.voice ?? voiceOf(project, block) ?? t("speech.baseVoice");
  const fits = (engineId: string, voice: string | null) => {
    const e = engines.find((x) => x.id === engineId);
    const v = voice ?? voiceOf(project, block);
    const lang = block.lang || project.settings.lang;
    if (!e || e.languages.length && lang !== "auto" && !e.languages.includes(lang)) return false;
    const preset = !!v && e.presets.some((p) => p.id === v);
    const library = !!v && !engines.some((x) => x.presets.some((p) => p.id === v));
    return preset || (library && e.clone !== "none") || (!v && !e.needs_voice);
  };
  const shown: Side[] = sides ?? [{ engine: current, voice: null }, { engine: engines.find((e) => e.id !== current && fits(e.id, null))?.id ?? current, voice: null }];
  const unfit = shown.filter((s) => !fits(s.engine, s.voice));
  const setSide = (i: number, change: Partial<Side>) => setSides(shown.map((s, k) => {
    if (k !== i) return s;
    const next = { ...s, ...change };
    if (change.engine && !fits(next.engine, next.voice)) next.voice = engines.find((e) => e.id === next.engine)?.presets[0]?.id ?? null;
    return next;
  }));
  const run = async () => {
    stop();
    setBusy(true);
    setError(null);
    setVote(null);
    try {
      const r = (await followJob((await speechApi.ab(pid, block.id, shown)).job_id, "synth")) as { takes: number[] };
      await onReload();
      setMade(shown.map((s, i) => ({ ...s, take: r.takes[i] })));
      setOrder(blindOrder());
    } catch (e) { setError(errorText(e)); } finally { setBusy(false); }
  };
  const play = (side: number) => {
    if (playing === side) { stop(); return; }
    stop();
    const take = block.takes.find((tk) => tk.n === made?.[side].take);
    if (!take) return;
    const el = new Audio(speechApi.takeUrl(pid, take.file));
    el.onended = () => setPlaying(null);
    audio.current = el;
    setPlaying(side);
    void el.play().catch((e: unknown) => { setError(errorText(e)); setPlaying(null); });
  };
  const choose = (picked: number | null) => {
    if (!made) return;
    stop();
    const v: AbVote = { sides: made, picked };
    setVote(v);
    const ab = [...(block.ab ?? []), v];
    onPick(picked === null ? { ab } : { ab, engine: made[picked].engine, voice: made[picked].voice ?? block.voice, active: made[picked].take });
  };
  const tally = useMemo(() => {
    const wins = new Map<string, number>();
    let ties = 0;
    for (const b of project.chapters.flatMap((c) => c.blocks)) {
      for (const v of b.ab ?? []) {
        if (v.picked === null) ties += 1;
        else { const e = v.sides[v.picked]?.engine; if (e) wins.set(e, (wins.get(e) ?? 0) + 1); }
      }
    }
    return { wins: [...wins.entries()].sort((a, b) => b[1] - a[1]), ties };
  }, [project]);
  const pick = "w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]";
  return (
    <div className="fixed inset-0 z-[60] grid place-items-center glass-scrim anim-fade p-4" onClick={() => { if (!busy) close(); }}>
      <div role="dialog" aria-modal="true" aria-labelledby="ab-title" onClick={(e) => e.stopPropagation()} className="w-[min(94vw,600px)] max-h-[88vh] overflow-y-auto rounded-xl glass-panel anim-pop p-5 space-y-3">
        <div className="flex items-center gap-2">
          <h2 id="ab-title" className="font-semibold flex items-center gap-2"><Scale size={16} className="text-[var(--color-accent)]" />{t("ab.title")}</h2>
          <button onClick={close} disabled={busy} className="ml-auto p-1.5 rounded-md text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30" {...named(t("a11y.close"))}><X size={16} /></button>
        </div>
        <p className="text-[12px] text-[var(--color-muted)] leading-relaxed">{t("ab.hint")}</p>
        <p className="text-[12px] rounded-md bg-[var(--color-surface-2)] px-2 py-1.5 line-clamp-3">{block.text}</p>
        {!made && (
          <>
            <div className="grid gap-3 sm:grid-cols-2">
              {shown.map((s, i) => (
                <fieldset key={i} className="space-y-1.5 rounded-lg border border-[var(--color-border)] p-2.5">
                  <legend className="px-1 text-[11px] text-[var(--color-muted)]">{t("ab.engineN", { n: i + 1 })}</legend>
                  <select value={s.engine} onChange={(e) => setSide(i, { engine: e.target.value })} aria-label={t("ab.engineN", { n: i + 1 })} className={pick}>
                    {engines.map((e) => <option key={e.id} value={e.id}>{e.name}</option>)}
                  </select>
                  <VoicePicker engine={s.engine} value={s.voice} onChange={(v) => setSide(i, { voice: v })} inherit={t("ab.voiceAsBlock")} label={t("ab.voiceN", { n: i + 1 })} className={pick} />
                </fieldset>
              ))}
            </div>
            {shown[0].engine === shown[1].engine && shown[0].voice === shown[1].voice && <p className="text-[12px] text-[var(--color-warn)]">{t("ab.same")}</p>}
            {unfit.map((s) => <p key={s.engine} className="text-[12px] text-[var(--color-warn)]">{t("ab.needsVoice", { engine: name(s.engine) })}</p>)}
            <button onClick={() => void run()} disabled={busy || !engines.length || unfit.length > 0} className="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[13px] font-semibold disabled:opacity-40">
              {busy ? <Loader2 size={14} className="animate-spin" /> : <Scale size={14} />}{t("ab.make")}
            </button>
          </>
        )}
        {made && !vote && (
          <>
            <div className="grid gap-2 sm:grid-cols-2">
              {order.map((side, k) => (
                <button key={side} onClick={() => play(side)} className="inline-flex items-center justify-center gap-2 px-4 py-3 rounded-lg border border-[var(--color-border)] hover:border-[var(--color-accent)] text-[13px] font-medium">
                  {playing === side ? <Pause size={15} /> : <Play size={15} />}{t("ab.variant", { n: k + 1 })}
                </button>
              ))}
            </div>
            <div className="flex flex-wrap gap-2">
              {order.map((side, k) => <button key={side} onClick={() => choose(side)} className="px-3 py-1.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[12px] font-semibold">{t("ab.better", { n: k + 1 })}</button>)}
              <button onClick={() => choose(null)} className="px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]">{t("ab.tie")}</button>
            </div>
          </>
        )}
        {made && vote && (
          <div className="space-y-1.5 text-[12px]" aria-live="polite">
            {order.map((side, k) => (
              <p key={side} className={vote.picked === side ? "font-semibold" : "text-[var(--color-muted)]"}>{t("ab.reveal", { n: k + 1, engine: name(made[side].engine), voice: voiceName(made[side]) })}</p>
            ))}
            <p>{vote.picked === null ? t("ab.keptTie") : t("ab.kept", { engine: name(made[vote.picked].engine) })}</p>
            <button onClick={() => { setMade(null); setVote(null); }} className="px-3 py-1.5 rounded-lg border border-[var(--color-border)] hover:border-[var(--color-accent)]">{t("ab.again")}</button>
          </div>
        )}
        {error && <p role="alert" className="text-[12px] text-[var(--color-warn)] break-words">{error}</p>}
        {(tally.wins.length > 0 || tally.ties > 0) && (
          <p className="text-[11px] text-[var(--color-muted)] border-t border-[var(--color-border)] pt-2">
            {t("ab.tally")} {[...tally.wins.map(([e, n]) => `${name(e)} — ${n}`), tally.ties ? t("ab.ties", { n: tally.ties }) : ""].filter(Boolean).join(" · ")}
          </p>
        )}
      </div>
    </div>
  );
}
