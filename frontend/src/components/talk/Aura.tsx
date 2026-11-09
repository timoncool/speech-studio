import { useEffect, useRef } from "react";
import { fitCanvas, fullscreenProgram, readTheme, type TalkState, type VisualProps } from "./visual";

type Look = { speed: number; scale: number; amplitude: number; frequency: number; brightness: number; pulse: number };

const LOOKS: Record<TalkState, Look> = {
  off: { speed: 10, scale: 0.2, amplitude: 1.2, frequency: 0.4, brightness: 0.7, pulse: 0 },
  muted: { speed: 8, scale: 0.25, amplitude: 1.0, frequency: 0.4, brightness: 0.9, pulse: 0 },
  listening: { speed: 20, scale: 0.3, amplitude: 1.0, frequency: 0.7, brightness: 1.75, pulse: 0.25 },
  hearing: { speed: 35, scale: 0.3, amplitude: 0.9, frequency: 0.9, brightness: 1.8, pulse: 0 },
  thinking: { speed: 30, scale: 0.3, amplitude: 0.5, frequency: 1.0, brightness: 1.5, pulse: 1.0 },
  speaking: { speed: 70, scale: 0.3, amplitude: 0.75, frequency: 1.25, brightness: 1.5, pulse: 0 },
};

const VERTEX = "attribute vec2 p; void main() { gl_Position = vec4(p, 0.0, 1.0); }";

const FRAGMENT = `precision highp float;
uniform vec2 iResolution;
uniform float iTime, uSpeed, uScale, uAmplitude, uFrequency, uMix, uMode, uColorShift;
uniform vec3 uColor;
const float TAU = 6.283185;
const float BLUR = 0.2;
const float SPACING = 0.5;
const float VARIANCE = 0.1;

vec2 randFibo(vec2 p) {
  p = fract(p * vec2(443.897, 441.423));
  p += dot(p, p.yx + 19.19);
  return fract((p.xx + p.yx) * p.xy);
}
vec3 tonemap(vec3 x) { x *= 4.0; return x / (1.0 + x); }
float luma(vec3 c) { return dot(c, vec3(0.299, 0.587, 0.114)); }
vec3 rgb2hsv(vec3 c) {
  vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
  vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
  vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
  float d = q.x - min(q.w, q.y);
  return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + 1.0e-10)), d / (q.x + 1.0e-10), q.x);
}
vec3 hsv2rgb(vec3 c) {
  vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
  return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}
vec2 turb(vec2 pos, float t, float it) {
  mat2 rotation = mat2(0.6, -0.25, 0.25, 0.9);
  mat2 layerRotation = mat2(0.6, -0.8, 0.8, 0.6);
  float frequency = mix(2.0, 15.0, uFrequency);
  float amplitude = uAmplitude;
  float animTime = t * 0.1 * uSpeed;
  for (int i = 0; i < 4; i++) {
    vec2 wave = sin(frequency * (pos * rotation) + float(i) * animTime + it);
    pos += (amplitude / frequency) * rotation[0] * wave;
    rotation *= layerRotation;
    amplitude *= mix(1.0, max(wave.x, wave.y), VARIANCE);
    frequency *= 1.4;
  }
  return pos;
}
const float ITERATIONS = 36.0;
void main() {
  vec2 uv = gl_FragCoord.xy / iResolution.xy;
  vec3 pp = vec3(0.0);
  vec3 bloom = vec3(0.0);
  float t = iTime * 0.5;
  vec2 pos = uv - 0.5;
  vec2 prevPos = turb(pos, t, -1.0 / ITERATIONS);
  float spacing = mix(1.0, TAU, SPACING);
  vec3 hsv = rgb2hsv(uColor);
  for (float i = 1.0; i < ITERATIONS + 1.0; i++) {
    float iter = i / ITERATIONS;
    vec2 st = turb(pos, t, iter * spacing);
    float d = abs(length(st) - uScale);
    float pd = distance(st, prevPos);
    prevPos = st;
    float dynamicBlur = exp2(pd * 2.0 * 1.4426950408889634) - 1.0;
    float ds = smoothstep(0.0, BLUR * 0.05 + max(dynamicBlur, 0.001), d);
    vec3 color = uColor;
    if (uColorShift > 0.01) color = hsv2rgb(vec3(fract(hsv.x + (1.0 - iter) * uColorShift * 0.3), hsv.yz));
    pp += (ds - 1.0) * color;
    bloom += clamp(1.0 / max(d + dynamicBlur, 0.001), 0.0, 250.0) * color;
  }
  pp *= 1.0 / ITERATIONS;
  float fade = 1.0 - smoothstep(0.34, 0.5, length(uv - 0.5));
  vec3 color;
  if (uMode < 0.5) {
    color = -pp * 1.2;
    color += (randFibo(gl_FragCoord.xy).x - 0.5) / 255.0;
    color = tonemap(color);
    gl_FragColor = vec4(color * uMix, luma(color) * uMix) * fade;
  } else {
    color = -pp;
    color += (randFibo(gl_FragCoord.xy).x - 0.5) / 255.0;
    float brightness = length(color);
    vec3 direction = brightness > 0.0 ? color / brightness : color;
    float mapped = (brightness * 2.0) / (1.0 + brightness * 2.0);
    color = direction * mapped;
    color = clamp(mix(vec3(dot(color, vec3(0.2, 0.5, 0.1))), color, 3.0), 0.0, 1.0);
    float alpha = mapped * clamp(uMix, 1.0, 2.0);
    gl_FragColor = vec4(color * alpha, alpha) * fade;
  }
}`;



/** The talk screen's aura: a turbulent ring of light whose speed, size and pulse follow the conversation's state and,
 * while someone speaks, its loudness. The shader is LiveKit Agents UI's Aura (Apache-2.0). */
export default function Aura({ state, level, token, className }: VisualProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const target = useRef(LOOKS[state]);
  const levelRef = useRef(level);
  const tokenRef = useRef(token);
  useEffect(() => {
    target.current = LOOKS[state];
    levelRef.current = level;
    tokenRef.current = token;
  });

  useEffect(() => {
    const el = canvas.current;
    const gl = el?.getContext("webgl", { premultipliedAlpha: true, alpha: true, antialias: false });
    if (!el || !gl) return;
    const program = fullscreenProgram(gl, VERTEX, FRAGMENT);
    const u = (name: string) => gl.getUniformLocation(program, name);
    const loc = { res: u("iResolution"), time: u("iTime"), speed: u("uSpeed"), scale: u("uScale"), amp: u("uAmplitude"), freq: u("uFrequency"), mix: u("uMix"), mode: u("uMode"), shift: u("uColorShift"), color: u("uColor") };
    gl.clearColor(0, 0, 0, 0);

    const calm = window.matchMedia("(prefers-reduced-motion: reduce)");
    const probe = document.createElement("canvas").getContext("2d");
    if (!probe) return;
    const now: Look = { ...target.current };
    let theme = readTheme(tokenRef.current, probe);
    let looked = performance.now();
    let phase = 0;
    let last = performance.now();
    let loud = 0;
    let swell = 0;
    let frame = 0;
    const draw = (time: number) => {
      const dt = Math.min(0.1, (time - last) / 1000);
      last = time;
      fitCanvas(el, gl);
      const goal = target.current;
      // every value glides to its goal: a state change or a step of the loudness never jumps the picture
      const glide = (seconds: number) => 1 - Math.exp(-dt / seconds);
      for (const key of ["scale", "amplitude", "frequency", "brightness", "speed", "pulse"] as const) now[key] += (goal[key] - now[key]) * glide(0.45);
      const raw = Math.min(1, Math.max(0, levelRef.current()));
      loud += (raw - loud) * glide(raw > loud ? 0.06 : 0.3);
      const reactive = target.current === LOOKS.speaking || target.current === LOOKS.hearing;
      swell += ((reactive ? loud : 0) - swell) * glide(0.12);
      const scale = now.scale - 0.06 + 0.16 * swell;
      phase += dt * (calm.matches ? 0.3 : 1) * now.speed;
      const pulse = now.pulse * Math.sin((time / 1000) * Math.PI / 0.9) * 0.35;
      if (time - looked > 500) { theme = readTheme(tokenRef.current, probe); looked = time; }
      gl.uniform3fv(loc.color, theme.color);
      gl.uniform2f(loc.res, el.width, el.height);
      gl.uniform1f(loc.time, phase / 10);
      gl.uniform1f(loc.speed, 10);
      gl.uniform1f(loc.scale, scale);
      gl.uniform1f(loc.amp, now.amplitude);
      gl.uniform1f(loc.freq, now.frequency);
      gl.uniform1f(loc.mix, Math.max(0, now.brightness + (calm.matches ? 0 : pulse)));
      gl.uniform1f(loc.mode, theme.light ? 1 : 0);
      gl.uniform1f(loc.shift, 0.05);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => { cancelAnimationFrame(frame); gl.getExtension("WEBGL_lose_context")?.loseContext(); };
  }, []);

  return <canvas ref={canvas} className={className} aria-hidden />;
}
