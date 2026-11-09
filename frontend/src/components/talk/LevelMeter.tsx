import { useEffect, useRef } from "react";

const FLOOR_DB = -60;
/** OBS's zones: speech sits around -20 dBFS, -9 dBFS is the most it should reach. */
const ALIGN_DB = -20;
const LIMIT_DB = -9;
/** OBS's default "fast" fall: 40 dB in 1.7 s. */
const FALL_DB_PER_S = 40 / 1.7;
const HOLD_S = 2;

const toDb = (peak: number) => (peak > 0 ? Math.max(FLOOR_DB, 20 * Math.log10(peak)) : FLOOR_DB);
const at = (db: number) => (db - FLOOR_DB) / -FLOOR_DB;

/** A level meter as audio mixers draw one: dBFS from -60 to 0 in green, yellow and red zones, a fast fall, the
 * loudest moment held for two seconds as a mark, and its value in dB. `peak` is read every frame, linear 0..1. */
export default function LevelMeter({ peak, label }: { peak: () => number; label: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const readout = useRef<HTMLSpanElement>(null);
  const peakRef = useRef(peak);
  useEffect(() => {
    peakRef.current = peak;
  });

  useEffect(() => {
    const el = canvas.current;
    const c = el?.getContext("2d");
    if (!el || !c) return;
    let shown = FLOOR_DB;
    let held = FLOOR_DB;
    let heldAt = 0;
    let last = performance.now();
    let frame = 0;
    const style = getComputedStyle(document.documentElement);
    const colour = (name: string) => style.getPropertyValue(name).trim();
    const green = colour("--color-good");
    const yellow = colour("--color-cue-rate");
    const red = colour("--color-bad");
    const track = colour("--color-surface-2");
    const draw = (now: number) => {
      const dt = (now - last) / 1000;
      last = now;
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      const w = Math.max(1, Math.round(el.clientWidth * ratio));
      const h = Math.max(1, Math.round(el.clientHeight * ratio));
      if (el.width !== w || el.height !== h) { el.width = w; el.height = h; }
      const db = toDb(peakRef.current());
      shown = db >= shown ? db : Math.max(db, shown - FALL_DB_PER_S * dt);
      if (db >= held || now - heldAt > HOLD_S * 1000) { held = db; heldAt = now; }
      c.clearRect(0, 0, w, h);
      const segments = 40;
      const gap = Math.max(1, ratio);
      const seg = (w - gap * (segments - 1)) / segments;
      for (let i = 0; i < segments; i++) {
        const segDb = FLOOR_DB + ((i + 0.5) / segments) * -FLOOR_DB;
        const lit = segDb <= shown;
        c.fillStyle = !lit ? track : segDb < ALIGN_DB ? green : segDb < LIMIT_DB ? yellow : red;
        c.globalAlpha = lit ? 1 : 0.6;
        c.fillRect(i * (seg + gap), 0, seg, h);
      }
      c.globalAlpha = 1;
      if (held > FLOOR_DB) {
        c.fillStyle = held < ALIGN_DB ? green : held < LIMIT_DB ? yellow : red;
        c.fillRect(Math.min(w - 2 * ratio, at(held) * w), 0, 2 * ratio, h);
      }
      if (readout.current) readout.current.textContent = held > FLOOR_DB ? `${held.toFixed(0)} dB` : "-∞ dB";
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, []);

  return (
    <div className="flex items-center gap-2" role="meter" aria-label={label} aria-valuemin={FLOOR_DB} aria-valuemax={0}>
      <canvas ref={canvas} className="h-2.5 flex-1 rounded-sm" />
      <span ref={readout} className="mono w-12 shrink-0 text-right text-[10px] text-[var(--color-muted)]" />
    </div>
  );
}
