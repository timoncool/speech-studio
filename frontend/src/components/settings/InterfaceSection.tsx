import { useEffect, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { AudioLines, Footprints, Languages, Music, Scaling, SunMoon, Volume2 } from "lucide-react";
import { livePlaybackEnabled, setLivePlaybackEnabled } from "../../lib/livePlayback";
import { setSimpleMode, useSimpleMode } from "../../lib/simpleMode";
import { DUB_LANGS, LANGS, setLang, type Lang } from "../../lib/i18n";
import { playSfx, setSfxEnabled, sfxEnabled } from "../../lib/sfx";
import SettingSwitch from "./SettingSwitch";
import ShortcutsSection from "./ShortcutsSection";
import { THEMES, savedTheme, setTheme, type Theme } from "../../lib/theme";
import { masterVolume, setMasterVolume } from "../../lib/masterVolume";
import { setUiScale, storedUiScale, UI_SCALE_EVENT, UI_SCALE_MAX, UI_SCALE_MIN, UI_SCALE_STEP } from "../../lib/uiScale";

const langName = (code: Lang) => DUB_LANGS.find((l) => l.code === code)?.name ?? code;

export default function InterfaceSection() {
  const { t, i18n } = useTranslation();
  const [sfx, setSfx] = useState(sfxEnabled());
  const [live, setLive] = useState(livePlaybackEnabled());
  const [theme, setThemeState] = useState<Theme>(savedTheme());
  const [volume, setVolume] = useState(Math.round(masterVolume() * 100));
  const [scale, setScale] = useState(storedUiScale());
  const simple = useSimpleMode();
  const selectId = useId();
  const themeId = useId();
  const volumeId = useId();
  const scaleId = useId();
  useEffect(() => {
    const onScale = (e: Event) => setScale((e as CustomEvent<number>).detail);
    window.addEventListener(UI_SCALE_EVENT, onScale);
    return () => window.removeEventListener(UI_SCALE_EVENT, onScale);
  }, []);
  return (
    <div className="space-y-3">
    <div className="max-w-2xl rounded-lg border border-[var(--color-border)] divide-y divide-[var(--color-border)] bg-[var(--color-surface-2)]/40">
      <div className="flex items-center gap-3 px-3 py-2.5">
        <Languages size={15} className="shrink-0 text-[var(--color-accent-2)]" />
        <label htmlFor={selectId} className="min-w-0 flex-1">
          <span className="block text-[13px] font-medium">{t("prefs.uiLanguage")}</span>
          <span className="block text-[11px] leading-snug text-[var(--color-muted)]">{t("prefs.uiLanguageHint")}</span>
        </label>
        <select id={selectId} value={i18n.language as Lang} onChange={(e) => setLang(e.target.value as Lang)}
          className="shrink-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px] focus:border-[var(--color-accent)] focus:outline-none">
          {LANGS.map((l) => <option key={l} value={l}>{langName(l)}</option>)}
        </select>
      </div>
      <div className="flex items-center gap-3 px-3 py-2.5">
        <SunMoon size={15} className="shrink-0 text-[var(--color-accent-2)]" />
        <label htmlFor={themeId} className="min-w-0 flex-1">
          <span className="block text-[13px] font-medium">{t("prefs.theme")}</span>
          <span className="block text-[11px] leading-snug text-[var(--color-muted)]">{t("prefs.themeHint")}</span>
        </label>
        <select id={themeId} value={theme} onChange={(e) => { const v = e.target.value as Theme; setThemeState(v); setTheme(v); }}
          className="shrink-0 bg-[var(--color-surface)] border border-[var(--color-border)] rounded-md px-2 py-1 text-[12px] focus:border-[var(--color-accent)] focus:outline-none">
          {THEMES.map((v) => <option key={v} value={v}>{t(`prefs.theme_${v}`)}</option>)}
        </select>
      </div>
      <div className="flex items-center gap-3 px-3 py-2.5">
        <Scaling size={15} className="shrink-0 text-[var(--color-accent-2)]" />
        <label htmlFor={scaleId} className="min-w-0 flex-1">
          <span className="block text-[13px] font-medium">{t("prefs.uiScale")}</span>
          <span className="block text-[11px] leading-snug text-[var(--color-muted)]">{t("prefs.uiScaleHint")}</span>
        </label>
        <input id={scaleId} type="range" min={UI_SCALE_MIN} max={UI_SCALE_MAX} step={UI_SCALE_STEP} value={scale}
          onChange={(e) => { const v = Number(e.target.value); setScale(v); setUiScale(v); }}
          className="shrink-0 w-32 accent-[var(--color-accent)]" />
        <span className="w-9 text-right mono text-[11px] tabular-nums text-[var(--color-muted)]">{scale}%</span>
      </div>
      <div className="flex items-center gap-3 px-3 py-2.5">
        <Volume2 size={15} className="shrink-0 text-[var(--color-accent-2)]" />
        <label htmlFor={volumeId} className="min-w-0 flex-1">
          <span className="block text-[13px] font-medium">{t("prefs.volume")}</span>
          <span className="block text-[11px] leading-snug text-[var(--color-muted)]">{t("prefs.volumeHint")}</span>
        </label>
        <input id={volumeId} type="range" min={0} max={100} step={1} value={volume}
          onChange={(e) => { const v = Number(e.target.value); setVolume(v); setMasterVolume(v / 100); }}
          className="shrink-0 w-32 accent-[var(--color-accent)]" />
        <span className="w-9 text-right mono text-[11px] tabular-nums text-[var(--color-muted)]">{volume}%</span>
      </div>
      <SettingSwitch icon={Music} label={t("settings.sounds")} hint={t("prefs.soundsHint")} on={sfx}
        onToggle={() => { const v = !sfx; setSfx(v); setSfxEnabled(v); if (v) playSfx("notify"); }} />
      <SettingSwitch icon={AudioLines} label={t("prefs.livePlayback")} hint={t("prefs.livePlaybackHint")} on={live}
        onToggle={() => { const v = !live; setLive(v); setLivePlaybackEnabled(v); }} />
      <SettingSwitch icon={Footprints} label={t("simple.title")} hint={t("simple.hint")} on={simple} onToggle={() => setSimpleMode(!simple)} />
    </div>
    <ShortcutsSection />
    </div>
  );
}
