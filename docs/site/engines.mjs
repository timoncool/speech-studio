// Speech engines as measured on 2026-10-20: Speech Studio 0.2.0 as installed, RTX 4090 (24 GB) under Windows 11, the
// same 17-second Russian text, the second run of each engine (its model loaded); Higgs in the audio.cpp engine on
// 2026-10-22, three runs, its first sound not measured. speed: audio seconds per second of synthesis; firstMs: time to
// the first sound in the window; vramGB: peak while speaking less the desktop's own use; diskGB: the model as
// downloaded (Qwen3-TTS Q4_K_M and Higgs Q8_0, the defaults). null: not measured. Audio8 (Realistic-Air Q8_0) on
// 2026-11-09 in a build of 0.4.0's code, an 18-second Russian text in a cloned voice, the second run; it answers a
// phrase whole, so its first sound is the first phrase's own synthesis: 2.3 s on that text, 0.4-1.0 s for a phrase
// of 1-6 seconds said in Live.
export const ENGINE_BENCH = [
  { engine: 'supertonic', speed: 168, firstMs: 103, stream: 'phrase', diskGB: 0.1, vramGB: 1.0 },
  { engine: 'qwen3-tts', speed: 12, firstMs: 19, stream: 'yes', diskGB: 2.2, vramGB: 3.1 },
  { engine: 'qwen3-tts-ru', speed: 10, firstMs: 26, stream: 'yes', diskGB: 2.3, vramGB: 3.5 },
  { engine: 'drama3', speed: 7.0, firstMs: 326, stream: 'yes', diskGB: 21.4, vramGB: 21.9 },
  { engine: 'audio8', speed: 5.8, firstMs: 2261, stream: 'phrase', diskGB: 1.5, vramGB: 2.0 },
  { engine: 'higgs', speed: 3.4, firstMs: null, stream: 'yes', diskGB: 5.3, vramGB: 7, vramApprox: true },
];

// Recognition as measured on 2026-10-20: Speech Studio 0.2.1 as installed, the same RTX 4090 and Windows 11, one
// 5-minute Russian recording of one speaker (299.7 s) transcribed in Listening without telling speakers apart, the
// model loading included; CrisperWhisper in the audio.cpp engine on 2026-10-22, a 625.8-second Russian recording, the
// engine loading included. gpu and cpu: seconds of recording per second of work; vramGB: peak during the job above
// idle; diskGB: the model as downloaded. null: not measured.
export const ASR_BENCH = [
  { model: 'parakeet', name: 'Parakeet TDT v3', gpu: 96, cpu: 33, langs: 25, diskGB: 0.7, vramGB: 1.2 },
  { model: 'parakeet-ultra', name: 'Parakeet Ultra', gpu: 73, cpu: 33, langs: 25, diskGB: 0.7, vramGB: 1.0 },
  { model: 'nemotron', name: 'Nemotron 3.5', gpu: 73, cpu: 29, langs: 28, diskGB: 0.7, vramGB: 1.0 },
  { model: 'whisper', name: 'CrisperWhisper 2.0 turbo', gpu: 96, cpu: null, langs: 99, diskGB: 1.6, vramGB: null },
];

// One turn of a Live conversation, ms: the phrase as text after the speech ends (Parakeet on the card, the pause that
// ends a phrase included; three runs of a phrase fed as the microphone, 0.2.1), the agent writing its answer (Claude
// Sonnet, measured 2026-10-19, from the phrase heard to its first text) and the first sound of the answer (the
// fastest and slowest engine as measured on 0.2.0).
export const LIVE_BENCH = [
  { step: 'heard', ms: [641, 735] },
  { step: 'agent', ms: [2000, 3900] },
  { step: 'voice', ms: [19, 1023] },
];
