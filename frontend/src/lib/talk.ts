import { getCoded, getCodedPolled, postCoded, sendCoded } from "./api";
import type { SpeechParams } from "./speech";

export type SayPhrase = { id: string; text: string; state: "said" | "waiting" | "failed" | "interrupted"; first_ms: number | null; seconds: number | null };
/** The voice agents speak aloud with when they give none: chosen in Live's sound window. */
export type TalkVoice = { engine: string; voice: string | null; params: SpeechParams | null };
export type HeardLine = { start: number; end: number; text: string };
export type LiveHeard = { running: boolean; lines: HeardLine[]; partial: { start: number; text: string } | null; error: string | null; muted: boolean };

/** A role for a Live conversation: a ready idea of who the agent plays, a file in the studio's roles folder. */
export type LiveRole = {
  id: string;
  name: string;
  goal?: string | null;
  opens?: string | null;
  turns?: number;
  difficulty?: number;
  checks?: string[];
  notes?: string[];
  problem?: string;
  last_review?: { at: string; goal: string | null } | null;
  turns_taken?: number;
};
export type LiveRoles = { folder: string; roles: LiveRole[]; playing: LiveRole | null };

/** The spoken conversation: the microphone heard live, and what an agent says aloud through the studio. */
export const talkApi = {
  listenStart: (lang: string, device?: string, engine?: string) => postCoded<{ pid: string }>("/speech/listen", { lang, device, engine }),
  listenStop: () => sendCoded<unknown>("DELETE", "/speech/listen"),
  toggleMute: () => postCoded<{ muted: boolean }>("/speech/listen/mute", { toggle: true }),
  type: (text: string) => postCoded<{ ok: boolean }>("/speech/listen/text", { text }),
  heard: () => getCodedPolled<LiveHeard>("/listen/record/live"),
  level: () => getCodedPolled<{ level: number }>("/record/level"),
  voice: () => getCoded<TalkVoice>("/speech/say/voice"),
  setVoice: (v: TalkVoice) => postCoded<TalkVoice>("/speech/say/voice", v),
  micTest: (device: string) => postCoded<{ testing: boolean }>("/record/test", { device }),
  micTestStop: () => postCoded<{ seconds: number; peak: number }>("/record/test/stop", {}),
  session: (id: string) => getCoded<{ session: string; open: boolean; phrases: SayPhrase[] }>(`/speech/say/${encodeURIComponent(id)}`),
  silence: () => sendCoded<unknown>("DELETE", "/speech/say"),
  roles: () => getCodedPolled<LiveRoles>("/speech/roles"),
  setRole: (id: string | null) => postCoded<{ playing: LiveRole | null }>("/speech/role", { id }),
};
