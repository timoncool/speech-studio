import { useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { motion } from "motion/react";
import { Check, Download, ExternalLink, FolderDown, Loader2, X } from "lucide-react";
import { api, SetupError, type DownloadJob, type SetupComponent, type SetupStatus } from "../../lib/api";
import { useStore } from "../../store";
import { playSfx } from "../../lib/sfx";
import { useSetupStatus } from "../../lib/useSetupStatus";
import { useDownloadErrorText } from "../../lib/setupText";
import { fmtBytes } from "../../lib/format";
import { setTelemetry } from "../../lib/studioHub";
import GpuNotice from "../GpuNotice";
import DownloadProgress from "../DownloadProgress";
import ModelsFolder from "../ModelsFolder";
import OpenRouterKey from "../OpenRouterKey";
import HubTelemetryPreview from "../HubTelemetryPreview";
import PresetsSection from "./PresetsSection";
import { ASR_VARIANT_I18N } from "../../lib/modelVariants";
import { takenFromLabel } from "../../lib/programs";

const EASE = [0.22, 1, 0.36, 1] as const;

// Семейства квантов: разные варианты ОДНОЙ модели (выбор одного, «ИЛИ»). Явная карта по id — надёжнее
// префикса (higgs — квант TTS, higgs-engine — движок, это РАЗНЫЕ вещи). Остальные компоненты — одиночные.
const QUANT_GROUP: Record<string, string> = {
  higgs: "higgs", "higgs-q6_k": "higgs", "higgs-q4_k_m": "higgs",
  gemma: "gemma", "gemma-q5_0": "gemma", "gemma-q6_k": "gemma", "gemma-q8_0": "gemma",
  parakeet: "parakeet", "parakeet-ultra": "parakeet",
  roformer: "roformer", "roformer-q5": "roformer", "roformer-q4": "roformer",
  "qwen3-tts": "qwen3", "qwen3-tts-q8_0": "qwen3",
  "qwen3-tts-design": "qwen3design", "qwen3-tts-design-q8_0": "qwen3design",
};

// Engines, runtimes and helper models: the studio takes them for the models chosen and for this computer's card, so they
// are not a choice on this screen.
const SERVICE_IDS = new Set([
  "higgs-engine", "qwen3-tts-engine", "transcribe-engine", "transcribe-engine-cpu", "transcribe-engine-vulkan",
  "langid", "silero-vad", "bsroformer-engine", "bsroformer-engine-cpu", "llama", "onnxruntime", "onnxruntime-gpu",
  "ffmpeg", "cuda-runtime", "cuda12-runtime", "cudnn", "vcruntime",
]);
// the other builds of the audio.cpp server and of Qwen3-TTS, fetched when a stage is set to their device
const isService = (id: string) => SERVICE_IDS.has(id) || /^(audiocpp|qwen3-tts)-(cuda13|cuda12|vulkan|cpu)$/.test(id);

/** The service parts a chosen model runs on; of two builds of one engine, the one the server asks for on this computer. */
function needs(id: string, s: SetupStatus): string[] {
  const build = (gpu: string, cpu: string) => {
    const req = (x: string) => s.components.find((c) => c.id === x)?.requirement ?? "optional";
    if (req(gpu) !== "optional") return gpu;
    if (req(cpu) !== "optional") return cpu;
    return s.gpu?.cuda13Ok ? gpu : cpu;
  };
  if (id.startsWith("higgs")) return ["higgs-engine"];
  if (id.startsWith("qwen3-tts")) return ["qwen3-tts-engine"];
  if (id.startsWith("parakeet") || id === "asr-gguf-nemotron") {
    // the build the server owes for this computer (CUDA, Vulkan or the processor)
    const owed = ["transcribe-engine", "transcribe-engine-vulkan", "transcribe-engine-cpu"].find((x) => s.components.find((c) => c.id === x)?.requirement === "required");
    return [owed ?? build("transcribe-engine", "transcribe-engine-cpu"), "langid", "silero-vad"];
  }
  if (id === "crisperwhisper-turbo") {
    // «auto» is heard by VoxLingua107 on the transcribe.cpp build the server owes for this computer
    const owed = ["transcribe-engine", "transcribe-engine-vulkan", "transcribe-engine-cpu"].find((x) => s.components.find((c) => c.id === x)?.requirement === "required");
    return [owed ?? build("transcribe-engine", "transcribe-engine-cpu"), "langid"];
  }
  if (id.startsWith("roformer")) return [build("bsroformer-engine", "bsroformer-engine-cpu")];
  if (id.startsWith("gemma")) return ["llama"];
  return [];
}

// ── «Первый запуск»: панель автозакачки компонентов (модели/движки/системные библиотеки) ──
// По умолчанию отмечено всё недостающее скачиваемое, кроме опциональных квантов (их качают вручную).
const preselect = (s: SetupStatus) =>
  new Set(s.components.filter((c) => c.delivery === "download" && !c.installed && c.requirement !== "optional").map((c) => c.id));

/** Обязательные компоненты, без которых «Первый запуск» не пускает дальше, — по именам, для подсказки. */

function owedNames(s: SetupStatus): string {
  return s.components.filter((c) => c.requirement === "required" && !c.installed && c.delivery !== "external").map((c) => c.name).join(", ");
}

export default function FirstRun() {
  const { t } = useTranslation();
  // Галочка статистики «Первого запуска»: включена, уходит с первой закачкой или «Продолжить» (выбор увиден).
  const [telemetryOn, setTelemetryOn] = useState(true);
  const [telemetryPreview, setTelemetryPreview] = useState(false);
  const sendTelemetryChoice = () => setTelemetry(telemetryOn, true).catch((e: Error) => console.warn("[hub] the statistics choice was not saved:", e.message));
  const errText = useDownloadErrorText();
  const setStage = useStore((s) => s.setStage);
  // Выбор пользователя; null — ещё не трогал, берём преселект из текущего статуса (опрос его не сбрасывает).
  const [picked, setPicked] = useState<Set<string> | null>(null);
  const [err, setErr] = useState<string | null>(null);
  // Закачка идёт фоном на сервере: прогресс — опросом статуса, переживает перезагрузку окна и приложения.
  const onSettled = (job: DownloadJob, s: SetupStatus) => {
    if (job.status === "completed") useStore.getState().pushActivity(s.ready ? t("setup.ready") : t("setup.owed", { list: owedNames(s) }), "done");
    if (job.status === "completed" && s.ready) { setStage("ready"); playSfx("success"); }   // всё скачано -> сразу на стартовый экран
    else if (job.status === "failed") useStore.getState().pushActivity(`${errText(job.errorCode)} · ${job.error ?? ""}`, "error");
  };
  const { status, error: statusErr, refresh, adopt, downloading: busy } = useSetupStatus(onSettled);
  const owed = status && !status.ready ? owedNames(status) : "";
  const sel = picked ?? (status ? preselect(status) : new Set<string>());
  const setSel = (f: (prev: Set<string>) => Set<string>) => setPicked(f(sel));
  const active = status?.active ?? null;
  const partPct = (id: string): number | null => {
    if (!busy || !active) return null;
    const p = active.parts.find((x) => x.id === id);
    return p && p.total > 0 ? (p.done / p.total) * 100 : active.ids.includes(id) ? 0 : null;
  };

  async function download(ids: string[]) {
    if (ids.length === 0 || busy) return;
    setErr(null);
    try {
      await api.setupDownload(ids);
      await refresh();
    } catch (e) {
      const msg = e instanceof SetupError ? `${errText(e.code)} · ${e.detail}` : String(e);
      setErr(msg); useStore.getState().pushActivity(msg, "error");
    }
  }
  const pause = () => { api.setupCancel().then(() => refresh()).catch((e) => setErr(String(e))); };

  const toggle = (id: string) => setSel((prev) => { const n = new Set(prev); if (n.has(id)) n.delete(id); else n.add(id); return n; });
  // what Download fetches: the models chosen, what they run on and the service parts this computer needs anyway
  const fetchIds = new Set<string>(sel);
  if (status) for (const id of sel) needs(id, status).forEach((x) => fetchIds.add(x));
  const selectedComps = status ? status.components.filter((c) => fetchIds.has(c.id) && c.delivery === "download" && !c.installed) : [];
  const serviceComps = selectedComps.filter((c) => isService(c.id));
  const selectedBytes = selectedComps.reduce((a, c) => a + Math.max(0, c.size - c.bytesOnDisk), 0);
  const selectedSpace = selectedComps.reduce((a, c) => a + c.spaceNeeded, 0);
  const noSpace = status?.freeBytes != null && selectedSpace > status.freeBytes;

  const reqLabel = (r: string) => t(r === "required" ? "setup.required" : r === "recommended" ? "setup.recommended" : "setup.optional");
  const deliveryNote = (c: SetupComponent) =>
    c.delivery === "bundled" ? t("setup.reinstallHint") : c.delivery === "external" ? t("setup.external") : "";
  const GROUP_LABEL: Record<string, string> = {
    higgs: t("setup.grpHiggs"), gemma: t("setup.grpGemma"), parakeet: t("setup.grpParakeet"), roformer: t("setup.grpRoformer"),
    qwen3: t("setup.grpQwen"), qwen3design: t("setup.grpQwenDesign"),
  };
  const pickOne = (id: string | null, group: string) => setSel((prev) => {   // radio внутри семейства: выбрать этот квант, снять остальные того же семейства
    const n = new Set(prev);
    Object.entries(QUANT_GROUP).forEach(([cid, g]) => { if (g === group) n.delete(cid); });
    if (id) n.add(id);
    return n;
  });
  const comps = (status?.components ?? []).filter((c) => !isService(c.id) && !(c.delivery === "external" && c.installed));
  const updating = !!status?.updates?.length;
  const gseen = new Set<string>();
  type SetupRow = { kind: "one"; c: SetupComponent } | { kind: "group"; group: string; members: SetupComponent[] };
  const rows: SetupRow[] = comps.flatMap((c): SetupRow[] => {   // одиночные компоненты + по одной группе квантов (на первом её члене)
    const g = QUANT_GROUP[c.id];
    if (!g) return [{ kind: "one", c }];
    if (gseen.has(g)) return [];
    gseen.add(g);
    return [{ kind: "group", group: g, members: comps.filter((x) => QUANT_GROUP[x.id] === g) }];
  });

  return (
    <div className="flex-1 min-h-0 overflow-y-auto grid place-items-center px-6 py-10">
      <motion.div initial={{ opacity: 0, y: 14 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.5, ease: EASE }}
        className="w-full max-w-2xl">
        <div className="mono text-[11px] tracking-[0.2em] text-[var(--color-muted)] flex items-center gap-2">
          <span className="w-1.5 h-1.5 rounded-full bg-[var(--color-accent)] shadow-[0_0_8px_var(--color-accent)]" />{t(updating ? "setup.updateTitle" : "setup.title")}
        </div>
        <h1 className="mt-4 text-2xl lg:text-[28px] leading-tight font-extrabold tracking-tight">{t(updating ? "setup.updateTitle" : "setup.title")}</h1>
        <p className="mt-3 text-[14px] leading-relaxed text-[var(--color-muted)]">{t(updating ? "setup.updateSubtitle" : "setup.subtitle")}</p>
        {updating && (
          <ul aria-label={t("setup.updateList")} className="mt-3 space-y-1.5 rounded-lg border border-[var(--color-border)] px-3 py-2.5 text-[13px]">
            {status?.updates.map((u) => (
              <li key={u.id}><span className="font-semibold">{u.name}</span><span className="text-[var(--color-muted)]"> — {u.reason}</span></li>
            ))}
          </ul>
        )}

        {err && (
          <div className="mt-4 rounded-lg border border-[var(--color-warn)]/40 bg-[color-mix(in_oklab,var(--color-warn)_10%,transparent)] px-3 py-2 text-[13px] text-[var(--color-warn)]">
            <span className="font-semibold">{t("setup.error")}</span> · <span className="mono text-[11px] break-words">{err}</span>
          </div>
        )}

        {statusErr && (
          <div className="mt-4 rounded-lg border border-[var(--color-warn)]/40 px-3 py-2 mono text-[11px] text-[var(--color-warn)] break-words">{statusErr}</div>
        )}

        {status && (
          <div className="mt-4 space-y-2">
            <GpuNotice gpu={status.gpu} driverUrl={status.components.find((c) => c.delivery === "external")?.externalUrl} />
            {active && active.status !== "completed" && <DownloadProgress job={active} onPause={pause} onResume={(ids) => { download(ids); }} />}
            <ModelsFolder status={status} need={selectedSpace} />
          </div>
        )}

        <div className="mt-4 rounded-lg border border-[var(--color-accent)]/30 bg-[color-mix(in_oklab,var(--color-accent)_8%,transparent)] px-3.5 py-2.5 text-[12.5px] leading-snug text-[var(--color-text)]">
          <span className="font-semibold text-[var(--color-accent)]">{t("setup.pickNoteTitle")}</span> {t("setup.pickNote")}
        </div>

        {/* Пресет под железо + ключ OpenRouter: выбрал «Облако» → облачные движки, локальные модели ниже
            становятся необязательными (не тянешь лишние гигабайты). Refresh обновляет чеклист под выбор. */}
        <div className="mt-4 rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] px-3.5 py-3 space-y-2">
          <PresetsSection onApplied={() => { setPicked(null); refresh().catch(() => {}); }} />
          <OpenRouterKey onSaved={() => { setPicked(null); refresh().catch(() => {}); }} />
          <div className="text-[11px] text-[var(--color-muted)] leading-snug"><Trans i18nKey="setup.cloudHint" components={{ b: <span className="text-[var(--color-text)]" /> }} /></div>
        </div>

        <div className="mt-4 space-y-2">
          {rows.map((row) => {
            if (row.kind === "group") {
              const { group, members } = row;
              return (
                <div key={group} className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] px-3.5 py-3">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-[13px] font-semibold">{GROUP_LABEL[group]}</span>
                    <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-[color-mix(in_oklab,var(--color-accent)_16%,transparent)] text-[var(--color-accent)]">{t("setup.pickOneQuant")}</span>
                  </div>
                  <div className="space-y-0.5">
                    {members.every((m) => m.requirement === "optional" && !m.installed) && (
                      <label className="flex items-center gap-2.5 rounded-lg px-2 py-1.5 cursor-pointer hover:bg-[var(--color-surface-2)]">
                        <input type="radio" name={`grp-${group}`} checked={members.every((m) => !sel.has(m.id))} onChange={() => pickOne(null, group)} disabled={busy}
                          className="w-4 h-4 accent-[var(--color-accent)] shrink-0" />
                        <span className="text-[13px] text-[var(--color-muted)]">{t("setup.notNeeded")}</span>
                      </label>
                    )}
                    {members.map((c) => {
                      const isDefault = c.requirement !== "optional";
                      const checked = sel.has(c.id);
                      const pct = partPct(c.id);
                      return (
                        <label key={c.id} className={`flex items-center gap-2.5 rounded-lg px-2 py-1.5 ${c.installed ? "" : "cursor-pointer hover:bg-[var(--color-surface-2)]"} ${checked && !c.installed ? "bg-[color-mix(in_oklab,var(--color-accent)_8%,transparent)]" : ""}`}>
                          {c.installed
                            ? <span className="w-4 h-4 shrink-0 grid place-items-center text-[var(--color-accent)]"><Check size={15} strokeWidth={3} /></span>
                            : <input type="radio" name={`grp-${group}`} checked={checked} onChange={() => pickOne(c.id, group)} disabled={busy || c.fitsVram === false} className="w-4 h-4 accent-[var(--color-accent)] shrink-0" />}
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                              <span className="text-[13px] truncate">{c.name}</span>
                              {isDefault
                                ? <span className="text-[9px] px-1.5 py-0.5 rounded bg-[color-mix(in_oklab,var(--color-accent)_16%,transparent)] text-[var(--color-accent)] uppercase tracking-wide shrink-0">{t("setup.default")}</span>
                                : <span className="text-[9px] px-1.5 py-0.5 rounded bg-[var(--color-surface-2)] text-[var(--color-muted)] uppercase tracking-wide shrink-0">{t("setup.alt")}</span>}
                              {c.installed && <span className="text-[10px] text-[var(--color-accent)] shrink-0">{c.takenFrom ? takenFromLabel(t, c.takenFrom) : t("setup.installed")}</span>}
                            </div>
                            {ASR_VARIANT_I18N[c.id] && <div className="text-[11px] leading-snug text-[var(--color-muted)]">{t(ASR_VARIANT_I18N[c.id].hint)}</div>}
                            {c.fitsVram === false && !c.installed && <div className="text-[11px] leading-snug text-[var(--color-warn)]">{t("setup.noVram")}</div>}
                            {pct != null && <div className="mt-1 h-1 rounded-full bg-[var(--color-surface-2)] overflow-hidden"><div className="h-full bg-[var(--color-accent)]" style={{ width: `${pct}%` }} /></div>}
                          </div>
                          <span className="mono text-[11px] text-[var(--color-muted)] shrink-0">{fmtBytes(c.size)}{c.vram ? ` · ${fmtBytes(c.vram)} VRAM` : ""}</span>
                        </label>
                      );
                    })}
                  </div>
                  <div className="mt-1.5 text-[10px] text-[var(--color-muted)]">{t("setup.quantHint")}</div>
                  {members.every((m) => m.fitsVram === false) && <div className="mt-1 text-[11px] leading-snug text-[var(--color-warn)]">{t("setup.noVramGroup")}</div>}
                </div>
              );
            }
            const c = row.c;
            const pct = partPct(c.id);
            const canPick = c.delivery === "download" && !c.installed;
            return (
              <div key={c.id} className="rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] px-3.5 py-3">
                <div className="flex items-center gap-3">
                  {canPick ? (
                    <input type="checkbox" checked={sel.has(c.id)} onChange={() => toggle(c.id)} disabled={busy} aria-label={c.name}
                      className="w-4 h-4 accent-[var(--color-accent)] shrink-0" />
                  ) : (
                    <span className={`w-4 h-4 shrink-0 grid place-items-center rounded ${c.installed ? "text-[var(--color-accent)]" : "text-[var(--color-muted)]"}`}>
                      {c.installed ? <Check size={16} strokeWidth={3} /> : <X size={14} />}
                    </span>
                  )}
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-semibold text-[14px] truncate">{c.name}</span>
                      <span className={`text-[10px] px-1.5 py-0.5 rounded uppercase tracking-wide ${c.requirement === "required" ? "bg-[color-mix(in_oklab,var(--color-accent)_16%,transparent)] text-[var(--color-accent)]" : "bg-[var(--color-surface-2)] text-[var(--color-muted)]"}`}>{reqLabel(c.requirement)}</span>
                    </div>
                    <div className="text-[12px] text-[var(--color-muted)] break-words">{c.purpose}</div>
                    {c.delivery === "external" && c.detail && <div className="mono text-[10.5px] text-[var(--color-muted)] break-words">{c.detail}</div>}
                    {pct != null && (
                      <div className="mt-1.5 h-1.5 rounded-full bg-[var(--color-surface-2)] overflow-hidden">
                        <div className="h-full bg-[var(--color-accent)] transition-[width] duration-200" style={{ width: `${pct}%` }} />
                      </div>
                    )}
                  </div>
                  <div className="text-right shrink-0">
                    {c.delivery === "external" ? (
                      c.installed ? <span className="text-[12px] text-[var(--color-accent)]">{t("setup.installed")}</span>
                        : <button onClick={() => c.externalUrl && window.open(c.externalUrl, "_blank")}
                            className="inline-flex items-center gap-1 text-[12px] text-[var(--color-accent-2)] hover:underline"><ExternalLink size={13} />{t("setup.openDriver")}</button>
                    ) : c.installed ? (
                      <span className="text-[12px] text-[var(--color-accent)]">{c.takenFrom ? takenFromLabel(t, c.takenFrom) : t("setup.installed")}</span>
                    ) : c.delivery === "bundled" ? (
                      <span className="text-[11px] text-[var(--color-muted)] max-w-[120px] inline-block leading-tight">{deliveryNote(c)}</span>
                    ) : (
                      <div className="flex flex-col items-end gap-1">
                        <span className="mono text-[11px] text-[var(--color-muted)]">{fmtBytes(c.size)}</span>
                        <button onClick={() => download([c.id])} disabled={busy} aria-label={`${t("setup.downloadOne")}: ${c.name}`}
                          className="inline-flex items-center gap-1 px-2 py-1 rounded-md bg-[var(--color-surface-2)] border border-[var(--color-border)] text-[11px] hover:border-[var(--color-accent)] disabled:opacity-40">
                          <Download size={12} />{t("setup.downloadOne")}</button>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        {serviceComps.length > 0 && (
          <details className="mt-3 text-[12px] text-[var(--color-muted)]">
            <summary className="cursor-pointer">{t("setup.serviceNote", { size: fmtBytes(serviceComps.reduce((a, c) => a + Math.max(0, c.size - c.bytesOnDisk), 0)) })}</summary>
            <div className="mt-1 pl-4 leading-relaxed">{serviceComps.map((c) => c.name).join(", ")}</div>
          </details>
        )}

        <div className="mt-6 flex items-center gap-3">
          {status && !status.ready && (
            <button onClick={() => { sendTelemetryChoice(); download(selectedComps.map((c) => c.id)); }} disabled={busy || selectedComps.length === 0 || noSpace} title={noSpace ? t("downloads.noSpace", { need: fmtBytes(selectedSpace), free: fmtBytes(status.freeBytes ?? 0) }) : undefined}
              className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-sm font-semibold disabled:opacity-40 hover:brightness-105">
              {busy ? <Loader2 size={16} className="animate-spin" /> : <Download size={16} />}
              {t("setup.download")} {selectedBytes > 0 && <span className="opacity-80">· {fmtBytes(selectedBytes)}</span>}
            </button>
          )}
          {!busy && (
            <button onClick={async () => {
                try {
                  const r = await api.setupBrowse();
                  if (r.picked) { adopt(r.status); setPicked(null); }
                  if (r.errors.length > 0) setErr(`${t("downloads.importErrors")} · ${r.errors.join("; ")}`);
                } catch (e) { setErr(e instanceof SetupError ? `${errText(e.code)} · ${e.detail}` : String(e)); }
              }}
              className="inline-flex items-center gap-1.5 px-3 py-2.5 rounded-lg border border-dashed border-[var(--color-border)] text-sm text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-text)] transition-colors">
              <FolderDown size={14} />{t("settings.browseFolder")}</button>
          )}
          {status?.ready && !busy && (
            <button onClick={() => { sendTelemetryChoice(); setStage("ready"); }}
              className="inline-flex items-center gap-2 px-4 py-2.5 rounded-lg bg-[var(--color-accent)] text-[var(--color-on-accent)] text-sm font-semibold hover:brightness-105">
              <Check size={16} />{t("setup.continue")}</button>
          )}
        </div>
        {owed && !busy && <div className="mt-3 text-[13px] leading-snug text-[var(--color-text)]">{t("setup.owed", { list: owed })}</div>}
        <div className="mt-4 flex flex-wrap items-center gap-x-3 gap-y-1 text-[13px] text-[var(--color-muted)]">
          <label className="inline-flex items-center gap-2 cursor-pointer">
            <input type="checkbox" className="h-4 w-4 accent-[var(--color-accent)]" checked={telemetryOn} onChange={(e) => { setTelemetryOn(e.target.checked); setTelemetry(e.target.checked, true).catch((err: Error) => console.warn("[hub] the statistics choice was not saved:", err.message)); }} />
            {t("hub.checkbox")}
          </label>
          <button type="button" onClick={() => setTelemetryPreview(true)} className="text-[13px] leading-5 text-[var(--color-muted)] underline underline-offset-2 hover:text-[var(--color-text)]">{t("hub.what")}</button>
          {telemetryPreview && <HubTelemetryPreview onClose={() => setTelemetryPreview(false)} />}
        </div>
      </motion.div>
    </div>
  );
}
