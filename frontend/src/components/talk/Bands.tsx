import { useEffect, useRef } from "react";
import { readTheme, type TalkState, type VisualProps } from "./visual";

export type BandsLook = "bars" | "radial" | "matrix";

const rgba = ([r, g, b]: [number, number, number], a: number) => `rgba(${Math.round(r * 255)},${Math.round(g * 255)},${Math.round(b * 255)},${a})`;

/** Lit cells while nobody speaks, as LiveKit's agent visualisers move: one cell blinking in the middle while listening,
 * a light running across while thinking, nothing lit when off. `t` in seconds. */
function idle(state: TalkState, t: number, count: number): (i: number) => number {
  const middle = Math.floor(count / 2);
  if (state === "listening") return (i) => (i === middle && Math.floor(t / 0.5) % 2 === 0 ? 0.55 : 0.08);
  if (state === "thinking") {
    const run = Math.floor(t / 0.09) % (count * 2 - 2);
    const at = run < count ? run : count * 2 - 2 - run;
    return (i) => Math.max(0.08, 0.7 - Math.abs(i - at) * 0.25);
  }
  if (state === "muted") return () => 0.05;
  return () => 0.08;
}

function drawBars(c: CanvasRenderingContext2D, w: number, h: number, levels: number[], color: [number, number, number]) {
  const n = levels.length;
  const bar = Math.min(h * 0.16, w / (n * 2));
  const gap = bar * 0.8;
  const left = (w - (n * bar + (n - 1) * gap)) / 2;
  levels.forEach((v, i) => {
    const height = Math.max(bar, v * h * 0.9);
    const x = left + i * (bar + gap);
    const y = (h - height) / 2;
    c.fillStyle = rgba(color, 0.35 + 0.65 * Math.min(1, v * 1.5));
    c.beginPath();
    c.roundRect(x, y, bar, height, bar / 2);
    c.fill();
  });
}

function drawRadial(c: CanvasRenderingContext2D, w: number, h: number, levels: number[], color: [number, number, number], t: number) {
  const n = levels.length;
  const cx = w / 2;
  const cy = h / 2;
  const inner = Math.min(w, h) * 0.22;
  const longest = Math.min(w, h) * 0.24;
  c.lineCap = "round";
  c.lineWidth = Math.max(2, Math.min(w, h) * 0.025);
  levels.forEach((v, i) => {
    const angle = (i / n) * Math.PI * 2 - Math.PI / 2 + t * 0.15;
    const length = c.lineWidth + v * longest;
    c.strokeStyle = rgba(color, 0.3 + 0.7 * Math.min(1, v * 1.5));
    c.beginPath();
    c.moveTo(cx + Math.cos(angle) * inner, cy + Math.sin(angle) * inner);
    c.lineTo(cx + Math.cos(angle) * (inner + length), cy + Math.sin(angle) * (inner + length));
    c.stroke();
  });
}

function drawMatrix(c: CanvasRenderingContext2D, w: number, h: number, levels: number[], color: [number, number, number], speaking: boolean, glow: number[]) {
  const cols = levels.length;
  const rows = 7;
  const cell = Math.min(w / cols, h / rows);
  const dot = cell * 0.36;
  const ox = (w - cell * cols) / 2;
  const oy = (h - cell * rows) / 2;
  for (let col = 0; col < cols; col++) {
    const lit = Math.round(Math.min(1, levels[col] * 1.4) * rows);
    for (let row = 0; row < rows; row++) {
      const fromBottom = rows - 1 - row;
      const on = speaking ? (fromBottom < lit ? 1 : 0.08) : row === Math.floor(rows / 2) ? glow[col] : 0.06;
      const x = ox + col * cell + cell / 2;
      const y = oy + row * cell + cell / 2;
      c.shadowBlur = on > 0.5 ? dot * 2.2 : 0;
      c.shadowColor = rgba(color, 0.9);
      c.fillStyle = rgba(color, on);
      c.beginPath();
      c.arc(x, y, dot, 0, Math.PI * 2);
      c.fill();
    }
  }
  c.shadowBlur = 0;
}

/** The talk screen's frequency looks: bars and a radial of bands (LiveKit Agents UI's Bar and Radial visualisers), and
 * a dot matrix lit like a level meter (ElevenLabs UI's Matrix). Each band follows the voice of whoever speaks; with
 * nobody speaking they move as LiveKit's do for listening and thinking. */
export default function Bands({ look, state, bands, token, className }: VisualProps & { look: BandsLook }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const stateRef = useRef(state);
  const bandsRef = useRef(bands);
  const tokenRef = useRef(token);
  useEffect(() => {
    stateRef.current = state;
    bandsRef.current = bands;
    tokenRef.current = token;
  });

  useEffect(() => {
    const el = canvas.current;
    const c = el?.getContext("2d");
    const probe = document.createElement("canvas").getContext("2d");
    if (!el || !c || !probe) return;
    const count = look === "bars" ? 5 : look === "radial" ? 24 : 15;
    const shown = new Array<number>(count).fill(0);
    let theme = readTheme(tokenRef.current, probe);
    let looked = performance.now();
    const calm = window.matchMedia("(prefers-reduced-motion: reduce)");
    let frame = 0;
    const draw = (now: number) => {
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      const w = Math.max(1, Math.round(el.clientWidth * ratio));
      const h = Math.max(1, Math.round(el.clientHeight * ratio));
      if (el.width !== w || el.height !== h) { el.width = w; el.height = h; }
      if (now - looked > 500) { theme = readTheme(tokenRef.current, probe); looked = now; }
      const s = stateRef.current;
      const t = (now / 1000) * (calm.matches ? 0.3 : 1);
      const speaking = s === "speaking" || s === "hearing";
      const goal = speaking ? bandsRef.current(count) : Array.from({ length: count }, (_, i) => idle(s, t, count)(i));
      for (let i = 0; i < count; i++) shown[i] += ((goal[i] ?? 0) - shown[i]) * (goal[i] > shown[i] ? 0.5 : 0.2);
      c.clearRect(0, 0, w, h);
      if (look === "bars") drawBars(c, w, h, shown, theme.color);
      else if (look === "radial") drawRadial(c, w, h, shown, theme.color, t);
      else drawMatrix(c, w, h, shown, theme.color, speaking, shown);
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, [look]);

  return <canvas ref={canvas} className={className} aria-hidden />;
}
