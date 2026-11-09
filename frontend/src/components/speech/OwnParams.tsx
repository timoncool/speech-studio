import { useTranslation } from "react-i18next";
import { useEngines } from "../../lib/engines";
import type { ParamSpec, SpeechParams } from "../../lib/speech";
import Hint from "../Hint";

import { PARAM_LABEL } from "./paramLabel";

/** Sampling of a speaker or a block of its own: off, it takes what it inherits; on, it starts from that and changes. */
export default function OwnParams({ engine, value, inherited, onChange, label }: { engine: string; value: SpeechParams | null | undefined; inherited: SpeechParams; onChange: (v: SpeechParams | null) => void; label: string }) {
  const { t } = useTranslation();
  const specs = useEngines()?.find((e) => e.id === engine)?.params ?? [];
  if (!specs.length) return null;
  const clamp = (raw: string, spec: ParamSpec) => { const n = Number(raw); const v = Number.isFinite(n) ? Math.min(spec.max, Math.max(spec.min, n)) : spec.default; return spec.kind === "int" ? Math.round(v) : v; };
  const field = "mt-0.5 w-full bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-1.5 py-0.5 text-[11px] mono";
  return (
    <div className="text-[11px] text-[var(--color-muted)]">
      <label className="flex items-center gap-1.5 cursor-pointer">
        <input type="checkbox" checked={!!value} onChange={(e) => onChange(e.target.checked ? { ...inherited } : null)} className="w-3.5 h-3.5 accent-[var(--color-accent)]" />
        {label} <Hint text={t("hint.ownParams")} />
      </label>
      {value && (
        <div className="mt-1 grid grid-cols-2 gap-1.5">
          {specs.map((spec) => {
            const meta = PARAM_LABEL[spec.key];
            return (
              <label key={spec.key}>{meta.label.startsWith("speech.") ? t(meta.label as "speech.temperature") : meta.label} <Hint text={t(meta.hint as "speech.topKHint")} />
                <input type="number" step={spec.step} min={spec.min} max={spec.max} value={value[spec.key]} onChange={(e) => onChange({ ...value, [spec.key]: clamp(e.target.value, spec) })} className={field} />
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}
