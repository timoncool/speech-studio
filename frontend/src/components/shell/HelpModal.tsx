import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Copy, HelpCircle, Star, X } from "lucide-react";
import { DONATE } from "../../lib/links";
import { named } from "../../lib/a11y";
import { useEscapeLayer } from "../../lib/escape";
import EngineTable from "./EngineTable";

function HelpSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mt-4">
      <div className="text-[11px] uppercase tracking-[0.16em] text-[var(--color-muted)] mb-1.5">{title}</div>
      {children}
    </div>
  );
}

function CryptoRow({ coin, addr }: { coin: string; addr: string }) {
  const { t } = useTranslation();
  const [done, setDone] = useState(false);
  return (
    <button title={t("help.copy")}
      onClick={async () => { try { await navigator.clipboard.writeText(addr); setDone(true); setTimeout(() => setDone(false), 1200); } catch { /* clipboard blocked */ } }}
      className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] hover:border-[var(--color-accent)] transition-colors text-left">
      <span className="text-[11px] uppercase tracking-[0.1em] text-[var(--color-muted)] w-[92px] shrink-0">{coin}</span>
      <span className="mono text-[11px] truncate flex-1">{addr}</span>
      {done ? <Check size={13} className="text-[var(--color-accent)] shrink-0" /> : <Copy size={13} className="text-[var(--color-muted)] shrink-0" />}
    </button>
  );
}

export default function HelpModal({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  useEscapeLayer(onClose);
  const how = t("help.how", { returnObjects: true }) as unknown as string[];
  const features = t("help.features", { returnObjects: true }) as unknown as string[];
  const sections = t("help.sections", { returnObjects: true }) as unknown as string[];
  const pay = "flex items-center justify-center gap-2 px-3 py-2 rounded-lg bg-[var(--color-surface-2)] border border-[var(--color-border)] text-[13px] font-medium text-[var(--color-text)] hover:border-[var(--color-accent)] transition-colors";
  const chip = "inline-flex items-center gap-1 px-2 py-1 rounded-md bg-[var(--color-surface-2)] border border-[var(--color-border)] text-[11px] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-text)] transition-colors";
  return (
    <div className="fixed inset-0 z-50 grid place-items-center glass-scrim anim-fade" onClick={onClose}>
      <div role="dialog" aria-modal="true" aria-labelledby="help-title" className="w-[min(92vw,760px)] max-h-[86vh] overflow-y-auto rounded-xl glass-panel anim-pop p-5" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-2">
          <span id="help-title" className="flex items-center gap-2 font-semibold"><HelpCircle size={17} className="text-[var(--color-accent)]" />{t("help.title")}</span>
          <button onClick={onClose} {...named(t("a11y.close"))} className="text-[var(--color-muted)] hover:text-[var(--color-text)]"><X size={16} /></button>
        </div>
        <p className="text-[13px] leading-relaxed text-[var(--color-muted)]">{t("help.intro")}</p>

        <HelpSection title={t("help.howTitle")}>
          <ol className="space-y-1.5">
            {how.map((s, i) => (
              <li key={i} className="flex gap-2.5 text-[13px]">
                <span className="grid place-items-center w-5 h-5 shrink-0 rounded-full bg-[var(--color-surface-2)] text-[var(--color-accent)] text-[11px] font-semibold">{i + 1}</span>
                <span className="leading-relaxed">{s}</span>
              </li>
            ))}
          </ol>
        </HelpSection>

        <HelpSection title={t("help.featuresTitle")}>
          <ul className="grid sm:grid-cols-2 gap-x-4 gap-y-1.5">
            {features.map((f, i) => (
              <li key={i} className="flex gap-2 text-[13px] leading-relaxed"><Check size={14} className="text-[var(--color-accent)] shrink-0 mt-[3px]" /><span>{f}</span></li>
            ))}
          </ul>
        </HelpSection>

        <HelpSection title={t("engineTable.title")}>
          <p className="text-[12px] leading-relaxed text-[var(--color-muted)] mb-2">{t("engineTable.hint")}</p>
          <EngineTable />
        </HelpSection>

        <HelpSection title={t("help.sectionsTitle")}>
          <ul className="space-y-1">
            {sections.map((s, i) => <li key={i} className="text-[13px] leading-relaxed text-[var(--color-muted)]">· {s}</li>)}
          </ul>
        </HelpSection>

        <HelpSection title={t("help.donateTitle")}>
          <p className="text-[13px] leading-relaxed text-[var(--color-muted)] mb-3">{t("help.donateIntro")}</p>
          <div className="grid sm:grid-cols-2 gap-2 mb-2.5">
            <a href={DONATE.dalink} target="_blank" rel="noreferrer" className={pay}>💳 {t("help.card")}</a>
            <a href={DONATE.boosty} target="_blank" rel="noreferrer" className={pay}>🚀 {t("help.boostySub")}</a>
          </div>
          <div className="space-y-1.5">
            {DONATE.crypto.map(([c, a]) => <CryptoRow key={c} coin={c} addr={a} />)}
          </div>
          <div className="mt-4 pt-3 border-t border-[var(--color-border)] text-[12px] leading-relaxed text-[var(--color-muted)]">
            {t("help.madeBy")} <a className="text-[var(--color-text)] hover:text-[var(--color-accent)] transition-colors" href={DONATE.telegram} target="_blank" rel="noreferrer">Nerual Dreming</a> — {t("help.founder")} <a className="text-[var(--color-text)] hover:text-[var(--color-accent)] transition-colors" href="https://artgeneration.me" target="_blank" rel="noreferrer">ArtGeneration.me</a>
            <div className="flex flex-wrap gap-1.5 mt-2">
              <a href="https://t.me/neuroport" target="_blank" rel="noreferrer" className={chip}>Нейро-Софт</a>
              <a href={DONATE.github} target="_blank" rel="noreferrer" className={chip}><Star size={11} />GitHub</a>
              <a href={DONATE.telegram} target="_blank" rel="noreferrer" className={chip}>Telegram</a>
              <a href={DONATE.x} target="_blank" rel="noreferrer" className={chip}>X</a>
            </div>
          </div>
        </HelpSection>
      </div>
    </div>
  );
}
