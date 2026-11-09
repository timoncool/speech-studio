import { useCallback, useEffect, useRef, useState } from "react";
import { api, type McpStatus } from "../../lib/api";
import { aloudPlayback, onLiveEvents } from "../../lib/livePlayback";
import { receiveTalkFeed } from "../../lib/talkFeed";
import { setTalkPrefs, talkPrefs } from "../../lib/talkPrefs";
import { talkApi, type HeardLine } from "../../lib/talk";
import { useAfterDownload } from "../../lib/useAfterDownload";
import { errorText } from "../../lib/voices";
import type { TalkState } from "./visual";

export type Turn = { who: "person" | "agent"; text: string; at: number };

/** A spoken conversation with an agent through the studio: what the person says heard live, what the agent says as it
 * starts each phrase, and the state of both. `fed`: the sound plays in another window, which sends its loudness. */
/** Whether another change came while the microphone was restarting (read through a call: the restart awaits). */
const changedAgain = (s: { current: string }) => s.current === "again";

export function useTalk({ fed }: { fed: boolean }) {
  const [running, setRunning] = useState(false);
  const [muted, setMuted] = useState(false);
  const [heard, setHeard] = useState<HeardLine[]>([]);
  const [partial, setPartial] = useState("");
  const [saying, setSaying] = useState<string | null>(null);
  const [voiceLoading, setVoiceLoading] = useState(false);
  const [turns, setTurns] = useState<Turn[]>([]);
  const [agent, setAgent] = useState<McpStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [speaking, setSpeaking] = useState(false);
  const micLevel = useRef(0);
  const fedLevel = useRef(0);
  const fedBands = useRef<number[]>([]);
  const seen = useRef(-1);
  const texts = useRef(new Map<string, string>());

  // what the person says; the listening itself is the studio's, so every window shows the same
  useEffect(() => {
    const id = window.setInterval(() => {
      talkApi.heard().then((h) => {
        setRunning(h.running);
        setMuted(h.muted);
        setHeard(h.lines);
        setPartial(h.partial?.text ?? "");
        if (h.error) setError(h.error);
        else if (h.turn_error) setError(h.turn_error);
        if (seen.current < 0 || h.lines.length < seen.current) seen.current = h.lines.length;
        if (h.lines.length > seen.current) {
          const fresh = h.lines.slice(seen.current).map((l) => ({ who: "person" as const, text: l.text, at: Date.now() }));
          seen.current = h.lines.length;
          setTurns((all) => [...all, ...fresh]);
        }
      }, (e: unknown) => setError(errorText(e)));
    }, 250);
    return () => window.clearInterval(id);
  }, []);

  // what the agent says: each phrase as it starts, its text from its session
  useEffect(() => {
    return onLiveEvents((ev) => {
      if (ev.kind === "voice") { setVoiceLoading(ev.loading !== null); return; }
      if (ev.kind === "asr") { setTalkPrefs({ asr: ev.engine }); return; }
      if (ev.kind === "chosen") return;
      if (ev.kind === "lagged" || ev.kind === "hello" || !ev.aloud) return;
      if (ev.kind === "stop") { setSaying(null); return; }
      if (ev.kind !== "start") return;
      const key = `${ev.project}/${ev.block}`;
      const show = (text: string) => {
        setSaying(text);
        setTurns((all) => all.some((x) => x.who === "agent" && x.text === text && Date.now() - x.at < 60_000) ? all : [...all, { who: "agent", text, at: Date.now() }]);
      };
      const known = texts.current.get(key);
      if (known) { show(known); return; }
      talkApi.session(ev.project).then((s) => {
        for (const p of s.phrases) texts.current.set(`${ev.project}/${p.id}`, p.text);
        const text = texts.current.get(key);
        if (text) show(text);
      }, (e: unknown) => setError(errorText(e)));
    });
  }, []);

  useEffect(() => {
    if (!running) { micLevel.current = 0; return; }
    const id = window.setInterval(() => { talkApi.level().then((l) => { micLevel.current = l.level; }, () => undefined); }, 80);
    return () => window.clearInterval(id);
  }, [running]);

  useEffect(() => {
    if (fed) return receiveTalkFeed((talking, level, bands) => { fedLevel.current = level; fedBands.current = bands; setSpeaking(talking); });
    const id = window.setInterval(() => setSpeaking(aloudPlayback()?.speaking ?? false), 100);
    return () => window.clearInterval(id);
  }, [fed]);

  useEffect(() => {
    const look = () => { api.mcpStatus().then(setAgent, () => setAgent(null)); };
    look();
    const id = window.setInterval(look, 3000);
    return () => window.clearInterval(id);
  }, []);

  const { waiting: fetching, caught } = useAfterDownload(setError);
  const again = useRef<() => void>(() => undefined);
  const listen = useCallback(async () => {
    const { lang, device, asr } = talkPrefs();
    try {
      const r = await talkApi.listenStart(lang, device || undefined, asr || undefined);
      if (r.turn?.note) setError(r.turn.note);
      setRunning(true);
      setMuted(false);
    } catch (e) {
      if (!caught(e, () => again.current())) setError(errorText(e));
    }
  }, [caught]);
  useEffect(() => {
    again.current = () => { void listen(); };
  }, [listen]);

  const toggleMic = useCallback(async () => {
    setError(null);
    if (!running) return listen();
    try {
      // ending the conversation ends what the agent is saying too, as hanging up a call does
      await talkApi.silence();
      await talkApi.listenStop();
      setRunning(false);
      setPartial("");
    } catch (e) {
      setError(errorText(e));
    }
  }, [running, listen]);

  // a recogniser, language or microphone chosen while listening applies at once: the microphone starts again with it
  const switching = useRef<"idle" | "busy" | "again">("idle");
  const relisten = useCallback(async () => {
    if (!running) return;
    // changes made while the microphone restarts end in one restart with the last of them
    if (switching.current !== "idle") {
      switching.current = "again";
      return;
    }
    switching.current = "busy";
    try {
      do {
        switching.current = "busy";
        setError(null);
        await talkApi.listenStop();
        await listen();
      } while (changedAgain(switching));
    } catch (e) {
      setError(errorText(e));
    } finally {
      switching.current = "idle";
    }
  }, [running, listen]);

  const toggleMute = useCallback(async () => {
    setError(null);
    try {
      const paused = (await talkApi.toggleMute()).muted;
      setMuted(paused);
      // a pause stops the agent's speech too: what it says next waits for the person's next phrase
      if (paused) await talkApi.silence();
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  const type = useCallback(async (text: string) => {
    setError(null);
    try {
      await talkApi.type(text);
      return true;
    } catch (e) {
      setError(errorText(e));
      return false;
    }
  }, []);

  const silence = useCallback(() => { talkApi.silence().catch((e: unknown) => setError(errorText(e))); }, []);

  const lastPerson = turns.filter((x) => x.who === "person").at(-1);
  const lastAgent = turns.filter((x) => x.who === "agent").at(-1);
  const state: TalkState = speaking ? "speaking"
    : running && muted ? "muted"
    : partial ? "hearing"
    : running && lastPerson && (!lastAgent || lastAgent.at < lastPerson.at) && agent?.agent_connected ? "thinking"
    : running ? "listening" : "off";
  const loudness = useCallback(
    () => (speaking ? (fed ? fedLevel.current : aloudPlayback()?.level() ?? 0) : micLevel.current),
    [speaking, fed],
  );

  // the agent's voice is heard in bands; the person's microphone is heard by the studio, which tells only its loudness,
  // so their bands are that loudness shaped like a voice: strongest low, a drift so they do not move as one
  const bands = useCallback((n: number): number[] => {
    if (speaking) {
      if (!fed) return aloudPlayback()?.bands(n) ?? new Array<number>(n).fill(0);
      const src = fedBands.current;
      if (!src.length) return new Array<number>(n).fill(fedLevel.current);
      return Array.from({ length: n }, (_, i) => src[Math.min(src.length - 1, Math.floor((i * src.length) / n))]);
    }
    const level = micLevel.current;
    const now = performance.now() / 1000;
    return Array.from({ length: n }, (_, i) => {
      const shape = 1 - 0.6 * (i / Math.max(1, n - 1));
      const drift = 0.75 + 0.25 * Math.sin(now * 7 + i * 1.7);
      return Math.min(1, level * 1.6 * shape * drift);
    });
  }, [speaking, fed]);

  return { state, running, muted, bands, toggleMute, type, heard, partial, saying, turns, lastPerson, lastAgent, agent, error, fetching, voiceLoading, speaking, loudness, toggleMic, relisten, silence };
}
