import { getCoded, postCoded, sendCoded } from "./api";

/** The voice gate: Live starts a conversation only on the owner's voice (and, when set, a wake word). Off by default. */
export type VoiceGate = {
  enabled: boolean;
  /** Cosine at or above which the speech is the owner. */
  threshold: number;
  /** A wake word that, when set, must be heard before the conversation starts; null for any speech (gated by voice). */
  wake_word: string | null;
  /** The wake phrase's language for grapheme-to-phoneme (an eSpeak-ng voice code such as "ru", "en"); null = English. */
  wake_lang: string | null;
  /** Phone overlap at or above which the wake phrase counts as heard — its strictness, set by the slider. */
  wake_threshold: number;
  /** Start Speech Studio on login. */
  autostart: boolean;
  /** Whether the owner has enrolled a voice. */
  enrolled: boolean;
  /** Example wake phrases for the field (hints only — any phrase in any language works). */
  wake_words: string[];
  /** Set only in the reply to a save: the wake-word recogniser is being downloaded in the background. */
  wake_downloading?: boolean;
};

export const voiceGateApi = {
  get: () => getCoded<VoiceGate>("/voice-gate"),
  set: (patch: Partial<Pick<VoiceGate, "enabled" | "threshold" | "wake_word" | "wake_lang" | "wake_threshold" | "autostart">>) =>
    postCoded<VoiceGate>("/voice-gate", patch),
  /** Learn the owner's voice from the last microphone recording; `text` is what the recogniser heard, shown back so the
   * person sees their voice came through. */
  enroll: () => postCoded<{ enrolled: boolean; seconds: number; text: string }>("/voice-gate/enroll", {}),
  /** Forget the enrolled voice. */
  forget: () => sendCoded<{ enrolled: boolean }>("DELETE", "/voice-gate/owner"),
  /** Start or stop the live voice check: the microphone records and recent speech is matched against the enrolled voice. */
  test: (on: boolean) => postCoded<{ testing: boolean }>("/voice-gate/test", { on }),
  /** The live match of the voice check against the enrolled voice (0..1), with the configured threshold; `ready` false until enough speech. */
  score: () => getCoded<{ score: number | null; threshold: number; ready: boolean }>("/voice-gate/score"),
  /** The live phone overlap of the voice check with the configured wake phrase (0..1), with the wake threshold; `ready` false until enough speech or no phrase set. */
  wakeScore: () => getCoded<{ score: number | null; threshold: number; ready: boolean }>("/voice-gate/wake-score"),
};
