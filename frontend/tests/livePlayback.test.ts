import { describe, expect, it } from "vitest";
import { LivePlayer, livePlaybackEnabled, setLivePlaybackEnabled } from "../src/lib/livePlayback";

class FakeNode {
  buffer: { duration: number } | null = null;
  startedAt: number | null = null;
  stopped = false;
  onended: (() => void) | null = null;
  connect() {}
  start(at: number) { this.startedAt = at; }
  stop() { this.stopped = true; }
}

function fakeContext() {
  const nodes: FakeNode[] = [];
  const buffers: { samples: Float32Array; rate: number }[] = [];
  const ctx = {
    currentTime: 10,
    state: "suspended",
    resumed: 0,
    resume: async () => { ctx.resumed++; ctx.state = "running"; },
    destination: {},
    createBuffer: (_channels: number, length: number, rate: number) => {
      const b = { samples: new Float32Array(length), rate, duration: length / rate, copyToChannel(s: Float32Array) { b.samples.set(s); } };
      buffers.push(b);
      return b;
    },
    createBufferSource: () => { const n = new FakeNode(); nodes.push(n); return n; },
    createAnalyser: () => ({ fftSize: 0, connect: (next: unknown) => next, getFloatTimeDomainData() {} }),
    createGain: () => ({ gain: { value: 1 }, connect: (next: unknown) => next }),
    close: async () => {},
  };
  return { ctx: ctx as unknown as AudioContext, raw: ctx, nodes, buffers };
}

const piece = (values: number[]) => btoa(String.fromCharCode(...new Uint8Array(new Float32Array(values).buffer)));

describe("live playback", () => {
  it("plays what is voiced aloud on any screen and leaves it to no project's player", () => {
    const fake = fakeContext();
    const aloud = new LivePlayer(null, () => fake.ctx);
    const editor = new LivePlayer("p1", () => fake.ctx);
    const said = { kind: "chunk" as const, project: "say1", block: "b1", sr: 24000, data: piece([0.1, 0.2]), aloud: true };
    aloud.handle(said);
    editor.handle({ ...said, project: "p1" });
    aloud.handle({ kind: "chunk", project: "p1", block: "b2", sr: 24000, data: piece([0.3]) });
    expect(fake.nodes.length).toBe(1);
    expect(aloud.speaking).toBe(true);
    aloud.handle({ kind: "stop", project: "say1", aloud: true });
    expect(fake.nodes[0].stopped).toBe(true);
    expect(aloud.speaking).toBe(false);
  });

  it("of two open windows only the phrase's player sounds it", () => {
    const first = fakeContext();
    const second = fakeContext();
    const a = new LivePlayer(null, () => first.ctx);
    const b = new LivePlayer(null, () => second.ctx);
    a.handle({ kind: "hello", id: 2 });
    b.handle({ kind: "hello", id: 3 });
    const said = { kind: "chunk" as const, project: "say1", block: "b1", sr: 24000, data: piece([0.1]), aloud: true, player: 2 };
    a.handle(said);
    b.handle(said);
    expect(first.nodes).toHaveLength(1);
    expect(second.nodes).toHaveLength(0);
  });

  it("plays the pieces of its project back to back and drops a restarted take", () => {
    const fake = fakeContext();
    const player = new LivePlayer("p1", () => fake.ctx);
    player.handle({ kind: "start", project: "p1", block: "b1" });
    player.handle({ kind: "chunk", project: "p1", block: "b1", sr: 4, data: piece([0.5, -0.5, 0.25, 0]) });
    player.handle({ kind: "chunk", project: "p1", block: "b1", sr: 4, data: piece([1, 1]) });
    player.handle({ kind: "chunk", project: "other", block: "x", sr: 4, data: piece([1]) });
    expect(fake.nodes).toHaveLength(2);
    expect(fake.raw.resumed).toBe(1);
    expect(Array.from(fake.buffers[0].samples)).toEqual([0.5, -0.5, 0.25, 0]);
    expect(fake.nodes[0].startedAt).toBeCloseTo(10.05);
    expect(fake.nodes[1].startedAt).toBeCloseTo(11.05);
    player.handle({ kind: "restart", project: "p1", block: "b1" });
    expect(fake.nodes.every((n) => n.stopped)).toBe(true);
    player.handle({ kind: "chunk", project: "p1", block: "b1", sr: 4, data: piece([0]) });
    expect(fake.nodes[2].startedAt).toBeCloseTo(10.05);
  });

  it("plays a quiet voice's piece up by the gain the studio measured for it", () => {
    const fake = fakeContext();
    const player = new LivePlayer("p1", () => fake.ctx);
    player.handle({ kind: "chunk", project: "p1", block: "b1", sr: 4, data: piece([0.1, -0.05]), gain_db: 12 });
    player.handle({ kind: "chunk", project: "p1", block: "b2", sr: 4, data: piece([0.1]), gain_db: 0 });
    const k = 10 ** (12 / 20);
    expect(fake.buffers[0].samples[0]).toBeCloseTo(0.1 * k, 5);
    expect(fake.buffers[0].samples[1]).toBeCloseTo(-0.05 * k, 5);
    expect(fake.buffers[1].samples[0]).toBeCloseTo(0.1, 6);
  });

  it("remembers the setting, on by default", () => {
    localStorage.removeItem("speech-live-playback");
    expect(livePlaybackEnabled()).toBe(true);
    setLivePlaybackEnabled(false);
    expect(livePlaybackEnabled()).toBe(false);
    setLivePlaybackEnabled(true);
  });
});
