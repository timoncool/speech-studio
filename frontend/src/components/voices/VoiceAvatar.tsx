import { api, type VoiceEntry } from "../../lib/api";
import { voiceLabel } from "../../lib/voices";

const TINTS = ["#4cc2ff", "#a5b8ff", "#7ee0c3", "#f7a8c8", "#ffc46b", "#b8a4ff"];

function tint(name: string): string {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return TINTS[h % TINTS.length];
}

/** The voice's photo, or its initial on a colour of its own. */
export default function VoiceAvatar({ voice, size = 44, rev = 0 }: { voice: VoiceEntry; size?: number; rev?: number }) {
  const label = voiceLabel(voice);
  if (voice.photo) {
    return <img src={api.voicePhotoUrl(voice.name, rev)} alt="" width={size} height={size} className="rounded-full object-cover shrink-0 border border-[var(--color-border)]" style={{ width: size, height: size }} />;
  }
  const c = tint(voice.name);
  return (
    <span aria-hidden className="rounded-full shrink-0 grid place-items-center font-semibold select-none"
      style={{ width: size, height: size, fontSize: size * 0.42, color: c, background: `color-mix(in oklab, ${c} 16%, transparent)`, border: `1px solid color-mix(in oklab, ${c} 35%, transparent)` }}>
      {label.slice(0, 1).toUpperCase()}
    </span>
  );
}
