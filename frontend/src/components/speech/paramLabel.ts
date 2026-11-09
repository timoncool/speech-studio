import type { ParamSpec } from "../../lib/speech";

/** How each sampling parameter is named and explained. */
export const PARAM_LABEL: Record<ParamSpec["key"], { label: string; hint: string }> = {
  temperature: { label: "speech.temperature", hint: "speech.temperatureHint" },
  top_k: { label: "top-k", hint: "speech.topKHint" },
  top_p: { label: "top-p", hint: "speech.topPHint" },
  max_tokens: { label: "speech.maxTokens", hint: "speech.maxTokensHint" },
  steps: { label: "speech.steps", hint: "speech.stepsHint" },
  speed: { label: "speech.speed", hint: "speech.speedHint" },
};
