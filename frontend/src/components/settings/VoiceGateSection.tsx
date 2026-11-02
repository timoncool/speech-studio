import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Fingerprint, Mic, Rocket, Square, Trash2 } from "lucide-react";
import SettingSwitch from "./SettingSwitch";
import LevelMeter from "../talk/LevelMeter";
import { voiceGateApi, type VoiceGate } from "../../lib/voiceGate";
import { talkApi } from "../../lib/talk";
import { errorText } from "../../lib/voices";

/// The voice gate: Live starts a conversation only on the owner's voice (and, when set, a wake word). Everything is
/// off by default; the owner enrols their voice once, and the audio never leaves the device.
export default function VoiceGateSection() {
  const { t } = useTranslation();
  const [state, setState] = useState<VoiceGate | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const [busy, setBusy] = useState(false);
  const level = useRef(0);

  useEffect(() => {
    voiceGateApi.get().then(setState, (e: unknown) => setProblem(errorText(e)));
  }, []);

  useEffect(() => {
    if (!recording) return;
    const id = window.setInterval(() => { talkApi.level().then((l) => { level.current = l.level; }, () => { level.current = 0; }); }, 80);
    return () => window.clearInterval(id);
  }, [recording]);

  const patch = (p: Partial<Pick<VoiceGate, "enabled" | "threshold" | "wake_word" | "autostart">>) => {
    setProblem(null);
    voiceGateApi.set(p).then(setState, (e: unknown) => setProblem(errorText(e)));
  };

  const startEnroll = async () => {
    setProblem(null);
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
    }
  };

  const forget = () => {
    setProblem(null);
    voiceGateApi.forget().then((r) => setState((s) => (s ? { ...s, enrolled: r.enrolled } : s)), (e: unknown) => setProblem(errorText(e)));
  };

  if (!state) return <p className="text-[11px] text-[var(--color-muted)]">…</p>;

  const field = "shrink-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px]";
  const small = "shrink-0 rounded-md border border-[var(--color-border)] px-2.5 py-1 text-[11px] text-[var(--color-text)] hover:border-[var(--color-accent)] disabled:opacity-40";

  return (
    <div className="space-y-3">
      <div className="max-w-2xl rounded-lg border border-[var(--color-border)] divide-y divide-[var(--color-border)] bg-[var(--color-surface-2)]/40">
        <SettingSwitch
          icon={Fingerprint}
          label={t("voiceGate.enable")}
          hint={t("voiceGate.enableHint")}
          on={state.enabled}
          onToggle={() => patch({ enabled: !state.enabled })}
        />

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
                <button onClick={() => void finishEnroll()} disabled={busy} className={`${small} inline-flex items-center gap-1.5`}>
                  <Square size={13} /> {t("voiceGate.enrollDone")}
                </button>
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

        <div className="flex items-center gap-3 px-3 py-2.5">
          <div className="min-w-0 flex-1">
            <span className="block text-[13px] font-medium">{t("voiceGate.wakeWord")}</span>
            <span className="block text-[11px] leading-snug text-[var(--color-muted)]">
              {state.wake_words.length ? t("voiceGate.wakeWordHint") : t("voiceGate.wakeWordSoon")}
            </span>
          </div>
          <select
            value={state.wake_word ?? ""} disabled={!state.wake_words.length}
            onChange={(e) => patch({ wake_word: e.target.value || null })} className={field} aria-label={t("voiceGate.wakeWord")}
          >
            <option value="">{t("voiceGate.anySpeech")}</option>
            {state.wake_words.map((w) => <option key={w} value={w}>{w}</option>)}
          </select>
        </div>

        <SettingSwitch
          icon={Rocket}
          label={t("voiceGate.autostart")}
          hint={t("voiceGate.autostartHint")}
          on={state.autostart}
          onToggle={() => patch({ autostart: !state.autostart })}
        />
      </div>

      <p className="max-w-2xl text-[11px] leading-snug text-[var(--color-muted)]">{t("voiceGate.privacy")}</p>
      {problem && <p role="alert" className="text-[11px] text-[var(--color-warn)]">{problem}</p>}
    </div>
  );
}
