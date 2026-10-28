import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Minus } from "lucide-react";
import { api, slot } from "../../lib/api";
import { useEngines } from "../../lib/engines";

/** What each engine needs from a voice's reference, against this voice: how long a clip it takes and whether the
 * clip's text is a must or a help. */
export default function RefNeeds({ hasText }: { hasText: boolean }) {
  const { t } = useTranslation();
  const engines = useEngines();
  const [limit, setLimit] = useState<string | null>(null);
  useEffect(() => { api.capabilities().then((c) => setLimit(slot(c.selection, "higgs_ref_secs") || "12"), () => setLimit("12")); }, []);
  if (!engines) return null;
  const cloning = engines.filter((e) => e.clone !== "none" && !e.unavailable);
  const own = engines.filter((e) => e.clone === "none").map((e) => e.name).join(", ");
  return (
    <div className="rounded-lg border border-[var(--color-border)] px-3 py-2 space-y-1.5 text-[12px]">
      <p className="font-medium">{t("voices.refNeeds")}</p>
      {limit && <p className="text-[11px] leading-snug text-[var(--color-muted)]">{t("voices.refLength", { s: limit, setting: t("settings.higgsRef"), models: t("prefs.sections.models"), pick: t("voices.findPieces") })}</p>}
      <ul className="space-y-1">
        {cloning.map((e) => {
          return (
            <li key={e.id} className="flex items-start gap-2">
              {hasText ? <Check size={13} className="mt-0.5 shrink-0 text-[var(--color-accent)]" /> : <Minus size={13} className="mt-0.5 shrink-0 text-[var(--color-muted)]" />}
              <span><span className="font-medium">{e.name}</span>{e.ref_max_secs != null && limit && e.ref_max_secs < Number(limit) ? ` (${t("voices.refUpTo", { s: e.ref_max_secs })})` : ""} — <span className="text-[var(--color-muted)]">{e.ref_text_required ? (hasText ? t("voices.refTextHave") : t("voices.refTextMust")) : hasText ? t("voices.refTextHave") : t("voices.refTextHelps")}</span></span>
            </li>
          );
        })}
      </ul>
      {own && <p className="text-[11px] text-[var(--color-muted)]">{t("voices.refNoClone", { engines: own })}</p>}
    </div>
  );
}
