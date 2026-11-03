import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AudioLines, Fingerprint, KeyRound, Mic, Trash2, X } from "lucide-react";
import SettingSwitch from "./SettingSwitch";
import LevelMeter from "../talk/LevelMeter";
import { voiceGateApi, type VoiceGate } from "../../lib/voiceGate";
import { talkApi } from "../../lib/talk";
import { errorText } from "../../lib/voices";
import { api } from "../../lib/api";
import { useSetupStatus } from "../../lib/useSetupStatus";
import DownloadProgress from "../DownloadProgress";

// Enough clean speech (seconds) for a solid speaker print: a text-dependent passphrase wants a couple of seconds at
// least (Azure Speaker Recognition, Nuance VocalPassword, Neurotechnology); we take a little more, then auto-finish.
const ENROLL_SPEECH_SECONDS = 3.5;
// A hard cap on the recording so enrolment always finishes on its own, even on a quiet microphone whose speech never
// clears the floor — the enrol call then uses whatever was captured (and says so if it is too short).
const ENROLL_MAX_SECONDS = 12;
// RMS level above which the microphone is hearing speech rather than room noise.
const SPEECH_FLOOR = 0.06;
const POLL_MS = 120;
// How close below the threshold still counts as "almost" in the live check — a borderline zone, not a flat no.
const CLOSE_MARGIN = 0.08;
// The live match meter spans this slice of the cosine range, where a speaker's own voice and an impostor's separate.
const METER_LOW = 0.2;
const METER_HIGH = 0.9;

/// The voice gate: Live starts a conversation hands-free. Two independent ways to start it — on the owner's own voice
/// and/or on a wake phrase — that can be used alone or together. Everything is off by default; the voice print and the
/// wake model stay on the device.
export default function VoiceGateSection() {
  const { t, i18n } = useTranslation();
  const [state, setState] = useState<VoiceGate | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const [heard, setHeard] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  const [match, setMatch] = useState<number | null>(null);
  const [wakeOn, setWakeOn] = useState<boolean | null>(null);
  const [wakeTesting, setWakeTesting] = useState(false);
  const [wakeMatch, setWakeMatch] = useState<number | null>(null);
  const [wakeThresh, setWakeThresh] = useState(0.6);
  const level = useRef(0);
  const speechSecs = useRef(0);
  const elapsedSecs = useRef(0);
  const finishing = useRef(false);
  // the wake-word model downloads like any other component; its progress is shown here, not on another screen
  const setup = useSetupStatus();

  useEffect(() => {
    voiceGateApi.get().then(setState, (e: unknown) => setProblem(errorText(e)));
  }, []);

  // While enrolling: show the level and auto-finish once the studio has heard enough clean speech — no manual button.
  useEffect(() => {
    if (!recording) {
      level.current = 0;
      speechSecs.current = 0;
      elapsedSecs.current = 0;
      return;
    }
    const id = window.setInterval(() => {
      elapsedSecs.current += POLL_MS / 1000;
      talkApi.level().then((l) => {
        level.current = l.level;
        if (l.level > SPEECH_FLOOR) speechSecs.current += POLL_MS / 1000;
        const enough = speechSecs.current >= ENROLL_SPEECH_SECONDS || elapsedSecs.current >= ENROLL_MAX_SECONDS;
        if (enough && !finishing.current) {
          finishing.current = true;
          void finishEnroll();
        }
      }, () => { level.current = 0; });
    }, POLL_MS);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [recording]);

  // While testing: poll the live match and the level, so the person speaks and watches how the model hears them.
  useEffect(() => {
    if (!testing) {
      level.current = 0;
      return;
    }
    let scoring = false;
    const id = window.setInterval(() => {
      talkApi.level().then((l) => { level.current = l.level; }, () => { level.current = 0; });
      if (!scoring) {
        scoring = true;
        voiceGateApi.score().then((s) => setMatch(s.ready ? s.score : null)).catch(() => {}).finally(() => { scoring = false; });
      }
    }, POLL_MS);
    return () => window.clearInterval(id);
  }, [testing]);

  // While checking the wake phrase: poll how strongly it is heard and the level, so the person says it and sees it fire.
  useEffect(() => {
    if (!wakeTesting) return;
    let scoring = false;
    const id = window.setInterval(() => {
      talkApi.level().then((l) => { level.current = l.level; }, () => { level.current = 0; });
      if (!scoring) {
        scoring = true;
        voiceGateApi.wakeScore().then((s) => { setWakeMatch(s.ready ? s.score : null); setWakeThresh(s.threshold); }).catch(() => {}).finally(() => { scoring = false; });
      }
    }, POLL_MS);
    return () => window.clearInterval(id);
  }, [wakeTesting]);

  // Merge the save's reply into the current state: the POST reply carries the saved fields but not `wake_words` /
  // `enrolled`, so replacing state outright would drop them and crash the render.
  const patch = (p: Partial<Pick<VoiceGate, "enabled" | "threshold" | "wake_word" | "wake_lang">>) => {
    setProblem(null);
    voiceGateApi.set(p).then((r) => setState((s) => (s ? { ...s, ...r } : s)), (e: unknown) => setProblem(errorText(e)));
  };

  const startEnroll = async () => {
    setProblem(null);
    finishing.current = false;
    try {
      await talkApi.micTest("");
      setRecording(true);
    } catch (e) {
      setProblem(errorText(e));
    }
  };

  const finishEnroll = async () => {
    setBusy(true);
    try {
      await talkApi.micTestStop();
      const r = await voiceGateApi.enroll();
      setState((s) => (s ? { ...s, enrolled: r.enrolled } : s));
      setHeard(r.text || null);
    } catch (e) {
      setProblem(errorText(e));
    } finally {
      setRecording(false);
      setBusy(false);
      finishing.current = false;
    }
  };

  const cancelEnroll = async () => {
    try {
      await talkApi.micTestStop();
    } catch {
      // nothing to stop
    }
    setRecording(false);
    finishing.current = false;
  };

  const forget = () => {
    setProblem(null);
    setHeard(null);
    voiceGateApi.forget().then((r) => setState((s) => (s ? { ...s, enrolled: r.enrolled } : s)), (e: unknown) => setProblem(errorText(e)));
  };

  // the voice check and the wake-phrase check share one microphone tap: start it when the first turns on, stop it when
  // the last turns off, so toggling one never cuts the other short.
  const toggleTest = async () => {
    setProblem(null);
    const next = !testing;
    try {
      if (!wakeTesting) await voiceGateApi.test(next);
      setTesting(next);
      setMatch(null);
    } catch (e) {
      setTesting(false);
      setProblem(errorText(e));
    }
  };

  const toggleWakeTest = async () => {
    setProblem(null);
    const next = !wakeTesting;
    try {
      if (!testing) await voiceGateApi.test(next);
      setWakeTesting(next);
      setWakeMatch(null);
    } catch (e) {
      setWakeTesting(false);
      setProblem(errorText(e));
    }
  };

  // stop the live check (voice or wake) if the panel goes away
  useEffect(() => () => { if (testing || wakeTesting) void voiceGateApi.test(false).catch(() => {}); }, [testing, wakeTesting]);

  if (!state) return <p className="text-[11px] text-[var(--color-muted)]">…</p>;

  // the phrase's language for grapheme-to-phoneme: [eSpeak-ng voice code, BCP-47 code for the display name]; the name
  // itself comes localized from the browser, so nothing is hardcoded. Default to the interface language.
  const uiLang = i18n.language.slice(0, 2);
  const wakeLangOf = (ui: string) => (ui === "zh" ? "cmn" : ui);
  const wakeLang = state.wake_lang ?? wakeLangOf(uiLang);
  const langNames = new Intl.DisplayNames([i18n.language], { type: "language" });
  const WAKE_LANGS: [string, string][] = [
    ["en", "en"], ["ru", "ru"], ["cmn", "zh"], ["es", "es"], ["pt", "pt"], ["fr", "fr"], ["de", "de"], ["it", "it"],
    ["ja", "ja"], ["ko", "ko"], ["ar", "ar"], ["hi", "hi"], ["tr", "tr"], ["pl", "pl"], ["uk", "uk"],
  ];

  const active = setup.status?.active;
  const wakeJob = active && active.ids.includes("wake-word") && (active.total <= 0 || active.downloaded < active.total) ? active : null;
  const wakeActive = wakeOn === null ? state.wake_word != null : wakeOn;
  const toggleWake = () => {
    const next = !wakeActive;
    setWakeOn(next);
    if (!next && state.wake_word != null) patch({ wake_word: null });
  };

  // the live match, mapped into the meter's slice and judged against the sensitivity the slider sets
  const pct = (v: number) => Math.max(0, Math.min(100, ((v - METER_LOW) / (METER_HIGH - METER_LOW)) * 100));
  const verdict = match === null ? null : match >= state.threshold ? "yes" : match >= state.threshold - CLOSE_MARGIN ? "close" : "no";
  const verdictColor = verdict === "yes" ? "var(--color-good)" : verdict === "close" ? "var(--color-cue-rate)" : "var(--color-muted)";
  // the wake-phrase check: phone overlap runs 0..1, so the bar fills the whole slider and the marker sits at the threshold
  const pctWake = (v: number) => Math.max(0, Math.min(100, v * 100));
  const wakeHit = wakeMatch !== null && wakeMatch >= wakeThresh;

  const field = "shrink-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]";
  const small = "shrink-0 rounded-md border border-[var(--color-border)] px-2.5 py-1 text-[11px] text-[var(--color-text)] hover:border-[var(--color-accent)] disabled:opacity-40";
  const card = "max-w-2xl rounded-lg border border-[var(--color-border)] bg-[var(--color-surface-2)]/40";

  return (
    <div className="space-y-4">
      <p className="max-w-2xl text-[12px] leading-snug text-[var(--color-muted)]">{t("voiceGate.intro")}</p>

      {/* 1 — start on the owner's own voice */}
      <div className={card}>
        <SettingSwitch
          icon={Fingerprint}
          label={t("voiceGate.byVoice")}
          hint={t("voiceGate.byVoiceHint")}
          on={state.enabled}
          onToggle={() => patch({ enabled: !state.enabled })}
        />
        {state.enabled && (
          <div className="border-t border-[var(--color-border)] divide-y divide-[var(--color-border)]">
            <div className="flex items-start gap-3 px-3 py-2.5" data-settings-part="enroll">
              <Mic size={15} className="mt-0.5 shrink-0 text-[var(--color-accent-2)]" />
              <div className="min-w-0 flex-1 space-y-2">
                <div>
                  <span className="block text-[13px] font-medium">{t("voiceGate.myVoice")}</span>
                  <span className="block text-[11px] leading-snug text-[var(--color-muted)]">
                    {state.enrolled ? t("voiceGate.enrolled") : t("voiceGate.notEnrolled")}
                  </span>
                  {heard && !recording && (
                    <span className="block text-[11px] leading-snug text-[var(--color-good)]">{t("voiceGate.heard", { text: heard })}</span>
                  )}
                </div>
                {recording ? (
                  <div className="space-y-2">
                    <p className="text-[12px] text-[var(--color-text)]">{t("voiceGate.enrollPrompt")}</p>
                    <LevelMeter peak={() => level.current} label={t("talk.inputLevel")} />
                    <div className="flex items-center gap-2">
                      <span className="text-[11px] text-[var(--color-muted)]">{t("voiceGate.enrollListening")}</span>
                      <button onClick={() => void cancelEnroll()} className={`${small} inline-flex items-center gap-1.5`}>
                        <X size={13} /> {t("voiceGate.enrollCancel")}
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <button onClick={() => void startEnroll()} disabled={busy || testing} className={`${small} inline-flex items-center gap-1.5`}>
                      <Mic size={13} /> {state.enrolled ? t("voiceGate.reEnroll") : t("voiceGate.enroll")}
                    </button>
                    {state.enrolled && (
                      <button onClick={forget} disabled={testing} className={`${small} inline-flex items-center gap-1.5 text-[var(--color-warn)]`}>
                        <Trash2 size={13} /> {t("voiceGate.forget")}
                      </button>
                    )}
                  </div>
                )}
              </div>
            </div>

            {/* live voice check + the sensitivity it tunes — only once there is a voice to check against */}
            {state.enrolled && (
              <div className="px-3 py-2.5 space-y-2.5">
                <div className="flex items-center gap-3">
                  <div className="min-w-0 flex-1">
                    <span className="block text-[13px] font-medium">{t("voiceGate.testVoice")}</span>
                    <span className="block text-[11px] leading-snug text-[var(--color-muted)]">{t("voiceGate.testHint")}</span>
                  </div>
                  <button onClick={() => void toggleTest()} disabled={recording}
                    className={`${small} inline-flex items-center gap-1.5 ${testing ? "border-[var(--color-accent)] text-[var(--color-accent)]" : ""}`}>
                    {testing ? <X size={13} /> : <AudioLines size={13} />}{testing ? t("voiceGate.testStop") : t("voiceGate.testStart")}
                  </button>
                </div>
                <div className="flex items-center gap-3">
                  <div className="min-w-0 flex-1">
                    <span className="block text-[13px] font-medium">{t("voiceGate.sensitivity")}</span>
                    <span className="block text-[11px] leading-snug text-[var(--color-muted)]">{t("voiceGate.sensitivityHint")}</span>
                  </div>
                  <input
                    type="range" min={0.3} max={0.8} step={0.01} value={state.threshold}
                    onChange={(e) => setState((s) => (s ? { ...s, threshold: Number(e.target.value) } : s))}
                    onPointerUp={(e) => patch({ threshold: Number((e.target as HTMLInputElement).value) })}
                    className="shrink-0 w-32 accent-[var(--color-accent)]" aria-label={t("voiceGate.sensitivity")}
                  />
                  <span className="w-9 text-right mono text-[11px] tabular-nums text-[var(--color-muted)]">{state.threshold.toFixed(2)}</span>
                </div>
                {testing && (
                  <div className="space-y-2">
                    <LevelMeter peak={() => level.current} label={t("talk.inputLevel")} />
                    {/* the match meter: the live score as a bar, a marker where the sensitivity cuts off */}
                    <div className="relative h-2.5 rounded-full bg-[var(--color-surface-2)] overflow-hidden">
                      <div className="h-full transition-[width] duration-200" style={{ width: `${match === null ? 0 : pct(match)}%`, background: verdictColor }} />
                      <div className="absolute top-0 bottom-0 w-0.5 bg-[var(--color-text)]/70" style={{ left: `${pct(state.threshold)}%` }} title={t("voiceGate.thresholdMark")} />
                    </div>
                    <div className="flex items-center justify-between text-[11px]">
                      <span style={{ color: verdictColor }} className="font-medium">
                        {verdict === null ? t("voiceGate.speakNow") : verdict === "yes" ? t("voiceGate.matchYes") : verdict === "close" ? t("voiceGate.matchClose") : t("voiceGate.matchNo")}
                      </span>
                      {match !== null && <span className="mono tabular-nums text-[var(--color-muted)]">{t("voiceGate.matchPct", { pct: Math.round(match * 100) })}</span>}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {/* 2 — start on a wake phrase, in any language */}
      <div className={card}>
        <SettingSwitch
          icon={KeyRound}
          label={t("voiceGate.byWord")}
          hint={state.enabled ? t("voiceGate.byWordHintVoice") : t("voiceGate.byWordHint")}
          on={wakeActive}
          onToggle={toggleWake}
        />
        {wakeActive && (
          <div className="border-t border-[var(--color-border)] px-3 py-3 space-y-2">
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-[11px] text-[var(--color-muted)] shrink-0">{t("voiceGate.wakePhrase")}</span>
              <input
                type="text" list="voice-gate-wake-words" placeholder={t("voiceGate.wakePlaceholder")}
                defaultValue={state.wake_word ?? ""}
                onBlur={(e) => { const v = e.target.value.trim(); if (v !== (state.wake_word ?? "")) patch({ wake_word: v || null }); }}
                onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
                className={`${field} flex-1 min-w-40 max-w-xs`} aria-label={t("voiceGate.wakePhrase")}
              />
              <datalist id="voice-gate-wake-words">
                {(state.wake_words ?? []).map((w) => <option key={w} value={w} />)}
              </datalist>
              <select
                value={wakeLang} onChange={(e) => patch({ wake_lang: e.target.value })}
                className={field} aria-label={t("voiceGate.wakeLang")} title={t("voiceGate.wakeLang")}
              >
                {WAKE_LANGS.map(([code, bcp]) => <option key={code} value={code}>{langNames.of(bcp)}</option>)}
              </select>
            </div>
            {wakeJob ? (
              <DownloadProgress job={wakeJob} onPause={() => { void api.setupCancel(); }} onResume={(ids) => { void api.setupDownload(ids); }} />
            ) : state.wake_downloading ? (
              <p className="text-[11px] leading-snug text-[var(--color-accent-2)]">{t("voiceGate.wakeDownloading")}</p>
            ) : null}

            {/* live phrase check: say the phrase and watch whether, and how strongly, it is recognised */}
            {state.wake_word?.trim() && !wakeJob && !state.wake_downloading && (
              <div className="pt-1 space-y-2">
                <div className="flex items-center gap-3">
                  <div className="min-w-0 flex-1">
                    <span className="block text-[13px] font-medium">{t("voiceGate.testWake")}</span>
                    <span className="block text-[11px] leading-snug text-[var(--color-muted)]">{t("voiceGate.testWakeHint")}</span>
                  </div>
                  <button onClick={() => void toggleWakeTest()} disabled={recording}
                    className={`${small} inline-flex items-center gap-1.5 ${wakeTesting ? "border-[var(--color-accent)] text-[var(--color-accent)]" : ""}`}>
                    {wakeTesting ? <X size={13} /> : <AudioLines size={13} />}{wakeTesting ? t("voiceGate.testStop") : t("voiceGate.testStart")}
                  </button>
                </div>
                {wakeTesting && (
                  <div className="space-y-2">
                    <LevelMeter peak={() => level.current} label={t("talk.inputLevel")} />
                    <div className="relative h-2.5 rounded-full bg-[var(--color-surface-2)] overflow-hidden">
                      <div className="h-full transition-[width] duration-200" style={{ width: `${wakeMatch === null ? 0 : pctWake(wakeMatch)}%`, background: wakeHit ? "var(--color-good)" : "var(--color-cue-rate)" }} />
                      <div className="absolute top-0 bottom-0 w-0.5 bg-[var(--color-text)]/70" style={{ left: `${pctWake(wakeThresh)}%` }} title={t("voiceGate.thresholdMark")} />
                    </div>
                    <div className="flex items-center justify-between text-[11px]">
                      <span style={{ color: wakeHit ? "var(--color-good)" : "var(--color-muted)" }} className="font-medium">
                        {wakeMatch === null ? t("voiceGate.wakeSay") : wakeHit ? t("voiceGate.wakeHit") : t("voiceGate.wakeMiss")}
                      </span>
                      {wakeMatch !== null && <span className="mono tabular-nums text-[var(--color-muted)]">{t("voiceGate.matchPct", { pct: Math.round(wakeMatch * 100) })}</span>}
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>

      <p className="max-w-2xl text-[11px] leading-snug text-[var(--color-muted)]">{t("voiceGate.privacy")}</p>
      {problem && <p role="alert" className="text-[11px] text-[var(--color-warn)]">{problem}</p>}
    </div>
  );
}
