import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { api, SELECTION_CHANGED } from "./lib/api";
import { SETTINGS_CHANGED } from "./lib/mcpBridge";
import { useStore } from "./store";
import { hubChanged, setTelemetry, showsNow, useHubState, type HubView } from "./lib/studioHub";
import { HubBars } from "./components/HubBars";
import HubPopup from "./components/HubPopup";
import ServerOffline from "./components/ServerOffline";
import ResourceMonitor from "./components/ResourceMonitor";
import BridgeHost from "./components/BridgeHost";
import CloudConsentHost from "./components/CloudConsentHost";
import TopBar from "./components/shell/TopBar";
import Hotkeys from "./components/shell/Hotkeys";
import FirstRun from "./components/setup/FirstRun";
import ListeningPage from "./components/listening/ListeningPage";
import SpeechPage from "./components/speech/SpeechPage";
import TalkPage from "./components/talk/TalkPage";
import { aloudPlayback, useAloudPlayback } from "./lib/livePlayback";
import { serveTalkFeed } from "./lib/talkFeed";
import { applyUiScale, scaleForKey, setUiScale, storedUiScale, UI_SCALE_EVENT } from "./lib/uiScale";

const modelName = (p?: string) => (p ? (p.split(/[\\/]/).pop() || p).replace(/\.(gguf|onnx|bin|pt|safetensors)$/i, "") : "");

export default function App() {
  const { t, i18n } = useTranslation();
  const stage = useStore((s) => s.stage);
  const mode = useStore((s) => s.mode);
  const setStage = useStore((s) => s.setStage);
  // Анонимная статистика включена по умолчанию: на первом запуске решает галочка «Первого запуска», а установка,
  // обновившаяся мимо него, остаётся при умолчании, пока галочку не снимут в Настройках.
  const hub = useHubState(i18n.language);
  useAloudPlayback();
  useEffect(serveTalkFeed, []);
  // The interface scale kept from the last session, and Ctrl with plus, minus, nought or the wheel to change it.
  useEffect(() => {
    let scale = storedUiScale();
    applyUiScale(scale);
    const change = (next: number | null) => {
      if (next === null || next === scale) return;
      scale = setUiScale(next);
    };
    const onKey = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      const next = scaleForKey(event.key, scale);
      if (next === null) return;
      event.preventDefault();
      change(next);
    };
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey || event.deltaY === 0) return;
      event.preventDefault();
      change(scaleForKey(event.deltaY < 0 ? "+" : "-", scale));
    };
    const onScale = (event: Event) => { scale = (event as CustomEvent<number>).detail; };
    window.addEventListener("keydown", onKey);
    window.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener(UI_SCALE_EVENT, onScale);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("wheel", onWheel);
      window.removeEventListener(UI_SCALE_EVENT, onScale);
    };
  }, []);
  useEffect(() => {
    const tel = hub?.telemetry;
    if (stage === "setup" || !tel || tel.acknowledged || tel.disabledByEnv) return;
    setTelemetry(tel.enabled, true).catch((e: Error) => console.warn("[hub] the statistics default was not saved:", e.message)).finally(hubChanged);
  }, [stage, hub?.telemetry.acknowledged]);   // eslint-disable-line react-hooks/exhaustive-deps
  const [cap, setCap] = useState<{ device: string; tts: string; rest: string[] } | null>(null);
  const speechEngine = useStore((s) => s.speechEngine);
  const [capOffline, setCapOffline] = useState(false);
  useEffect(() => {
    if (stage !== "ready") return;
    const load = () => api.capabilities().then((c) => {
      const rest = [`ASR ${c.asr_label}`];
      const llm = modelName(c.models?.llm);
      if (c.llm_provider === "agent") rest.push(`LLM ${t("providers.agent")}`);
      else if (c.llm_provider === "openrouter") rest.push(`LLM ${llm ? `${llm} (OpenRouter)` : "OpenRouter"}`);
      else if (c.llm_provider === "server") rest.push(`LLM ${llm ? `${llm} (${t("providers.server")})` : t("providers.server")}`);
      else if (llm) rest.push(`LLM ${llm}`);
      const voice = modelName(c.models?.tts);
      const cloud = c.tts_provider === "openrouter" ? "OpenRouter" : c.tts_provider === "google" ? "Google" : "";
      const tts = cloud ? (voice ? `${voice} (${cloud})` : cloud) : voice ? `${c.models?.tts}${c.tts_quant ? ` ${c.tts_quant}` : ""}` : "";
      setCap({ device: c.device, tts, rest });
      setCapOffline(false);
    }).catch(() => setCapOffline(true));
    load();
    window.addEventListener(SELECTION_CHANGED, load);
    window.addEventListener(SETTINGS_CHANGED, load);
    return () => {
      window.removeEventListener(SELECTION_CHANGED, load);
      window.removeEventListener(SETTINGS_CHANGED, load);
    };
  }, [stage, t]);
  const tts = mode === "speech" && speechEngine ? speechEngine : cap?.tts;
  const capText = cap ? [cap.device, ...(tts ? [`TTS ${tts}`] : []), ...cap.rest].join(" · ") : "";
  // Сетевая ошибка (fetch отклонён TypeError) = сервиса нет на порту -> экран «сервер не отвечает»; HTTP-ошибка
  // значит, что сервис ответил, и ведёт к режимам.
  useEffect(() => {
    api.setupStatus()
      .then((s) => setStage(s.ready ? "ready" : "setup"))
      .catch((e: unknown) => setStage(e instanceof TypeError ? "offline" : "ready"));
  }, [setStage]);
  // the hub's sections: Speech is where a work is made, Listening and Live are tools, the open window names its own
  const overlay = useStore((s) => s.overlay);
  const progress = useStore((s) => s.progress);
  const view: HubView = overlay ?? (mode === "speech" ? "create" : "tools");
  const busy = progress.pct != null || (!!progress.stage && !["", "done", "error"].includes(progress.stage));
  const ready = stage === "ready";
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    if (!ready) return;
    const readyAt = Date.now();
    const tick = window.setInterval(() => setElapsed(Math.floor((Date.now() - readyAt) / 1000)), 1000);
    return () => window.clearInterval(tick);
  }, [ready]);
  const [popupClosed, setPopupClosed] = useState<Set<string>>(() => new Set());
  // a popup waits while something is being made: a job, or the agent's turn said aloud
  const popup = ready && hub && !busy && !aloudPlayback()?.speaking
    ? hub.items.find((item) => item.kind === "popup" && !popupClosed.has(item.id) && showsNow(item, elapsed, view))
    : undefined;
  const closePopup = (id: string) => {
    setPopupClosed((previous) => new Set(previous).add(id));
    hubChanged();
  };
  return (
    <div className="h-full flex flex-col">
      {hub && stage !== "setup" && <HubBars items={hub.items} view={view} />}
      {popup && <HubPopup item={popup} onClose={() => closePopup(popup.id)} />}
      <TopBar />
      {stage === "boot" && <div className="flex-1 grid place-items-center"><Loader2 size={22} className="animate-spin text-[var(--color-muted)]" /></div>}
      {stage === "offline" && <ServerOffline />}
      {stage === "setup" && <FirstRun />}
      {stage === "ready" && <div className="flex-1 min-h-0 flex flex-col lg:[[data-assistant]_&]:pr-[420px]">{mode === "speech" ? <SpeechPage /> : mode === "talk" ? <TalkPage /> : <ListeningPage />}</div>}
      <footer className="mono h-6 px-4 flex items-center gap-2 text-[10px] text-[var(--color-muted)] border-t border-[var(--color-border)] bg-[var(--color-surface)] overflow-hidden">
        <span className="w-1.5 h-1.5 rounded-full bg-[var(--color-accent)] shrink-0" /><span className="truncate" title={capOffline ? t("status.backendOffline") : capText}>{capOffline ? t("status.backendOffline") : capText}</span>
      </footer>
      <BridgeHost />
      <CloudConsentHost />
      <Hotkeys />
      {stage === "ready" && <ResourceMonitor />}
    </div>
  );
}
