import { useEffect, useRef, useState } from "react";
import { ASR_VARIANT_I18N, activeVariantId, type VariantI18n } from "../../lib/modelVariants";
import { useTranslation } from "react-i18next";
import { Download, FolderDown, RefreshCw, X } from "lucide-react";
import { api, llmProviderOf, SELECTION_CHANGED, slot, SetupError, type Capabilities, type SetupComponent } from "../../lib/api";
import { openSettings } from "../../lib/settingsNav";
import { SETTINGS_CHANGED, useChanged } from "../../lib/mcpBridge";
import { useSetupStatus } from "../../lib/useSetupStatus";
import { useDownloadErrorText, useGpuReasonText } from "../../lib/setupText";
import { fmtBytes } from "../../lib/format";
import OpenRouterKey from "../OpenRouterKey";
import GoogleTts, { GoogleKey } from "../GoogleTts";
import OpenRouterModelSelect, { OpenRouterCatalogRow } from "../OpenRouterModelSelect";
import LlmProviders from "../LlmProviders";
import YtDlpTool from "../YtDlpTool";
import ModelsFolder from "../ModelsFolder";
import GpuNotice from "../GpuNotice";
import DownloadProgress from "../DownloadProgress";
import RemoveComponent from "../RemoveComponent";
import ConfirmDialog from "../ConfirmDialog";
import Hint from "../Hint";
import { takenFromLabel } from "../../lib/programs";

// Модели и компоненты в настройках: список из /setup/status с кнопками скачки/докачки и прогрессом.
// part="cloud" — раздел «Облако» (ключ OpenRouter и потоки) на том же состоянии выбора.

export default function ModelsSection({ part = "models" }: { part?: "models" | "cloud" }) {
  const { t } = useTranslation();
  const errText = useDownloadErrorText();
  const gpuReasonText = useGpuReasonText();
  // Выбор ASR-движка (parakeet | nemotron | whisper = CrisperWhisper) — из capabilities.selection.
  const [cap, setCap] = useState<Capabilities | null>(null);
  const [asrEngine, setAsrEngine] = useState<string>("parakeet");
  const [dramaVram, setDramaVram] = useState<string>("0");
  // Сохранения по очереди: при быстрой смене (стрелками) последним на сервер приходит последнее значение.
  const dramaSaves = useRef<Promise<unknown>>(Promise.resolve());
  // Выбор варианта в дропдаунах поднят СЮДА (а не в локальный useState VariantPicker) — иначе ре-рендер
  // секции ремаунтил пикер и сбрасывал выбор на дефолт. Ключ = ids[0] группы.
  const [picks, setPicks] = useState<Record<string, string>>({});
  const [err, setErr] = useState<string | null>(null);            // ошибки скачки/импорта — показываем, не глотаем
  // Облачные движки OpenRouter — НЕ отдельный блок, а альтернатива локальному движку ВНУТРИ каждой группы
  // (перевод: Gemma|свой сервер|OpenRouter, TTS: Higgs|OpenRouter), как Parakeet|Whisper в ASR. Ключ общий на все стадии.
  const [orVoices, setOrVoices] = useState<{ name: string; gender: string; age: string; ru: boolean }[]>([]);
  const [ttsRu, setTtsRu] = useState<boolean | null>(null);
  const loadCap = () => api.capabilities().then((c) => {
    setCap(c);
    setAsrEngine(slot(c.selection, "asr_engine") ?? "parakeet");
    setDramaVram(slot(c.selection, "drama_vram_gb") || "0");
  }).catch(() => {});
  // Статус компонентов + фоновая закачка (опрос, пока она идёт). Скачанный вариант становится активным на
  // бэке — после закачки перечитываем выбор.
  const { status, error: statusErr, refresh: refreshStatus, adopt, downloading } = useSetupStatus(() => { loadCap(); });
  const prog = downloading ? status?.active ?? null : null;
  const partPct = (id: string) => {
    const p = prog?.parts.find((x) => x.id === id);
    return p && p.total > 0 ? (p.done / p.total) * 100 : prog?.ids.includes(id) ? 0 : null;
  };
  useEffect(() => { loadCap(); }, []);
  const selv = (k: string) => slot(cap?.selection, k) ?? "";
  const hasOrKey = cap?.selection?.or_key_set === true;
  // the status follows too: which engine build and which components a choice owes is the server's answer
  const setSel = (k: string, v: string) => api.setSelection(k, v).catch(() => {});
  // Голоса выбранной облачной TTS-модели (пол/возраст/русский) для дропдауна + предупреждения о русском.
  const ttsProvider = slot(cap?.selection, "tts_provider") ?? (selv("or_tts_on") === "1" ? "openrouter" : "local");
  const orTtsModel = slot(cap?.selection, ttsProvider === "google" ? "google_tts_model" : "or_tts_model") ?? "";
  useEffect(() => {
    let active = true;
    const voices = orTtsModel ? api.openrouterVoices(orTtsModel) : Promise.resolve({voices:[],supportsRussian:null});
    voices.then((r) => { if (active) { setOrVoices(r.voices); setTtsRu(r.supportsRussian); } }).catch((e:unknown)=>{ if (active) setErr(e instanceof Error ? e.message : String(e)); });
    return () => { active = false; };
  }, [orTtsModel]);
  const setupErr = (e: unknown) => setErr(e instanceof SetupError ? `${errText(e.code)} · ${e.detail}` : e instanceof Error ? e.message : String(e));
  const [asking, setAsking] = useState<string[] | null>(null);
  const dl = (ids: string[]) => { if (!downloading) setAsking(ids); };
  const startDownload = async (ids: string[]) => {
    setAsking(null);
    setErr(null);
    try { await api.setupDownload(ids); await refreshStatus(); } catch (e) { setupErr(e); }
  };
  const confirmDownload = () => {
    if (!asking || !status) return null;
    const comps = asking.map((id) => status.components.find((c) => c.id === id)).filter((c): c is SetupComponent => !!c);
    const size = comps.reduce((n, c) => n + Math.max(0, c.size - c.bytesOnDisk), 0);
    const need = comps.reduce((n, c) => n + c.spaceNeeded, 0);
    const vram = Math.max(0, ...comps.map((c) => c.vram ?? 0));
    const fits = status.freeBytes == null || need <= status.freeBytes;
    const message = [
      t("settings.dlSize", { size: fmtBytes(size), need: fmtBytes(need) }),
      status.freeBytes != null ? t(fits ? "settings.dlFree" : "settings.dlNoSpace", { free: fmtBytes(status.freeBytes) }) : "",
      vram ? t("settings.dlVram", { vram: fmtBytes(vram) }) : "",
    ].filter(Boolean).join(" ");
    return <ConfirmDialog title={t("settings.dlTitle", { name: comps.map((c) => c.name).join(", ") })} message={message} confirmLabel={t("setup.downloadOne")}
      disabled={!fits} onConfirm={() => startDownload(asking)} onCancel={() => setAsking(null)} />;
  };
  const pause = () => { api.setupCancel().then(() => refreshStatus()).catch(setupErr); };
  useChanged(SETTINGS_CHANGED, () => { loadCap(); refreshStatus().catch(setupErr); });
  useChanged(SELECTION_CHANGED, () => { loadCap(); refreshStatus().catch(setupErr); });
  const browseId = async (id: string) => {
    if (prog) return;
    try {
      const r = await api.setupBrowse(id);
      if (r.picked) adopt(r.status);
      if (r.errors.length > 0) setErr(`${t("downloads.importErrors")} · ${r.errors.join("; ")}`);
    } catch (e) { setupErr(e); }
  };
  const browseBtn = (id: string) => (
    <button onClick={() => browseId(id)} disabled={!!prog} title={t("settings.browseFolder")}
      className="shrink-0 inline-flex items-center px-1.5 py-1 rounded-md border border-[var(--color-border)] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-text)] disabled:opacity-40">
      <FolderDown size={12} />
    </button>
  );
  if (part === "cloud") return (
    <div className="max-w-2xl space-y-2">
      <div data-settings-part="key"><OpenRouterKey onSaved={loadCap} /></div>
      <GoogleKey onSaved={loadCap} />
      {hasOrKey && <OpenRouterCatalogRow />}
      {hasOrKey && (
        <div className="px-2.5 py-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
          <div className="flex items-center gap-2.5">
            <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-[var(--color-muted)]" />
            <div className="min-w-0 flex-1">
              <div className="text-[12px] font-medium truncate">{t("cloud.threads")} <Hint text={t("cloud.threadsHint")} /></div>
            </div>
            <select value={selv("or_concurrency") || "6"} onChange={(e) => setSel("or_concurrency", e.target.value)}
              className="shrink-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[11px] mono focus:border-[var(--color-accent)] focus:outline-none">
              {["1", "2", "4", "6", "8", "12", "16"].map((o) => <option key={o} value={o}>{o}</option>)}
            </select>
          </div>
        </div>
      )}
    </div>
  );

  if (!status) return <div className="mono text-[11px] text-[var(--color-muted)]">{statusErr ?? "…"}</div>;
  const byId = Object.fromEntries(status.components.map((c) => [c.id, c]));
  const get = (id: string) => byId[id] as SetupComponent | undefined;

  // строка одного компонента (кнопка скачать/докачать + прогресс)
  const rowView = (c: SetupComponent) => {
    const pct = partPct(c.id);
    const active = pct != null;
    return (
      <div key={c.id} className="flex items-center gap-2.5 px-2.5 py-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
        <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${c.installed ? "bg-[var(--color-accent)]" : "bg-[var(--color-muted)]"}`} />
        <div className="min-w-0 flex-1">
          <div className="text-[12px] font-medium truncate">{c.name} {c.purpose && <Hint text={c.purpose} />}</div>
          <div className="mono text-[10px] text-[var(--color-muted)] break-words">{c.vram ? `${fmtBytes(c.vram)} VRAM · ` : ""}{fmtBytes(c.size)} {t("settings.disk")}</div>
          {c.takenFrom && <div className="text-[10px] text-[var(--color-accent)]">{takenFromLabel(t, c.takenFrom)}</div>}
        </div>
        {active ? (
          <div className="w-24 shrink-0">
            <div className="h-1.5 rounded-full bg-white/10 overflow-hidden"><div className="h-full bg-[var(--color-accent)] transition-[width]" style={{ width: `${pct ?? 0}%` }} /></div>
            <div className="mono text-[10px] text-[var(--color-muted)] text-right mt-0.5">{Math.round(pct ?? 0)}%</div>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 shrink-0">
            <RemoveComponent c={c} disabled={!!prog} onDone={(s) => { adopt(s); loadCap(); }} onError={setErr} />
            {browseBtn(c.id)}
            {c.fitsVram === false && !c.installed
              ? <span className="text-[11px] text-[var(--color-warn)]">{t("setup.noVramShort")}</span>
              : <button onClick={() => dl([c.id])} disabled={!!prog}
                  className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-[12px] border border-[var(--color-border)] text-[var(--color-accent-2)] hover:border-[var(--color-accent)] disabled:opacity-40">
                  {c.installed ? <RefreshCw size={12} /> : <Download size={12} />}{c.installed ? t("settings.redownload") : t("setup.downloadOne")}
                </button>}
          </div>
        )}
      </div>
    );
  };

  // модель с выбором кванта: дропдаун вариантов + скачать выбранный. КОНТРОЛИРУЕМЫЙ — выбранное значение
  // берётся из поднятого picks / активного выбора (active.json), НЕ из внутреннего useState (иначе ре-рендер
  // секции сбрасывал бы дропдаун на дефолт). При смене — пишем picks и активируем на бэке (если установлен).
  const variantPicker = (base: string, ids: string[], i18n?: VariantI18n) => {
    const variants = ids.map(get).filter(Boolean) as SetupComponent[];
    const installed = variants.find((v) => v.installed);
    const sel = cap?.selection ?? {};
    const pick = picks[ids[0]] ?? activeVariantId(ids, sel) ?? installed?.id ?? variants.find((v) => v.fitsVram !== false)?.id ?? variants[0]?.id ?? "";
    const c = get(pick);
    if (!c) return null;
    const quant = (v: SetupComponent) => i18n?.[v.id] ? t(i18n[v.id].label) : (v.name.match(/\b(q\d[\w]*|int8|fp32|f16|large-v3-turbo|large-v3|tiny|base|small|medium)\b/i)?.[1] ?? v.name);
    const hint = i18n?.[c.id] ? t(i18n[c.id].hint) : "";
    const pct = partPct(c.id);
    const active = pct != null;
    return (
      <div className="px-2.5 py-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
        <div className="flex items-center gap-2.5">
          <span className={`w-1.5 h-1.5 rounded-full shrink-0 ${variants.some((v) => v.installed) ? "bg-[var(--color-accent)]" : "bg-[var(--color-muted)]"}`} />
          <div className="min-w-0 flex-1">
            <div className="text-[12px] font-medium truncate">{base} {hint && <Hint text={hint} />}</div>
            <div className="mono text-[10px] text-[var(--color-muted)] break-words">{c.vram ? `${fmtBytes(c.vram)} VRAM · ` : ""}{fmtBytes(c.size)} {t("settings.disk")}{c.installed ? "" : ` · ${t("settings.notInstalled")}`}</div>
          </div>
          <select value={pick} onChange={(e) => {
              const id = e.target.value;
              setPicks((p) => ({ ...p, [ids[0]]: id }));
              const sv = get(id);
              // установлен -> активируем сразу; не установлен -> активируется автоматически после скачки
              if (sv?.installed) api.selectModel(id).then(loadCap).catch((er) => setErr(er instanceof Error ? er.message : String(er)));
            }}
            className="shrink-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[11px] mono focus:border-[var(--color-accent)] focus:outline-none">
            {variants.map((v) => <option key={v.id} value={v.id} disabled={v.fitsVram === false && !v.installed}>{quant(v)}{v.installed ? " ✓" : ""} · {fmtBytes(v.size)}{v.vram ? ` / ${fmtBytes(v.vram)} VRAM` : ""}{v.fitsVram === false ? ` · ${t("setup.noVramShort")}` : ""}</option>)}
          </select>
          {active ? (
            <span className="mono text-[11px] text-[var(--color-accent)] w-10 text-right">{Math.round(pct ?? 0)}%</span>
          ) : (
            <>
              <RemoveComponent c={c} disabled={!!prog} onDone={(s) => { adopt(s); loadCap(); }} onError={setErr} />
              {browseBtn(c.id)}
              <button onClick={() => dl([c.id])} disabled={!!prog || (c.fitsVram === false && !c.installed)} title={c.fitsVram === false && !c.installed ? t("setup.noVram") : c.installed ? t("settings.redownload") : t("setup.downloadOne")}
                className="shrink-0 inline-flex items-center gap-1 px-2 py-1 rounded-md text-[12px] border border-[var(--color-border)] text-[var(--color-accent-2)] hover:border-[var(--color-accent)] disabled:opacity-40">
                {c.installed ? <RefreshCw size={12} /> : <Download size={12} />}
              </button>
            </>
          )}
        </div>
      </div>
    );
  };

  const group = (label: string, children: React.ReactNode) => (
    <div className="mb-3">
      <div className="text-[11px] uppercase tracking-[0.14em] text-[var(--color-muted)] mb-1.5">{label}</div>
      <div className="space-y-1.5">{children}</div>
    </div>
  );

  const rowOf = (id: string) => { const c = get(id); return c ? rowView(c) : null; };
  // the build of an engine the voicing device runs; the device «auto» picks is the component itself
  const voicingBuild = (component: string, prefix: string) => {
    const device = selv("tts_backend");
    const build = device === "cpu" ? `${prefix}-cpu` : device === "vulkan" ? `${prefix}-vulkan` : null;
    return build && get(build) ? build : component;
  };

  // Тумблер движка «локально | OpenRouter» — тот же вид, что переключатель Parakeet|Whisper в группе ASR.
  const orRowCls = "px-2.5 py-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]";
  // На чём считать стадию (устройство): Авто / GPU (CUDA) / Vulkan / CPU — свои табы в каждом разделе, по
  // аналогии с провайдерами. Независимый ключ на стадию (tts_backend / sep_backend / diar_backend / asr_backend).
  // Озвучка, распознавание, сепарация и диаризация идут на audio.cpp, transcribe.cpp и BSRoformer, у которых
  // есть сборки Vulkan и CUDA 12.9.
  const backendTabs = (k: string) => {
    const cur = selv(k) || "auto";
    const gpuOff = !status.gpu.cudaBuild;
    return (
      <div className="flex gap-1 mb-1.5">
        {([["auto", t("settings.auto")], ["gpu", "GPU (CUDA)"], ["vulkan", "Vulkan"], ["cpu", "CPU"]] as const).map(([id, label]) => (
          <button key={id} onClick={() => setSel(k, id)} disabled={id === "gpu" && gpuOff && cur !== "gpu"}
            title={id === "gpu" && gpuOff ? gpuReasonText(status.gpu) : undefined}
            className={`flex-1 px-2 py-1.5 rounded-md text-[12px] font-medium border transition-colors ${cur === id ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_14%,transparent)] text-[var(--color-text)]" : "border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]"} disabled:opacity-40`}>{label}</button>
        ))}
      </div>
    );
  };
  const orSelectCls = "w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[11px] mono focus:border-[var(--color-accent)] focus:outline-none";
  // Модели OpenRouter стадии: имя, цена, контекст — из каталога (кэш на сервере).
  const orModelSelect = (kind: "tts" | "asr", k: string, empty: string) => (
    <OpenRouterModelSelect kind={kind} value={selv(k)} onChange={(id) => { if (id) setSel(k, id); }} placeholder={empty} />
  );
  const needsGemma = llmProviderOf(cap?.selection, "llm") === "local";

  const browse = async () => {
    if (prog) return;
    try {
      const r = await api.setupBrowse();
      if (r.picked) adopt(r.status);
      if (r.errors.length > 0) setErr(`${t("downloads.importErrors")} · ${r.errors.join("; ")}`);
    } catch (e) { setupErr(e); }
  };
  const driverUrl = get("nvidia-driver")?.externalUrl;

  return (
    <div>
      {confirmDownload()}
      <div className="mb-3 space-y-2">
        <ModelsFolder status={status} />
        <GpuNotice gpu={status.gpu} driverUrl={driverUrl} />
        {(status.gpu.cards.length > 1 || status.gpu.reason === "gpu_missing") && (
          <label className="block px-2.5 py-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] text-[12px]">
            <span className="font-medium">{t("downloads.gpuCard")}</span>
            <select value={selv("gpu_index") || "auto"} onChange={(e) => setSel("gpu_index", e.target.value)} className={`mt-1 ${orSelectCls}`}>
              <option value="auto">{t("downloads.gpuCardAuto")}</option>
              {status.gpu.cards.map((c) => <option key={c.index} value={String(c.index)}>{`${c.index}: ${c.name} · ${Math.round(c.memoryMb / 1024)} ${t("units.gb")}`}</option>)}
              {status.gpu.chosen != null && !status.gpu.cards.some((c) => c.index === status.gpu.chosen) && <option value={String(status.gpu.chosen)} disabled>{`${status.gpu.chosen}: —`}</option>}
            </select>
            <span className="block mt-1 text-[11px] text-[var(--color-muted)]">{t("downloads.gpuCardHint")}</span>
          </label>
        )}
        {status.active && status.active.status !== "completed" && (
          <DownloadProgress job={status.active} onPause={pause} onResume={(ids) => { void startDownload(ids); }} />
        )}
      </div>
      <button onClick={browse} disabled={!!prog}
        className="mb-3 w-full inline-flex items-center justify-center gap-2 px-3 py-2 rounded-lg border border-dashed border-[var(--color-border)] text-[12px] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-text)] transition-colors disabled:opacity-40">
        <FolderDown size={14} />{t("settings.browseFolder")}
      </button>
      {err && (
        <div className="mb-3 flex items-start gap-2 px-2.5 py-2 rounded-lg border border-[var(--color-danger,#ef4444)]/40 bg-[color-mix(in_oklab,#ef4444_10%,transparent)] text-[11px]">
          <span className="mono text-[var(--color-danger,#ef4444)] break-words flex-1">{err}</span>
          <button onClick={() => setErr(null)} className="shrink-0 text-[var(--color-muted)] hover:text-[var(--color-text)]"><X size={12} /></button>
        </div>
      )}
      {group(t("settings.roleTts"), <>
        <div className="flex gap-1 mb-2">
          {(["local", "openrouter", "google"] as const).map(provider => <button key={provider} onClick={() => api.setSelection("tts_provider",provider).then(loadCap).catch(e=>setErr(String(e)))}
            className={`flex-1 px-2 py-1.5 rounded-md border text-[12px] ${ttsProvider === provider ? "border-[var(--color-accent)]" : "border-[var(--color-border)]"}`}>{t(`google.provider_${provider}`)}</button>)}
        </div>
        {ttsProvider !== "local" ? (
          <div className={`${orRowCls} space-y-2`}>
            {ttsProvider === "google" ? <GoogleTts model={selv("google_tts_model")} mode={selv("google_tts_mode")} onChange={(key,value)=>{ api.setSelection(key,value).then(loadCap).catch(e=>setErr(String(e))); }} /> : orModelSelect("tts", "or_tts_model", t("providers.pickTtsModel"))}
            {ttsRu === false && <div className="text-[11px] text-[var(--color-warn)]">{t("cloud.ttsNoRussian")}</div>}
            <div className="flex items-center gap-2">
              <button onClick={() => setSel("or_tts_autocast", (selv("or_tts_autocast") || "1") !== "0" ? "0" : "1")}
                role="switch" aria-checked={(selv("or_tts_autocast") || "1") !== "0"} aria-label={t("cloud.autocast")} className={`relative w-9 h-5 rounded-full transition-colors shrink-0 ${(selv("or_tts_autocast") || "1") !== "0" ? "bg-[var(--color-accent)]" : "bg-[var(--color-surface)] border border-[var(--color-border)]"}`}>
                <span className={`absolute top-0.5 w-4 h-4 rounded-full bg-white transition-all ${(selv("or_tts_autocast") || "1") !== "0" ? "left-[18px]" : "left-0.5"}`} />
              </button>
              <span className="text-[12px]">{t("cloud.autocast")} <Hint text={t("cloud.autocastHint")} /></span>
              <span className="text-[9px] px-1.5 py-0.5 rounded bg-[var(--color-accent)]/20 text-[var(--color-accent)] uppercase tracking-wider">{t("cloud.beta")}</span>
            </div>
            {(selv("or_tts_autocast") || "1") === "0" && (
              <select value={selv("or_tts_voice")} onChange={(e) => setSel("or_tts_voice", e.target.value)} className={orSelectCls}>
                <option value="">{t("cloud.oneVoice")}</option>
                {orVoices.map((v) => <option key={v.name} value={v.name}>{v.gender === "male" ? "♂" : v.gender === "female" ? "♀" : "•"} {v.name}{v.age === "teen" ? ` · ${t("voice.ageTeen")}` : v.age === "child" ? ` · ${t("voice.ageChild")}` : ""}</option>)}
              </select>
            )}
          </div>
        ) : (
          <>
            {backendTabs("tts_backend")}
            {variantPicker("Higgs Audio v3", ["higgs", "higgs-q6_k", "higgs-q4_k_m"])}
            {rowOf(voicingBuild("higgs-engine", "audiocpp"))}
            {rowOf("drama3")}
            {get("drama3") && <>
              <p className="text-[11px] text-[var(--color-muted)] px-1">{t("settings.dramaNote")}</p>
              <div className="flex items-center gap-2 px-1">
                <label htmlFor="drama-vram" className="text-[12px] flex-1">{t("settings.dramaVram")} <Hint text={t("settings.dramaVramHint")} /></label>
                <select id="drama-vram" value={dramaVram} onChange={(e) => {
                  const v = e.target.value;
                  setDramaVram(v);
                  dramaSaves.current = dramaSaves.current.then(() => api.setSelection("drama_vram_gb", v)).catch((er) => { setErr(er instanceof Error ? er.message : String(er)); loadCap(); });
                }}
                  className="bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px] focus:border-[var(--color-accent)] focus:outline-none">
                  <option value="0">{t("settings.auto")}</option>
                  {[5, 6, 8, 10, 12, 16, 20, 24].map(gb => <option key={gb} value={String(gb)}>{gb} {t("units.gb")}</option>)}
                </select>
              </div>
            </>}
            {rowOf("supertonic3")}
            {rowOf("audio8")}
            {variantPicker("Qwen3-TTS 1.7B", ["qwen3-tts", "qwen3-tts-q8_0"])}
            {rowOf("qwen3-tts-engine")}
            {(() => {
              // Qwen's library and its GPU backend module are two components (audio.cpp builds are whole engines):
              // the backend of the voicing device goes beside the engine. CPU needs none.
              const device = selv("tts_backend");
              const id = device === "cpu" ? null
                : device === "vulkan" ? "qwen3-tts-vulkan"
                : get("qwen3-tts-cuda") ? "qwen3-tts-cuda"
                : status.gpu.cards.length > 0 ? "qwen3-tts-vulkan"
                : null;
              return id && get(id) ? rowOf(id) : null;
            })()}
            {rowOf("qwen3-tts-ru-stress")}
            {rowOf(selv("qwen_tts_quant") === "Q8_0" ? "qwen3-tts-design-q8_0" : "qwen3-tts-design")}
          </>
        )}
      </>)}
      {group(t("settings.roleText"), <>{rowOf("ruaccent")}{rowOf("ruaccent-omograph-big")}</>)}
      {group(t("settings.roleAsr"), <>
        {/* Модель ASR — одна кнопка на модель: Parakeet-TDT и Nemotron 3.5 на transcribe.cpp, CrisperWhisper в audiocpp_server, OpenRouter (облако). */}
        <div className="flex gap-1 mb-1.5">
          {[{ id: "parakeet", label: "Parakeet-TDT", cloud: false }, { id: "nemotron", label: "Nemotron 3.5", cloud: false }, { id: "whisper", label: "CrisperWhisper", cloud: false }, { id: "openrouter", label: "OpenRouter", cloud: true }].map((e) => {
            const asrCloud = selv("or_asr_on") === "1";
            const active = e.cloud ? asrCloud : (!asrCloud && asrEngine === e.id);
            const dis = e.cloud && !hasOrKey;
            return (
              <button key={e.id} title={dis ? t("cloud.needKey") : ""}
                onClick={() => { if (dis) { openSettings("cloud:key"); } else if (e.cloud) { setSel("or_asr_on", "1"); } else { setSel("or_asr_on", "0"); setAsrEngine(e.id); api.setSelection("asr_engine", e.id).catch((er) => { setErr(er instanceof Error ? er.message : String(er)); loadCap(); }); } }}
                className={`flex-1 px-2 py-1.5 rounded-md text-[12px] font-medium border transition-colors ${active ? "border-[var(--color-accent)] bg-[color-mix(in_oklab,var(--color-accent)_14%,transparent)] text-[var(--color-text)]" : "border-[var(--color-border)] text-[var(--color-muted)] hover:text-[var(--color-text)]"} ${dis ? "opacity-50" : ""}`}>
                {e.label}
              </button>
            );
          })}
        </div>
        {selv("or_asr_on") !== "1" && backendTabs("asr_backend")}
        {selv("or_asr_on") === "1" ? (
          <div className={`${orRowCls} space-y-2`}>
            {orModelSelect("asr", "or_asr", t("providers.pickAsrModel"))}
            <div className="text-[11px] text-[var(--color-muted)]">{t("providers.asrCloudHint")}</div>
          </div>
        ) : asrEngine === "parakeet" ? (
          <>
            {rowOf(["transcribe-engine", "transcribe-engine-vulkan", "transcribe-engine-cpu"].find((x) => get(x)?.requirement === "required") ?? "transcribe-engine")}
            {variantPicker("Parakeet-TDT 0.6B v3", ["parakeet", "parakeet-ultra"], ASR_VARIANT_I18N)}
          </>
        ) : asrEngine === "nemotron" ? (
          <>
            {rowOf(["transcribe-engine", "transcribe-engine-vulkan", "transcribe-engine-cpu"].find((x) => get(x)?.requirement === "required") ?? "transcribe-engine")}
            {rowOf("asr-gguf-nemotron")}
          </>
        ) : (
          <>
            {rowOf("crisperwhisper-turbo")}
          </>
        )}
        {rowOf("silero-vad")}
        {rowOf("langid")}
      </>)}
      {group(t("settings.roleMt"), <>
        {/* Перевод и vision — каждый своим провайдером: своя Gemma, локальный сервер или OpenRouter. */}
        <LlmProviders selection={cap?.selection} hasOrKey={hasOrKey} onChanged={loadCap} />
        {needsGemma && (
          <>
            {variantPicker("Gemma-4 12B QAT + vision", ["gemma", "gemma-q5_0", "gemma-q6_k", "gemma-q8_0"])}
            {rowOf("llama")}
            {status.llamaNote && <p className="text-[11px] text-[var(--color-muted)] px-1">{status.llamaNote}</p>}
          </>
        )}
      </>)}
      {group(t("settings.roleSep"), <>
        {/* На чём считать сепарацию — свои табы (GPU CUDA-сборка / CPU-сборка BSRoformer). */}
        {backendTabs("sep_backend")}
        {variantPicker("Mel-Band Roformer voc_fv6", ["roformer", "roformer-q5", "roformer-q4"])}
        {rowOf(selv("sep_backend") === "cpu" ? "bsroformer-engine-cpu" : selv("sep_backend") === "vulkan" ? "bsroformer-engine-vulkan" : get("bsroformer-engine-cuda12") ? "bsroformer-engine-cuda12" : "bsroformer-engine")}
        {rowOf("dpdfnet")}
      </>)}
      {group(t("settings.roleDiar"), <>
        {/* На чём считать диаризацию — свои табы (Nemotron 3 Diarization на сервере audio.cpp). */}
        {backendTabs("diar_backend")}
        {rowOf("sortformer")}
      </>)}
      {group(t("settings.roleRuntime"), <>{rowOf("onnxruntime")}{selv("asr_backend") === "gpu" && rowOf("onnxruntime-gpu")}{selv("asr_backend") === "gpu" && rowOf("cudnn")}{rowOf("ffmpeg")}{rowOf("cuda-runtime")}{rowOf("cuda12-runtime")}{rowOf("vcruntime")}</>)}
      <YtDlpTool installed={!!get("ytdlp")?.installed} row={rowOf("ytdlp")} />
      {/* Производительность / экономия RAM — ВИДИМЫЕ контролы (не авто-магия): против OOM на слабой памяти. */}
      {group(t("settings.perfTitle"), <>
        {[
          { key: "llama_ubatch", label: t("settings.llamaUbatch"), hint: t("settings.llamaUbatchHint"), opts: cap?.llama_ubatches ?? ["0", "512", "256", "128"], def: "0", fmt: (v: string) => (v === "0" ? t("settings.auto") : v) },
          { key: "higgs_ref_secs", label: t("settings.higgsRef"), hint: t("settings.higgsRefHint"), opts: cap?.higgs_ref_secs_opts ?? ["12", "8", "6", "4"], def: "12", fmt: (v: string) => `${v}${t("settings.sec")}` },
        ].map((row) => {
          const cur = slot(cap?.selection, row.key) ?? row.def;
          return (
            <div key={row.key} className="px-2.5 py-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)]">
              <div className="flex items-center gap-2.5">
                <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-[var(--color-muted)]" />
                <div className="min-w-0 flex-1">
                  <div className="text-[12px] font-medium truncate">{row.label} <Hint text={row.hint} /></div>
                </div>
                <select value={cur} onChange={(e) => { api.setSelection(row.key, e.target.value).then(loadCap).catch((er) => setErr(er instanceof Error ? er.message : String(er))); }}
                  className="shrink-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[11px] mono focus:border-[var(--color-accent)] focus:outline-none">
                  {row.opts.map((o) => <option key={o} value={o}>{row.fmt(o)}</option>)}
                </select>
              </div>
            </div>
          );
        })}
      </>)}
    </div>
  );
}
