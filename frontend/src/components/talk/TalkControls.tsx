import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AudioLines, ChevronUp, Pause, Play, SendHorizontal, Square, Volume2, VolumeX, X } from "lucide-react";
import { byPerson, personOnly } from "../../lib/person";
import { named } from "../../lib/a11y";
import { api, BASE, slot, type Selection } from "../../lib/api";
import { SETTINGS_CHANGED, useChanged } from "../../lib/mcpBridge";
import { voiceGateApi } from "../../lib/voiceGate";
import { aloudPlayback, onLiveEvents } from "../../lib/livePlayback";
import { talkApi } from "../../lib/talk";
import { errorText } from "../../lib/voices";
import LevelMeter from "./LevelMeter";
import VoicePicker from "../speech/VoicePicker";
import OwnParams from "../speech/OwnParams";
import { useEngines } from "../../lib/engines";
import type { EngineCaps, ParamSpec, SpeechParams } from "../../lib/speech";
import type { TalkVoice } from "../../lib/talk";
import { setTalkPrefs, useTalkPrefs } from "../../lib/talkPrefs";
import { LANG_CHOICES } from "../speech/langs";
import Hint, { Tip } from "../Hint";
import type { useTalk } from "./useTalk";

type Talk = ReturnType<typeof useTalk>;

/** The recognisers that hear a conversation as it goes: the local ones. */
const LIVE_ASR = ["", "parakeet", "nemotron", "whisper"] as const;

const round_ = "grid place-items-center rounded-full transition-colors disabled:opacity-40";
const quiet = "border border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-accent)]";

/** The speakers the browser can play on; names come only once the page may use a microphone. */
function useSpeakers() {
  const [speakers, setSpeakers] = useState<MediaDeviceInfo[] | null>(null);
  const look = useCallback(() => {
    navigator.mediaDevices.enumerateDevices().then((all) => setSpeakers(all.filter((d) => d.kind === "audiooutput")), () => setSpeakers([]));
  }, []);
  useEffect(() => {
    look();
    navigator.mediaDevices.addEventListener("devicechange", look);
    return () => navigator.mediaDevices.removeEventListener("devicechange", look);
  }, [look]);
  const name = useCallback(async () => {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    for (const track of stream.getTracks()) track.stop();
    look();
  }, [look]);
  return { speakers, name };
}

type Sinkable = AudioContext & { setSinkId?: (id: string) => Promise<void> };

/** Plays a test on the chosen speaker at the agent's volume, and how loud it plays: a chime, or what the microphone
 * test heard. */
class SpeakerTest {
  private ctx: Sinkable | null = null;
  private meter: AnalyserNode | null = null;
  private gain: GainNode | null = null;
  private element: HTMLAudioElement | null = null;

  private async open(output: string, volume: number) {
    this.ctx ??= new AudioContext() as Sinkable;
    if (!this.meter) {
      this.meter = this.ctx.createAnalyser();
      this.meter.fftSize = 1024;
      this.gain = this.ctx.createGain();
      this.meter.connect(this.gain).connect(this.ctx.destination);
    }
    if (this.gain) this.gain.gain.value = volume;
    if (this.ctx.setSinkId) await this.ctx.setSinkId(output);
    if (this.ctx.state === "suspended") await this.ctx.resume();
    return this.ctx;
  }

  async chime(output: string, volume: number) {
    const ctx = await this.open(output, volume);
    const start = ctx.currentTime;
    [523.25, 659.25, 783.99].forEach((hz, i) => {
      const tone = ctx.createOscillator();
      const env = ctx.createGain();
      tone.frequency.value = hz;
      env.gain.setValueAtTime(0, start + i * 0.18);
      env.gain.linearRampToValueAtTime(0.5, start + i * 0.18 + 0.02);
      env.gain.exponentialRampToValueAtTime(0.001, start + i * 0.18 + 0.6);
      tone.connect(env).connect(this.meter as AnalyserNode);
      tone.start(start + i * 0.18);
      tone.stop(start + i * 0.18 + 0.65);
    });
  }

  async play(url: string, output: string, volume: number): Promise<void> {
    const ctx = await this.open(output, volume);
    this.element?.pause();
    const element = new Audio(url);
    element.crossOrigin = "anonymous";
    this.element = element;
    ctx.createMediaElementSource(element).connect(this.meter as AnalyserNode);
    await element.play();
    await new Promise<void>((done) => { element.onended = () => done(); element.onpause = () => done(); });
  }

  peak(): number {
    if (!this.meter) return 0;
    const data = new Float32Array(this.meter.fftSize);
    this.meter.getFloatTimeDomainData(data);
    let peak = 0;
    for (const v of data) peak = Math.max(peak, Math.abs(v));
    return peak;
  }

  close() {
    this.element?.pause();
    void this.ctx?.close();
    this.ctx = null;
  }
}

/** An engine's own sampling, what a voice without settings of its own speaks with. */
function engineDefaults(caps: EngineCaps): SpeechParams {
  const of = (key: ParamSpec["key"], fallback: number) => caps.params.find((p) => p.key === key)?.default ?? fallback;
  return { temperature: of("temperature", 0.7), top_k: of("top_k", 50), top_p: of("top_p", 0.95), seed_mode: caps.seed_modes[0] ?? "random", seed: 0, max_tokens: of("max_tokens", 0), steps: of("steps", 8), speed: of("speed", 1) };
}

/** The voice agents speak with when they give none, as a project's voice is set: engine, voice of the library and the
 * engine's sampling. */
function VoiceTab() {
  const { t } = useTranslation();
  const engines = useEngines();
  const [chosen, setChosen] = useState<TalkVoice | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  useEffect(() => {
    const read = () => { talkApi.voice().then(setChosen, (e: unknown) => setProblem(errorText(e))); };
    read();
    return onLiveEvents((ev) => { if (ev.kind === "chosen") read(); });
  }, []);
  const save = (next: TalkVoice) => {
    setChosen(next);
    talkApi.setVoice(next).then(() => setProblem(null), (e: unknown) => setProblem(errorText(e)));
  };
  if (!chosen || !engines) return <p className="text-[11px] text-[var(--color-muted)]">…</p>;
  const engine = chosen.engine || "qwen3-tts";
  const caps = engines.find((e) => e.id === engine);
  const field = "w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1.5 text-[12px]";
  return (
    <div className="space-y-3">
      <label className="block space-y-1.5 text-[11px] text-[var(--color-muted)]">{t("speech.engine")}
        <select value={engine} onChange={(e) => save({ engine: e.target.value, voice: null, params: null })} className={field}>
          {engines.map((e) => <option key={e.id} value={e.id} disabled={!!e.unavailable} title={e.unavailable ?? undefined}>{e.name}</option>)}
        </select>
      </label>
      <label className="block space-y-1.5 text-[11px] text-[var(--color-muted)]">{t("speech.voice")}
        <VoicePicker value={chosen.voice} onChange={(v) => save({ ...chosen, engine, voice: v })} label={t("speech.voice")} engine={engine} className={field} />
      </label>
      {caps && <OwnParams engine={engine} value={chosen.params} inherited={engineDefaults(caps)} onChange={(p) => save({ ...chosen, engine, params: p })} label={t("talk.ownParams")} />}
      {problem && <p role="alert" className="text-[11px] text-[var(--color-warn)]">{problem}</p>}
    </div>
  );
}

const INTERRUPT = ["agent", "always", "never"] as const;
const ECHO_GUARD = ["duplex", "gate", "text"] as const;

/** Who stops the agent when the person speaks over it, and how long the agent has to decide. */
function InterruptSetting() {
  const { t } = useTranslation();
  const [sel, setSel] = useState<Selection | null>(null);
  const [draft, setDraft] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [reread, setReread] = useState(0);
  const [enrolled, setEnrolled] = useState<boolean | null>(null);
  useChanged(SETTINGS_CHANGED, () => setReread((n) => n + 1));
  useEffect(() => {
    api.capabilities().then((c) => setSel(c.selection ?? {}), (e: unknown) => setProblem(errorText(e)));
    voiceGateApi.get().then((g) => setEnrolled(g.enrolled), (e: unknown) => setProblem(errorText(e)));
  }, [reread]);
  const save = (key: string, value: string) => {
    api.setSelection(key, value)
      .then(() => api.capabilities())
      .then((c) => { setSel(c.selection ?? {}); setDraft(null); setProblem(null); }, (e: unknown) => setProblem(errorText(e)));
  };
  if (!sel) return problem ? <p role="alert" className="text-[11px] text-[var(--color-warn)]">{problem}</p> : null;
  const mode = slot(sel, "live_interrupt") ?? "";
  const ms = draft ?? slot(sel, "live_interrupt_ms") ?? "";
  const guard = slot(sel, "live_echo_guard") ?? "";
  const field = "w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1.5 text-[12px]";
  const label = "block space-y-1.5 text-[11px] text-[var(--color-muted)]";
  return (
    <div className="space-y-3 border-t border-[var(--color-border)] pt-3">
      <label className={label}>
        <span className="flex items-center gap-1">{t("talk.interrupt")} <Hint text={t("talk.interruptHint")} /></span>
        <select value={mode} onChange={(e) => save("live_interrupt", e.target.value)} className={field}>
          {INTERRUPT.map((m) => <option key={m} value={m}>{t(`talk.interrupt_${m}`)}</option>)}
        </select>
      </label>
      {mode === "agent" && (
        <label className={label}>{t("talk.interruptMs")}
          <input type="number" min={200} max={10000} step={100} value={ms} onChange={(e) => setDraft(e.target.value)}
            onBlur={() => { if (draft !== null) save("live_interrupt_ms", draft); }}
            onKeyDown={(e) => { if (e.key === "Enter" && draft !== null) save("live_interrupt_ms", draft); }} className={field} />
        </label>
      )}
      <label className={label}>
        <span className="flex items-center gap-1">{t("talk.echoGuard")} <Hint text={t("talk.echoGuardHint")} /></span>
        <select value={guard} onChange={(e) => save("live_echo_guard", e.target.value)} className={field}>
          {ECHO_GUARD.map((g) => <option key={g} value={g}>{t(`talk.echoGuard_${g}`)}</option>)}
        </select>
      </label>
      {guard === "gate" && enrolled === false && <p className="text-[11px] text-[var(--color-warn)]">{t("talk.echoGuardNoOwner")}</p>}
      {problem && <p role="alert" className="text-[11px] text-[var(--color-warn)]">{problem}</p>}
    </div>
  );
}

type SettingsTab = "mic" | "voice" | "speaker";

/** The microphone and the speaker as call apps set them up: each with a live level meter, the microphone heard back
 * (Discord's mic test), the speaker tested with a chime (Zoom's speaker test), the agent's volume and the language.
 * The microphone test runs only when the person opened it (`test`): an agent opening it tests nothing. */
function TalkSettings({ running, relisten, test }: { running: boolean; relisten: () => void; test: boolean }) {
  const { t } = useTranslation();
  const prefs = useTalkPrefs();
  const [devices, setDevices] = useState<string[] | null>(null);
  const { speakers, name } = useSpeakers();
  const [problem, setProblem] = useState<string | null>(null);
  const [hearing, setHearing] = useState(false);
  const [round, setRound] = useState(0);
  const [tab, setTab] = useState<SettingsTab>("mic");
  const [studioAsr, setStudioAsr] = useState("");
  useEffect(() => { api.capabilities().then((c) => setStudioAsr(c.asr_label), (e: unknown) => setProblem(errorText(e))); }, []);
  // Parakeet takes no language: it picks one itself, so a set language does not keep words of another out
  const parakeet = prefs.asr === "parakeet" || (prefs.asr === "" && studioAsr.startsWith("Parakeet"));
  const micPeak = useRef(0);
  const tester = useRef<SpeakerTest | null>(null);
  tester.current ??= new SpeakerTest();
  /** Ends the test recording now under way, once: whoever asks first stops it, a later ask is a no-op. */
  const endTest = useRef<() => Promise<void>>(() => Promise.resolve());

  useEffect(() => {
    api.recordDevices().then((d) => setDevices(d.devices), (e: unknown) => { setDevices([]); setProblem(errorText(e)); });
    return () => tester.current?.close();
  }, []);

  // the microphone heard while this is open: the conversation's own when it runs, a test recording otherwise; a test
  // that answers after it was ended is stopped as soon as it has started
  useEffect(() => {
    if (running || hearing || !test) return;
    let state: "starting" | "open" | "ended" = "starting";
    const end = async () => {
      const was = state;
      state = "ended";
      if (was === "open") await talkApi.micTestStop();
    };
    endTest.current = end;
    talkApi.micTest(prefs.device).then(() => {
      if (state === "ended") { talkApi.micTestStop().catch((e: unknown) => console.error("[ERROR] mic test stop:", e)); return; }
      state = "open";
      setProblem(null);
    }, (e: unknown) => setProblem(errorText(e)));
    return () => { end().catch((e: unknown) => console.error("[ERROR] mic test stop:", e)); };
  }, [running, hearing, test, prefs.device, round]);

  useEffect(() => {
    const id = window.setInterval(() => { talkApi.level().then((l) => { micPeak.current = l.level; }, () => { micPeak.current = 0; }); }, 50);
    return () => window.clearInterval(id);
  }, []);

  const hearBack = async () => {
    setProblem(null);
    const end = endTest.current;
    setHearing(true);
    try {
      await end();
      await (tester.current as SpeakerTest).play(`${BASE}/record/test/audio?at=${Date.now()}`, prefs.output, prefs.muted ? 0 : prefs.volume);
    } catch (e) {
      setProblem(errorText(e));
    } finally {
      setHearing(false);
      setRound((r) => r + 1);
    }
  };

  const outPeak = () => Math.max(tester.current?.peak() ?? 0, aloudPlayback()?.level() ?? 0);
  const canChoose = typeof (AudioContext.prototype as { setSinkId?: unknown }).setSinkId === "function";
  const unnamed = !!speakers?.length && speakers.every((s) => !s.label);
  const field = "w-full bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-md px-2 py-1.5 text-[12px] disabled:opacity-50";
  const label = "block space-y-1.5 text-[11px] text-[var(--color-muted)]";
  const small = "shrink-0 rounded-md border border-[var(--color-border)] px-2 py-1 text-[11px] text-[var(--color-text)] hover:border-[var(--color-accent)] disabled:opacity-40";

  return (
    <div role="dialog" aria-label={t("talk.settings")} className="absolute left-0 bottom-full mb-2 z-50 w-80 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] shadow-xl p-3 space-y-4 text-[var(--color-text)]">
      <div role="tablist" aria-label={t("talk.settings")} className="flex gap-1 rounded-lg bg-[var(--color-surface-2)] p-0.5">
        {(["mic", "voice", "speaker"] as const).map((id) => (
          <button key={id} role="tab" aria-selected={tab === id} onClick={() => setTab(id)} className={`flex-1 rounded-md px-2 py-1 text-[12px] font-medium transition-colors ${tab === id ? "bg-[var(--color-surface)] text-[var(--color-text)] shadow-sm" : "text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>
            {t(`talk.tab_${id}`)}
          </button>
        ))}
      </div>
      {tab === "mic" && (
        <div role="tabpanel" className="space-y-4">
      <div className={label}>
        <span>{t("talk.device")}</span>
        <select value={prefs.device} disabled={hearing} onChange={(e) => { setTalkPrefs({ device: e.target.value }); relisten(); }} className={field} {...named(t("talk.device"))}>
          <option value="">{t("talk.deviceDefault")}</option>
          {devices?.map((d) => <option key={d} value={d}>{d}</option>)}
        </select>
        <LevelMeter peak={() => micPeak.current} label={t("talk.inputLevel")} />
        <span className="flex items-center justify-between gap-2">
          <span>{hearing ? t("talk.hearingBack") : t("talk.speakToTest")}</span>
          <button onClick={() => void hearBack()} disabled={running || hearing} className={small}>{t("talk.hearBack")}</button>
        </span>
      </div>
      <label className={label}>{t("talk.asr")}
        <select value={prefs.asr} onChange={(e) => { setTalkPrefs({ asr: e.target.value }); relisten(); }} className={field}>
          {LIVE_ASR.map((e) => <option key={e} value={e}>{t(`listen.engine_${e || "default"}`)}</option>)}
        </select>
      </label>
      <label className={label}>{t("talk.lang")}
        <select value={prefs.lang} onChange={(e) => { setTalkPrefs({ lang: e.target.value }); relisten(); }} className={field}>
          {LANG_CHOICES.map((l) => <option key={l} value={l}>{l === "auto" ? t("talk.langAuto") : l.toUpperCase()}</option>)}
        </select>
      </label>
      {parakeet && prefs.lang !== "auto" && <p className="text-[11px] text-[var(--color-muted)]">{t("talk.parakeetLang")}</p>}
        </div>
      )}
      {tab === "voice" && <div role="tabpanel" className="space-y-3"><VoiceTab /><InterruptSetting /></div>}
      {tab === "speaker" && (
        <div role="tabpanel" className="space-y-4">
      <div className={label}>
        <span>{t("talk.output")}</span>
        <select value={prefs.output} disabled={!canChoose} onChange={(e) => setTalkPrefs({ output: e.target.value })} className={field} {...named(t("talk.output"))}>
          <option value="">{t("talk.outputDefault")}</option>
          {speakers?.filter((s) => s.deviceId && s.deviceId !== "default").map((s, i) => <option key={s.deviceId} value={s.deviceId}>{s.label || `${t("talk.output")} ${i + 1}`}</option>)}
        </select>
        {!canChoose && <span className="block">{t("talk.outputUnsupported")}</span>}
        {canChoose && unnamed && (
          <span className="flex items-center justify-between gap-2">
            <span>{t("talk.outputNamesHint")}</span>
            <button onClick={() => { name().catch((e: unknown) => setProblem(errorText(e))); }} className={small}>{t("talk.outputNames")}</button>
          </span>
        )}
        <LevelMeter peak={outPeak} label={t("talk.outputLevel")} />
        <div className="flex items-center gap-2">
          <Tip text={prefs.muted ? t("talk.agentUnmute") : t("talk.agentMute")}>
            <button onClick={() => setTalkPrefs({ muted: !prefs.muted })} aria-pressed={prefs.muted} className={`${round_} ${quiet} w-8 h-8 shrink-0`} {...named(prefs.muted ? t("talk.agentUnmute") : t("talk.agentMute"))}>
              {prefs.muted ? <VolumeX size={14} /> : <Volume2 size={14} />}
            </button>
          </Tip>
          <input type="range" min={0} max={1} step={0.05} value={prefs.volume} disabled={prefs.muted} onChange={(e) => setTalkPrefs({ volume: Number(e.target.value) })} className="flex-1 accent-[var(--color-accent)]" {...named(t("talk.volume"))} />
          <span className="mono w-9 text-right text-[11px]">{Math.round(prefs.volume * 100)}%</span>
          <button onClick={() => { (tester.current as SpeakerTest).chime(prefs.output, prefs.muted ? 0 : prefs.volume).catch((e: unknown) => setProblem(errorText(e))); }} className={small}>{t("talk.testSpeaker")}</button>
        </div>
      </div>
        </div>
      )}
      {problem && <p role="alert" className="text-[11px] text-[var(--color-warn)]">{problem}</p>}
    </div>
  );
}

/** The conversation's controls, as voice modes have them: start or end, mute the microphone (M), the microphone and
 * sound, stop the agent, and a field to write to the agent instead of saying it. */
export default function TalkControls({ talk, compact = false, children }: { talk: Talk; compact?: boolean; children?: ReactNode }) {
  const { t } = useTranslation();
  const [menu, setMenu] = useState<false | "person" | "agent">(false);
  const [draft, setDraft] = useState("");
  const { running, muted, speaking } = talk;

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMenu(false);
      const typing = e.target instanceof HTMLElement && (e.target.closest("input, textarea, select, [contenteditable]") !== null);
      if (!typing && running && !e.ctrlKey && !e.metaKey && !e.altKey && e.code === "KeyM") {
        e.preventDefault();
        if (muted && !byPerson(e)) return;
        void talk.toggleMute();
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [running, muted, talk]);

  const send = async () => {
    const text = draft.trim();
    if (!text) return;
    if (await talk.type(text)) setDraft("");
  };

  const size = compact ? "w-9 h-9" : "w-11 h-11";
  const main = compact ? "w-11 h-11" : "w-14 h-14";
  return (
    <div className={`shrink-0 flex flex-col items-center gap-3 ${compact ? "" : "w-full max-w-xl"}`}>
      {!compact && (
        <form onSubmit={(e) => { e.preventDefault(); void send(); }} className="flex w-full items-center gap-2 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)] pl-4 pr-1 py-1 focus-within:border-[var(--color-accent)]">
          <input value={draft} onChange={(e) => setDraft(e.target.value)} disabled={!running} placeholder={t("talk.typePlaceholder")} className="flex-1 min-w-0 bg-transparent text-[13px] outline-none disabled:opacity-50" {...named(t("talk.typePlaceholder"))} />
          <button type="submit" disabled={!running || !draft.trim()} className={`${round_} w-8 h-8 bg-[var(--color-accent)] text-[var(--color-on-accent)]`} {...named(t("talk.send"))}>
            <SendHorizontal size={14} />
          </button>
        </form>
      )}
      <div className="flex items-center gap-2">
        <div className="relative">
          <Tip text={t("talk.settings")}>
            <button onClick={(e) => setMenu((m) => (m ? false : byPerson(e) ? "person" : "agent"))} aria-haspopup="dialog" aria-expanded={menu !== false} className={`${round_} ${quiet} ${size}`} {...named(t("talk.settings"))}>
              <ChevronUp size={16} className={`transition-transform ${menu ? "" : "rotate-180"}`} />
            </button>
          </Tip>
          {menu && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setMenu(false)} />
              <TalkSettings running={running} relisten={() => void talk.relisten()} test={menu === "person"} />
            </>
          )}
        </div>
        {running && (
          <Tip text={t("talk.muteHint")}>
            <button onClick={(e) => { if (muted && !byPerson(e)) return; void talk.toggleMute(); }} aria-pressed={muted} className={`${round_} ${size} ${muted ? "bg-[var(--color-warn)] text-[var(--color-bg)]" : quiet}`} {...personOnly(muted)} {...named(muted ? t("talk.unmute") : t("talk.mute"))}>
              {muted ? <Play size={16} /> : <Pause size={16} />}
            </button>
          </Tip>
        )}
        <Tip text={running ? t("talk.micOffHint") : t("talk.micOnHint")}>
          <button onClick={(e) => { if (!running && !byPerson(e)) return; void talk.toggleMic(); }} {...personOnly(!running)} className={`${round_} ${main} ${running ? "bg-[var(--color-bad)] text-white" : "bg-[var(--color-accent)] text-[var(--color-on-accent)]"}`} {...named(running ? t("talk.micOff") : t("talk.micOn"))}>
            {running ? <X size={compact ? 18 : 22} /> : <AudioLines size={compact ? 18 : 22} />}
          </button>
        </Tip>
        <Tip text={t("talk.silenceHint")}>
          <button onClick={talk.silence} disabled={!speaking} className={`${round_} ${quiet} ${size}`} {...named(t("talk.silence"))}>
            <Square size={14} />
          </button>
        </Tip>
        {children}
      </div>
    </div>
  );
}
