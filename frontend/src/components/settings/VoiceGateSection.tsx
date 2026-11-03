import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Fingerprint, KeyRound, Mic, Trash2, X } from "lucide-react";
import SettingSwitch from "./SettingSwitch";
import LevelMeter from "../talk/LevelMeter";
import { voiceGateApi, type VoiceGate } from "../../lib/voiceGate";
import { talkApi } from "../../lib/talk";
import { errorText } from "../../lib/voices";

// Enough clean speech (seconds) for a solid speaker print: a text-dependent passphrase wants a couple of seconds at
// least (Azure Speaker Recognition, Nuance VocalPassword, Neurotechnology); we take a little more, then auto-finish.
const ENROLL_SPEECH_SECONDS = 3.5;
// RMS level above which the microphone is hearing speech rather than room noise.
const SPEECH_FLOOR = 0.06;
const POLL_MS = 120;

/// The voice gate: Live starts a conversation hands-free. Two independent ways to start it — on the owner's own voice
/// and/or on a wake phrase — that can be used alone or together. Everything is off by default; the voice print and the
/// wake model stay on the device.
export default function VoiceGateSection() {
  const { t, i18n } = useTranslation();
  const [state, setState] = useState<VoiceGate | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const level = useRef(0);
  const speechSecs = useRef(0);
  const finishing = useRef(false);

  useEffect(() => {
    voiceGateApi.get().then(setState, (e: unknown) => setProblem(errorText(e)));
  }, []);

  // While enrolling: show the level and auto-finish once the studio has heard enough clean speech — no manual button.
  useEffect(() => {
    if (!recording) {
      level.current = 0;
      speechSecs.current = 0;
      return;
    }
    const id = window.setInterval(() => {
      talkApi.level().then((l) => {
        level.current = l.level;
        if (l.level > SPEECH_FLOOR) {
          speechSecs.current += POLL_MS / 1000;
          if (speechSecs.current >= ENROLL_SPEECH_SECONDS && !finishing.current) {
            finishing.current = true;
            void finishEnroll();
          }
        }
      }, () => { level.current = 0; });
    }, POLL_MS);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [recording]);

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
    voiceGateApi.forget().then((r) => setState((s) => (s ? { ...s, enrolled: r.enrolled } : s)), (e: unknown) => setProblem(errorText(e)));
  };

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
                    <button onClick={() => void startEnroll()} disabled={busy} className={`${small} inline-flex items-center gap-1.5`}>
                      <Mic size={13} /> {state.enrolled ? t("voiceGate.reEnroll") : t("voiceGate.enroll")}
                    </button>
                    {state.enrolled && (
                      <button onClick={forget} className={`${small} inline-flex items-center gap-1.5 text-[var(--color-warn)]`}>
                        <Trash2 size={13} /> {t("voiceGate.forget")}
                      </button>
                    )}
                  </div>
                )}
              </div>
            </div>

            <div className="flex items-center gap-3 px-3 py-2.5">
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
          </div>
        )}
      </div>

      {/* 2 — start on a wake phrase, in any language */}
      <div className={`${card} px-3 py-3 space-y-2`}>
        <div className="flex items-start gap-3">
          <KeyRound size={15} className="mt-0.5 shrink-0 text-[var(--color-accent-2)]" />
          <div className="min-w-0 flex-1">
            <span className="block text-[13px] font-medium">{t("voiceGate.byWord")}</span>
            <span className="block text-[11px] leading-snug text-[var(--color-muted)]">
              {state.enabled ? t("voiceGate.byWordHintVoice") : t("voiceGate.byWordHint")}
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2 pl-[27px]">
          <input
            type="text" list="voice-gate-wake-words" placeholder={t("voiceGate.anySpeech")}
            defaultValue={state.wake_word ?? ""}
            onBlur={(e) => { const v = e.target.value.trim(); if (v !== (state.wake_word ?? "")) patch({ wake_word: v || null }); }}
            onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }}
            className={`${field} flex-1 max-w-xs`} aria-label={t("voiceGate.byWord")}
          />
          <datalist id="voice-gate-wake-words">
            {(state.wake_words ?? []).map((w) => <option key={w} value={w} />)}
          </datalist>
          {state.wake_word && (
            <select
              value={wakeLang} onChange={(e) => patch({ wake_lang: e.target.value })}
              className={field} aria-label={t("voiceGate.wakeLang")} title={t("voiceGate.wakeLang")}
            >
              {WAKE_LANGS.map(([code, bcp]) => <option key={code} value={code}>{langNames.of(bcp)}</option>)}
            </select>
          )}
        </div>
        {state.wake_downloading && (
          <p className="pl-[27px] text-[11px] leading-snug text-[var(--color-accent-2)]">{t("voiceGate.wakeDownloading")}</p>
        )}
      </div>

      <p className="max-w-2xl text-[11px] leading-snug text-[var(--color-muted)]">{t("voiceGate.privacy")}</p>
      {problem && <p role="alert" className="text-[11px] text-[var(--color-warn)]">{problem}</p>}
    </div>
  );
}
