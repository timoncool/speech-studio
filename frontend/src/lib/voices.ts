import { useCallback, useEffect, useState } from "react";
import { api, ApiError, type VoiceEntry, type VoiceListing } from "./api";
import { VOICES_CHANGED, useChanged } from "./mcpBridge";
import { DUB_LANGS } from "./i18n";

/** A voice name as people read it: the pack's `RU_Female_` prefix and the underscores go. */
export const prettyVoice = (name: string) => name.replace(/^[a-z]{2}_(female|famale|male)_/i, "").replace(/_/g, " ");

/** A language code's name as the studio lists languages, else the code. */
export const langName = (code: string) => DUB_LANGS.find((l) => l.code === code)?.name ?? code.toUpperCase();

/** A library voice's name as shown: the one its source gives it, else the file's. */
export const voiceLabel = (v: { name: string; card: { display?: string } }) => v.card.display || prettyVoice(v.name);

// the names the library's last listing gives, for the places that know a voice by its file name alone
const shownNames = new Map<string, string>();

/** A voice known by its file name, shown as the library shows it. */
export const libraryVoiceName = (name: string) => shownNames.get(name) ?? prettyVoice(name);

const RECENT = "speech.recentVoices";

/** The library voices last picked in this window, newest first. */
export function recentVoices(): string[] {
  try { return (JSON.parse(localStorage.getItem(RECENT) ?? "[]") as string[]).slice(0, 5); } catch { return []; }
}

export function rememberVoice(name: string): void {
  try { localStorage.setItem(RECENT, JSON.stringify([name, ...recentVoices().filter((v) => v !== name)].slice(0, 5))); } catch { /* the list stays as it was */ }
}

/** The library voice an engine that only clones starts with: in the project's language, a favourite first, then one of
 * the pack; an engine that needs the reference text takes only voices that have it. Null when the library has none. */
export function defaultLibraryVoice(listing: VoiceListing | null, lang: string, needsText: boolean): string | null {
  const fit = (listing?.voices ?? []).filter((v) => v.card.kind !== "preset" && (!needsText || !!v.transcript?.trim()) && (!lang || lang === "auto" || v.card.lang === lang));
  const rank = (v: VoiceEntry) => (v.card.favorite ? 0 : v.card.source === "pack" ? 1 : 2);
  return [...fit].sort((a, b) => rank(a) - rank(b))[0]?.name ?? null;
}

export const errorText = (e: unknown) => (e instanceof ApiError ? e.detail || e.code : e instanceof Error ? e.message : String(e));

/** The voice library, read again whenever the studio says the voices changed. */
export function useVoices(): { listing: VoiceListing | null; error: string | null; reload: () => void; patch: (v: VoiceEntry) => void } {
  const [listing, setListing] = useState<VoiceListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const reload = useCallback(() => {
    api.voices().then((l) => {
      for (const v of l.voices) shownNames.set(v.name, voiceLabel(v));
      setListing(l);
      setError(null);
    }, (e: unknown) => setError(errorText(e)));
  }, []);
  useEffect(() => { reload(); }, [reload]);
  useChanged(VOICES_CHANGED, reload);
  const patch = useCallback((v: VoiceEntry) => {
    setListing((cur) => (cur ? { ...cur, voices: cur.voices.map((x) => (x.name === v.name ? v : x)) } : cur));
  }, []);
  return { listing, error, reload, patch };
}
