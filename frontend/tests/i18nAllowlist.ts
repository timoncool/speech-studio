import type { Allow } from "./scanHardcoded";

export const ALLOW: Allow = {
  tokens: new Set<string>([
    "OpenRouter", "top-k", "top-p", "GitHub", "Telegram", "Claude Code", "Nerual Dreming", "ArtGeneration.me", "Нейро-Софт",
    "Parakeet-TDT", "Nemotron 3.5", "Higgs Audio v3", "Hugging Face", "HTTP", "HTTPS", "SOCKS5", "SOCKS4", "CrisperWhisper", "GPU", "VRAM", "RAM", "VRAM ·",
    "v",
    "Content-Type", "Aa",
    "Bahasa Indonesia", "Bahasa Melayu", "Basa Jawa", "Basa Sunda",
    "YuE2 Studio", "MiniMax Music3 Studio", "ACE-Step Studio", "Dub Studio", "Speech Studio", "Dungeon Ultimate", "Bulka", "VideoSOS",
  ]),
  cyrillicFiles: new Set(["lib/i18n.ts"]),
  // lib/mcpBridge.ts speaks only to the agent over MCP, in English: none of its text reaches the UI.
  skipFiles: new Set(["i18next.d.ts", "lib/mcpBridge.ts"]),
};

/**
 * Values that legitimately equal the English text: brands, units, abbreviations and words that are
 * spelled the same in the language. "*" applies to every language.
 */
export const SAME_AS_EN: Record<"*" | "ru" | "zh" | "es" | "pt" | "fr", string[]> = {
  "*": ["speech.ab", "voices.seconds", "prep.layer_ssml", "voices.attr_accentHint", "voices.variantBadge", "share.places.telegram", "share.places.x", "share.places.whatsapp", "share.places.reddit", "prefs.sections.api", "api.base", "api.logMs", "journal.line", "journal.quoted", "speech.takeWer", "modes.speech", "modes.listening", "modes.talk", "talk.title", "mic.what_live", "app.name", "asrVariant.int8", "asrVariant.fp32", "asrVariant.ultra", "asrVariant.ultraInt8", "jobs.atStage", "downloads.speed", "providers.localGemma", "providers.serverUrlPlaceholder", "url.qHeight", "url.tool.version", "dict.kind_regex", "book.minutes", "book.hours"],
  ru: [],
  zh: ["share.places.vk", "units.gb", "units.mb", "units.kb", "units.b"],
  es: ["talk.style_aura", "talk.widget", "voices.attr_timbre", "share.places.vk", "speech.rateKhz", "batch.error", "prefs.idleMinutes", "journal.kind.proxy", 
    "tags.rate",    "speech.strictness_normal", "speech.pauseAuto", "voices.snr", "voice.g_female", "voice.g_male", "settings.auto", "settings.sec", "history.ms", "common.error", "setup.error", "units.gb", "units.mb", "units.kb", "units.b", "units.watt", "cloud.beta", "hwPreset.ram", "secrets.proxyTitle",
    ],
  pt: ["speech.way_podcast", "listen.source_link", "talk.style_aura", "talk.widget", "voices.attr_timbre", "share.places.vk", "speech.rateKhz", "prefs.idleMinutes", "journal.kind.podcast", "journal.kind.download", "journal.kind.proxy", 
    "speech.strictness_normal", "speech.pauseAuto", "voices.snr", "voice.g_female", "voice.g_male", "settings.auto", "settings.sec", "history.ms", "units.gb", "units.mb", "units.kb", "units.b", "units.watt", "cloud.beta", "hwPreset.ram", "secrets.proxyTitle",
    ],
  fr: ["speech.way_podcast", "talk.style_aura", "talk.widget", "talk.history", "talk.agent", "voices.attr_timbre", "share.places.vk", "speech.rateKhz", "batch.formats", "prefs.idleMinutes", "privacy.stage_assistant", "api.logEndpoint", "api.logRoute", "chat.title", "journal.kind.podcast", "journal.kind.proxy", 
    "listen.engine_fal", "jobs.kind.write", "podcast.minutes", "listen.pause", "listen.position", "listen.engine_openrouter", "listen.fixes", "listen.fixSay", "listen.format",
    "speech.note", "speech.marker_violet",    "tags.rate",    "dict.note",
    "modes.label", "tags.style", "tags.amusement", "tags.confusion", "tags.surprise", "tags.pause", "jobs.kind.export", "speech.langAuto", "speech.pauseAuto", "speech.format", "voices.pause", "voices.age_senior", "voices.source", "voices.src_pack", "voices.photo", "voice.recordStop", "voice.g_female", "settings.vision",
    "settings.auto", "settings.sec", "history.ms", "help.sectionsTitle", "common.pause", "transport.pause", "units.watt", "secrets.proxyTitle", "bridge.agent",
    ],
};
