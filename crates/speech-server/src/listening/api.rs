//! Маршруты Listening: запись в проект (файл, микрофон; ссылка — через загрузку по ссылке), транскрипт, распознавание
//! и монтаж по тексту задачами общей очереди, звук записи с Range, экспорт, проверка по эталонному тексту, проект
//! Speech из транскрипта и голоса из спикеров.

use std::path::Path;

use axum::extract::{Path as AxPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::listening::{jobs as listen_jobs, project as lp};
use crate::listening::jobs::ListenPaths;
use crate::listening::project::ListenProject;
use crate::app::{joined, refuse};
use crate::{jobs, AppState};

pub fn paths(st: &AppState) -> ListenPaths {
    crate::app::listening_paths(st)
}

fn saved(r: Result<Result<Value, String>, Response>, missing: StatusCode) -> Response {
    match r {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) if e.starts_with(crate::app::PROJECT_CHANGED) => refuse(StatusCode::CONFLICT, crate::app::PROJECT_CHANGED, e),
        Ok(Err(e)) => refuse(missing, "listen_failed", e),
        Err(r) => r,
    }
}

fn make(workspace: &Path, pid: &str, name: &str, source: &str, origin: &str) -> Result<Value, String> {
    let project = super::create_from_source(workspace, pid, name, source, origin)?;
    Ok(json!({ "pid": pid, "project": project }))
}

/// POST /listen/upload (multipart, поле file) — новый проект из файла записи; файл пишется на диск по частям.
pub async fn upload(State(st): State<AppState>, mut multipart: axum::extract::Multipart) -> Response {
    let pid = crate::platform::ids::new_id();
    let dir = crate::listening::project_dir(&st.workspace, &pid);
    let mut got: Option<(String, String)> = None;
    if let Ok(Some(mut field)) = multipart.next_field().await {
        let original = field.file_name().unwrap_or("record.wav").to_string();
        if !lp::is_media(&original) {
            return refuse(StatusCode::BAD_REQUEST, "not_media", t!("listen-not-media", file = original));
        }
        let ext = Path::new(&original).extension().and_then(|e| e.to_str()).unwrap_or("wav").to_ascii_lowercase();
        let source = format!("source.{ext}");
        if let Err(e) = tokio::fs::create_dir_all(&dir).await {
            return refuse(StatusCode::INTERNAL_SERVER_ERROR, "io", format!("{}: {e}", dir.display()));
        }
        let path = dir.join(&source);
        let mut out = match tokio::fs::File::create(&path).await {
            Ok(f) => f,
            Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "io", format!("{}: {e}", path.display())),
        };
        loop {
            match field.chunk().await {
                Ok(Some(bytes)) => {
                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut out, &bytes).await {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        return refuse(StatusCode::INTERNAL_SERVER_ERROR, "io", format!("{}: {e}", path.display()));
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = tokio::fs::remove_dir_all(&dir).await;
                    return refuse(StatusCode::BAD_REQUEST, "bad_request", e.to_string());
                }
            }
        }
        got = Some((original, source));
    }
    let Some((original, source)) = got else { return refuse(StatusCode::BAD_REQUEST, "bad_request", "no file") };
    let ws = st.workspace.clone();
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || {
        let made = make(&ws, &pid, &super::stem(&original), &source, &original);
        if made.is_err() {
            let _ = std::fs::remove_dir_all(crate::listening::project_dir(&ws, &pid));
        }
        made
    }))
    .await);
    saved(res, StatusCode::BAD_REQUEST)
}

use super::recording::RECORDING;


pub async fn record_stop(State(st): State<AppState>) -> Response {
    super::stop_recording_response(super::stop_recording(&st.workspace).await)
}

/// Live recognition starts only with its recogniser and the speech detector on disk: what is missing is downloaded in
/// the background (the setup download, with its progress), and the request is refused with `asr_downloading` and the
/// components until it is done.
async fn live_parts(st: &AppState, engine: &str) -> Result<(), Response> {
    let (root, models, engine) = (st.repo_root.clone(), st.models_root.clone(), engine.to_string());
    let ids = joined(tokio::task::spawn_blocking(move || {
        let mut ids = crate::app::recognition::missing_parts(&root, &models, &engine);
        for id in crate::engines::vad::missing(&models) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    })
    .await)?;
    if ids.is_empty() {
        return Ok(());
    }
    let dl = st.downloads.clone();
    let wanted = ids.clone();
    match joined(tokio::task::spawn_blocking(move || dl.start(wanted)).await)? {
        Err(e) if e.code != "busy" => Err(refuse(StatusCode::CONFLICT, &e.code, e.detail)),
        _ => Err((StatusCode::CONFLICT, Json(json!({ "error": "asr_downloading", "detail": t!("listen-live-downloading"), "args": { "ids": ids } }))).into_response()),
    }
}

/// POST /listen/record/start {device?, live?, lang?, engine?} — запись с микрофона в новый проект; с `live` она распознаётся по
/// фразам, пока идёт (`/listen/record/live`).
pub async fn record_start(State(st): State<AppState>, headers: HeaderMap, body: Option<Json<Value>>) -> Response {
    if crate::app::agent::by_agent(&headers) {
        return refuse(StatusCode::FORBIDDEN, "mic_only_person", t!("mic-only-person"));
    }
    let body = body.map(|Json(v)| v).unwrap_or(Value::Null);
    let device = body.get("device").and_then(Value::as_str).map(str::to_string).filter(|d| !d.is_empty());
    let live = body.get("live").and_then(Value::as_bool).unwrap_or(false);
    let lang = body.get("lang").and_then(Value::as_str).unwrap_or("auto").to_string();
    let engine = body.get("engine").and_then(Value::as_str).unwrap_or_default().to_string();
    if !matches!(engine.as_str(), "" | "parakeet" | "nemotron" | "whisper") {
        return refuse(StatusCode::BAD_REQUEST, "bad_engine", t!("listen-live-engine", engine = engine));
    }
    if live {
        if let Err(r) = live_parts(&st, &engine).await {
            return r;
        }
    }
    let pid = crate::platform::ids::new_id();
    let dir = crate::listening::project_dir(&st.workspace, &pid);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, "io", format!("{}: {e}", dir.display()));
    }
    let path = dir.join("source.wav");
    match joined(tokio::task::spawn_blocking(move || crate::voices::record::start(&path, device)).await) {
        Ok(Ok(())) => {
            if live {
                let (root, models) = (st.repo_root.clone(), st.models_root.clone());
                if let Err(e) = joined(tokio::task::spawn_blocking(move || crate::listening::live::start(&root, &models, &lang, &engine)).await).and_then(|r| r.map_err(|e| refuse(StatusCode::CONFLICT, "live_failed", e))) {
                    let _ = tokio::task::spawn_blocking(crate::voices::record::stop).await;
                    let _ = std::fs::remove_dir_all(&dir);
                    return e;
                }
            }
            *RECORDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(pid.clone());
            crate::speech::talk::guard_mic(st.workspace.clone());
            Json(json!({ "pid": pid, "live": live })).into_response()
        }
        Ok(Err(e)) => {
            let _ = std::fs::remove_dir_all(&dir);
            refuse(StatusCode::CONFLICT, "record_failed", e)
        }
        Err(r) => r,
    }
}


/// GET /listen/watch — the folder of recordings recognised in the background: its settings, files and how far it got.
pub async fn watch_status() -> Json<Value> {
    Json(crate::listening::watch::status())
}

/// POST /listen/watch {folder, out?, formats, keep?, engine?, lang?, rules?, digits?} — go through a folder of
/// recordings in the background (and keep watching it with `keep`).
pub async fn watch_start(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let config: crate::listening::watch::WatchConfig = match serde_json::from_value(body) {
        Ok(c) => c,
        Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e.to_string()),
    };
    match crate::listening::watch::start(crate::app::watch_dependencies(&st), config) {
        Ok(v) => Json(v).into_response(),
        Err(e) => refuse(StatusCode::BAD_REQUEST, "bad_watch", e),
    }
}

/// POST /listen/watch/stop — stop after the recording being recognised now.
pub async fn watch_stop(State(st): State<AppState>) -> Response {
    match crate::listening::watch::stop(&st.repo_root) {
        Ok(v) => Json(v).into_response(),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "watch_failed", e),
    }
}

/// GET /listen/record/live — what live recognition of the recording under way has heard: phrases with their times.
pub async fn record_live() -> Json<Value> {
    Json(crate::listening::live::state_with_words())
}

/// GET /listen/{pid} — проект.
pub async fn get(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    match crate::listening::load(&st.workspace, &pid) {
        Ok(p) => Json(json!(p)).into_response(),
        Err(e) => refuse(StatusCode::NOT_FOUND, "no_project", e),
    }
}

/// PUT /listen/{pid} — сохранить правки транскрипта (тексты, спикеры, скрытие, пометки к вырезке, настройки).
pub async fn put(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    let ws = st.workspace.clone();
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || -> Result<Value, String> {
        let mut project: ListenProject = serde_json::from_value(body).map_err(|e| e.to_string())?;
        let was = crate::listening::load(&ws, &pid)?;
        project.kind = lp::KIND.into();
        project.source = was.source;
        crate::listening::save(&ws, &pid, &project)?;
        Ok(json!(project))
    }))
    .await);
    saved(res, StatusCode::BAD_REQUEST)
}

/// GET /listen/{pid}/audio — запись (или её AAC-копия для формата, который webview не играет) с Range.
pub async fn audio(State(st): State<AppState>, AxPath(pid): AxPath<String>, req: axum::http::Request<axum::body::Body>) -> Response {
    let project = match crate::listening::load(&st.workspace, &pid) {
        Ok(p) => p,
        Err(e) => return refuse(StatusCode::NOT_FOUND, "no_project", e),
    };
    let dir = crate::listening::project_dir(&st.workspace, &pid);
    let copy = dir.join("work").join("play.m4a");
    let path = if copy.is_file() { copy } else { dir.join(&project.source) };
    if !path.is_file() {
        return refuse(StatusCode::NOT_FOUND, "no_file", t!("listen-no-source"));
    }
    crate::app::serve_file_range(&path, req, None).await
}

fn enqueue_job(st: &AppState, pid: &str, kind: jobs::JobKind, run: fn(&ListenPaths, &str, &dyn Fn(Value)) -> Result<Value, String>) -> impl std::future::Future<Output = Response> {
    let p = paths(st);
    let (jobs, pid) = (st.jobs.clone(), pid.to_string());
    async move {
        if let Err(e) = crate::listening::load(&p.workspace, &pid) {
            return refuse(StatusCode::NOT_FOUND, "no_project", e);
        }
        let pid2 = pid.clone();
        let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| run(&p, &pid2, &|ev| progress(ev)));
        match jobs.enqueue(jobs::JobMeta::new(kind, Some(&pid)), job).await {
            Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
            Err(e) => crate::app::enqueue_error(e),
        }
    }
}

/// POST /listen/{pid}/transcribe — распознать запись задачей.
pub async fn transcribe(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    enqueue_job(&st, &pid, jobs::JobKind::Analyze, listen_jobs::transcribe).await
}

/// POST /listen/{pid}/cut — «Удалить отмеченное»: новая запись и новый проект без вырезанного.
pub async fn cut(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    enqueue_job(&st, &pid, jobs::JobKind::Remix, listen_jobs::cut).await
}

/// GET /listen/{pid}/stats — who speaks how much: talk time, share, lines, words, pace and longest stretch per speaker.
pub async fn stats(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    match crate::listening::load(&st.workspace, &pid) {
        Ok(p) => Json(p.speaker_stats()).into_response(),
        Err(e) => refuse(StatusCode::NOT_FOUND, "no_project", e),
    }
}

/// GET /listen/subtitle-rules — the broadcasters' subtitle rules the export lays subtitles out by.
pub async fn subtitle_rules() -> Json<Value> {
    Json(json!({ "rules": dub_captions::PROFILES.iter().map(|p| json!({ "name": p.name, "max_line_len": p.max_line_len, "max_lines": p.max_lines, "max_cps": p.max_cps, "min_display_ms": p.min_display_ms, "max_display_ms": p.max_display_ms })).collect::<Vec<_>>() }))
}

/// POST /listen/{pid}/export {format, speakers?, times?, rules?, digits?} — файл транскрипта в exports проекта: {file, text,
/// too_fast}; с `rules` (профиль вещателя) субтитры раскладываются на экраны по его правилам.
pub async fn export(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    let format = body.get("format").and_then(Value::as_str).unwrap_or("srt").to_string();
    let rules = match body.get("rules").and_then(Value::as_str).filter(|r| !r.is_empty()) {
        Some(name) => match dub_captions::RulesProfile::by_name(name) {
            Some(p) => Some(p),
            None => return refuse(StatusCode::BAD_REQUEST, "bad_rules", t!("listen-export-rules", rules = name.to_string(), known = dub_captions::PROFILES.iter().map(|p| p.name).collect::<Vec<_>>().join(", "))),
        },
        None => None,
    };
    let o = crate::listening::export::Opts {
        speakers: body.get("speakers").and_then(Value::as_bool).unwrap_or(true),
        times: body.get("times").and_then(Value::as_bool).unwrap_or(true),
        rules,
        digits: body.get("digits").and_then(Value::as_bool).unwrap_or(false),
    };
    let ws = st.workspace.clone();
    let res = joined(tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let project = crate::listening::load(&ws, &pid)?;
        let text = crate::listening::export::render(&project, &format, &o)?;
        let (rows, unknown) = if o.digits { crate::listening::export::with_digits(crate::listening::export::shown_rows(&project), &project.settings.lang)? } else { (crate::listening::export::shown_rows(&project), 0) };
        let fast = o.rules.filter(|_| matches!(format.as_str(), "srt" | "vtt" | "ass")).map(|rules| crate::listening::export::by_rules(&rows, rules, &project.settings.lang).1);
        let dir = crate::listening::project_dir(&ws, &pid).join("exports");
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let base: String = project.name.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { ' ' } else { c }).collect::<String>().trim().to_string();
        let file = format!("{}.{format}", if base.is_empty() { "transcript" } else { &base });
        crate::platform::fs::write_atomic(&dir.join(&file), text.as_bytes())?;
        Ok(json!({ "file": file, "text": text, "too_fast": fast, "digits_unknown": o.digits.then_some(unknown) }))
    })
    .await);
    saved(res, StatusCode::BAD_REQUEST)
}

/// POST /listen/{pid}/aligned {reference} — строки записи с текстом эталона по времени распознанных слов; окно
/// применяет их правкой проекта (её можно отменить).
pub async fn aligned(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    let reference = body.get("reference").and_then(Value::as_str).unwrap_or_default().to_string();
    if reference.trim().is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "no_reference", t!("listen-align-no-text"));
    }
    match crate::listening::load(&st.workspace, &pid) {
        Ok(p) if p.lines.is_empty() => refuse(StatusCode::CONFLICT, "not_transcribed", t!("listen-transcribe-first")),
        Ok(p) => Json(json!({ "lines": listen_jobs::align_to_text(&p, &reference) })).into_response(),
        Err(e) => refuse(StatusCode::NOT_FOUND, "no_project", e),
    }
}

/// POST /listen/{pid}/check {reference} — распознанное против эталонного текста по словам.
pub async fn check(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    let reference = body.get("reference").and_then(Value::as_str).unwrap_or_default();
    match crate::listening::load(&st.workspace, &pid) {
        Ok(p) => match listen_jobs::check_reference(&p, reference) {
            Ok(report) => Json(report).into_response(),
            Err(e) => refuse(StatusCode::BAD_REQUEST, "check_failed", e),
        },
        Err(e) => refuse(StatusCode::NOT_FOUND, "no_project", e),
    }
}

/// POST /listen/{pid}/to-speech — проект Speech из транскрипта: реплики с теми же спикерами на таймкодах записи.
pub async fn to_speech(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let (ws, voices_dir) = (st.workspace.clone(), st.voices_dir.clone());
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || {
        crate::app::convert_listening_to_speech(&ws, &voices_dir, &pid).map(|pid| json!({ "pid": pid }))
    }))
    .await);
    saved(res, StatusCode::BAD_REQUEST)
}

/// POST /listen/{pid}/voices {speaker?} — голос из спикера записи (или всех): лучший кусок его речи по границам
/// слов — в галерею голосов, с текстом и словами куска.
pub async fn voices(State(st): State<AppState>, AxPath(pid): AxPath<String>, body: Option<Json<Value>>) -> Response {
    let only = body.and_then(|Json(v)| v.get("speaker").and_then(Value::as_str).map(str::to_string));
    let (ws, voices_dir, models_root) = (st.workspace.clone(), st.voices_dir.clone(), st.models_root.clone());
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || crate::app::extract_listening_voices(&ws, &voices_dir, &models_root, &pid, only.as_deref()))).await);
    saved(res, StatusCode::BAD_REQUEST)
}

fn profile_paths(st: &AppState) -> crate::app::profiles::Paths {
    crate::app::profiles::Paths {
        workspace: st.workspace.clone(),
        profiles: crate::voices::matching::Paths { repo_root: st.repo_root.clone(), voices_dir: st.voices_dir.clone(), models_root: st.models_root.clone() },
    }
}

/// POST /listen/{pid}/speakers/match — who each speaker of the recording is like among the named speakers and the
/// library voices, by the voice print.
pub async fn speakers_match(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let p = profile_paths(&st);
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || crate::app::profiles::matches(&p, &pid))).await);
    saved(res, StatusCode::BAD_REQUEST)
}

/// POST /listen/{pid}/speakers/remember {speaker} — keep this speaker's voice print under the name it has here.
pub async fn speaker_remember(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    let speaker = body.get("speaker").and_then(Value::as_str).unwrap_or_default().to_string();
    let p = profile_paths(&st);
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || crate::app::profiles::remember(&p, &pid, &speaker))).await);
    saved(res, StatusCode::BAD_REQUEST)
}

/// GET /speaker-profiles — the named speakers the studio knows by voice.
pub async fn speaker_profiles(State(st): State<AppState>) -> Response {
    match crate::voices::matching::listing(&st.repo_root) {
        Ok(v) => Json(v).into_response(),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "read_failed", e),
    }
}

/// DELETE /speaker-profiles/{name} — forget a named speaker's voice print.
pub async fn speaker_forget(State(st): State<AppState>, AxPath(name): AxPath<String>) -> Response {
    match crate::voices::matching::forget(&st.repo_root, &name) {
        Ok(v) => Json(v).into_response(),
        Err(e) => refuse(StatusCode::NOT_FOUND, "no_profile", e),
    }
}


/// POST /listen/from-path {path} — проект Listening файла этого компьютера (для агента): тот же файл с теми же
/// размером и временем находит прежний проект.
pub async fn from_path(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let path = std::path::PathBuf::from(body.get("path").and_then(Value::as_str).unwrap_or_default());
    let ws = st.workspace.clone();
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || -> Result<Value, String> {
        let (pid, existing) = super::project_of_file(&ws, &path)?;
        Ok(json!({ "pid": pid, "existing": existing }))
    }))
    .await);
    saved(res, StatusCode::BAD_REQUEST)
}

/// POST /separate {path} — голос файла отдельно от музыки и шума (Roformer) задачей; готовый — сразу путь.
pub async fn separate_file(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let path = std::path::PathBuf::from(body.get("path").and_then(Value::as_str).unwrap_or_default());
    if !path.is_file() || !lp::is_media(&path.to_string_lossy()) {
        return refuse(StatusCode::BAD_REQUEST, "not_media", t!("listen-not-media", file = path.display().to_string()));
    }
    let key = match super::file_key(&path, "separate") {
        Ok(k) => k,
        Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e),
    };
    let out = st.workspace.join(".separate").join(&key);
    let vocals = out.join("vocals.flac");
    if vocals.is_file() {
        return Json(json!({ "done": true, "vocals": vocals, "instrumental": out.join("instrumental.flac") })).into_response();
    }
    let (repo_root, models_root) = (st.repo_root.clone(), st.models_root.clone());
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| crate::listening::jobs::separate_to(&repo_root, &models_root, &path, &out, &|ev| progress(ev)));
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Separate, None), job).await {
        Ok(job_id) => Json(json!({ "done": false, "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}
