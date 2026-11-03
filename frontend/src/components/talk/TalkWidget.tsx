import { useTranslation } from "react-i18next";
import { AudioLines, Pause, Play, Square, X } from "lucide-react";
import { byPerson, personOnly } from "../../lib/person";
import { named } from "../../lib/a11y";
import { useTalkPrefs } from "../../lib/talkPrefs";
import { Tip } from "../Hint";
import TalkVisual from "./TalkVisual";
import { useTalk } from "./useTalk";
import { TALK_STYLES, type TalkStyle } from "./visual";

/** The desktop shell closes the widget's window when it goes here; a browser's popup closes itself on it. */
const CLOSE = "/?widget&close";

/** Live as a bar over every other window: the picture, what is being said, and the microphone. The sound plays in the
 * main window, which sends the bar its loudness. */
export default function TalkWidget() {
  const { t } = useTranslation();
  const prefs = useTalkPrefs();
  const look: TalkStyle = TALK_STYLES.includes(prefs.style as TalkStyle) ? (prefs.style as TalkStyle) : "aura";
  const talk = useTalk({ fed: true });
  const { state, running, muted, partial, saying, lastAgent, speaking, loudness, error } = talk;
  const line = error ?? saying ?? (state === "hearing" ? partial : lastAgent?.text ?? "");
  const button = "grid h-8 w-8 shrink-0 place-items-center rounded-full transition-colors disabled:opacity-40";

  return (
    <main data-tauri-drag-region title={t("talk.widgetMove")} className="flex h-full w-full items-center p-6 select-none">
      <div data-tauri-drag-region className="flex h-full w-full items-center gap-2 rounded-full border border-white/10 bg-[#0d1117]/85 pl-1.5 pr-2 text-white shadow-[0_4px_16px_rgba(0,0,0,0.45)] backdrop-blur-xl">
        <TalkVisual look={look} state={state} level={loudness} bands={talk.bands} token={state === "thinking" ? "--color-accent-2" : "--color-accent"} className={look === "wave" || look === "bars" || look === "matrix" ? "pointer-events-none h-12 w-20 shrink-0" : "pointer-events-none h-14 w-14 shrink-0"} />
        <div data-tauri-drag-region className="min-w-0 flex-1 leading-tight">
          <p role="status" aria-live="polite" className="text-[11px] font-medium text-white/60">{t(`talk.state_${state}`)}</p>
          <p className={`truncate text-[13px] ${error ? "text-[var(--color-warn)]" : ""}`} title={line}>{line}</p>
        </div>
        {running && (
          <Tip text={t("talk.muteHint")}>
            <button onClick={(e) => { if (muted && !byPerson(e)) return; void talk.toggleMute(); }} aria-pressed={muted} className={`${button} ${muted ? "bg-[var(--color-warn)] text-black" : "bg-white/10 hover:bg-white/20"}`} {...personOnly(muted)} {...named(muted ? t("talk.unmute") : t("talk.mute"))}>
              {muted ? <Play size={15} /> : <Pause size={15} />}
            </button>
          </Tip>
        )}
        <Tip text={running ? t("talk.micOffHint") : t("talk.micOnHint")}>
          <button onClick={(e) => { if (!running && !byPerson(e)) return; void talk.toggleMic(); }} {...personOnly(!running)} className={`${button} ${running ? "bg-[var(--color-bad)]" : "bg-[var(--color-accent)] text-[var(--color-on-accent)]"}`} {...named(running ? t("talk.micOff") : t("talk.micOn"))}>
            {running ? <X size={15} /> : <AudioLines size={15} />}
          </button>
        </Tip>
        <Tip text={t("talk.silenceHint")}>
          <button onClick={talk.silence} disabled={!speaking} className={`${button} bg-white/10 hover:bg-white/20`} {...named(t("talk.silence"))}>
            <Square size={13} />
          </button>
        </Tip>
        <Tip text={t("talk.close")}>
          <button onClick={() => window.location.assign(CLOSE)} className={`${button} hover:bg-white/15`} {...named(t("talk.close"))}>
            <X size={15} />
          </button>
        </Tip>
      </div>
    </main>
  );
}
