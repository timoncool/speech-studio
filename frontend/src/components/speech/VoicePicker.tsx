import { useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { api, slot } from "../../lib/api";
import VoiceRecorder from "../voices/VoiceRecorder";
import { prettyVoice, recentVoices, rememberVoice, useVoices, voiceLabel } from "../../lib/voices";
import { useEngines } from "../../lib/engines";
import Combobox, { type ComboOption } from "../Combobox";

/** A voice from the library, or the engine's own base voice (no clone); for an engine with voices of its own, those
 * first — alone when it does not clone, beside the library when it does. `inherit` adds "as the project". */
const RECORD = "\u0000record";

export default function VoicePicker({ value, onChange, inherit, label, className, engine }: {
  value: string | null;
  onChange: (v: string | null) => void;
  inherit?: string;
  label: string;
  className?: string;
  engine?: string;
}) {
  const { t } = useTranslation();
  const { listing, reload } = useVoices();
  const [recording, setRecording] = useState<boolean | null>(null);
  const caps = useEngines()?.find((e) => e.id === engine);
  const presets = caps?.presets ?? [];
  const clones = !presets.length || caps?.clone !== "none";
  const style = className ?? "bg-[var(--color-surface-2)] border border-[var(--color-border)] rounded-lg px-2 py-1.5 text-[12px] max-w-full";
  const outer = style.split(/\s+/).filter((c) => /^(m[trblxy]?-|w-|max-w-|min-w-)/.test(c));
  const inner = style.split(/\s+/).filter((c) => !outer.includes(c));
  const voices = clones ? [...(listing?.voices ?? [])].sort((a, b) => Number(b.card.favorite) - Number(a.card.favorite) || voiceLabel(a).localeCompare(voiceLabel(b))) : [];
  const known = presets.some((p) => p.id === value) || (clones && listing?.voices.some((v) => v.name === value));
  const missing = value && (presets.length || listing) && !known;
  const own = presets.length > 0 ? t("speech.engineVoices") : undefined;
  const recent = recentVoices().filter((name) => voices.some((v) => v.name === name));
  const lib = (presets.length > 0 || recent.length > 0) && voices.length > 0 ? t("speech.libraryClone") : undefined;
  const voiceOption = (v: (typeof voices)[number], group: string | undefined): ComboOption => ({
    value: v.name,
    label: `${v.card.favorite ? "★ " : ""}${voiceLabel(v)}${v.card.lang ? ` · ${v.card.lang.toUpperCase()}` : ""}`,
    search: `${prettyVoice(v.name)} ${v.card.gender} ${v.card.age} ${v.card.description ?? ""} ${(v.card.tags ?? []).join(" ")}`,
    group,
  });
  const options: ComboOption[] = [
    ...(inherit ? [{ value: "", label: inherit }]
      : caps?.needs_voice ? [{ value: "", label: t("speech.pickVoice"), disabled: true }]
      : !presets.length ? [{ value: "", label: t("speech.baseVoice") }] : []),
    ...(missing ? [{ value, label: t(presets.length && !clones ? "speech.voiceNotPreset" : "speech.voiceMissing", { name: prettyVoice(value) }) }] : []),
    ...presets.map((p) => ({
      value: p.id,
      label: `${p.name} · ${t(p.gender === "female" ? "voice.g_female" : "voice.g_male")}${p.native ? ` · ${t(`speech.native_${p.native}` as "speech.native_en")}` : ""}`,
      hint: String(t(`presetVoice.${caps?.id}.${p.id}` as "presetVoice.supertonic.F1", { defaultValue: p.description })),
      group: own,
    })),
    ...recent.flatMap((name) => voices.filter((v) => v.name === name).map((v) => voiceOption(v, t("speech.recentVoices")))),
    ...voices.filter((v) => !recent.includes(v.name)).map((v) => voiceOption(v, lib)),
    ...(clones ? [{ value: RECORD, label: t("speech.recordVoice"), group: lib }] : []),
  ];
  const record = () => { api.capabilities().then((c) => setRecording((slot(c.selection ?? undefined, "voice_clean") ?? "1") !== "0"), () => setRecording(true)); };
  const picker = (
    <Combobox value={value ?? ""} options={options} onChange={(v) => { if (v === RECORD) { record(); return; } if (v && voices.some((x) => x.name === v)) rememberVoice(v); onChange(v || null); }} label={label} noResults={t("speech.noVoiceFound")}
      className={outer.join(" ")} inputClassName={`w-full text-[var(--color-text)] ${inner.join(" ")} focus:border-[var(--color-accent)] focus:outline-none`} />
  );
  return recording === null ? picker : (
    <>
      {picker}
      {createPortal(<VoiceRecorder clean={recording} onClose={() => setRecording(null)} onRecorded={(name) => { setRecording(null); reload(); rememberVoice(name); onChange(name); }} />, document.body)}
    </>
  );
}
