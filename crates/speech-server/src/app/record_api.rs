use super::{
    enqueue_error,
    files::serve_file_range,
    setup_api::{internal_error, join_failure},
};
use crate::{app, jobs, voices as voice_library, AppState};
use crate::app::tools;
use crate::engines::models;
use crate::speech::talk as say_api;
use crate::voices::{http as voice_api, record};
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use serde_json::json;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;

/// Безопасное имя голоса: только буквы/цифры/пробел/дефис/подчёркивание (без разделителей путей).
/// GET /record/devices — список микрофонов.
pub(super) async fn record_devices() -> Json<Value> {
    Json(json!({ "devices": tokio::task::spawn_blocking(record::input_devices).await.unwrap_or_default() }))
}

/// GET /record/level — текущий пик 0..1 (метр уровня).
pub(super) async fn record_level() -> Json<Value> {
    Json(json!({ "level": record::level() }))
}

/// Where a microphone test is recorded: one file, each test replaces the last.
pub(super) fn mic_test_path(workspace: &Path) -> PathBuf {
    workspace.join(".mic-test.wav")
}

/// POST /record/test {device?} — a microphone test, as call apps have one: records until /record/test/stop while
/// /record/level shows how loud it hears; the recording is then played back to the person.
pub(super) async fn record_test(State(st): State<AppState>, headers: HeaderMap, body: Option<Json<Value>>) -> Response {
    if tools::by_agent(&headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "mic_only_person", "detail": t!("mic-only-person") })),
        )
            .into_response();
    }
    let device = body
        .as_ref()
        .and_then(|Json(v)| v.get("device"))
        .and_then(Value::as_str)
        .filter(|d| !d.is_empty())
        .map(str::to_string);
    let path = mic_test_path(&st.workspace);
    match tokio::task::spawn_blocking(move || record::start_test(&path, device)).await {
        Ok(Ok(())) => {
            say_api::guard_mic(st.workspace.clone());
            Json(json!({ "testing": true })).into_response()
        }
        Ok(Err(e)) => (StatusCode::CONFLICT, Json(json!({ "error": "record_busy", "detail": e }))).into_response(),
        Err(e) => join_failure(e),
    }
}

/// POST /record/test/stop — ends the microphone test: how long it heard and how loud at most; the file to play back.
pub(super) async fn record_test_stop(State(st): State<AppState>) -> Response {
    let test = mic_test_path(&st.workspace);
    let path = match tokio::task::spawn_blocking(move || record::stop_only(&test)).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return (StatusCode::CONFLICT, Json(json!({ "error": "not_a_test", "detail": e }))).into_response(),
        Err(e) => return join_failure(e),
    };
    let (seconds, peak) = match hound::WavReader::open(&path) {
        Ok(mut r) => {
            let spec = r.spec();
            let samples: Vec<f32> = r
                .samples::<i16>()
                .filter_map(Result::ok)
                .map(|s| s as f32 / i16::MAX as f32)
                .collect();
            let seconds = samples.len() as f64 / (spec.sample_rate as f64 * spec.channels as f64);
            (seconds, samples.iter().fold(0.0f32, |m, s| m.max(s.abs())))
        }
        Err(e) => return internal_error(format!("{}: {e}", path.display())),
    };
    Json(json!({ "path": path.display().to_string(), "seconds": (seconds * 100.0).round() / 100.0, "peak": (peak * 1000.0).round() / 1000.0 })).into_response()
}

/// GET /record/test/audio — the last microphone test's recording.
pub(super) async fn record_test_audio(State(st): State<AppState>, req: axum::http::Request<axum::body::Body>) -> Response {
    let path = mic_test_path(&st.workspace);
    if !path.is_file() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no_file", "detail": t!("record-test-none") })),
        )
            .into_response();
    }
    serve_file_range(&path, req, None).await
}

/// POST /record/start {name, device?} — начать запись голоса в новый файл voices/<имя>.wav (занятое имя получает номер).
pub(super) async fn record_start(State(st): State<AppState>, headers: HeaderMap, Json(body): Json<Value>) -> Json<Value> {
    if tools::by_agent(&headers) {
        return Json(json!({ "ok": false, "error": t!("mic-only-person") }));
    }
    let base = voice_library::sanitize(body.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    let base = if base.is_empty() { t!("voice-default-name") } else { base };
    let device = body.get("device").and_then(|v| v.as_str()).map(|s| s.to_string());
    if let Err(e) = std::fs::create_dir_all(&st.voices_dir) {
        return Json(json!({ "ok": false, "error": format!("{}: {e}", st.voices_dir.display()) }));
    }
    let name = voice_library::free_name(&st.voices_dir, &base);
    let path = st.voices_dir.join(format!("{name}.wav"));
    match tokio::task::spawn_blocking(move || record::start(&path, device)).await {
        Ok(Ok(())) => {
            say_api::guard_mic(st.workspace.clone());
            Json(json!({ "ok": true, "name": name }))
        }
        Ok(Err(e)) => Json(json!({ "ok": false, "error": e })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

/// POST /record/stop {clean?, transcribe?} — остановить запись: голос получает карточку (свой голос, с
/// микрофона) и задачу очистки и транскрипта рефа. Ответ — имя, задача и библиотека.
pub(super) async fn record_stop(State(st): State<AppState>, body: Option<Json<Value>>) -> Response {
    let body = body.map(|Json(v)| v).unwrap_or(Value::Null);
    let path = match tokio::task::spawn_blocking(record::stop).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": "record_failed", "detail": e }))).into_response(),
        Err(e) => return join_failure(e),
    };
    match record::peak(&path) {
        Ok(p) if p < record::SILENT_PEAK => {
            let _ = std::fs::remove_file(&path);
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "record_silent", "detail": t!("record-silent") }))).into_response();
        }
        Ok(_) => {}
        Err(e) => return internal_error(e),
    }
    let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else {
        return internal_error(format!("{}: no name", path.display()));
    };
    let card = voice_library::VoiceCard {
        source: "mic".into(),
        consent: "own".into(),
        ..Default::default()
    };
    if let Err(e) = voice_library::write_card(&st.voices_dir, &name, &card) {
        return internal_error(e);
    }
    let clean = body
        .get("clean")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| models::voice_clean_default(&st.models_root));
    let transcribe = body.get("transcribe").and_then(Value::as_bool).unwrap_or(true);
    let job = if clean {
        Some(
            voice_api::enqueue(
                crate::voices::VoicePaths {
                    repo_root: st.repo_root.clone(),
                    models_root: st.models_root.clone(),
                    voices_dir: st.voices_dir.clone(),
                },
                &st.jobs,
                jobs::JobKind::VoiceClean,
                name.clone(),
                transcribe,
            )
            .await,
        )
    } else if transcribe {
        Some(
            voice_api::enqueue(
                crate::voices::VoicePaths {
                    repo_root: st.repo_root.clone(),
                    models_root: st.models_root.clone(),
                    voices_dir: st.voices_dir.clone(),
                },
                &st.jobs,
                jobs::JobKind::VoiceTranscript,
                name.clone(),
                false,
            )
            .await,
        )
    } else {
        None
    };
    let job = match job {
        Some(r) if !r.status().is_success() => return r,
        Some(r) => {
            let bytes = axum::body::to_bytes(r.into_body(), 1 << 16).await.unwrap_or_default();
            serde_json::from_slice::<Value>(&bytes).ok().and_then(|v| v.get("job_id").cloned())
        }
        None => None,
    };
    app::events::publish(json!({ "changed": "voices" }));
    let mut out = voice_api::listing(&st.voices_dir);
    out["name"] = json!(name);
    out["job_id"] = job.unwrap_or(Value::Null);
    Json(out).into_response()
}

/// A voice's card from its catalogue entry: everything the dataset gives; `over` — over what the card says (a fresh
/// download), else only what the card leaves empty (a voice taken before its card kept the catalogue's data).
fn fill_card(card: &mut voice_library::VoiceCard, e: &Value, over: bool) {
    let text = |k: &str| e[k].as_str().map(str::trim).filter(|s| !s.is_empty());
    let set = |field: &mut String, value: Option<String>| {
        if let Some(v) = value.filter(|_| over || field.is_empty()) {
            *field = v;
        }
    };
    set(&mut card.gender, text("gender").map(str::to_string));
    set(&mut card.lang, text("language").map(str::to_ascii_lowercase));
    set(&mut card.age, text("age_group").filter(|a| *a == "child" || *a == "adult").map(str::to_string));
    set(&mut card.display, text("displayName").filter(|d| Some(*d) != e["name"].as_str()).map(str::to_string));
    set(&mut card.description, text("description").map(str::to_string));
    if over || card.birth_year.is_none() {
        card.birth_year = e["birth_year"].as_u64().and_then(|y| u32::try_from(y).ok());
    }
    if over || card.duration_s.is_none() {
        card.duration_s = e["duration_s"].as_f64().filter(|d| *d > 0.0);
    }
}

#[cfg(test)]
mod card_tests {
    use super::*;

    #[test]
    fn a_download_takes_the_catalogue_and_a_backfill_keeps_what_the_card_says() {
        let e = json!({ "name": "RU_Female_volkova_anna", "displayName": "Анна Волкова", "gender": "female", "age_group": "child",
            "language": "RU", "description": " Юная актриса ", "duration_s": 69.0, "birth_year": 2014 });
        let mut fresh = voice_library::VoiceCard::default();
        fill_card(&mut fresh, &e, true);
        assert_eq!((fresh.display.as_str(), fresh.gender.as_str(), fresh.age.as_str(), fresh.lang.as_str()), ("Анна Волкова", "female", "child", "ru"));
        assert_eq!((fresh.description.as_str(), fresh.birth_year, fresh.duration_s), ("Юная актриса", Some(2014), Some(69.0)));
        let mut own = voice_library::VoiceCard { gender: "male".into(), description: "своё".into(), ..Default::default() };
        fill_card(&mut own, &e, false);
        assert_eq!((own.gender.as_str(), own.description.as_str(), own.display.as_str(), own.age.as_str()), ("male", "своё", "Анна Волкова", "child"));
    }
}

/// The voices of the library that came from the catalogue before their cards kept its data get it now.
fn backfill_cards(voices_dir: &std::path::Path, catalog: &[Value]) {
    let mut changed = false;
    for v in voice_library::list(voices_dir).0 {
        if !v.card.display.is_empty() {
            continue;
        }
        let Some(e) = catalog.iter().find(|e| e["name"] == v.name.as_str()) else { continue };
        let mut card = v.card.clone();
        fill_card(&mut card, e, false);
        if card != v.card {
            match voice_library::write_card(voices_dir, &v.name, &card) {
                Ok(()) => changed = true,
                Err(err) => tracing::warn!("voices: the card of {} keeps its old data: {err}", v.name),
            }
        }
    }
    if changed {
        app::events::publish(json!({ "changed": "voices" }));
    }
}

/// GET /voices/catalog — каталог доп-голосов (HF датасет nerualdreming/russia_voices): на каждый голос name, file,
/// displayName, пол, возраст (adult/child), язык, описание, длительность, год рождения, URL-превью, размер, SHA-256,
/// размер txt. Необязательные query-фильтры (их же шлёт MCP): gender, age, language, q (имя/описание), min_duration,
/// max_duration — сервер отдаёт отобранное и сколько всего. Hugging Face не ответил — 502 {error, detail}.
pub(super) async fn voices_catalog(State(st): State<AppState>, Query(q): Query<std::collections::HashMap<String, String>>) -> Response {
    let dir = st.voices_dir.clone();
    let got = tokio::task::spawn_blocking(move || {
        let v = record::catalog()?;
        backfill_cards(&dir, v["voices"].as_array().map(Vec::as_slice).unwrap_or_default());
        Ok::<_, String>(v)
    })
    .await;
    match got {
        Ok(Ok(v)) => {
            let voices = v["voices"].as_array().cloned().unwrap_or_default();
            let filter = record::CatalogFilter {
                gender: q.get("gender").cloned(),
                age_group: q.get("age").or_else(|| q.get("age_group")).cloned(),
                language: q.get("language").or_else(|| q.get("lang")).cloned(),
                query: q.get("q").or_else(|| q.get("query")).cloned(),
                min_duration: q.get("min_duration").and_then(|s| s.parse().ok()),
                max_duration: q.get("max_duration").and_then(|s| s.parse().ok()),
            };
            let filtered = record::filter_catalog(&voices, &filter);
            Json(json!({ "voices": filtered, "total": voices.len(), "shown": filtered.len() })).into_response()
        }
        Ok(Err(e)) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "catalog_unavailable", "detail": e })),
        )
            .into_response(),
        Err(e) => join_failure(e),
    }
}


/// GET /voice-gate — the gate's settings, whether the owner has enrolled, and the wake words on offer.
pub(super) async fn voice_gate_get(State(st): State<AppState>) -> Response {
    let cfg = crate::voice_gate::config(&st.models_root);
    Json(json!({
        "enabled": cfg.enabled,
        "threshold": cfg.threshold,
        "wake_word": cfg.wake_word,
        "wake_lang": cfg.wake_lang,
        "wake_threshold": cfg.wake_threshold,
        "autostart": cfg.autostart,
        "enrolled": crate::voice_gate::enrolled(&st.models_root),
        "wake_words": crate::voice_gate::wake_words(),
    }))
    .into_response()
}

/// POST /voice-gate {enabled?, threshold?, wake_word?, wake_lang?, autostart?} — save the gate's settings.
pub(super) async fn voice_gate_set(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let mut cfg = crate::voice_gate::config(&st.models_root);
    if let Some(v) = body.get("enabled").and_then(Value::as_bool) {
        cfg.enabled = v;
    }
    if let Some(v) = body.get("threshold").and_then(Value::as_f64) {
        cfg.threshold = (v as f32).clamp(0.0, 1.0);
    }
    if let Some(v) = body.get("wake_threshold").and_then(Value::as_f64) {
        cfg.wake_threshold = (v as f32).clamp(0.0, 1.0);
    }
    if body.get("wake_word").is_some() {
        cfg.wake_word = body.get("wake_word").and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string());
    }
    if body.get("wake_lang").is_some() {
        cfg.wake_lang = body.get("wake_lang").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    }
    if let Some(v) = body.get("autostart").and_then(Value::as_bool) {
        cfg.autostart = v;
    }
    if let Err(e) = crate::voice_gate::save_config(&st.models_root, &cfg) {
        return internal_error(e);
    }
    // a wake word needs its recogniser; if it is not on disk yet, pull it in the background (progress in /setup/status)
    // so it is ready by the time Live runs — never let the gate fail silently for a missing model
    let wake_downloading = cfg.wake_word.is_some() && !crate::wake_word::model_path(&st.models_root).is_file();
    if wake_downloading {
        let _ = st.downloads.start(vec!["wake-word".into()]); // "busy" just means a download is already under way
    }
    // the full state, as GET returns it, so a caller can replace its state with the reply and not lose a field
    Json(json!({ "enabled": cfg.enabled, "threshold": cfg.threshold, "wake_word": cfg.wake_word, "wake_lang": cfg.wake_lang, "wake_threshold": cfg.wake_threshold, "autostart": cfg.autostart, "enrolled": crate::voice_gate::enrolled(&st.models_root), "wake_words": crate::voice_gate::wake_words(), "wake_downloading": wake_downloading })).into_response()
}

/// POST /voice-gate/enroll — learn the owner's voice from the last microphone recording (`.mic-test.wav`). The model is
/// pulled on first use. One recording, in order: recognise it (ASR) so the person sees their voice was captured, then
/// reuse the same recording for the voiceprint. CPU: a short clip is light and keeps off the GPU the agent uses.
pub(super) async fn voice_gate_enroll(State(st): State<AppState>) -> Response {
    let (repo, models, workspace) = (st.repo_root.clone(), st.models_root.clone(), st.workspace.clone());
    let done = tokio::task::spawn_blocking(move || -> Result<(f64, String), String> {
        app::ensure_components(&repo, &["voice-gate"], &|_| {})?;
        let wav = mic_test_path(&workspace);
        if !wav.is_file() {
            return Err(t!("record-test-none"));
        }
        // recognise what was said first, with the studio's own recogniser, so the person sees their voice came through;
        // nothing recognised means silence or noise, not a voice to enrol — reject so they can speak and retry.
        let text = super::asr::transcribe(&repo, &models, &wav, "auto", "").map(|h| h.text.trim().to_string()).unwrap_or_default();
        if text.is_empty() {
            return Err(t!("voice-gate-not-heard"));
        }
        let tmp = tempfile::Builder::new().prefix("enroll-").tempdir().map_err(|e| e.to_string())?;
        let k = tmp.path().join("16k.wav");
        crate::audio::media::to_16k_mono(&wav, &k)?;
        let (samples, _) = crate::audio::wavio::read_mono_f32(&k)?;
        if (samples.len() as f64 / crate::voice_gate::SR as f64) < 1.0 {
            return Err(t!("voice-gate-too-short"));
        }
        let seconds = crate::voice_gate::enroll(&models, false, &samples)?;
        Ok((seconds, text))
    })
    .await;
    match done {
        Ok(Ok((seconds, text))) => Json(json!({ "enrolled": true, "seconds": (seconds * 100.0).round() / 100.0, "text": text })).into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, Json(json!({ "error": "enroll_failed", "detail": e }))).into_response(),
        Err(e) => join_failure(e),
    }
}


/// POST /voice-gate/test {on} — start or stop the live voice check: while on, the microphone records and the most
/// recent speech is matched against the enrolled voice (see GET /voice-gate/score), so the person speaks and watches
/// how the model hears them and tunes the sensitivity by it. The microphone is the person's: an agent only reads.
pub(super) async fn voice_gate_test(State(st): State<AppState>, headers: HeaderMap, body: Option<Json<Value>>) -> Response {
    if tools::by_agent(&headers) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "mic_only_person", "detail": t!("mic-only-person") }))).into_response();
    }
    let on = body.as_ref().and_then(|Json(v)| v.get("on")).and_then(Value::as_bool).unwrap_or(false);
    if !on {
        record::tap(None);
        let _ = record::stop();
        crate::voice_gate::test_clear();
        return Json(json!({ "testing": false })).into_response();
    }
    let path = mic_test_path(&st.workspace);
    // tap set before the recording starts, so the window fills from its first sound; the recorder's device rate is
    // resampled to the 16 kHz the voice model wants.
    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();
    record::tap(Some(tx));
    crate::voice_gate::test_clear();
    match tokio::task::spawn_blocking(move || record::start_test(&path, None)).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            record::tap(None);
            return (StatusCode::CONFLICT, Json(json!({ "error": "record_busy", "detail": e }))).into_response();
        }
        Err(e) => {
            record::tap(None);
            return join_failure(e);
        }
    }
    std::thread::spawn(move || {
        while let Ok(chunk) = rx.recv() {
            let rate = record::rate().max(1);
            crate::voice_gate::test_push(&crate::audio::wavio::to_16k(&chunk, rate));
        }
    });
    say_api::guard_mic(st.workspace.clone());
    Json(json!({ "testing": true })).into_response()
}

/// GET /voice-gate/score — the live match of the voice check (POST /voice-gate/test) against the enrolled voice, 0..=1,
/// and the configured threshold; `ready` is false until there is enough speech. The person compares the two to tune the
/// sensitivity in real time.
pub(super) async fn voice_gate_score(State(st): State<AppState>) -> Response {
    let models = st.models_root.clone();
    let done = tokio::task::spawn_blocking(move || -> Result<(Option<f32>, f32), String> {
        let score = crate::voice_gate::test_score(&models)?;
        Ok((score, crate::voice_gate::config(&models).threshold))
    })
    .await;
    match done {
        Ok(Ok((score, threshold))) => Json(json!({ "score": score.map(|s| (s * 100.0).round() / 100.0), "threshold": threshold, "ready": score.is_some() })).into_response(),
        Ok(Err(e)) => internal_error(e),
        Err(e) => join_failure(e),
    }
}


/// GET /voice-gate/wake-score — the live phone overlap of the voice check (POST /voice-gate/test) with the configured
/// wake phrase, 0..=1, and the wake threshold; `ready` is false until there is enough speech or no phrase is set. The
/// person says the phrase and watches whether, and how strongly, it is recognised, trying different voices.
pub(super) async fn voice_gate_wake_score(State(st): State<AppState>) -> Response {
    let models = st.models_root.clone();
    let done = tokio::task::spawn_blocking(move || -> Result<(Option<f32>, f32), String> {
        let score = crate::voice_gate::wake_test_score(&models, false)?;
        Ok((score, crate::voice_gate::config(&models).wake_threshold))
    })
    .await;
    match done {
        Ok(Ok((score, threshold))) => Json(json!({ "score": score.map(|s| (s * 100.0).round() / 100.0), "threshold": threshold, "ready": score.is_some() })).into_response(),
        Ok(Err(e)) => internal_error(e),
        Err(e) => join_failure(e),
    }
}

/// DELETE /voice-gate/owner — forget the enrolled voice (the privacy delete).
pub(super) async fn voice_gate_forget(State(st): State<AppState>) -> Response {
    match crate::voice_gate::forget_owner(&st.models_root) {
        Ok(()) => Json(json!({ "enrolled": false })).into_response(),
        Err(e) => internal_error(e),
    }
}

/// POST /voices/get {name} — скачать один голос (mp3+txt) из датасета в каталог голосов.
pub(super) async fn voices_get(State(st): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let dir = st.voices_dir.clone();
    let fetch_name = name.clone();
    let ok = tokio::task::spawn_blocking(move || record::fetch_voice(&dir, &fetch_name)).await;
    match ok {
        Ok(Ok(())) => {
            let base = name.clone();
            let mut card = voice_library::VoiceCard {
                source: "russia_voices".into(),
                ..voice_library::guess_card(&base)
            };
            // the dataset's metadata is surer than the guess from the file name (and gives the language, so «auto»
            // recognition routes the reference right away)
            if let Ok(cat) = record::catalog() {
                if let Some(e) = cat["voices"].as_array().into_iter().flatten().find(|e| e["name"] == name.as_str()) {
                    fill_card(&mut card, e, true);
                }
            }
            if let Err(e) = voice_library::write_card(&st.voices_dir, &base, &card) {
                return Json(json!({ "ok": false, "error": e }));
            }
            app::events::publish(json!({ "changed": "voices" }));
            Json(json!({ "ok": true, "voices": voice_api::listing(&st.voices_dir)["voices"] }))
        }
        Ok(Err(e)) => Json(json!({ "ok": false, "error": e })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

/// POST /voices/download-pack — скачать пак голосов (VibeVoice) и распаковать в каталог голосов.
pub(super) async fn voices_download_pack(State(st): State<AppState>) -> Response {
    let dir = st.voices_dir.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        record::download_pack(&dir, &cb)
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::VoicesPack, None), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => enqueue_error(e),
    }
}
