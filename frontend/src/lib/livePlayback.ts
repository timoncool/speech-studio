import { useEffect } from "react";
import { onTalkPrefs, talkPrefs, type TalkPrefs } from "./talkPrefs";
import { BASE } from "./api";

const KEY = "speech-live-playback";

export function livePlaybackEnabled(): boolean {
  return localStorage.getItem(KEY) !== "0";
}
export function setLivePlaybackEnabled(on: boolean): void {
  localStorage.setItem(KEY, on ? "1" : "0");
  window.dispatchEvent(new Event(KEY));
}

export type LiveEvent =
  | { kind: "start" | "restart"; project: string; block: string; aloud?: boolean; player?: number | null }
  | { kind: "chunk"; project: string; block: string; sr: number; data: string; gain_db?: number; aloud?: boolean; player?: number | null }
  | { kind: "end"; project: string; block: string; ok: boolean; first_ms: number | null; aloud?: boolean; player?: number | null }
  | { kind: "stop"; project: string; aloud: true }
  | { kind: "voice"; loading: string | null; aloud: true }
  | { kind: "chosen"; aloud: true }
  | { kind: "asr"; engine: string; label: string; aloud: true }
  | { kind: "duck"; db: number; aloud: true }
  | { kind: "hello"; id: number }
  | { kind: "lagged"; missed: number };

type LiveListener = (ev: LiveEvent) => void;
const liveListeners = new Set<LiveListener>();
let liveSource: EventSource | null = null;
let livePlayers = 0;

function reconnect(): void {
  liveSource?.close();
  liveSource = null;
  if (!liveListeners.size) return;
  liveSource = new EventSource(`${BASE}/speech/live${livePlayers > 0 ? "?aloud=1" : ""}`);
  liveSource.onmessage = (msg: MessageEvent<string>) => {
    const ev = JSON.parse(msg.data) as LiveEvent;
    for (const listen of [...liveListeners]) listen(ev);
  };
}

/** The studio's live voicing events, from one connection of this window shared by all who listen: a browser keeps six
 * connections to the studio, and a stream for each listener of each window left none for the window's requests.
 * `player`: the listener plays what is said aloud, so the window takes its share of that (`?aloud=1`). */
export function onLiveEvents(listen: LiveListener, player = false): () => void {
  liveListeners.add(listen);
  if (player) livePlayers += 1;
  if (!liveSource || (player && livePlayers === 1)) reconnect();
  return () => {
    liveListeners.delete(listen);
    if (player) livePlayers -= 1;
    if (!liveListeners.size || (player && livePlayers === 0)) reconnect();
  };
}

function samplesOf(data: string): Float32Array<ArrayBuffer> {
  const bin = atob(data);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return new Float32Array(bytes.buffer, 0, bytes.length >> 2);
}

/** Plays pieces back to back as they arrive: one project's blocks (`pid`), or with `null` whatever is voiced aloud on
 * any screen (an agent speaking through the studio); a restarted take or a stop drops what was queued. */
export class LivePlayer {
  private ctx: AudioContext | null = null;
  private out: AnalyserNode | null = null;
  private gain: GainNode | null = null;
  private volume = 1;
  private sink = "";
  private cursor = 0;
  private playing: AudioBufferSourceNode[] = [];
  private readonly pid: string | null;
  private readonly makeContext: () => AudioContext;

  constructor(pid: string | null, makeContext: () => AudioContext = () => new AudioContext()) {
    this.pid = pid;
    this.makeContext = makeContext;
  }

  /** This window's number on the server's aloud stream: of several open windows only the phrase's `player` sounds it. */
  private me: number | null = null;

  private accepts(ev: Exclude<LiveEvent, { kind: "lagged" | "hello" | "voice" | "chosen" | "asr" | "duck" }>): boolean {
    if (this.pid !== null) return !ev.aloud && ev.project === this.pid;
    const player = "player" in ev ? ev.player : undefined;
    return ev.aloud === true && (player == null || player === this.me);
  }

  /** Whether sound is queued or playing now. */
  get speaking(): boolean {
    return this.playing.length > 0;
  }

  /** The loudness of what plays now, 0..1. */
  level(): number {
    if (!this.out) return 0;
    const data = new Float32Array(this.out.fftSize);
    this.out.getFloatTimeDomainData(data);
    let sum = 0;
    for (const v of data) sum += v * v;
    return Math.min(1, Math.sqrt(sum / data.length) * 4);
  }

  /** What plays now split into `n` frequency bands from low to high, each 0..1, spaced as the ear hears pitch. */
  bands(n: number): number[] {
    if (!this.out || !this.ctx) return new Array<number>(n).fill(0);
    const data = new Uint8Array(this.out.frequencyBinCount);
    this.out.getByteFrequencyData(data);
    const nyquist = this.ctx.sampleRate / 2;
    const bin = (hz: number) => Math.min(data.length - 1, Math.max(1, Math.round((hz / nyquist) * data.length)));
    const low = 80;
    const high = 8000;
    return Array.from({ length: n }, (_, i) => {
      const a = bin(low * (high / low) ** (i / n));
      const z = Math.max(a + 1, bin(low * (high / low) ** ((i + 1) / n)));
      let sum = 0;
      for (let k = a; k < z; k++) sum += data[k];
      return sum / (z - a) / 255;
    });
  }

  /** How loud it plays, 0..1; the loudness `level` reports is taken before it, so a muted voice still moves the picture. */
  setVolume(volume: number): void {
    this.volume = volume;
    this.applyGain();
  }

  /** How much quieter than its volume it plays, in dB: the person speaks over the agent and it decides meanwhile. */
  private duck = 1;

  setDuck(db: number): void {
    this.duck = 10 ** (db / 20);
    this.applyGain();
  }

  private applyGain(): void {
    if (!this.gain || !this.ctx) return;
    this.gain.gain.setTargetAtTime(this.volume * this.duck, this.ctx.currentTime, 0.03);
  }

  /** The speaker it plays on, by device id; empty is the system's default. */
  async setOutput(id: string): Promise<void> {
    this.sink = id;
    await this.applyOutput();
  }

  private async applyOutput(): Promise<void> {
    const ctx = this.ctx as (AudioContext & { setSinkId?: (id: string) => Promise<void> }) | null;
    if (!ctx) return;
    if (!ctx.setSinkId) {
      if (this.sink) throw new Error("this browser cannot choose the speaker");
      return;
    }
    await ctx.setSinkId(this.sink);
  }

  handle(ev: LiveEvent): void {
    if (ev.kind === "hello") { this.me = ev.id; return; }
    if (ev.kind === "duck") { if (this.pid === null) this.setDuck(ev.db); return; }
    if (ev.kind === "lagged" || ev.kind === "voice" || ev.kind === "chosen" || ev.kind === "asr" || !this.accepts(ev)) return;
    if (ev.kind === "restart" || ev.kind === "stop") { this.stop(); return; }
    if (ev.kind !== "chunk") return;
    this.ctx ??= this.makeContext();
    if (!this.out) {
      this.out = this.ctx.createAnalyser();
      this.out.fftSize = 1024;
      this.gain = this.ctx.createGain();
      this.gain.gain.value = this.volume * this.duck;
      this.out.connect(this.gain).connect(this.ctx.destination);
      if (this.sink) void this.applyOutput().catch((e: unknown) => console.error("[ERROR] aloud speaker:", e));
    }
    if (this.ctx.state === "suspended") void this.ctx.resume();
    const samples = samplesOf(ev.data);
    if (ev.gain_db) {
      const k = 10 ** (ev.gain_db / 20);
      for (let i = 0; i < samples.length; i++) samples[i] *= k;
    }
    const buffer = this.ctx.createBuffer(1, samples.length, ev.sr);
    buffer.copyToChannel(samples, 0);
    const node = this.ctx.createBufferSource();
    node.buffer = buffer;
    node.connect(this.out);
    const at = Math.max(this.ctx.currentTime + 0.05, this.cursor);
    node.start(at);
    this.cursor = at + buffer.duration;
    this.playing.push(node);
    node.onended = () => { this.playing = this.playing.filter((s) => s !== node); };
  }

  stop(): void {
    for (const s of this.playing) s.stop();
    this.playing = [];
    this.cursor = 0;
  }

  close(): void {
    this.stop();
    void this.ctx?.close();
    this.ctx = null;
    this.out = null;
    this.gain = null;
  }
}

/** Plays the blocks of project `pid` while the engine voices them, one after another, when the setting is on. */
export function useLivePlayback(pid: string): void {
  useEffect(() => {
    const player = new LivePlayer(pid);
    let off: (() => void) | null = null;
    const sync = () => {
      if (livePlaybackEnabled() && !off) {
        off = onLiveEvents((ev) => player.handle(ev));
      } else if (!livePlaybackEnabled() && off) {
        off();
        off = null;
        player.stop();
      }
    };
    sync();
    window.addEventListener(KEY, sync);
    return () => {
      window.removeEventListener(KEY, sync);
      off?.();
      player.close();
    };
  }, [pid]);
}

let aloudPlayer: LivePlayer | null = null;

/** The player of what an agent says aloud through the studio, while the window is open: its state and loudness. */
export function aloudPlayback(): LivePlayer | null {
  return aloudPlayer;
}

/** Plays whatever is voiced aloud (`say_*`, `speak_text` with aloud) on any screen, for as long as the window is open;
 * the open connection is what tells the server there is somewhere to play it. */
export function useAloudPlayback(): void {
  useEffect(() => {
    const player = new LivePlayer(null);
    aloudPlayer = player;
    const apply = (p: TalkPrefs) => {
      player.setVolume(p.muted ? 0 : p.volume);
      player.setOutput(p.output).catch((e: unknown) => console.error("[ERROR] aloud speaker:", e));
    };
    apply(talkPrefs());
    const unfollow = onTalkPrefs(apply);
    const off = onLiveEvents((ev) => player.handle(ev), true);
    return () => {
      unfollow();
      off();
      player.close();
      aloudPlayer = null;
    };
  }, []);
}
