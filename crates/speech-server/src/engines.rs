//! Капабилити-манифест движков синтеза — данные, из которых окно строит контролы, а синтез проверяет запрос до
//! запуска: языки (из карточки модели), параметры с типом, диапазоном и умолчанием, режимы сида, теги экспрессии,
//! вид клона, продолжение, стриминг, что из разметки текста движок читает.

pub(crate) mod acpp;
pub(crate) mod asr;
pub(crate) mod audio8;
pub(crate) mod denoise;
pub(crate) mod hw;
pub(crate) mod models;
pub(crate) mod presets;
pub(crate) mod qwen_tts;
pub(crate) mod supertonic;
pub(crate) mod stress;
pub(crate) mod tcpp_asr;
pub(crate) mod tts_host;
pub(crate) mod vad;

use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct ParamSpec {
    pub key: &'static str,
    /// `float` или `int`.
    pub kind: &'static str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

#[derive(Serialize, Clone, Debug)]
pub struct Reads {
    pub stress: bool,
    pub yo: bool,
    pub phonemes: bool,
    pub accent: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Engine {
    pub id: &'static str,
    pub name: &'static str,
    pub languages: &'static [&'static str],
    pub params: Vec<ParamSpec>,
    pub seed_modes: &'static [&'static str],
    pub tag_format: &'static str,
    pub tags: Vec<(&'static str, &'static [&'static str])>,
    /// Категории и значения тегов, что ставятся в начало предложения; остальные — по месту.
    pub sentence_tags: &'static [&'static str],
    /// Категории тегов, за которыми вплотную, без пробела, идёт звукоподражание: `<|sfx:laughter|>Ха-ха,`.
    pub glued_tags: &'static [&'static str],
    /// `ref_text` — реф и его транскрипт; `timbre` — только тембр; `none` — только свои голоса (`presets`).
    pub clone: &'static str,
    pub continuation: bool,
    pub streaming: bool,
    pub deterministic_seed: bool,
    /// Сколько знаков блока движок берёт за раз; длиннее — режется на фразы.
    pub max_chars: usize,
    pub reads: Reads,
    /// Указание блока («как сказать») доходит до движка: у Drama 3 — стилевой приставкой строки, у Qwen3-TTS — указанием
    /// голосам модели, у облаков — стилем. Higgs и Supertonic его не читают.
    pub direction: bool,
    /// Указание доходит только с голосами самого движка (`presets`): Qwen3-TTS CustomVoice берёт `instruct`, клон Base — нет.
    pub direction_presets_only: bool,
    /// Голоса самого движка: у движка без клона голос блока — один из них.
    pub presets: Vec<Preset>,
    /// Без голоса из библиотеки не говорит: своих голосов нет, базового голоса тоже.
    pub needs_voice: bool,
    /// Клонирует только по рефу с его текстом: без текста рефа голос не повторит.
    pub ref_text_required: bool,
    /// Свой предел длины рефа, секунды, ниже общей настройки `higgs_ref_secs`; None — общая настройка.
    pub ref_max_secs: Option<f64>,
    /// Почему движок на этой машине не пойдёт (видеокарта, файлы); None — доступен.
    pub unavailable: Option<String>,
}

/// A voice the engine carries: its id, the name it is shown by, `female` or `male`, its native language (empty — any) and
/// the maker's description of it.
#[derive(Serialize, Clone, Debug)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub gender: &'static str,
    pub native: &'static str,
    pub description: &'static str,
}

/// Языки Higgs Audio v3 — список `language` карточки `bosonai/higgs-audio-v3-tts-4b`.
const HIGGS_LANGS: &[&str] = &[
    "af", "ar", "as", "ast", "az", "ba", "be", "bg", "bn", "bs", "ca", "ceb", "ckb", "cs", "cy", "da", "de", "el", "en", "eo", "es",
    "et", "eu", "fa", "fi", "fr", "ga", "gl", "gu", "ha", "he", "hi", "hr", "ht", "hu", "hy", "id", "is", "it", "jv", "ka", "kab",
    "kam", "kea", "kk", "kn", "ko", "ky", "la", "lb", "lg", "ln", "lt", "luo", "lv", "mhr", "mi", "mk", "ml", "mn", "mr", "ms",
    "mt", "ne", "nl", "no", "nso", "ny", "oc", "om", "pa", "pl", "ps", "pt", "ro", "ru", "rw", "sd", "sk", "sl", "sn", "so",
    "sq", "sr", "sv", "sw", "ta", "te", "tg", "tl", "tr", "ug", "uk", "umb", "ur", "uz", "vi", "xh", "zh", "zu",
];

pub const MAX_TOKENS_LIMIT: u32 = 4096;

pub fn higgs() -> Engine {
    Engine {
        id: "higgs",
        name: "Higgs Audio v3",
        languages: HIGGS_LANGS,
        // defaults as audio.cpp recommends for Higgs v3 (model_specs/higgs_audio_tts.json): temperature 0.8,
        // top_k 30, top_p 0.8 (plus its own repetition penalty 1.1) — the engine running the voice, not the Python one.
        params: vec![
            ParamSpec { key: "temperature", kind: "float", min: 0.05, max: 2.0, step: 0.05, default: 0.8 },
            ParamSpec { key: "top_k", kind: "int", min: 1.0, max: 500.0, step: 1.0, default: 30.0 },
            ParamSpec { key: "top_p", kind: "float", min: 0.05, max: 1.0, step: 0.01, default: 0.8 },
            ParamSpec { key: "max_tokens", kind: "int", min: 0.0, max: MAX_TOKENS_LIMIT as f64, step: 64.0, default: 0.0 },
        ],
        seed_modes: &["random", "fixed", "inc", "dec"],
        tag_format: "<|{category}:{value}|>",
        tags: crate::text::adapter::HIGGS_TAGS.to_vec(),
        sentence_tags: crate::text::adapter::HIGGS_SENTENCE_TAGS,
        glued_tags: &["sfx"],
        clone: "ref_text",
        continuation: true,
        streaming: true,
        deterministic_seed: true,
        max_chars: 400,
        reads: Reads { stress: false, yo: true, phonemes: false, accent: false },
        direction: false,
        direction_presets_only: false,
        presets: Vec::new(),
        needs_voice: false,
        ref_text_required: false,
        ref_max_secs: None,
        unavailable: None,
    }
}


/// Языки файнтюна Audio8 Realistic-Air (карточка Edge0/Audio8-TTS-Realistic-Air: 23 языка, русский есть); прочее — авто.
const AUDIO8_LANGS: &[&str] = &["ru", "en", "zh", "ja", "ko", "de", "fr", "es", "it", "pt", "pl", "nl", "ar", "fi", "hi", "id", "ms", "no", "sv", "th", "tr", "vi", "yue"];

/// Audio8 TTS: компактный мультиязычный голос с zero-shot клоном по рефу и его тексту; без рефа говорит своим.
/// Теги экспрессии не читает; параметры выборки — по карточке Realistic-Air (temperature 0.8, top_k 50, top_p 0.95).
/// Русские ударения модель ставит сама: со знаком U+0301 после гласной она читает хуже, чем без разметки.
pub fn audio8() -> Engine {
    Engine {
        id: crate::engines::audio8::ENGINE,
        name: "Audio8 TTS",
        languages: AUDIO8_LANGS,
        params: vec![
            ParamSpec { key: "temperature", kind: "float", min: 0.05, max: 2.0, step: 0.05, default: 0.8 },
            ParamSpec { key: "top_k", kind: "int", min: 1.0, max: 500.0, step: 1.0, default: 50.0 },
            ParamSpec { key: "top_p", kind: "float", min: 0.05, max: 1.0, step: 0.01, default: 0.95 },
            ParamSpec { key: "max_tokens", kind: "int", min: 0.0, max: MAX_TOKENS_LIMIT as f64, step: 64.0, default: 0.0 },
        ],
        seed_modes: &["random", "fixed", "inc", "dec"],
        tag_format: "",
        tags: Vec::new(),
        sentence_tags: &[],
        glued_tags: &[],
        clone: "ref_text",
        continuation: false,
        streaming: false,
        deterministic_seed: true,
        max_chars: 200,
        reads: Reads { stress: false, yo: true, phonemes: false, accent: false },
        direction: false,
        direction_presets_only: false,
        presets: Vec::new(),
        needs_voice: false,
        ref_text_required: true,
        ref_max_secs: None,
        unavailable: None,
    }
}

/// OpenRouter: языки и голоса — у выбранной модели; параметров выборки и клона нет.
pub fn openrouter() -> Engine {
    Engine {
        id: "openrouter",
        name: "OpenRouter",
        languages: &[],
        params: Vec::new(),
        seed_modes: &["random"],
        tag_format: "",
        tags: Vec::new(),
        sentence_tags: &[],
        glued_tags: &[],
        clone: "none",
        continuation: false,
        streaming: false,
        deterministic_seed: false,
        max_chars: 1000,
        reads: Reads { stress: false, yo: true, phonemes: false, accent: false },
        direction: true,
        direction_presets_only: false,
        presets: Vec::new(),
        needs_voice: false,
        ref_text_required: false,
        ref_max_secs: None,
        unavailable: None,
    }
}

/// Fish Audio Drama 3: 83 языка по карточке `fishaudio/drama-3` (списка она не даёт — язык не ограничивается),
/// сэмплинг по умолчанию карточки (temperature 1.0, top-k 30, top-p 0.9), точечные теги `[laughs]` и теги на отрезок
/// `<whisper>…</whisper>`, указание блока — стилевой приставкой строки (Table 10 техотчёта), клон только по
/// рефу с текстом; видеокарты от 6 ГБ (см. `models::drama_unavailable`).
pub fn drama() -> Engine {
    Engine {
        id: crate::engines::models::DRAMA_TTS,
        name: "Drama 3",
        languages: &[],
        params: vec![
            ParamSpec { key: "temperature", kind: "float", min: 0.05, max: 2.0, step: 0.05, default: 1.0 },
            ParamSpec { key: "top_k", kind: "int", min: 1.0, max: 500.0, step: 1.0, default: 30.0 },
            ParamSpec { key: "top_p", kind: "float", min: 0.05, max: 1.0, step: 0.01, default: 0.9 },
            ParamSpec { key: "max_tokens", kind: "int", min: 0.0, max: 2048.0, step: 64.0, default: 0.0 },
        ],
        seed_modes: &["random", "fixed", "inc", "dec"],
        tag_format: "[{value}]",
        tags: crate::text::adapter::DRAMA_TAGS.to_vec(),
        sentence_tags: &[],
        glued_tags: &[],
        clone: "ref_text",
        continuation: false,
        streaming: true,
        deterministic_seed: true,
        max_chars: 400,
        reads: Reads { stress: false, yo: true, phonemes: false, accent: false },
        direction: true,
        direction_presets_only: false,
        presets: Vec::new(),
        needs_voice: true,
        ref_text_required: true,
        ref_max_secs: None,
        unavailable: None,
    }
}


/// Supertonic 3: десять голосов движка (F1–F5, M1–M5), 31 язык, клона нет; теги `<laugh>`, `<breath>`, `<sigh>` по
/// карточке модели; шаги потока и скорость — параметры официального рантайма (8 и 1,05).
pub fn supertonic() -> Engine {
    let preset = |(id, description): &(&'static str, &'static str)| Preset { id, name: id, gender: if id.starts_with('F') { "female" } else { "male" }, native: "", description };
    Engine {
        id: crate::engines::supertonic::ENGINE,
        name: "Supertonic 3",
        languages: crate::engines::supertonic::LANGS,
        params: vec![
            ParamSpec { key: "steps", kind: "int", min: 1.0, max: 32.0, step: 1.0, default: crate::engines::supertonic::DEFAULT_STEPS as f64 },
            ParamSpec { key: "speed", kind: "float", min: 0.5, max: 2.0, step: 0.05, default: crate::engines::supertonic::DEFAULT_SPEED },
        ],
        seed_modes: &["random", "fixed", "inc", "dec"],
        tag_format: "<{value}>",
        tags: crate::text::adapter::SUPERTONIC_TAGS.to_vec(),
        sentence_tags: &[],
        glued_tags: &[],
        clone: "none",
        continuation: false,
        streaming: false,
        deterministic_seed: true,
        max_chars: crate::engines::supertonic::MAX_CHARS,
        reads: Reads { stress: true, yo: true, phonemes: false, accent: false },
        direction: false,
        direction_presets_only: false,
        presets: crate::engines::supertonic::VOICE_DESCRIPTIONS.iter().map(preset).collect(),
        needs_voice: false,
        ref_text_required: false,
        ref_max_secs: None,
        unavailable: None,
    }
}


/// Qwen3-TTS 1.7B через qwentts.cpp: девять голосов модели (CustomVoice) и клон голоса из библиотеки по рефу с
/// текстом (Base), десять языков; пометка блока идёт голосам модели указанием, как говорить. Сэмплинг по умолчанию — из портативки Qwen3-TTS_portable_rus (0,7 / 0,9).
pub fn qwen() -> Engine {
    Engine {
        id: crate::engines::qwen_tts::ENGINE,
        name: "Qwen3-TTS",
        languages: crate::engines::qwen_tts::LANGS,
        // defaults as qwentts.cpp ships (src/sampling-defaults.h): temperature 0.9, top_k 50, top_p 1.0 (and its own
        // repetition penalty 1.05) — the engine running the voice.
        params: vec![
            ParamSpec { key: "temperature", kind: "float", min: 0.05, max: 2.0, step: 0.05, default: 0.9 },
            ParamSpec { key: "top_k", kind: "int", min: 1.0, max: 500.0, step: 1.0, default: 50.0 },
            ParamSpec { key: "top_p", kind: "float", min: 0.05, max: 1.0, step: 0.01, default: 1.0 },
            ParamSpec { key: "max_tokens", kind: "int", min: 0.0, max: 2048.0, step: 64.0, default: 0.0 },
        ],
        seed_modes: &["random", "fixed", "inc", "dec"],
        tag_format: "",
        tags: Vec::new(),
        sentence_tags: &[],
        glued_tags: &[],
        clone: "ref_text",
        continuation: false,
        streaming: true,
        deterministic_seed: true,
        max_chars: 400,
        reads: Reads { stress: false, yo: true, phonemes: false, accent: false },
        direction: true,
        direction_presets_only: true,
        presets: crate::engines::qwen_tts::SPEAKERS.iter().map(|(id, name, gender, native, description)| Preset { id, name, gender, native, description }).collect(),
        needs_voice: false,
        ref_text_required: false,
        ref_max_secs: None,
        unavailable: None,
    }
}

/// Qwen3-TTS 1.7B Base с адаптером русских ударений (siriusfreak, 96% знаков по словарю против 17% у исходной):
/// только клон голоса из библиотеки, ударение «+» доходит до модели знаком после гласной.
pub fn qwen_stress() -> Engine {
    Engine {
        id: crate::engines::qwen_tts::ENGINE_STRESS,
        name: "Qwen3-TTS · русские ударения",
        languages: &["ru"],
        presets: Vec::new(),
        reads: Reads { stress: true, yo: true, phonemes: false, accent: false },
        direction: false,
        needs_voice: true,
        ref_text_required: false,
        ref_max_secs: Some(crate::engines::qwen_tts::STRESS_REF_SECS),
        ..qwen()
    }
}

/// fal, Runware и Google Gemini: модель из каталога или настроек, голоса — у модели; языки — у модели.
pub fn catalog(id: &'static str, name: &'static str) -> Engine {
    Engine { id, name, ..openrouter() }
}

/// The expression tags an engine reads, as rules for whoever writes its text (an agent, the director model).
pub fn tag_rules(engine: &Engine) -> String {
    let mut out = String::new();
    if engine.tags.is_empty() {
        out.push_str("No expression tags.");
    } else {
        out.push_str(&format!("A tag is written as {}.", engine.tag_format));
        for (category, values) in &engine.tags {
            let wrap = if *category == "scope" { " (each wraps a span: <value>...</value>)" } else { "" };
            out.push_str(&format!("\n- {category}{wrap}: {}", values.join(", ")));
        }
        if !engine.sentence_tags.is_empty() {
            out.push_str(&format!("\nAt the start of the sentence: {}; the others at the spot.", engine.sentence_tags.join(", ")));
        }
        if !engine.glued_tags.is_empty() {
            out.push_str(&format!("\nGlued to the onomatopoeia that follows, with no space: {}.", engine.glued_tags.join(", ")));
        }
    }
    out
}

/// Every tag text the engine reads: `tag_format` filled per value, and both ends of a `scope` span.
pub fn tag_texts(engine: &Engine) -> Vec<String> {
    engine
        .tags
        .iter()
        .flat_map(|(category, values)| {
            values.iter().flat_map(move |v| {
                if *category == "scope" {
                    vec![format!("<{v}>"), format!("</{v}>")]
                } else {
                    vec![engine.tag_format.replace("{category}", category).replace("{value}", v)]
                }
            })
        })
        .collect()
}

pub fn all() -> Vec<Engine> {
    vec![higgs(), audio8(), drama(), supertonic(), qwen(), qwen_stress(), openrouter(), catalog("google", "Google Gemini"), catalog("fal", "fal.ai"), catalog("runware", "Runware")]
}

/// Движки для окна: у Higgs, Drama 3, Supertonic, Audio8 и Qwen3-TTS — почему они здесь не пойдут (видеокарта или
/// нескачанные файлы).
pub fn for_window(models_root: &std::path::Path) -> Vec<Engine> {
    let mut all = all();
    for e in all.iter_mut() {
        if e.id == "higgs" && !crate::engines::acpp::higgs_gguf(models_root).is_file() {
            e.unavailable = Some(t!("render-higgs-model-missing", path = crate::engines::acpp::higgs_gguf(models_root).display().to_string()));
        } else if e.id == crate::engines::models::DRAMA_TTS {
            e.unavailable = crate::engines::models::drama_unavailable(models_root).or_else(|| (!crate::engines::models::drama_installed(models_root)).then(|| t!("render-drama-missing")));
        } else if e.id == crate::engines::supertonic::ENGINE && !crate::engines::supertonic::installed(models_root) {
            e.unavailable = Some(t!("render-supertonic-missing"));
        } else if e.id == crate::engines::audio8::ENGINE && !crate::engines::audio8::installed(models_root) {
            e.unavailable = Some(t!("render-audio8-missing"));
        } else if e.id == crate::engines::qwen_tts::ENGINE && !crate::engines::qwen_tts::installed(models_root, &crate::engines::models::qwen_quant(models_root), false, false) {
            e.unavailable = Some(t!("render-qwen-missing"));
        } else if e.id == crate::engines::qwen_tts::ENGINE_STRESS && !crate::engines::qwen_tts::installed(models_root, &crate::engines::models::qwen_quant(models_root), true, true) {
            e.unavailable = Some(t!("render-qwen-stress-missing"));
        }
    }
    all
}

/// Облачные движки: модель и голос у них — одна настройка на проект.
pub fn is_cloud(id: &str) -> bool {
    matches!(id, "openrouter" | "google" | "fal" | "runware")
}

pub fn get(id: &str) -> Option<Engine> {
    all().into_iter().find(|e| e.id == id)
}


/// Рекомендуемые дефолты движка для повтора: `params` с температурой, top_k и top_p из его `ParamSpec`-дефолтов
/// (остальное — как у пользователя). После дефектного дубля `synth` повторяет на них, потому что на совсем низкой
/// температуре движок уходит за конец фразы.
pub fn recommended_sampling(id: &str, params: &tts_host::Params) -> tts_host::Params {
    let specs = get(id).map(|e| e.params).unwrap_or_default();
    let def = |key: &str, fallback: f64| specs.iter().find(|s| s.key == key).map_or(fallback, |s| s.default);
    tts_host::Params {
        temperature: def("temperature", params.temperature),
        top_k: def("top_k", f64::from(params.top_k)).round() as u32,
        top_p: def("top_p", params.top_p),
        ..params.clone()
    }
}

/// Язык проекта годится движку; `auto` годится всегда — язык берётся по письму блока.
pub fn supports(engine: &Engine, lang: &str) -> bool {
    lang.is_empty() || lang == "auto" || engine.languages.is_empty() || engine.languages.contains(&lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higgs_reads_russian_but_not_japanese() {
        let h = higgs();
        assert_eq!(h.languages.len(), 100);
        assert!(supports(&h, "ru") && supports(&h, "auto") && !supports(&h, "ja"));
    }
}
