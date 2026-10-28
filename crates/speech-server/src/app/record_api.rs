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

/// GET /voices/catalog — каталог доп-голосов (HF датасет nerualdreming/russia_voices): на каждый голос name, file,
/// displayName, пол, возраст (adult/child), язык, описание, длительность, год рождения, URL-превью, размер, SHA-256,
/// размер txt. Необязательные query-фильтры (их же шлёт MCP): gender, age, language, q (имя/описание), min_duration,
/// max_duration — сервер отдаёт отобранное и сколько всего. Hugging Face не ответил — 502 {error, detail}.
pub(super) async fn voices_catalog(Query(q): Query<std::collections::HashMap<String, String>>) -> Response {
    match tokio::task::spawn_blocking(record::catalog).await {
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
                    if let Some(g) = e["gender"].as_str().filter(|s| !s.is_empty()) {
                        card.gender = g.to_string();
                    }
                    if let Some(l) = e["language"].as_str().filter(|s| !s.is_empty()) {
                        card.lang = l.to_ascii_lowercase();
                    }
                    if let Some(a) = e["age_group"].as_str().filter(|a| *a == "child" || *a == "adult") {
                        card.age = a.to_string();
                    }
                    if let Some(d) = e["displayName"].as_str().filter(|s| !s.is_empty() && *s != name.as_str()) {
                        card.display = d.to_string();
                    }
                    if let Some(d) = e["description"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        card.description = d.to_string();
                    }
                    card.birth_year = e["birth_year"].as_u64().and_then(|y| u32::try_from(y).ok());
                    card.duration_s = e["duration_s"].as_f64().filter(|d| *d > 0.0);
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
