// What the project page says about this studio beyond its text: the release it offers, the features
// its tiles show (places in each language's features list, with the screenshot each shows), and the answers it points to.
export const STUDIO = {
  id: 'speech',
  name: 'Speech Studio',
  repo: 'https://github.com/timoncool/speech-studio',
  site: 'https://timoncool.github.io/speech-studio/',
  installerMB: 21.37,
  updated: '2026-11-10',
  hero: '01-speech',
  bento: [
    { feature: 0, shot: '02-voices' },
    { feature: 1, shot: '03-stress' },
    { feature: 2, shot: '04-quality' },
    { feature: 3, shot: '05-dialog' },
    { feature: 5, shot: '06-listening' },
    { feature: 7, shot: '07-agent' },
  ],
  faq: { stress: 1, mcp: 7 },
  arch: "React UI ─┐\n          ├─ Speech Studio.exe   (Tauri window + Rust/axum speech-server)\nRust axum ┘        │\n                   ├─ Drama 3, Qwen3-TTS   (C++/CUDA, GGUF)\n                   ├─ Parakeet Ultra, Silero VAD   (transcribe.cpp, GGUF)\n                   ├─ RUAccent, DPDFNet   (ONNX Runtime)\n                   └─ audiocpp_server   (Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, diarization)",
};
