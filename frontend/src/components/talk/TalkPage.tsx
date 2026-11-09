import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AppWindow, Copy, History } from "lucide-react";
import { named } from "../../lib/a11y";
import { setTalkPrefs, useTalkPrefs } from "../../lib/talkPrefs";
import Hint, { Tip } from "../Hint";
import TalkControls from "./TalkControls";
import TalkVisual from "./TalkVisual";
import { useTalk } from "./useTalk";
import { TALK_STYLES, type TalkStyle } from "./visual";

const CONNECT = "claude mcp add --transport http speech-studio http://127.0.0.1:8797/mcp";

/** The talk widget: a small bar with no frame and no background, over every other window in the desktop app. */
function openTalkWidget() {
  window.open(`${window.location.origin}/?widget`, "speech-talk-widget", "popup,width=420,height=96");
}

/** A spoken conversation with an agent through the studio: the person's words heard live, the agent's answer said aloud
 * as it writes it, the state of both, and the conversation as text. */
export default function TalkPage() {
  const { t } = useTranslation();
  const [history, setHistory] = useState(true);
  const prefs = useTalkPrefs();
  const look: TalkStyle = TALK_STYLES.includes(prefs.style as TalkStyle) ? (prefs.style as TalkStyle) : "aura";
  const talk = useTalk({ fed: false });
  const { state, running, heard, partial, saying, turns, lastPerson, lastAgent, agent, error, fetching, voiceLoading, loudness } = talk;
  const side = "grid place-items-center w-11 h-11 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)] hover:border-[var(--color-accent)]";

  return (
    <main className="flex-1 min-h-0 flex" data-mcp-page={t("talk.title")}>
      <section className="flex-1 min-w-0 min-h-0 flex flex-col items-center gap-3 px-4 py-4 md:py-6">
        <div className="w-full max-w-3xl shrink-0 flex items-center justify-between gap-3">
          <h1 className="text-[15px] font-semibold">{t("talk.title")} <Hint text={t("talk.hint")} /></h1>
          <div role="radiogroup" aria-label={t("talk.style")} className="flex items-center gap-0.5 rounded-full border border-[var(--color-border)] bg-[var(--color-surface)] p-0.5 text-[12px]">
            {TALK_STYLES.map((s) => (
              <button key={s} role="radio" aria-checked={look === s} onClick={() => setTalkPrefs({ style: s })} className={`rounded-full px-3 py-1 transition-colors ${look === s ? "bg-[var(--color-accent)] text-[var(--color-on-accent)]" : "text-[var(--color-muted)] hover:text-[var(--color-text)]"}`}>
                {t(`talk.style_${s}`)}
              </button>
            ))}
          </div>
          <span className={`inline-flex items-center gap-1.5 text-[12px] ${agent?.agent_connected ? "text-[var(--color-good)]" : "text-[var(--color-muted)]"}`}>
            <span className="w-2 h-2 rounded-full" style={{ background: agent?.agent_connected ? "var(--color-good)" : "var(--color-muted)" }} aria-hidden />
            {agent?.agent_connected ? (agent.agent_name ? t("talk.agentNamed", { name: agent.agent_name }) : t("talk.agentOn")) : t("talk.agentOff")}
          </span>
        </div>

        <div className="flex-1 min-h-0 w-full max-w-3xl overflow-y-auto flex flex-col items-center justify-center gap-4">
          <TalkVisual look={look} state={state} level={loudness} bands={talk.bands} token={state === "thinking" ? "--color-accent-2" : "--color-accent"} className={look === "wave" || look === "bars" || look === "matrix" ? "shrink-0 h-[clamp(7rem,26vh,14rem)] w-full max-w-2xl" : "shrink-0 h-[clamp(8rem,34vh,20rem)] aspect-square"} />
          <p role="status" aria-live="polite" className="text-[15px] font-medium">{t(`talk.state_${state}`)}</p>
          <div className="w-full space-y-2 text-center">
            <p className="text-[13px] text-[var(--color-muted)]">{partial || lastPerson?.text || (running ? t("talk.saySomething") : "")}</p>
            <p className="text-[18px] md:text-[20px] leading-snug">{saying ?? lastAgent?.text ?? ""}</p>
          </div>
          {!agent?.agent_connected && (
            <div className="w-full max-w-xl rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)] p-4 space-y-2 text-[13px]">
              <p>{t("talk.connect")}</p>
              <div className="flex items-center gap-2">
                <code className="flex-1 min-w-0 truncate mono text-[12px] bg-[var(--color-surface-2)] rounded-md px-2 py-1.5">{CONNECT}</code>
                <Tip text={t("talk.copy")}><button onClick={() => void navigator.clipboard.writeText(CONNECT)} className="p-2 rounded-md border border-[var(--color-border)] hover:border-[var(--color-accent)]" {...named(t("talk.copy"))}><Copy size={14} /></button></Tip>
              </div>
              <p className="text-[var(--color-muted)]">{t("talk.ask")}</p>
            </div>
          )}
          {voiceLoading && <p role="status" className="text-[13px] text-[var(--color-muted)] text-center">{t("talk.voiceLoading")}</p>}
          {fetching && <p role="status" className="text-[13px] text-[var(--color-muted)] text-center">{fetching}</p>}
          {error && <p role="alert" className="text-[13px] text-[var(--color-warn)] text-center">{error}</p>}
        </div>

        <TalkControls talk={talk}>
          <Tip text={t("talk.widgetHint")}>
            <button onClick={openTalkWidget} className={side} {...named(t("talk.widget"))}><AppWindow size={16} /></button>
          </Tip>
          <Tip text={t("talk.historyHint")}>
            <button onClick={() => setHistory((v) => !v)} aria-pressed={history} className={side} {...named(t("talk.history"))}><History size={16} /></button>
          </Tip>
        </TalkControls>
      </section>

      {history && (
        <aside aria-label={t("talk.history")} className="hidden lg:flex w-80 shrink-0 flex-col border-l border-[var(--color-border)] bg-[var(--color-surface)]/60">
          <h2 className="px-4 py-3 text-[12px] font-semibold text-[var(--color-muted)] border-b border-[var(--color-border)]">{t("talk.history")}</h2>
          <ol className="flex-1 overflow-y-auto p-3 space-y-2 text-[13px]">
            {turns.length === 0 && <li className="text-[var(--color-muted)]">{t("talk.empty")}</li>}
            {turns.map((x, i) => (
              <li key={i} className={`rounded-lg px-3 py-2 ${x.who === "person" ? "bg-[var(--color-surface-2)]" : "border border-[var(--color-border)]"}`}>
                <span className="block text-[11px] text-[var(--color-muted)]">{x.who === "person" ? t("talk.you") : agent?.agent_name ?? t("talk.agent")}</span>
                {x.text}
              </li>
            ))}
          </ol>
          <p className="px-4 py-2 text-[11px] text-[var(--color-muted)] border-t border-[var(--color-border)]">{t("talk.heardCount", { n: heard.length })}</p>
        </aside>
      )}
    </main>
  );
}
