use crate::engines::{hw, models};
use crate::{setup, AppState};
use axum::extract::State;
use axum::Json;
use serde_json::json;
use serde_json::Value;

pub(super) async fn capabilities(State(st): State<AppState>) -> Json<Value> {
    let o = &st.opts;
    let ffmpeg = which_ffmpeg();
    // Текущий выбор вариантов (active.json) — фронт рисует по нему селекторы движка/модели/кванта ASR.
    // Без секретов: вместо ключа OpenRouter и пароля прокси — флаги or_key_set / proxy_password_set.
    let sel = crate::speech::floor_settings::with_defaults(crate::app::cloud_tts::with_defaults(models::public_selection(&st.models_root)));
    // Языки контента = полный набор Whisper (99). Единый источник — dub_translate::WHISPER_LANGS.
    let langs: Vec<&str> = dub_translate::WHISPER_LANGS.iter().map(|(c, _)| *c).collect();
    let tts_provider = models::tts_provider(&st.models_root);
    let chosen = |key: &str| sel.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let tts = match tts_provider {
        "openrouter" => chosen("or_tts_model"),
        "google" => chosen("google_tts_model"),
        _ => o.tts_model.to_string(),
    };
    let llm_provider = models::llm_backend(&st.models_root, "llm");
    let llm = match llm_provider {
        models::LlmBackend::Local => models::resolve_mt(&st.models_root, &models::load_selection(&st.models_root)).0.to_string_lossy().into_owned(),
        models::LlmBackend::Server => models::server_model(&st.models_root, "llm"),
        models::LlmBackend::OpenRouter => models::openrouter_model(&st.models_root, "llm"),
        models::LlmBackend::Agent => String::new(),
    };
    Json(json!({
        "device": o.device,
        "tts_quant": o.tts_quant,
        "asr_model": o.asr_model,
        "asr_label": models::resolve_asr_choice(&st.repo_root, &st.models_root, &models::load_selection(&st.models_root)).label(),
        "models": {
            "asr": o.asr_model,
            "llm": llm,
            "vision": o.mmproj_path.to_string_lossy(),
            "tts": tts,
        },
        // the voicing model is `models.tts`: the local engine's, or the OpenRouter or Google model id
        "tts_provider": tts_provider,
        // the assistant's model is `models.llm`: a Gemma file, the server's or OpenRouter's model id, none for the agent
        "llm_provider": llm_provider.as_str(),
        "ffmpeg": ffmpeg,
        "languages": langs,
        "modes": ["speech","listening"],
        // ASR: parakeet | nemotron | whisper (CrisperWhisper 2.0 turbo).
        "selection": sel,
        "asr_engines": ["parakeet","nemotron","whisper"],
        // Видимые лимиты RAM (настройки, не авто-магия): против OOM на слабой памяти. "0" = авто (дефолт).
        "llama_ubatches": ["0","512","256","128"],      // prefill-батч Gemma (меньше = меньше RAM)
        "higgs_ref_secs_opts": ["12","8","6","4"],       // длина реф-клипа клона (сек; <12 спасает 32ГБ)
    }))
}

pub(super) fn which_ffmpeg() -> bool {
    setup::ffmpeg_on_path()
}

/// GET /hw/snapshot — снимок GPU/VRAM/темп/мощность + RAM для монитора ресурсов.
pub(super) async fn hw_snapshot() -> Json<hw::HardwareSnapshot> {
    Json(tokio::task::spawn_blocking(hw::snapshot).await.unwrap_or_default())
}
