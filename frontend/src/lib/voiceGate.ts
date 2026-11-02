import { getCoded, postCoded, sendCoded } from "./api";

/** The voice gate: Live starts a conversation only on the owner's voice (and, when set, a wake word). Off by default. */
export type VoiceGate = {
  enabled: boolean;
  /** Cosine at or above which the speech is the owner. */
  threshold: number;
  /** A wake word that must also be heard, or null for any speech from the owner. */
  wake_word: string | null;
  /** Start Speech Studio on login. */
  autostart: boolean;
  /** Whether the owner has enrolled a voice. */
  enrolled: boolean;
  /** The wake words on offer (empty until the wake-word model is installed). */
  wake_words: string[];
};

export const voiceGateApi = {
  get: () => getCoded<VoiceGate>("/voice-gate"),
  set: (patch: Partial<Pick<VoiceGate, "enabled" | "threshold" | "wake_word" | "autostart">>) =>
    postCoded<VoiceGate>("/voice-gate", patch),
  /** Learn the owner's voice from the last microphone recording. */
  enroll: () => postCoded<{ enrolled: boolean; seconds: number }>("/voice-gate/enroll", {}),
  /** Forget the enrolled voice. */
  forget: () => sendCoded<{ enrolled: boolean }>("DELETE", "/voice-gate/owner"),
};
