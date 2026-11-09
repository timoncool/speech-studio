import { useEffect, useRef } from "react";
import SiriWave from "siriwave";
import type { TalkState, VisualProps } from "./visual";

const AMPLITUDE: Record<TalkState, number> = { off: 0.1, muted: 0.3, listening: 1.0, hearing: 1.5, thinking: 1.2, speaking: 2.0 };
const SPEED: Record<TalkState, number> = { off: 0.03, muted: 0.04, listening: 0.08, hearing: 0.16, thinking: 0.12, speaking: 0.2 };

/** The talk screen's wave: Siri's iOS 9 curves (SiriWave, MIT), louder voice taller curves. */
export default function Wave({ state, level, className }: VisualProps) {
  const box = useRef<HTMLDivElement>(null);
  const stateRef = useRef(state);
  const levelRef = useRef(level);
  useEffect(() => {
    stateRef.current = state;
    levelRef.current = level;
  });

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    let wave: SiriWave | null = null;
    let frame = 0;
    let shown = 0;
    const make = () => {
      wave?.dispose();
      wave = new SiriWave({ container: el, style: "ios9", width: el.clientWidth, height: el.clientHeight, autostart: true, cover: true, amplitude: 1, speed: 0.08 });
    };
    make();
    const resize = new ResizeObserver(make);
    resize.observe(el);
    const calm = window.matchMedia("(prefers-reduced-motion: reduce)");
    let last = performance.now();
    const tick = (time: number) => {
      const dt = Math.min(0.1, (time - last) / 1000);
      last = time;
      const s = stateRef.current;
      const reactive = s === "speaking" || s === "hearing";
      const goal = reactive ? Math.min(3, 0.6 + levelRef.current() * 5) : AMPLITUDE[s];
      // the loudness arrives in steps: rise fast, fall slowly, so the curves swell instead of twitching
      shown += (goal - shown) * (1 - Math.exp(-dt / (goal > shown ? 0.08 : 0.35)));
      wave?.setAmplitude(shown);
      wave?.setSpeed(SPEED[s] * (calm.matches ? 0.3 : 1));
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => { cancelAnimationFrame(frame); resize.disconnect(); wave?.dispose(); };
  }, []);

  return <div ref={box} className={className} aria-hidden />;
}
