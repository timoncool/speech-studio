import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Download, Loader2, Pause, Play, X } from "lucide-react";
import { api } from "../../lib/api";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import { DUB_LANGS } from "../../lib/i18n";
import { errorText, prettyVoice } from "../../lib/voices";

type Entry = {
  name: string;
  file?: string;
  displayName?: string;
  gender: string;
  age_group?: string | null;
  language?: string | null;
  description?: string | null;
  duration_s?: number | null;
  birth_year?: number | null;
  url: string;
  txt_size?: number | null;
};

const langName = (code: string) => DUB_LANGS.find((l) => l.code === code)?.name ?? code.toUpperCase();
const DURATIONS = [0, 10, 30, 60] as const;

/** The cloud voice catalogue (nerualdreming/russia_voices): load the list with its metadata, filter and search,
 * listen, pick, and download the picked or everything the filters show (its sample transcript comes with it). */
export default function VoiceCatalog({ have, onClose, onGot }: { have: Set<string>; onClose: () => void; onGot: () => void }) {
  const { t } = useTranslation();
  const [all, setAll] = useState<Entry[] | null>(null);
  const [q, setQ] = useState("");
  const [gender, setGender] = useState<"all" | "female" | "male">("all");
  const [age, setAge] = useState<"all" | "adult" | "child">("all");
  const [language, setLanguage] = useState<string>("all");
  const [minDur, setMinDur] = useState<number>(0);
  const [sort, setSort] = useState<"name" | "duration">("name");
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [playing, setPlaying] = useState<string | null>(null);
  const [getting, setGetting] = useState<{ done: number; total: number; current: string } | null>(null);
  const [problems, setProblems] = useState<string[]>([]);
  const audio = useRef<HTMLAudioElement | null>(null);
  const stopRef = useRef(false);

  useEffect(() => {
    api.voicesCatalog().then((r) => setAll(r.voices), (e: unknown) => { setAll([]); setProblems([t("voice.catalogFailed", { error: errorText(e) })]); });
    return () => { audio.current?.pause(); stopRef.current = true; };
  }, [t]);
  useEscapeLayer(() => { if (!getting) onClose(); });

  const languages = useMemo(() => {
    const codes = new Set<string>();
    for (const v of all ?? []) if (v.language) codes.add(v.language);
    return [...codes].sort((a, b) => langName(a).localeCompare(langName(b)));
  }, [all]);

  const owned = (n: string) => have.has(n) || have.has(n.split("/").pop() || n);
  const name = (v: Entry) => v.displayName || prettyVoice(v.name);
  const play = (v: Entry) => {
    audio.current?.pause();
    if (playing === v.name) { setPlaying(null); return; }
    const a = new Audio(v.url);
    audio.current = a;
    setPlaying(v.name);
    a.onended = () => setPlaying(null);
    a.play().catch((e: unknown) => { setPlaying(null); setProblems([errorText(e)]); });
  };

  const needle = q.trim().toLowerCase();
  const filtered = useMemo(() => {
    const out = (all ?? []).filter((v) =>
      (gender === "all" || v.gender === gender) &&
      (age === "all" || v.age_group === age) &&
      (language === "all" || v.language === language) &&
      (minDur === 0 || (v.duration_s ?? 0) >= minDur) &&
      (!needle || [name(v), v.name, v.description ?? ""].some((s) => s.toLowerCase().includes(needle))));
    out.sort((a, b) => sort === "duration" ? (b.duration_s ?? 0) - (a.duration_s ?? 0) : name(a).localeCompare(name(b)));
    return out;
  }, [all, gender, age, language, minDur, sort, needle]);

  const toggle = (n: string) => setPicked((p) => { const s = new Set(p); if (s.has(n)) s.delete(n); else s.add(n); return s; });

  const download = async (names: string[]) => {
    const todo = names.filter((n) => !owned(n));
    if (!todo.length) return;
    setProblems([]);
    stopRef.current = false;
    const failed: string[] = [];
    for (let i = 0; i < todo.length; i++) {
      if (stopRef.current) break;
      setGetting({ done: i, total: todo.length, current: todo[i] });
      try {
        const r = await api.voicesGet(todo[i]);
        if (!r.ok) failed.push(t("voice.getFailed", { name: prettyVoice(todo[i]), error: r.error ?? "" }));
      } catch (e) { failed.push(t("voice.getFailed", { name: prettyVoice(todo[i]), error: errorText(e) })); }
    }
    setGetting(null);
    setPicked(new Set());
    setProblems(failed);
    onGot();
  };

  const selClass = "shrink-0 bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2 py-1.5 text-[12px] focus:border-[var(--color-accent)] focus:outline-none";

  return (
    <div className="fixed inset-0 z-[70] grid place-items-center glass-scrim anim-fade" onClick={() => { if (!getting) onClose(); }}>
      <div role="dialog" aria-modal="true" aria-labelledby="catalog-title" className="w-[min(94vw,680px)] h-[min(86vh,740px)] flex flex-col rounded-xl glass-panel anim-pop p-5" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-1">
          <span id="catalog-title" className="font-semibold">{t("voices.catalogTitle")} {all && <span className="mono text-[11px] text-[var(--color-muted)]">{filtered.length}/{all.length}</span>}</span>
          <button onClick={onClose} disabled={!!getting} {...named(t("a11y.close"))} className="text-[var(--color-muted)] hover:text-[var(--color-text)] disabled:opacity-30"><X size={16} /></button>
        </div>
        <p className="text-[12px] text-[var(--color-muted)] mb-3">{t("voices.catalogHint")}</p>
        <div className="flex flex-wrap items-center gap-2 mb-2">
          <input value={q} onChange={(e) => setQ(e.target.value)} placeholder={t("voice.search")} aria-label={t("voice.search")}
            className="flex-1 min-w-[160px] bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2.5 py-1.5 text-[13px] focus:border-[var(--color-accent)] focus:outline-none" />
          <div className="flex rounded-lg border border-[var(--color-border)] overflow-hidden text-[12px] shrink-0" role="group" aria-label={t("voices.gender")}>
            {(["all", "female", "male"] as const).map((g) => (
              <button key={g} onClick={() => setGender(g)} aria-pressed={gender === g}
                className={`px-2.5 py-1.5 ${gender === g ? "bg-[var(--color-accent)] text-[var(--color-on-accent)]" : "text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>{t(`voices.gender_${g}`)}</button>
            ))}
          </div>
          <div className="flex rounded-lg border border-[var(--color-border)] overflow-hidden text-[12px] shrink-0" role="group" aria-label={t("voices.age")}>
            {(["all", "adult", "child"] as const).map((a) => (
              <button key={a} onClick={() => setAge(a)} aria-pressed={age === a}
                className={`px-2.5 py-1.5 ${age === a ? "bg-[var(--color-accent)] text-[var(--color-on-accent)]" : "text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>{t(`voices.age_${a}`)}</button>
            ))}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 mb-3">
          <select value={language} onChange={(e) => setLanguage(e.target.value)} aria-label={t("voices.lang")} className={selClass}>
            <option value="all">{t("voices.anyLanguage")}</option>
            {languages.map((c) => <option key={c} value={c}>{langName(c)}</option>)}
          </select>
          <select value={minDur} onChange={(e) => setMinDur(Number(e.target.value))} aria-label={t("voices.minDuration")} className={selClass}>
            {DURATIONS.map((s) => <option key={s} value={s}>{s === 0 ? t("voices.durationAny") : t("voices.durationMin", { s })}</option>)}
          </select>
          <select value={sort} onChange={(e) => setSort(e.target.value as "name" | "duration")} aria-label={t("voices.sortBy")} className={selClass}>
            <option value="name">{t("voices.sortName")}</option>
            <option value="duration">{t("voices.sortDuration")}</option>
          </select>
        </div>
        {problems.length > 0 && <div role="alert" className="mb-2 max-h-20 overflow-y-auto text-[12px] text-[var(--color-warn)] break-words space-y-0.5">{problems.map((p, i) => <div key={i}>{p}</div>)}</div>}
        <div className="flex-1 overflow-y-auto -mr-2 pr-2 space-y-1">
          {all === null ? <div className="grid place-items-center py-10"><Loader2 size={18} className="animate-spin text-[var(--color-muted)]" /></div> :
            filtered.slice(0, 400).map((v) => {
              const has = owned(v.name);
              return (
                <label key={v.name} className={`flex items-center gap-2 px-2.5 py-1.5 rounded-lg border ${picked.has(v.name) ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_8%,transparent)]" : "border-[var(--color-border)] bg-[var(--color-surface-2)]"} ${has ? "" : "cursor-pointer"}`}>
                  {has ? <Check size={14} className="text-[var(--color-accent)] shrink-0" aria-label={t("voices.have")} /> :
                    <input type="checkbox" checked={picked.has(v.name)} onChange={() => toggle(v.name)} disabled={!!getting} className="w-3.5 h-3.5 accent-[var(--color-accent)] shrink-0" />}
                  <span className="text-[11px] text-[var(--color-muted)] w-4 shrink-0" aria-label={t(`voices.gender_${v.gender === "female" ? "female" : "male"}`)}>{v.gender === "female" ? "♀" : "♂"}</span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <span className="text-[13px] truncate">{name(v)}</span>
                      {v.language && <span className="mono text-[10px] text-[var(--color-muted)] shrink-0">{v.language.toUpperCase()}</span>}
                      {v.age_group === "child" && <span className="text-[10px] text-[var(--color-accent-2)] shrink-0">{t("voices.age_child")}</span>}
                      {typeof v.duration_s === "number" && <span className="mono text-[10px] text-[var(--color-muted)] shrink-0">{t("voices.seconds", { s: Math.round(v.duration_s) })}</span>}
                    </div>
                    {v.description && <div className="text-[11px] text-[var(--color-muted)] truncate" title={v.description}>{v.description}</div>}
                  </div>
                  <button type="button" onClick={(e) => { e.preventDefault(); play(v); }} {...named(t("voice.preview"))}
                    className="shrink-0 p-1 rounded-md text-[var(--color-muted)] hover:text-[var(--color-accent)]">{playing === v.name ? <Pause size={14} /> : <Play size={14} />}</button>
                </label>
              );
            })}
          {all !== null && filtered.length > 400 && <div className="mono text-[10px] text-[var(--color-muted)] text-center py-2">{t("voice.narrowSearch")}</div>}
        </div>
        <div className="mt-3 flex items-center gap-2">
          {getting ? (
            <>
              <Loader2 size={14} className="animate-spin text-[var(--color-accent)]" />
              <span className="text-[12px] text-[var(--color-muted)] flex-1 truncate">{t("voices.downloading", { done: getting.done + 1, total: getting.total, name: prettyVoice(getting.current) })}</span>
              <button onClick={() => { stopRef.current = true; }} className="px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)]">{t("voices.stop")}</button>
            </>
          ) : (
            <>
              <span className="text-[12px] text-[var(--color-muted)] flex-1">{t("voices.pickedN", { n: picked.size })}</span>
              <button onClick={() => download(filtered.map((v) => v.name))} disabled={!filtered.length}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-[var(--color-border)] text-[12px] hover:border-[var(--color-accent)] disabled:opacity-40"><Download size={13} />{t("voices.downloadAllShown")}</button>
              <button onClick={() => download([...picked])} disabled={!picked.size}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-[12px] font-semibold disabled:opacity-40"><Download size={13} />{t("voices.downloadPicked")}</button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
