/** The studio's own volume over everything it plays: media elements and Web Audio graphs alike. Each element keeps the
 * volume its code set; what it plays is that times the studio's level. Kept in this window's storage. */

const KEY = "speech-studio:volume";

let level = readLevel();
const elements = new Set<WeakRef<HTMLMediaElement>>();
const gains = new Set<WeakRef<GainNode>>();
const ownVolume = new WeakMap<HTMLMediaElement, number>();
const outputOf = new WeakMap<BaseAudioContext, GainNode>();

function readLevel(): number {
  try {
    const v = Number(localStorage.getItem(KEY));
    return localStorage.getItem(KEY) !== null && Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 1;
  } catch {
    return 1;
  }
}

/** Fired on the window whenever the studio's level changes, so every control (Settings, the player) stays in step;
 * detail is the new level in 0..1. */
export const MASTER_VOLUME_EVENT = "speech-studio:volume-changed";

export function masterVolume(): number {
  return level;
}

export function setMasterVolume(v: number): void {
  level = Math.min(1, Math.max(0, v));
  try { localStorage.setItem(KEY, String(level)); } catch { /* the level is only remembered where storage works */ }
  for (const ref of elements) {
    const el = ref.deref();
    if (!el) { elements.delete(ref); continue; }
    nativeVolume?.set?.call(el, (ownVolume.get(el) ?? 1) * level);
  }
  for (const ref of gains) {
    const g = ref.deref();
    if (!g) { gains.delete(ref); continue; }
    g.gain.value = level;
  }
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(MASTER_VOLUME_EVENT, { detail: level }));
  }
}

const nativeVolume = typeof HTMLMediaElement !== "undefined" ? Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "volume") : undefined;

function track(el: HTMLMediaElement) {
  if (!ownVolume.has(el)) {
    ownVolume.set(el, 1);
    elements.add(new WeakRef(el));
  }
}

/** Routes every element and every Web Audio output through the studio's level; called once at startup. */
export function installMasterVolume(): void {
  if (typeof HTMLMediaElement === "undefined" || !nativeVolume?.set || !nativeVolume.get) return;
  const { get, set } = nativeVolume;
  Object.defineProperty(HTMLMediaElement.prototype, "volume", {
    configurable: true,
    get(this: HTMLMediaElement) { track(this); return ownVolume.get(this) ?? get.call(this); },
    set(this: HTMLMediaElement, v: number) { track(this); ownVolume.set(this, v); set.call(this, v * level); },
  });
  const play = HTMLMediaElement.prototype.play;
  HTMLMediaElement.prototype.play = function (this: HTMLMediaElement) {
    track(this);
    set.call(this, (ownVolume.get(this) ?? 1) * level);
    return play.call(this);
  };
  if (typeof AudioNode === "undefined") return;
  const connect = AudioNode.prototype.connect as (this: AudioNode, ...args: unknown[]) => unknown;
  AudioNode.prototype.connect = function (this: AudioNode, ...args: unknown[]) {
    const dest = args[0];
    if (dest instanceof AudioDestinationNode && !(this instanceof GainNode && outputOf.get(this.context) === this)) {
      let out = outputOf.get(this.context);
      if (!out) {
        out = this.context.createGain();
        out.gain.value = level;
        connect.call(out, dest);
        outputOf.set(this.context, out);
        gains.add(new WeakRef(out));
      }
      return connect.call(this, out, ...args.slice(1));
    }
    return connect.call(this, ...args);
  } as typeof AudioNode.prototype.connect;
}
