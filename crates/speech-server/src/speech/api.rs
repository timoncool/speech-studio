//! Маршруты проектов: создать (Speech из текста), список, проект со статусами блоков, запись целиком (ревизии Dub:
//! 409 на чужую правку), удаление в корзину системы, синтез и экспорт задачами, звук дублей и файлов экспорта.

pub(crate) use super::naming::safe_file;

use crate::app::{refuse, joined};
use std::collections::HashMap;
use std::path::Path;

use axum::extract::{Path as AxPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::speech::jobs as speech_jobs;
use crate::speech::jobs::ExportOpts;

use crate::speech::jobs::SpeechPaths;
use crate::speech::project as sp;
use crate::speech::project::SpeechProject;
use crate::{jobs, AppState};



pub fn paths(st: &AppState) -> SpeechPaths {
    SpeechPaths {
        repo_root: st.repo_root.clone(),
        workspace: st.workspace.clone(),
        voices_dir: st.voices_dir.clone(),
        models_root: st.models_root.clone(),
    }
}

fn shown(p: &SpeechPaths, project: &SpeechProject) -> Value {
    sp::with_status(project, &speech_jobs::keys(p, project))
}

/// Карточка проекта для списка: вид, имя, время, число блоков и готовых.
fn summary(p: &SpeechPaths, dir: &Path) -> Option<Value> {
    let pid = dir.file_name()?.to_str()?.to_string();
    let text = std::fs::read_to_string(dir.join("project.json")).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let mtime = std::fs::metadata(dir.join("project.json")).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    let kind = v.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
    let mut card = json!({ "pid": pid, "kind": kind, "mtime": mtime });
    if kind == sp::KIND {
        let project: SpeechProject = serde_json::from_value(v).ok()?;
        let keys = speech_jobs::keys(p, &project);
        let total = project.blocks().filter(|b| !b.hidden && !b.text.trim().is_empty()).count();
        let ready = project.blocks().filter(|b| sp::status(b, keys.get(&b.id).map(String::as_str).unwrap_or("")) == "ready").count();
        card["name"] = json!(project.name);
        card["created"] = json!(project.created);
        card["blocks"] = json!(total);
        card["ready"] = json!(ready);
    } else if kind == crate::listening::KIND {
        let project: crate::listening::ListenProject = serde_json::from_value(v).ok()?;
        if let Value::Object(extra) = crate::listening::summary(&project) {
            card.as_object_mut()?.extend(extra);
        }
    } else {
        return None;
    }
    Some(card)
}

/// GET /projects — проекты Speech и Listening, новые первыми.
pub async fn list(State(st): State<AppState>) -> Response {
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(move || {
        let mut out: Vec<Value> = std::fs::read_dir(&p.workspace)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|d| d.join("project.json").is_file()).filter_map(|d| summary(&p, &d)).collect())
            .unwrap_or_default();
        out.sort_by_key(|v| std::cmp::Reverse(v["mtime"].as_u64().unwrap_or(0)));
        out
    })
    .await);
    match res {
        Ok(v) => Json(json!({ "projects": v })).into_response(),
        Err(r) => r,
    }
}

/// Проект Speech с ключом инструмента агента.
fn keyed_project(workspace: &Path, key: &str) -> Option<(String, SpeechProject)> {
    std::fs::read_dir(workspace).ok()?.flatten().find_map(|e| {
        let pid = e.file_name().to_string_lossy().into_owned();
        let project = crate::speech::load(workspace, &pid).ok()?;
        (project.key == key).then_some((pid, project))
    })
}

/// The engine settings a request may give a new project, each over the engine's default; an engine reads those it has.
struct SamplingArgs {
    temperature: Option<f64>,
    top_k: Option<u32>,
    top_p: Option<f64>,
    max_tokens: Option<u32>,
    steps: Option<u32>,
}

impl SamplingArgs {
    fn of(body: &Value) -> Self {
        let float = |n: &str| body.get(n).and_then(Value::as_f64);
        let int = |n: &str| body.get(n).and_then(Value::as_u64).map(|v| v.min(u32::MAX as u64) as u32);
        SamplingArgs { temperature: float("temperature"), top_k: int("top_k"), top_p: float("top_p"), max_tokens: int("max_tokens"), steps: int("steps") }
    }
}

/// POST /projects {name, text?, voice?, lang?, seed?, temperature?, top_k?, top_p?, max_tokens?, loudness?} — новый проект
/// Speech: текст режется на блоки (абзацы или диалог); `seed` фиксирует сид дублей, остальное — настройки проекта.
pub async fn create(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).map(str::to_string).unwrap_or_else(|| t!("speech-untitled"));
    let text = body.get("text").and_then(Value::as_str).unwrap_or("").to_string();
    let voice = body.get("voice").and_then(Value::as_str).map(str::to_string).filter(|v| !v.is_empty());
    let lang = body.get("lang").and_then(Value::as_str).map(str::to_string);
    let p = paths(&st);
    let split = body.get("split").and_then(Value::as_str).unwrap_or("paragraphs").to_string();
    let key = body.get("key").and_then(Value::as_str).unwrap_or("").to_string();
    let engine = body.get("engine").and_then(Value::as_str).filter(|e| !e.is_empty()).map(str::to_string);
    let direction = body.get("direction").and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
    let seed = body.get("seed").and_then(Value::as_u64);
    let sampling = SamplingArgs::of(&body);
    let loudness = body.get("loudness").and_then(Value::as_str).filter(|l| !l.is_empty()).map(str::to_string);
    let corpus = body.get("corpus").and_then(Value::as_str).filter(|c| !c.is_empty()).map(str::to_string);
    if let Some(c) = corpus.as_deref().filter(|c| *c != "ru_hard") {
        return refuse(StatusCode::BAD_REQUEST, "bad_project", t!("speech-corpus-unknown", corpus = c.to_string()));
    }
    let named = body.get("name").and_then(Value::as_str).is_some_and(|n| !n.trim().is_empty());
    if !key.is_empty() {
        if let Some((pid, project)) = keyed_project(&p.workspace, &key) {
            return Json(json!({ "project_id": pid, "project": shown(&p, &project), "existing": true })).into_response();
        }
    }
    let files: Vec<(String, String)> = body
        .get("files")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|f| Some((f.get("name")?.as_str()?.to_string(), f.get("text")?.as_str()?.to_string()))).collect())
        .unwrap_or_default();
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || -> Result<(String, Value), String> {
        let narrator = t!("speech-narrator");
        let mut project = if corpus.is_some() {
            crate::speech::corpus::project(&if named { name } else { t!("speech-corpus-name") }, &narrator)
        } else if !files.is_empty() {
            sp::from_files(&name, &files, &narrator)
        } else if split == "lines" {
            sp::from_lines(&name, &text, &narrator)
        } else {
            sp::from_text(&name, &text, &narrator)
        };
        if project.chapters[0].blocks.is_empty() {
            project.chapters[0].blocks.push(sp::Block { id: sp::new_id(), speaker: t!("speech-narrator"), ..Default::default() });
        }
        project.settings.voice = voice;
        if let Some(l) = lang {
            project.settings.lang = l;
        }
        if let Some(e) = engine {
            // the setup component's id names the same engine
            let e = if e == crate::engines::supertonic::DIR { crate::engines::supertonic::ENGINE.to_string() } else { e };
            let Some(manifest) = crate::engines::get(&e) else {
                return Err(t!("speech-engine-unknown", engine = e));
            };
            project.settings.params.engine_defaults(&manifest);
            project.settings.engine = e;
        }
        if let Some(seed) = seed {
            project.settings.params.seed_mode = "fixed".into();
            project.settings.params.seed = seed % crate::engines::tts_host::SEED_RANGE;
        }
        let params = &mut project.settings.params;
        if let Some(v) = sampling.temperature {
            params.temperature = v;
        }
        if let Some(v) = sampling.top_k {
            params.top_k = v;
        }
        if let Some(v) = sampling.top_p {
            params.top_p = v;
        }
        if let Some(v) = sampling.max_tokens {
            params.max_tokens = v;
        }
        if let Some(v) = sampling.steps {
            params.steps = v;
        }
        if let Some(l) = loudness {
            project.settings.loudness = l;
        }
        crate::speech::validate(&project)?;
        project.key = key;
        if !direction.is_empty() {
            for b in project.chapters.iter_mut().flat_map(|c| c.blocks.iter_mut()) {
                b.direction = direction.clone();
            }
        }
        let pid = sp::new_id();
        crate::speech::save(&p.workspace, &pid, &project)?;
        Ok((pid, shown(&p, &project)))
    }))
    .await);
    match res {
        Ok(Ok((pid, project))) => Json(json!({ "project_id": pid, "project": project })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "bad_project", e),
        Err(r) => r,
    }
}

/// GET /projects/{pid} — проект Speech со статусами блоков (Listening — как записан).
pub async fn get(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, String)> {
        if !sp::is_pid(&pid) {
            return Err((StatusCode::BAD_REQUEST, format!("bad project id {pid}")));
        }
        let file = crate::speech::project_path(&p.workspace, &pid);
        let text = std::fs::read_to_string(&file).map_err(|_| (StatusCode::NOT_FOUND, t!("speech-no-project", pid = pid.clone())))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{}: {e}", file.display())))?;
        if v.get("kind").and_then(Value::as_str) == Some(sp::KIND) {
            let project: SpeechProject = serde_json::from_value(v).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            return Ok(shown(&p, &project));
        }
        Ok(v)
    })
    .await);
    match res {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err((status, e))) => refuse(status, "no_project", e),
        Err(r) => r,
    }
}

/// PUT /projects/{pid} — проект Speech целиком (окно, undo, агент). Состояние, снятое до чужого сохранения, — 409.
pub async fn put(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || -> Result<Value, String> {
        let mut project: SpeechProject = serde_json::from_value(body).map_err(|e| e.to_string())?;
        crate::speech::load(&p.workspace, &pid)?;
        project.kind = sp::KIND.into();
        crate::speech::save(&p.workspace, &pid, &project)?;
        Ok(shown(&p, &project))
    }))
    .await);
    match res {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) if e.starts_with(crate::app::PROJECT_CHANGED) => refuse(StatusCode::CONFLICT, crate::app::PROJECT_CHANGED, e),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "bad_project", e),
        Err(r) => r,
    }
}

/// DELETE /projects/{pid} — проект в корзину системы (восстанавливается оттуда).
pub async fn delete(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", format!("bad project id {pid}"));
    }
    let dir = st.workspace.join(&pid);
    if !dir.join("project.json").is_file() {
        return refuse(StatusCode::NOT_FOUND, "no_project", t!("speech-no-project", pid = pid));
    }
    match joined(tokio::task::spawn_blocking(move || trash::delete(&dir).map_err(|e| e.to_string())).await) {
        Ok(Ok(())) => {
            let by = crate::app::current_actor();
            crate::app::publish(json!({ "changed": "project", "pid": pid, "deleted": true, "by": by }));
            Json(json!({ "deleted": pid })).into_response()
        }
        Ok(Err(e)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "delete_failed", e),
        Err(r) => r,
    }
}

/// POST /projects/{pid}/restore — the project deleted last under this id taken back from the system's recycle bin.
pub async fn restore(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", format!("bad project id {pid}"));
    }
    let dir = st.workspace.join(&pid);
    if dir.join("project.json").is_file() {
        return refuse(StatusCode::CONFLICT, "project_exists", t!("speech-restore-exists"));
    }
    let res = joined(tokio::task::spawn_blocking(move || restore_from_bin(&dir)).await);
    match res {
        Ok(Ok(())) => {
            let by = crate::app::current_actor();
            crate::app::publish(json!({ "changed": "project", "pid": pid, "restored": true, "by": by }));
            Json(json!({ "restored": pid })).into_response()
        }
        Ok(Err(e)) => refuse(StatusCode::NOT_FOUND, "restore_failed", e),
        Err(r) => r,
    }
}

/// The item of the system's recycle bin deleted last from `dir`, put back where it was.
#[cfg(not(target_os = "macos"))]
fn restore_from_bin(dir: &std::path::Path) -> Result<(), String> {
    let item = trash::os_limited::list()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|i| i.original_path() == dir)
        .max_by_key(|i| i.time_deleted)
        .ok_or_else(|| t!("speech-restore-gone"))?;
    trash::os_limited::restore_all([item]).map_err(|e| e.to_string())
}

/// macOS gives programs no way to list the Bin or put an item back from it; Finder's Put Back does it.
#[cfg(target_os = "macos")]
fn restore_from_bin(_dir: &std::path::Path) -> Result<(), String> {
    Err(t!("speech-restore-finder"))
}

pub async fn synth(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    super::synthesis::enqueue(&paths(&st), &st.jobs, pid, body).await
}

/// GET /projects/{pid}/queue — что ждёт своей очереди озвучки.
pub async fn queue(AxPath(pid): AxPath<String>) -> Json<Value> {
    Json(json!({ "queue": speech_jobs::queue_of(&pid) }))
}

/// GET /projects/{pid}/google — Gemini проекта: идущая пачка, итог последней задачи и история пачек.
pub async fn google_report(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", pid);
    }
    Json(crate::app::google::report(&st.workspace.join(&pid))).into_response()
}

/// DELETE /projects/{pid}/queue[/{n}] — убрать запрос из очереди или очистить её.
pub async fn queue_remove(AxPath((pid, n)): AxPath<(String, u64)>) -> Json<Value> {
    Json(json!({ "removed": speech_jobs::unqueue(&pid, Some(n)) }))
}

pub async fn queue_clear(AxPath(pid): AxPath<String>) -> Json<Value> {
    Json(json!({ "removed": speech_jobs::unqueue(&pid, None) }))
}

/// GET /history[?pid=] — все дубли (проекта или всех проектов) с параметрами, временем и скоростью: новые сверху.
pub async fn history(State(st): State<AppState>, axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>) -> Response {
    let ws = st.workspace.clone();
    let only = q.get("pid").cloned();
    let rows = joined(
        tokio::task::spawn_blocking(move || {
            let mut rows: Vec<Value> = Vec::new();
            let dirs: Vec<String> = match &only {
                Some(pid) => vec![pid.clone()],
                None => std::fs::read_dir(&ws).map(|rd| rd.flatten().filter_map(|e| e.file_name().to_str().map(str::to_string)).filter(|n| sp::is_pid(n)).collect()).unwrap_or_default(),
            };
            for pid in dirs {
                let Ok(project) = crate::speech::load(&ws, &pid) else { continue };
                for b in project.blocks() {
                    for t in &b.takes {
                        rows.push(json!({
                            "pid": pid, "project": project.name, "block": b.id, "speaker": b.speaker,
                            "text": sp::visible_snippet(if t.text.is_empty() { &b.text } else { &t.text }),
                            "n": t.n, "file": t.file, "created": t.created, "duration": t.duration, "synth_secs": t.synth_secs,
                            "rtf": if t.duration > 0.0 { t.synth_secs / t.duration } else { 0.0 }, "first_ms": t.first_ms,
                            "seed": t.seed, "temperature": t.temperature, "top_k": t.top_k, "top_p": t.top_p, "max_tokens": t.max_tokens,
                            "voice": t.voice, "engine": t.engine, "source": t.source, "wer": t.qc.as_ref().map(|c| c.wer), "active": b.active == Some(t.n),
                        }));
                    }
                }
            }
            rows.sort_by(|a, b| b["created"].as_u64().cmp(&a["created"].as_u64()));
            rows
        })
        .await,
    );
    match rows {
        Ok(rows) => Json(json!({ "takes": rows })).into_response(),
        Err(r) => r,
    }
}

/// GET /projects/{pid}/timeline — блоки проекта на общей шкале экспорта с перекрытиями.
pub async fn timeline(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let ws = st.workspace.clone();
    match joined(tokio::task::spawn_blocking(move || crate::speech::load(&ws, &pid).map(|project| speech_jobs::timeline(&project))).await) {
        Ok(Ok(rows)) => Json(json!({ "blocks": rows })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::NOT_FOUND, "no_project", e),
        Err(r) => r,
    }
}

/// GET /projects/{pid}/export-check — что мешает экспорту.
pub async fn quality(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let p = paths(&st);
    match joined(tokio::task::spawn_blocking(move || crate::speech::load(&p.workspace, &pid).map(|project| speech_jobs::quality(&p, &project))).await) {
        Ok(Ok(report)) => Json(report).into_response(),
        Ok(Err(e)) => refuse(StatusCode::NOT_FOUND, "no_project", e),
        Err(r) => r,
    }
}

/// A failed take mended where it went wrong, or the block voiced again when it cannot be.
pub async fn fix(State(st): State<AppState>, AxPath((pid, bid)): AxPath<(String, String)>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        speech_jobs::fix_take(&p, &pid2, &bid, &cb)
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Synth, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

/// A blind comparison: a take of the block by each of two engines (and voices), for the window to play unnamed.
pub async fn ab(State(st): State<AppState>, AxPath((pid, bid)): AxPath<(String, String)>, Json(body): Json<Value>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let sides: Vec<sp::AbSide> = match serde_json::from_value(body.get("sides").cloned().unwrap_or(Value::Null)) {
        Ok(s) => s,
        Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_ab", e.to_string()),
    };
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        speech_jobs::ab_takes(&p, &pid2, &bid, &sides, &cb)
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Synth, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

/// The ready takes checked by recognition now: `ids`, or all not checked yet.
pub async fn check(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let ids: Option<Vec<String>> = body.get("ids").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect());
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        speech_jobs::check_ready(&p, &pid2, ids.as_deref(), &cb)
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Synth, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

pub async fn export_check(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(move || crate::speech::load(&p.workspace, &pid).map(|project| speech_jobs::export_check(&p, &pid, &project))).await);
    match res {
        Ok(Ok(problems)) => Json(json!({ "problems": problems })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::NOT_FOUND, "no_project", e),
        Err(r) => r,
    }
}

/// GET /projects/{pid}/lint — what would go wrong voicing the project, block by block, before voicing it.
pub async fn lint(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(move || crate::speech::load(&p.workspace, &pid).map(|project| speech_jobs::lint(&p, &project))).await);
    match res {
        Ok(Ok(issues)) => Json(json!({ "issues": issues })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::NOT_FOUND, "no_project", e),
        Err(r) => r,
    }
}

/// GET /projects/{pid}/auto-cast — a library voice for each character with no voice in the cast, by its gender, age
/// and the project's language; nothing is changed, the window or the agent puts the voices into the cast.
pub async fn auto_cast(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    let p = paths(&st);
    let res = joined(
        tokio::task::spawn_blocking(move || {
            crate::speech::load(&p.workspace, &pid).map(|project| {
                let (library, _) = crate::voices::list(&p.voices_dir);
                crate::speech::auto_cast::propose(&project, &library, &t!("speech-narrator"))
            })
        })
        .await,
    );
    match res {
        Ok(Ok(picks)) => Json(json!({ "picks": picks.into_iter().map(|(speaker, voice)| json!({ "speaker": speaker, "voice": voice })).collect::<Vec<_>>() })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::NOT_FOUND, "no_project", e),
        Err(r) => r,
    }
}

/// POST /projects/{pid}/blocks/{bid}/ref (multipart: file, text?, whole?) — свой реф реплики: звук в `refs/` проекта
/// (WAV 24 кГц моно), его текст (пустой — распознаётся) и «весь клип в контекст».
pub async fn own_ref(State(st): State<AppState>, AxPath((pid, bid)): AxPath<(String, String)>, mut multipart: axum::extract::Multipart) -> Response {
    if !sp::is_pid(&pid) || !sp::is_pid(&bid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", format!("{pid}/{bid}"));
    }
    let (mut bytes, mut name, mut text, mut whole) = (Vec::new(), String::new(), String::new(), false);
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "file" => {
                name = field.file_name().unwrap_or("ref.wav").to_string();
                match field.bytes().await {
                    Ok(b) => bytes = b.to_vec(),
                    Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e.to_string()),
                }
            }
            "text" => text = field.text().await.unwrap_or_default(),
            "whole" => whole = matches!(field.text().await.unwrap_or_default().as_str(), "1" | "true"),
            _ => {}
        }
    }
    if bytes.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "no_file", t!("speech-own-ref-no-file"));
    }
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || -> Result<Value, String> {
        let project = crate::speech::load(&p.workspace, &pid)?;
        if !project.blocks().any(|b| b.id == bid) {
            return Err(t!("speech-no-block", id = bid.clone()));
        }
        let refs = p.workspace.join(&pid).join("refs");
        std::fs::create_dir_all(&refs).map_err(|e| format!("{}: {e}", refs.display()))?;
        let ext = Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("wav").to_ascii_lowercase();
        let raw = refs.join(format!(".upload-{bid}.{ext}"));
        std::fs::write(&raw, &bytes).map_err(|e| format!("{}: {e}", raw.display()))?;
        let file = format!("{bid}-{}.wav", &blake3::hash(&bytes).to_hex()[..8]);
        let wav = refs.join(&file);
        let made = crate::audio::media::extract_audio(&raw, &wav, crate::speech::jobs::SR, 1);
        let _ = std::fs::remove_file(&raw);
        made?;
        let text = if text.trim().is_empty() { crate::app::recognition::transcribe(&p.repo_root, &p.models_root, &wav, &project.settings.lang, &name)?.text } else { text.trim().to_string() };
        let project = crate::speech::edit(&p.workspace, &pid, |project| {
            for c in project.chapters.iter_mut() {
                for b in c.blocks.iter_mut().filter(|b| b.id == bid) {
                    b.own_ref = Some(sp::OwnRef { file: file.clone(), text: text.clone(), whole });
                }
            }
            project.clone()
        })?;
        Ok(shown(&p, &project))
    }))
    .await);
    match res {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) if e.starts_with(crate::app::PROJECT_CHANGED) => refuse(StatusCode::CONFLICT, crate::app::PROJECT_CHANGED, e),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "bad_ref", e),
        Err(r) => r,
    }
}

/// POST /projects/{pid}/export {format, chapter?, per_block?, part_minutes?, per_chapter?, stems?, allow_duration?, sample_rate?, bits?, kbps?} —
/// файл экспорта задачей.
pub async fn export(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let opts = ExportOpts {
        format: body.get("format").and_then(Value::as_str).unwrap_or("wav").to_string(),
        chapter: body.get("chapter").and_then(Value::as_str).map(str::to_string),
        per_block: body.get("per_block").and_then(Value::as_bool).unwrap_or(false),
        part_minutes: body.get("part_minutes").and_then(Value::as_u64).map(|m| m.min(24 * 60) as u32),
        per_chapter: body.get("per_chapter").and_then(Value::as_bool).unwrap_or(false),
        stems: body.get("stems").and_then(Value::as_bool).unwrap_or(false),
        allow_duration: body.get("allow_duration").and_then(Value::as_bool).unwrap_or(false),
        quality: crate::audio::media::Quality {
            rate: body.get("sample_rate").and_then(Value::as_u64).map(|r| r as u32),
            bits: body.get("bits").and_then(Value::as_u64).map(|b| b as u16),
            kbps: body.get("kbps").and_then(Value::as_u64).map(|k| k as u32),
        },
    };
    if !crate::audio::media::EXPORT_FORMATS.contains(&opts.format.as_str()) {
        return refuse(StatusCode::BAD_REQUEST, "bad_format", t!("media-bad-format", format = opts.format.clone()));
    }
    if let Err(e) = opts.quality.check(&opts.format) {
        return refuse(StatusCode::BAD_REQUEST, "bad_quality", e);
    }
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        speech_jobs::export(&p, &pid2, &opts, &cb)
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Export, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

pub async fn take_file(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>, axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>, req: axum::http::Request<axum::body::Body>) -> Response {
    if !sp::is_pid(&pid) || !safe_file(&file) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", file);
    }
    let path = st.workspace.join(&pid).join("takes").join(&file);
    if !path.is_file() {
        return refuse(StatusCode::NOT_FOUND, "no_file", file);
    }
    let fx = q.get("fx").filter(|f| !f.is_empty()).cloned();
    if let Some(fx) = fx.as_ref().filter(|fx| !crate::voices::fx::KINDS.contains(&fx.as_str())) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", format!("fx {fx}"));
    }
    let number = |key: &str| -> Result<Option<f64>, String> {
        q.get(key).filter(|v| !v.is_empty()).map(|v| v.parse::<f64>().map_err(|_| format!("{key} {v}"))).transpose()
    };
    let parsed = (|| -> Result<(sp::Mix, Option<[f64; 2]>), String> {
        let mix = sp::Mix { gain_db: number("gain")?.unwrap_or(0.0), fade_in: number("fin")?.unwrap_or(0.0), fade_out: number("fout")?.unwrap_or(0.0) };
        let cut = match (number("a")?, number("z")?) {
            (Some(a), Some(z)) if z > a => Some([a, z]),
            (None, None) => None,
            _ => return Err("a and z come together, a before z".into()),
        };
        Ok((mix, cut))
    })();
    let (mix, cut) = match parsed {
        Ok(found) => found,
        Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e),
    };
    if !sp::Mix::GAIN_DB.contains(&mix.gain_db) || mix.fade_in < 0.0 || mix.fade_out < 0.0 {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", format!("gain {} fades {} {}", mix.gain_db, mix.fade_in, mix.fade_out));
    }
    if fx.is_none() && mix.is_flat() {
        return crate::app::serve_file_range(&path, req, None).await;
    }
    let rendered = joined(
        tokio::task::spawn_blocking(move || -> Result<std::path::PathBuf, String> {
            let dir = path.parent().unwrap_or(Path::new(".")).join(".fx");
            let recipe = format!("{fx:?} {mix:?} {cut:?}");
            let out = dir.join(format!("{file}.{}.wav", &blake3::hash(recipe.as_bytes()).to_hex()[..16]));
            let fresh = |o: &Path| std::fs::metadata(o).and_then(|m| m.modified()).ok() >= std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if out.is_file() && fresh(&out) {
                return Ok(out);
            }
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let tmp = tempfile::Builder::new().prefix("take-fx-").tempdir().map_err(|e| e.to_string())?;
            let wav = tmp.path().join("take.wav");
            crate::audio::media::extract_audio(&path, &wav, crate::speech::jobs::SR, 1)?;
            let (samples, sr) = crate::audio::wavio::read_mono_f32(&wav)?;
            let mut done = match &fx {
                Some(fx) => crate::voices::fx::apply(&samples, sr, fx)?,
                None => samples,
            };
            // the fades lie where the project's clip starts and ends in the file, so the window keeps seeking by the cut
            let [a, z] = cut.unwrap_or([0.0, f64::INFINITY]);
            let from = ((a * sr as f64) as usize).min(done.len());
            let to = ((z * sr as f64) as usize).clamp(from, done.len());
            mix.apply(&mut done[from..to], sr);
            let part = out.with_extension("wav.part");
            crate::audio::wavio::write_mono_f32(&part, &done, sr)?;
            std::fs::rename(&part, &out).map_err(|e| format!("{}: {e}", out.display()))?;
            Ok(out)
        })
        .await,
    );
    match rendered {
        Ok(Ok(out)) => crate::app::serve_file_range(&out, req, None).await,
        Ok(Err(e)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "fx_failed", e),
        Err(r) => r,
    }
}

/// Peaks per second of a take's waveform for the track editor.
const PEAKS_PER_SECOND: u32 = 25;

pub async fn take_peaks(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>) -> Response {
    if !sp::is_pid(&pid) || !safe_file(&file) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", file);
    }
    peaks_of(st.workspace.join(&pid).join("takes").join(&file)).await
}

/// GET /projects/{pid}/media/{file}/peaks — a track clip's sound as the track editor draws it, as a take's.
pub async fn media_peaks(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>) -> Response {
    if !sp::is_pid(&pid) || !safe_file(&file) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", file);
    }
    peaks_of(crate::speech::tracks::media_dir(&st.workspace, &pid).join(&file)).await
}

/// GET /projects/{pid}/media/{file} — a track clip's sound file, to listen to.
pub async fn media_file(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>, req: axum::http::Request<axum::body::Body>) -> Response {
    if !sp::is_pid(&pid) || !safe_file(&file) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", file);
    }
    let path = crate::speech::tracks::media_dir(&st.workspace, &pid).join(&file);
    if !path.is_file() {
        return refuse(StatusCode::NOT_FOUND, "no_file", file);
    }
    crate::app::serve_file_range(&path, req, None).await
}

/// A sound's waveform as `PEAKS_PER_SECOND` peaks a second (the loudest sample of each slice, 0..1, not scaled, so
/// sounds compare), kept beside it in `.fx`.
async fn peaks_of(path: std::path::PathBuf) -> Response {
    if !path.is_file() {
        return refuse(StatusCode::NOT_FOUND, "no_file", path.display().to_string());
    }
    let made = joined(
        tokio::task::spawn_blocking(move || -> Result<Value, String> {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let out = path.parent().unwrap_or(Path::new(".")).join(".fx").join(format!("{name}.peaks.json"));
            let fresh = std::fs::metadata(&out).and_then(|m| m.modified()).ok() >= std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if out.is_file() && fresh {
                let text = std::fs::read_to_string(&out).map_err(|e| format!("{}: {e}", out.display()))?;
                return serde_json::from_str(&text).map_err(|e| format!("{}: {e}", out.display()));
            }
            let tmp = tempfile::Builder::new().prefix("peaks-").tempdir().map_err(|e| e.to_string())?;
            let wav = tmp.path().join("sound.wav");
            crate::audio::media::extract_audio(&path, &wav, crate::speech::jobs::SR, 1)?;
            let (samples, sr) = crate::audio::wavio::read_mono_f32(&wav)?;
            let slice = (sr / PEAKS_PER_SECOND).max(1) as usize;
            let peaks: Vec<f32> = samples.chunks(slice).map(|c| (c.iter().fold(0.0f32, |m, v| m.max(v.abs())).min(1.0) * 1000.0).round() / 1000.0).collect();
            let body = json!({ "secs": samples.len() as f64 / sr as f64, "per_second": PEAKS_PER_SECOND, "peaks": peaks });
            std::fs::create_dir_all(out.parent().unwrap_or(Path::new("."))).map_err(|e| format!("{}: {e}", out.display()))?;
            std::fs::write(&out, body.to_string()).map_err(|e| format!("{}: {e}", out.display()))?;
            Ok(body)
        })
        .await,
    );
    match made {
        Ok(Ok(body)) => Json(body).into_response(),
        Ok(Err(e)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "peaks_failed", e),
        Err(r) => r,
    }
}

#[derive(serde::Deserialize)]
pub struct TrackImport {
    paths: Vec<String>,
    #[serde(default)]
    track: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    at: Option<f64>,
}

/// POST /projects/{pid}/tracks/import {paths, track?, name?, at?} — sound files and folders of this computer onto a
/// track: copied into the project's `media/`, one after another from `at` (by default after the track's last clip), on
/// the track `track` or a new one named `name` (by default after the folder or the first file).
pub async fn tracks_import(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<TrackImport>) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", pid);
    }
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || -> Result<Value, String> {
        let given: Vec<std::path::PathBuf> = body.paths.iter().map(std::path::PathBuf::from).collect();
        let files = crate::speech::tracks::expand(&given)?;
        let name = body.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| given.first().and_then(|g| if g.is_dir() { g.file_name() } else { g.file_stem() }).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        place_files(&p, &pid, &files, body.track.as_deref(), &name, body.at)
    }))
    .await);
    project_answer(res)
}

/// POST /projects/{pid}/tracks/upload (multipart: file…, track?, name?, at?) — the window's sound files onto a track,
/// as `tracks/import` lays them.
pub async fn tracks_upload(State(st): State<AppState>, AxPath(pid): AxPath<String>, mut multipart: axum::extract::Multipart) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", pid);
    }
    let tmp = match tempfile::Builder::new().prefix("tracks-upload-").tempdir() {
        Ok(t) => t,
        Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "upload_failed", e.to_string()),
    };
    let (mut files, mut track, mut name, mut at) = (Vec::new(), None, String::new(), None);
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "file" => {
                let original = field.file_name().map(|n| n.rsplit(['/', '\\']).next().unwrap_or(n).to_string()).unwrap_or_else(|| "sound.wav".into());
                let bytes = match field.bytes().await {
                    Ok(b) => b,
                    Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e.to_string()),
                };
                let dir = tmp.path().join(files.len().to_string());
                if let Err(e) = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(dir.join(&original), &bytes)) {
                    return refuse(StatusCode::INTERNAL_SERVER_ERROR, "upload_failed", e.to_string());
                }
                files.push(dir.join(&original));
            }
            "track" => track = Some(field.text().await.unwrap_or_default()).filter(|t| !t.is_empty()),
            "name" => name = field.text().await.unwrap_or_default(),
            "at" => at = field.text().await.ok().and_then(|t| t.parse::<f64>().ok()),
            _ => {}
        }
    }
    if files.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "no_file", t!("tracks-no-files"));
    }
    let p = paths(&st);
    let res = joined(tokio::task::spawn_blocking(crate::app::carry(move || -> Result<Value, String> {
        let _keep = tmp;
        let files = crate::speech::tracks::expand(&files)?;
        let name = if name.trim().is_empty() { files.first().and_then(|f| f.file_stem()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() } else { name };
        place_files(&p, &pid, &files, track.as_deref(), &name, at)
    }))
    .await);
    project_answer(res)
}

/// GET /music/studios — the music studios of this computer, whether each answers now, and the form of its song.
pub async fn music_studios() -> Response {
    match joined(tokio::task::spawn_blocking(crate::integrations::music_studios::list).await) {
        Ok(Ok(list)) => Json(json!({ "studios": list })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "music_failed", e),
        Err(r) => r,
    }
}

#[derive(serde::Deserialize)]
pub struct TrackMusic {
    studio: String,
    #[serde(default)]
    values: serde_json::Map<String, Value>,
    #[serde(default)]
    track: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    at: Option<f64>,
}

/// POST /projects/{pid}/tracks/music {studio, values, track?, name?, at?} — music of the book or play made in a music
/// studio (the form of its song from /music/studios) as a job: the song stays in the studio's library, its audio comes
/// into the project's media as a clip on the track `track` or a new one, at `at` (by default after its last clip).
pub async fn tracks_music(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<TrackMusic>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let Some(studio) = crate::integrations::music_studios::studio(&body.studio) else {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", t!("music-unknown", studio = body.studio.clone()));
    };
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: crate::jobs::JobFn = Box::new(move |progress: crate::jobs::ProgressFn| {
        let step = |msg: String, pct: i64| progress(json!({ "type": "progress", "stage": "music", "msg": msg, "pct": pct }));
        step(t!("music-asking", studio = studio.name), 5);
        let songs = crate::integrations::music_studios::make_song(studio, &body.values, &|said| step(t!("music-making", studio = studio.name, detail = said), 40), &jobs::check_cancelled)?;
        step(t!("music-placing"), 95);
        let files: Vec<std::path::PathBuf> = songs.iter().map(|(audio, _, _)| audio.clone()).collect();
        let mut clips = crate::speech::tracks::clips_of(&p.workspace, &pid2, &files, body.at.unwrap_or(0.0))?;
        for (clip, (_, song, title)) in clips.iter_mut().zip(&songs) {
            clip.label = title.clone();
            clip.source = Some(json!({ "studio": studio.id, "song": song, "values": body.values }));
        }
        let name = body.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| t!("music-track"));
        let track = crate::speech::edit(&p.workspace, &pid2, |project| crate::speech::tracks::place(project, body.track.as_deref(), &name, clips, body.at))??;
        Ok(json!({ "track": track, "songs": songs.iter().map(|(_, song, title)| json!({ "song": song, "title": title })).collect::<Vec<_>>() }))
    });
    match st.jobs.enqueue(crate::jobs::JobMeta::new(crate::jobs::JobKind::Music, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

fn place_files(p: &SpeechPaths, pid: &str, files: &[std::path::PathBuf], track: Option<&str>, name: &str, at: Option<f64>) -> Result<Value, String> {
    crate::speech::load(&p.workspace, pid)?;
    let clips = crate::speech::tracks::clips_of(&p.workspace, pid, files, at.unwrap_or(0.0))?;
    let project = crate::speech::edit(&p.workspace, pid, |project| crate::speech::tracks::place(project, track, name, clips, at).map(|_| project.clone()))??;
    Ok(shown(p, &project))
}

fn project_answer(res: Result<Result<Value, String>, Response>) -> Response {
    match res {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) if e.starts_with(crate::app::PROJECT_CHANGED) => refuse(StatusCode::CONFLICT, crate::app::PROJECT_CHANGED, e),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "bad_tracks", e),
        Err(r) => r,
    }
}

/// POST /projects/{pid}/dictionary/try {phrase, rule, voice?} — правило словаря на слух: фраза как написана и с этим
/// правилом, движком проекта; задача озвучки отвечает файлами и тем, что движок прочтёт.
pub async fn try_rule(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let phrase = body.get("phrase").and_then(Value::as_str).unwrap_or_default().to_string();
    let rule: crate::text::pron_dict::Entry = match body.get("rule").cloned().map(serde_json::from_value::<crate::text::pron_dict::Entry>) {
        Some(Ok(rule)) if !rule.find.trim().is_empty() => rule,
        Some(Ok(_)) => return refuse(StatusCode::BAD_REQUEST, "bad_rule", "the rule finds nothing: find is empty"),
        Some(Err(e)) => return refuse(StatusCode::BAD_REQUEST, "bad_rule", e.to_string()),
        None => return refuse(StatusCode::BAD_REQUEST, "bad_rule", "rule is required"),
    };
    let voice = body.get("voice").and_then(Value::as_str).map(str::to_string);
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        speech_jobs::try_rule(&p, &pid2, &phrase, rule, voice, &cb)
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Synth, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

/// GET /projects/{pid}/tries/{file} — звук проверки правила на слух.
pub async fn try_file(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>, req: axum::http::Request<axum::body::Body>) -> Response {
    if !sp::is_pid(&pid) || !safe_file(&file) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", file);
    }
    let path = st.workspace.join(&pid).join("tries").join(&file);
    if !path.is_file() {
        return refuse(StatusCode::NOT_FOUND, "no_file", file);
    }
    crate::app::serve_file_range(&path, req, None).await
}

/// GET /projects/{pid}/exports/{file} — файл экспорта (скачивание).
pub async fn export_file(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>, req: axum::http::Request<axum::body::Body>) -> Response {
    if !sp::is_pid(&pid) || !safe_file(&file) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", file);
    }
    let path = st.workspace.join(&pid).join("exports").join(&file);
    if !path.is_file() {
        return refuse(StatusCode::NOT_FOUND, "no_file", file);
    }
    crate::app::serve_file_range(&path, req, Some(file)).await
}

/// POST /projects/{pid}/save-export {file, path} — копия файла экспорта туда, куда сказали (агент, «Сохранить как»).
pub async fn save_export(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    let file = body.get("file").and_then(Value::as_str).unwrap_or("").to_string();
    let target = body.get("path").and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
    if !sp::is_pid(&pid) || !safe_file(&file) || target.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", "file and path are needed");
    }
    let src = st.workspace.join(&pid).join("exports").join(&file);
    let mut dst = std::path::PathBuf::from(&target);
    if dst.is_dir() {
        dst = dst.join(&file);
    }
    match std::fs::copy(&src, &dst) {
        Ok(bytes) => Json(json!({ "path": dst.to_string_lossy(), "bytes": bytes })).into_response(),
        Err(e) => refuse(StatusCode::BAD_REQUEST, "save_failed", format!("{} -> {}: {e}", src.display(), dst.display())),
    }
}

/// GET /engines — капабилити-манифест движков: окно строит из него контролы.
pub async fn engines(State(st): State<AppState>) -> Json<Value> {
    Json(json!({ "engines": crate::engines::for_window(&st.models_root) }))
}

pub async fn tags(Query(q): Query<HashMap<String, String>>) -> Response {
    let id = q.get("engine").map(String::as_str).unwrap_or("higgs");
    let Some(engine) = crate::engines::get(id) else {
        return refuse(StatusCode::BAD_REQUEST, "bad_engine", t!("speech-engine-unknown", engine = id.to_string()));
    };
    let cats: Vec<Value> = engine.tags.iter().map(|(c, values)| json!({ "category": c, "values": values })).collect();
    Json(json!({ "engine": engine.id, "format": engine.tag_format, "direction": engine.direction, "direction_presets_only": engine.direction_presets_only, "sentence_tags": engine.sentence_tags, "glued_tags": engine.glued_tags, "categories": cats })).into_response()
}

#[derive(serde::Deserialize)]
pub struct StressReq {
    text: String,
    #[serde(default)]
    lang: String,
}

/// POST /text/stress — ударения и «ё» русского текста (RUAccent): текст с `+` и пословный разбор для подсветки
/// спорных омографов и «ё». Ручные `+` остаются как были.
pub async fn stress(State(st): State<AppState>, Json(req): Json<StressReq>) -> Response {
    if !crate::engines::stress::is_russian(&req.text, &req.lang) {
        return Json(json!({ "text": req.text, "words": [], "russian": false })).into_response();
    }
    if !crate::engines::stress::installed(&st.repo_root) {
        return refuse(StatusCode::CONFLICT, "stress-model-missing", t!("stress-model-missing"));
    }
    let models_root = st.models_root.clone();
    let done = joined(tokio::task::spawn_blocking(move || crate::engines::stress::shared(&models_root).and_then(|s| s.annotate(&req.text))).await);
    match done {
        Ok(Ok(r)) => Json(json!({ "text": r.text, "words": r.words, "russian": true })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "stress-failed", t!("stress-failed", detail = e)),
        Err(resp) => resp,
    }
}

#[derive(serde::Deserialize)]
pub struct DirectSpeechReq {
    text: String,
}

/// POST /text/direct-speech — a paragraph's direct speech apart from the author's words, in order; a text of several
/// paragraphs is cut paragraph by paragraph.
pub async fn direct_speech(Json(req): Json<DirectSpeechReq>) -> Response {
    let parts: Vec<Value> = req
        .text
        .split("\n\n")
        .flat_map(|p| crate::text::direct_speech::split(&p.split_whitespace().collect::<Vec<_>>().join(" ")))
        .map(|(speech, text)| json!({ "speech": speech, "text": text }))
        .collect();
    Json(json!({ "parts": parts, "speaker": t!("speech-lines") })).into_response()
}

/// POST /projects/{pid}/package {audio} — файл проекта для переноса в папку экспорта проекта.
pub async fn package(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", pid);
    }
    let audio = body.get("audio").and_then(Value::as_bool).unwrap_or(true);
    let p = paths(&st);
    match joined(tokio::task::spawn_blocking(move || crate::speech::package::pack(&p, &pid, audio)).await) {
        Ok(Ok((file, bytes))) => Json(json!({ "file": file, "bytes": bytes })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "speechproj-failed", e),
        Err(r) => r,
    }
}

/// POST /speechproj/import (multipart `file`) — новый проект из файла переноса; голоса — в библиотеку.
pub async fn import_package(State(st): State<AppState>, mut multipart: axum::extract::Multipart) -> Response {
    let mut bytes: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.bytes().await {
            Ok(b) => bytes = Some(b.to_vec()),
            Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e.to_string()),
        }
    }
    let Some(bytes) = bytes else { return refuse(StatusCode::BAD_REQUEST, "bad_request", "no file") };
    let p = paths(&st);
    match joined(tokio::task::spawn_blocking(move || crate::speech::package::unpack(&p, &bytes)).await) {
        Ok(Ok((pid, voices))) => Json(json!({ "project_id": pid, "voices_added": voices })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "speechproj-refused", e),
        Err(r) => r,
    }
}

/// POST /projects/from-subtitles (multipart `file`, `name`) — проект по таймкодам из SRT, VTT или ASS.
pub async fn from_subtitles(State(st): State<AppState>, mut multipart: axum::extract::Multipart) -> Response {
    let (mut name, mut file): (String, Option<(String, Vec<u8>)>) = (String::new(), None);
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        let file_name = field.file_name().map(str::to_string);
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e.to_string()),
        };
        if field_name == "name" {
            name = String::from_utf8_lossy(&data).trim().to_string();
        } else {
            file = Some((file_name.unwrap_or_else(|| "subtitles.srt".into()), data.to_vec()));
        }
    }
    let Some((file_name, bytes)) = file else { return refuse(StatusCode::BAD_REQUEST, "bad_request", "no file") };
    let ext = Path::new(&file_name).extension().and_then(|e| e.to_str()).unwrap_or("srt").to_ascii_lowercase();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let cues = crate::text::subimport::parse(&text, &ext);
    if cues.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "speech-subs-empty", t!("speech-subs-empty", name = file_name));
    }
    let title = if name.is_empty() { Path::new(&file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("subtitles").to_string() } else { name };
    let p = paths(&st);
    let done = joined(
        tokio::task::spawn_blocking(move || -> Result<(String, Value), String> {
            let project = sp::from_cues(&title, &cues, &t!("speech-narrator"));
            let pid = sp::new_id();
            crate::speech::save(&p.workspace, &pid, &project)?;
            Ok((pid, shown(&p, &project)))
        })
        .await,
    );
    match done {
        Ok(Ok((pid, project))) => Json(json!({ "project_id": pid, "project": project })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "bad_request", e),
        Err(r) => r,
    }
}

fn import_dir(st: &AppState, id: &str) -> std::path::PathBuf {
    st.workspace.join(".imports").join(id)
}

/// POST /books/preview (multipart `file`) — разобрать книгу и показать структуру до синтеза: главы с длиной,
/// началом текста, оценкой длительности и пометкой «пропустить» (обложка, титул, оглавление).
pub async fn book_preview(State(st): State<AppState>, mut multipart: axum::extract::Multipart) -> Response {
    let mut got: Option<(String, Vec<u8>)> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.file_name().unwrap_or("book.txt").to_string();
        match field.bytes().await {
            Ok(b) => got = Some((name, b.to_vec())),
            Err(e) => return refuse(StatusCode::BAD_REQUEST, "bad_request", e.to_string()),
        }
    }
    let Some((name, bytes)) = got else { return refuse(StatusCode::BAD_REQUEST, "bad_request", "no file") };
    let id = sp::new_id();
    let dir = import_dir(&st, &id);
    let done = joined(
        tokio::task::spawn_blocking(move || -> Result<Value, String> {
            let book = crate::speech::book_import::parse(&name, &bytes)?;
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            if let Some((cover, mime)) = &book.cover {
                let ext = if mime.contains("png") { "png" } else { "jpg" };
                std::fs::write(dir.join(format!("cover.{ext}")), cover).map_err(|e| e.to_string())?;
            }
            std::fs::write(dir.join("book.json"), serde_json::to_vec(&book).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let lang = book.lang.split(['-', '_']).next().unwrap_or("").to_lowercase();
            let cps = dub_core::fit::table_cps(if lang.is_empty() { "ru" } else { &lang });
            let chapters: Vec<Value> = book
                .chapters
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let chars = c.text.chars().count();
                    json!({ "index": i, "title": c.title, "chars": chars, "est_secs": dub_core::fit::estimate(&c.text, cps), "head": c.text.chars().take(240).collect::<String>(), "skip": c.skip })
                })
                .collect();
            let names: Vec<Value> = crate::speech::book_import::names(&book.chapters, 3, 40).into_iter().map(|(name, n)| json!({ "name": name, "count": n })).collect();
            Ok(json!({ "id": id, "title": book.title, "author": book.author, "lang": lang, "cover": book.cover.is_some(), "chapters": chapters, "removed": book.removed, "names": names }))
        })
        .await,
    );
    match done {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "book-refused", e),
        Err(r) => r,
    }
}

/// GET /books/{id}/chapters/{index} — полный текст главы для правки в предпросмотре.
pub async fn book_chapter(State(st): State<AppState>, AxPath((id, index)): AxPath<(String, usize)>) -> Response {
    if !sp::is_pid(&id) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", id);
    }
    let path = import_dir(&st, &id).join("book.json");
    let book: Result<crate::speech::book_import::Book, String> = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()));
    match book.ok().and_then(|b| b.chapters.into_iter().nth(index)) {
        Some(c) => Json(json!({ "title": c.title, "text": c.text })).into_response(),
        None => refuse(StatusCode::NOT_FOUND, "no_chapter", format!("{id}/{index}")),
    }
}

#[derive(serde::Deserialize)]
pub struct BookChoice {
    index: usize,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct FromBook {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    voice: Option<String>,
    chapters: Vec<BookChoice>,
    /// The book's names go into the project's dictionary as rules to check, so their stress can be set.
    #[serde(default)]
    names: Vec<String>,    /// Direct speech (a dialogue dash, quotations) apart from the author's words, under its own speaker.
    #[serde(default)]
    dialogue: bool,
    /// A project to rebuild from the edited book instead of making a new one: unchanged paragraphs keep their blocks.
    #[serde(default)]
    into: Option<String>,
    /// With `into`: only count what the rebuild keeps, adds and drops.
    #[serde(default)]
    dry_run: bool,
}

/// POST /projects/from-book — проект из выбранных глав (с правками окна): глава книги — глава проекта.
pub async fn from_book(State(st): State<AppState>, Json(req): Json<FromBook>) -> Response {
    if !sp::is_pid(&req.id) || req.chapters.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", "id and chapters are needed");
    }
    if req.into.as_deref().is_some_and(|pid| !sp::is_pid(pid)) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", "into is a project id");
    }
    let src = import_dir(&st, &req.id);
    let p = paths(&st);
    let done = joined(
        tokio::task::spawn_blocking(move || -> Result<(String, Value), String> {
            let book: crate::speech::book_import::Book = serde_json::from_slice(&std::fs::read(src.join("book.json")).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let narrator = t!("speech-narrator");
            let lines = t!("speech-lines");
            if let Some(pid) = &req.into {
                let old = crate::speech::load(&p.workspace, pid)?;
                let chapters: Vec<crate::speech::book_rebuild::NewChapter> = req
                    .chapters
                    .iter()
                    .filter_map(|choice| {
                        let c = book.chapters.get(choice.index)?;
                        Some(crate::speech::book_rebuild::NewChapter { title: choice.title.clone().unwrap_or_else(|| c.title.clone()), text: choice.text.clone().unwrap_or_else(|| c.text.clone()) })
                    })
                    .collect();
                let (mut project, stats) = crate::speech::book_rebuild::rebuild(&old, &chapters, &narrator, req.dialogue.then_some(lines.as_str()));
                if req.dry_run {
                    return Ok((pid.clone(), json!({ "stats": stats })));
                }
                let backups = p.workspace.join(pid).join("backups");
                std::fs::create_dir_all(&backups).map_err(|e| format!("{}: {e}", backups.display()))?;
                let backup = backups.join(format!("project-{}.json", sp::now()));
                std::fs::write(&backup, serde_json::to_vec_pretty(&old).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", backup.display()))?;
                for name in req.names.iter().map(|n| n.trim()).filter(|n| !n.is_empty()) {
                    if !project.dictionary.iter().any(|e| e.find == name) {
                        project.dictionary.push(crate::text::pron_dict::Entry { id: crate::text::pron_dict::new_id(), find: name.to_string(), say: name.to_string(), note: t!("book-name-note"), source: "book".into(), ..Default::default() });
                    }
                }
                crate::speech::save(&p.workspace, pid, &project)?;
                let _ = std::fs::remove_dir_all(&src);
                return Ok((pid.clone(), json!({ "stats": stats, "backup": backup.to_string_lossy(), "project": shown(&p, &project) })));
            }
            let mut project = sp::SpeechProject { name: req.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| book.title.clone()), created: sp::now(), ..Default::default() };
            project.settings.voice = req.voice.clone();
            let lang = book.lang.split(['-', '_']).next().unwrap_or("").to_lowercase();
            if crate::engines::get(&project.settings.engine).is_some_and(|e| e.languages.contains(&lang.as_str())) {
                project.settings.lang = lang;
            }
            for choice in &req.chapters {
                let Some(c) = book.chapters.get(choice.index) else { continue };
                let text = choice.text.clone().unwrap_or_else(|| c.text.clone());
                let blocks = sp::split_text_lines(&text, &narrator, req.dialogue.then_some(lines.as_str()))
                    .into_iter()
                    .map(|(speaker, text)| sp::Block { id: sp::new_id(), speaker, text, ..Default::default() })
                    .collect();
                project.chapters.push(sp::Chapter { id: sp::new_id(), title: choice.title.clone().unwrap_or_else(|| c.title.clone()), blocks });
            }
            let pid = sp::new_id();
            let cover = ["cover.jpg", "cover.png"].into_iter().find(|f| src.join(f).is_file());
            if let Some(f) = cover {
                let dst = p.workspace.join(&pid);
                std::fs::create_dir_all(&dst).map_err(|e| e.to_string())?;
                std::fs::copy(src.join(f), dst.join(f)).map_err(|e| e.to_string())?;
            }
            project.book = Some(sp::BookMeta { title: book.title.clone(), author: book.author.clone(), cover: cover.map(str::to_string) });
            for name in req.names.iter().map(|n| n.trim()).filter(|n| !n.is_empty()) {
                project.dictionary.push(crate::text::pron_dict::Entry { id: crate::text::pron_dict::new_id(), find: name.to_string(), say: name.to_string(), note: t!("book-name-note"), source: "book".into(), ..Default::default() });
            }
            crate::speech::save(&p.workspace, &pid, &project)?;
            let _ = std::fs::remove_dir_all(&src);
            Ok((pid, shown(&p, &project)))
        })
        .await,
    );
    match done {
        Ok(Ok((pid, rebuilt))) if rebuilt.get("stats").is_some() => {
            let mut answer = json!({ "project_id": pid });
            if let (Some(a), Some(b)) = (answer.as_object_mut(), rebuilt.as_object()) {
                a.extend(b.clone());
            }
            Json(answer).into_response()
        }
        Ok(Ok((pid, project))) => Json(json!({ "project_id": pid, "project": project })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "book-refused", e),
        Err(r) => r,
    }
}

fn export_path(st: &AppState, pid: &str, file: &str) -> Option<std::path::PathBuf> {
    (sp::is_pid(pid) && safe_file(file)).then(|| st.workspace.join(pid).join("exports").join(file)).filter(|p| p.is_file())
}

/// GET /projects/{pid}/exports — файлы экспорта проекта: имя, размер, время; новые сверху.
pub async fn exports_list(State(st): State<AppState>, AxPath(pid): AxPath<String>) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", pid);
    }
    let dir = st.workspace.join(&pid).join("exports");
    let mut files: Vec<Value> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_file())
                .filter_map(|e| {
                    let meta = e.metadata().ok()?;
                    let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
                    Some(json!({ "file": e.file_name().to_string_lossy(), "bytes": meta.len(), "mtime": mtime }))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by(|a, b| b["mtime"].as_u64().cmp(&a["mtime"].as_u64()));
    Json(json!({ "files": files, "dir": dir.to_string_lossy() })).into_response()
}

/// GET /renders — the audio files built for every Speech project, newest first, with whether a recipe is beside each.
pub async fn renders(State(st): State<AppState>) -> Response {
    let workspace = st.workspace.clone();
    let res = joined(tokio::task::spawn_blocking(move || {
        let mut out: Vec<Value> = Vec::new();
        for entry in std::fs::read_dir(&workspace).into_iter().flatten().flatten() {
            let pid = entry.file_name().to_string_lossy().into_owned();
            let exports = entry.path().join("exports");
            let Ok(listing) = std::fs::read_dir(&exports) else { continue };
            // only the project's kind and name are needed: a book's whole project is not parsed for them
            #[derive(serde::Deserialize)]
            struct Head {
                kind: String,
                #[serde(default)]
                name: String,
            }
            let Some(project) = std::fs::read(crate::speech::project_path(&workspace, &pid)).ok().and_then(|b| serde_json::from_slice::<Head>(&b).ok()).filter(|h| h.kind == sp::KIND) else { continue };
            for file in listing.flatten() {
                let path = file.path();
                let Some(ext) = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase) else { continue };
                if !path.is_file() || !crate::audio::media::EXPORT_FORMATS.contains(&ext.as_str()) {
                    continue;
                }
                let Ok(meta) = file.metadata() else { continue };
                let mtime = meta.modified().ok().and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
                let name = file.file_name().to_string_lossy().into_owned();
                let recipe = path.with_file_name(format!("{name}.recipe.json"));
                out.push(json!({
                    "pid": pid, "project": project.name, "file": name, "bytes": meta.len(), "mtime": mtime,
                    "recipe": recipe.is_file().then(|| recipe.file_name().map(|n| n.to_string_lossy().into_owned())).flatten(),
                }));
            }
        }
        out.sort_by_key(|v| std::cmp::Reverse(v["mtime"].as_u64().unwrap_or(0)));
        out
    })
    .await);
    match res {
        Ok(v) => Json(json!({ "renders": v })).into_response(),
        Err(r) => r,
    }
}

/// POST /projects/{pid}/exports/{file}/reveal — показать файл в проводнике.
pub async fn export_reveal(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>) -> Response {
    match export_path(&st, &pid, &file) {
        Some(p) => {
            crate::app::reveal_in_explorer(p.to_string_lossy().into_owned());
            Json(json!({ "ok": true })).into_response()
        }
        None => refuse(StatusCode::NOT_FOUND, "no_file", file),
    }
}

/// POST /projects/{pid}/exports/{file}/save-as — системный диалог «Сохранить как» и копия туда; отмена — `{path: null}`.
pub async fn export_save_as(State(st): State<AppState>, AxPath((pid, file)): AxPath<(String, String)>) -> Response {
    let Some(src) = export_path(&st, &pid, &file) else { return refuse(StatusCode::NOT_FOUND, "no_file", file) };
    let ext = Path::new(&file).extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
    let name = file.clone();
    let picked = tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new().set_title(t!("export-save-as")).set_file_name(&name);
        if !ext.is_empty() {
            dialog = dialog.add_filter(ext.to_uppercase(), &[ext.as_str()]);
        }
        dialog.save_file()
    })
    .await
    .ok()
    .flatten();
    let Some(dst) = picked else { return Json(json!({ "path": null })).into_response() };
    match std::fs::copy(&src, &dst) {
        Ok(bytes) => Json(json!({ "path": dst.to_string_lossy(), "bytes": bytes })).into_response(),
        Err(e) => refuse(StatusCode::BAD_REQUEST, "save_failed", format!("{} -> {}: {e}", src.display(), dst.display())),
    }
}

/// POST /projects/{pid}/exports-to-folder {files, folder?} — копии файлов экспорта в папку (без `folder` — выбрать
/// системным диалогом); `{folder: null}` — выбор отменён.
pub async fn exports_to_folder(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(body): Json<Value>) -> Response {
    if !sp::is_pid(&pid) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", pid);
    }
    let files: Vec<String> = body.get("files").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
    let folder = match body.get("folder").and_then(Value::as_str) {
        Some(f) => Some(std::path::PathBuf::from(f)),
        None => tokio::task::spawn_blocking(|| rfd::FileDialog::new().set_title(t!("export-pick-folder")).pick_folder()).await.ok().flatten(),
    };
    let Some(folder) = folder else { return Json(json!({ "folder": null })).into_response() };
    let src = st.workspace.join(&pid).join("exports");
    let mut copied = 0;
    for f in &files {
        let rel = Path::new(f);
        if rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            return refuse(StatusCode::BAD_REQUEST, "bad_request", f.clone());
        }
        let name = rel.file_name().map(|n| n.to_os_string()).unwrap_or_default();
        if let Err(e) = std::fs::copy(src.join(rel), folder.join(&name)) {
            return refuse(StatusCode::BAD_REQUEST, "save_failed", format!("{f}: {e}"));
        }
        copied += 1;
    }
    Json(json!({ "folder": folder.to_string_lossy(), "copied": copied })).into_response()
}
