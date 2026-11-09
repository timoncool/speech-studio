//! Маршруты библиотеки голосов: список с карточками, фото, транскрипт рефа, импорт файлов и архивов, экспорт,
//! очистка голоса и распознавание транскрипта задачами очереди.

use std::io::{Read, Write};
use std::path::Path;

use axum::extract::{Multipart, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::voices::{jobs as voice_jobs, library as lib};
use crate::voices::jobs::VoicePaths;
use crate::{jobs, AppState};

fn refuse(status: StatusCode, code: &str, detail: impl Into<String>) -> Response {
    (status, Json(json!({ "error": code, "detail": detail.into() }))).into_response()
}

fn bad(detail: impl Into<String>) -> Response {
    refuse(StatusCode::BAD_REQUEST, "bad_request", detail)
}

fn missing(name: &str) -> Response {
    refuse(StatusCode::NOT_FOUND, "voice_missing", t!("voice-missing", name = name.to_string()))
}

fn joined<T>(r: Result<T, tokio::task::JoinError>) -> Result<T, Response> {
    r.map_err(|e| refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()))
}

fn paths(st: &AppState) -> VoicePaths {
    VoicePaths { repo_root: st.repo_root.clone(), models_root: st.models_root.clone(), voices_dir: st.voices_dir.clone() }
}

fn body_name(body: &Value, key: &str) -> String {
    lib::sanitize(body.get(key).and_then(Value::as_str).unwrap_or(""))
}

pub fn listing(dir: &Path) -> Value {
    let (voices, broken) = lib::list(dir);
    json!({
        "voices": voices,
        "broken": broken.into_iter().map(|(name, error)| json!({ "name": name, "error": error })).collect::<Vec<_>>(),
    })
}

/// GET /voices — голоса с карточками; голос с испорченной карточкой — в `broken` с причиной. `?brief=true` — только
/// имя и kind, lang, gender карточки: расшифровки рефов раздувают ответ на сотни голосов.
pub async fn list(State(st): State<AppState>, Query(q): Query<std::collections::HashMap<String, String>>) -> Response {
    let dir = st.voices_dir.clone();
    let brief = q.get("brief").is_some_and(|v| v == "true" || v == "1");
    match joined(tokio::task::spawn_blocking(move || listing(&dir)).await) {
        Ok(v) if brief => Json(json!({
            "voices": v["voices"].as_array().into_iter().flatten().map(|voice| json!({
                "name": voice["name"],
                "card": { "kind": voice["card"]["kind"], "lang": voice["card"]["lang"], "gender": voice["card"]["gender"] },
            })).collect::<Vec<_>>(),
            "broken": v["broken"],
        }))
        .into_response(),
        Ok(v) => Json(v).into_response(),
        Err(r) => r,
    }
}

/// GET /voices/sample?name= — звук голоса с Range для `<audio>`.
pub async fn sample(
    State(st): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let name = lib::sanitize(q.get("name").map(String::as_str).unwrap_or(""));
    let original = q.get("original").is_some_and(|v| v == "1");
    let path = if original { lib::original_path(&st.voices_dir, &name) } else { lib::audio_path(&st.voices_dir, &name) };
    match path {
        Some(p) => crate::app::serve_file_range(&p, req, None).await,
        None => missing(&name),
    }
}

/// POST /voices/design {description, sample, lang?, count?} — кандидаты голоса по описанию задачей (VoiceDesign).
pub async fn design(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let text = |k: &str| body.get(k).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let (description, sample, lang) = (text("description"), text("sample"), text("lang"));
    if description.is_empty() || sample.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "design_empty", t!("voice-design-empty"));
    }
    let count = body.get("count").and_then(Value::as_u64).unwrap_or(3).clamp(1, voice_jobs::DESIGN_MAX as u64) as u32;
    let p = paths(&st);
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        voice_jobs::design(&p, &description, &sample, if lang.is_empty() { "auto" } else { &lang }, count, &cb)
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::VoiceDesign, Some("voice:design")), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

/// GET /voices/design/sample?id=&k= — звук кандидата дизайна.
pub async fn design_sample(
    State(st): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let id = q.get("id").map(String::as_str).unwrap_or("");
    let k = q.get("k").and_then(|v| v.parse::<u32>().ok());
    match (id.chars().all(|c| c.is_ascii_hexdigit()) && !id.is_empty(), k) {
        (true, Some(k)) => {
            let file = voice_jobs::design_dir(&st.voices_dir, id).join(format!("{k}.wav"));
            if file.is_file() { crate::app::serve_file_range(&file, req, None).await } else { refuse(StatusCode::NOT_FOUND, "design_gone", t!("voice-design-gone")) }
        }
        _ => refuse(StatusCode::NOT_FOUND, "design_gone", t!("voice-design-gone")),
    }
}

/// POST /voices/design/keep {id, k, name, description, sample, lang?} — кандидат становится голосом библиотеки.
pub async fn design_keep(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let text = |k: &str| body.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let k = body.get("k").and_then(Value::as_u64).unwrap_or(0) as u32;
    let p = paths(&st);
    let dir = st.voices_dir.clone();
    let (id, name, description, sample, lang) = (text("id"), text("name"), text("description"), text("sample"), text("lang"));
    let name = if name.trim().is_empty() { t!("voice-design-name") } else { name };
    match joined(tokio::task::spawn_blocking(move || voice_jobs::design_keep(&p, &id, k, &name, &description, &sample, if lang.is_empty() { "auto" } else { &lang }).and_then(|made| lib::get(&dir, &made))).await) {
        Ok(Ok(v)) => {
            crate::app::publish(json!({ "changed": "voices" }));
            Json(json!({ "voice": v })).into_response()
        }
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// GET /voices/photo?name= — фото персоны.
pub async fn photo(
    State(st): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let name = lib::sanitize(q.get("name").map(String::as_str).unwrap_or(""));
    match lib::photo_path(&st.voices_dir, &name) {
        Some(p) => crate::app::serve_file_range(&p, req, None).await,
        None => refuse(StatusCode::NOT_FOUND, "no_photo", name),
    }
}

/// POST /voices/photo (multipart: name, file) — поставить фото.
pub async fn set_photo(State(st): State<AppState>, mut multipart: Multipart) -> Response {
    let mut name = String::new();
    let mut file: Option<(Vec<u8>, String)> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        let fname = field.file_name().map(str::to_string);
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => return bad(e.to_string()),
        };
        match field_name.as_str() {
            "name" => name = lib::sanitize(&String::from_utf8_lossy(&data)),
            _ => {
                let ext = fname.as_deref().and_then(|f| Path::new(f).extension()).and_then(|e| e.to_str()).unwrap_or("").to_string();
                file = Some((data.to_vec(), ext));
            }
        }
    }
    let Some((bytes, ext)) = file else { return bad("no photo file") };
    let dir = st.voices_dir.clone();
    let n = name.clone();
    match joined(tokio::task::spawn_blocking(move || lib::set_photo(&dir, &n, &bytes, &ext).and_then(|_| lib::get(&dir, &n))).await) {
        Ok(Ok(v)) => {
            crate::app::publish(json!({ "changed": "voices" }));
            Json(json!({ "voice": v })).into_response()
        }
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// POST /voices/card {name, card: {...}} — поменять карточку персоны.
pub async fn set_card(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body_name(&body, "name");
    let patch = body.get("card").cloned().unwrap_or(Value::Null);
    let dir = st.voices_dir.clone();
    let res = joined(tokio::task::spawn_blocking(move || -> Result<lib::Voice, String> {
        lib::audio_path(&dir, &name).ok_or_else(|| t!("voice-missing", name = name.clone()))?;
        let mut card = lib::read_card(&dir, &name)?;
        lib::apply_patch(&mut card, &patch)?;
        lib::check_variant(&dir, &name, &card)?;
        lib::write_card(&dir, &name, &card)?;
        lib::get(&dir, &name)
    })
    .await);
    match res {
        Ok(Ok(v)) => Json(json!({ "voice": v })).into_response(),
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// POST /voices/transcript {name, text} — транскрипт рефа рукой: пустой текст убирает его.
pub async fn set_transcript(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body_name(&body, "name");
    let text = body.get("text").and_then(Value::as_str).unwrap_or("").to_string();
    let dir = st.voices_dir.clone();
    let res = joined(tokio::task::spawn_blocking(move || -> Result<lib::Voice, String> {
        lib::audio_path(&dir, &name).ok_or_else(|| t!("voice-missing", name = name.clone()))?;
        lib::write_transcript(&dir, &name, &text)?;
        let mut card = lib::read_card(&dir, &name)?;
        card.transcript_auto = false;
        lib::write_card(&dir, &name, &card)?;
        lib::get(&dir, &name)
    })
    .await);
    match res {
        Ok(Ok(v)) => Json(json!({ "voice": v })).into_response(),
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

pub(crate) async fn enqueue(p: VoicePaths, queue: &jobs::JobQueue, kind: jobs::JobKind, name: String, then_transcribe: bool) -> Response {
    if lib::audio_path(&p.voices_dir, &name).is_none() {
        return missing(&name);
    }
    if kind == jobs::JobKind::VoiceClean && !voice_jobs::clean_ready(&p) {
        return refuse(StatusCode::CONFLICT, "clean_missing", t!("voice-clean-missing"));
    }
    let key = format!("voice:{name}");
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        let res = match kind {
            jobs::JobKind::VoiceClean => voice_jobs::clean(&p, &name, &cb).and_then(|cleaned| {
                if then_transcribe { voice_jobs::transcribe(&p, &name, &cb).map(|t| json!({ "cleaned": cleaned, "transcript": t })) } else { Ok(cleaned) }
            }),
            _ => voice_jobs::transcribe(&p, &name, &cb),
        };
        crate::app::publish(json!({ "changed": "voices" }));
        res
    });
    match queue.enqueue(jobs::JobMeta::new(kind, Some(&key)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

/// POST /voices/transcribe {name} — распознать транскрипт рефа задачей.
pub async fn transcribe(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    enqueue(paths(&st), &st.jobs, jobs::JobKind::VoiceTranscript, body_name(&body, "name"), false).await
}

/// POST /voices/clean {name} — очистить голос от музыки и шума задачей. Нет Roformer — 409 clean_missing.
pub async fn clean(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    enqueue(paths(&st), &st.jobs, jobs::JobKind::VoiceClean, body_name(&body, "name"), false).await
}

/// POST /voices/denoise {name, db?} — убрать шум из голоса задачей (DPDFNet); `db` — предел подавления, 0 или нет — без
/// предела.
pub async fn denoise(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let p = paths(&st);
    let name = body_name(&body, "name");
    if lib::audio_path(&p.voices_dir, &name).is_none() {
        return missing(&name);
    }
    let limit = body.get("db").and_then(Value::as_f64).filter(|d| *d > 0.0).map(|d| d.min(100.0) as f32);
    let key = format!("voice:{name}");
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        let res = voice_jobs::denoise(&p, &name, limit, &cb);
        crate::app::publish(json!({ "changed": "voices" }));
        res
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::VoiceClean, Some(&key)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

/// POST /voices/level {name, measure?} — the voice's reference loudness: `measure` only measures it (and keeps it on the
/// card), otherwise the reference is brought to −20 LUFS as a voice added from a file, its sound before kept as the
/// original («Restore the original» brings it back). Answers `lufs` and whether it is `quiet` (under −26 LUFS).
pub async fn level(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let dir = st.voices_dir.clone();
    let name = body_name(&body, "name");
    if lib::audio_path(&dir, &name).is_none() {
        return missing(&name);
    }
    let measure = body.get("measure").and_then(Value::as_bool).unwrap_or(false);
    let n = name.clone();
    match joined(tokio::task::spawn_blocking(move || if measure { lib::measure_level(&dir, &n) } else { lib::level_voice(&dir, &n) }).await) {
        Ok(Ok(lufs)) => {
            if !measure {
                crate::app::publish(json!({ "changed": "voices" }));
            }
            Json(json!({ "name": name, "lufs": lufs, "quiet": lufs < lib::QUIET_LUFS })).into_response()
        }
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// POST /voices/restore {name} — вернуть звук до очистки.
pub async fn restore(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body_name(&body, "name");
    let dir = st.voices_dir.clone();
    match joined(tokio::task::spawn_blocking(move || lib::restore_original(&dir, &name).and_then(|_| lib::get(&dir, &name))).await) {
        Ok(Ok(v)) => Json(json!({ "voice": v })).into_response(),
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// POST /voices/rename {from, to}: the projects that speak with the voice follow its new name.
pub async fn rename(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let (from, to) = (body_name(&body, "from"), body_name(&body, "to"));
    let dir = st.voices_dir.clone();
    let app = st.clone();
    match joined(tokio::task::spawn_blocking(move || crate::app::rename_voice(&app, &from, &to).map(|_| listing(&dir))).await) {
        Ok(Ok(v)) => {
            crate::app::publish(json!({ "changed": "voices" }));
            Json(v).into_response()
        }
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// POST /voices/delete {name}.
pub async fn delete(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body_name(&body, "name");
    let dir = st.voices_dir.clone();
    match joined(tokio::task::spawn_blocking(move || lib::delete(&dir, &name).map(|_| listing(&dir))).await) {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// POST /voices/import (multipart: file…, name?, clean?, transcribe?) — голоса из звуковых файлов. Каждый файл
/// становится голосом с громкостью рефа; `clean=1` и `transcribe=1` ставят задачи на каждый новый голос.
pub async fn import(State(st): State<AppState>, mut multipart: Multipart) -> Response {
    let mut base = String::new();
    let (mut want_clean, mut want_transcribe) = (crate::engines::models::voice_clean_default(&st.models_root), true);
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        let fname = field.file_name().map(str::to_string);
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => return bad(e.to_string()),
        };
        match field_name.as_str() {
            "name" => base = lib::sanitize(&String::from_utf8_lossy(&data)),
            "clean" => want_clean = data.as_ref() == b"1",
            "transcribe" => want_transcribe = data.as_ref() == b"1",
            _ => files.push((fname.unwrap_or_default(), data.to_vec())),
        }
    }
    if files.is_empty() {
        return bad("no audio file");
    }
    let p = paths(&st);
    if want_clean && !voice_jobs::clean_ready(&p) {
        return refuse(StatusCode::CONFLICT, "clean_missing", t!("voice-clean-missing"));
    }
    let dir = st.voices_dir.clone();
    let single = files.len() == 1;
    let res = joined(tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        for (fname, data) in files {
            let path = Path::new(&fname);
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if !lib::AUDIO_EXTS.contains(&ext.as_str()) {
                return Err(t!("voice-bad-audio", name = fname.clone()));
            }
            let stem = lib::sanitize(path.file_stem().and_then(|s| s.to_str()).unwrap_or(""));
            let name = if single && !base.is_empty() { base.clone() } else { stem };
            let work = tempfile::Builder::new().prefix("import-").tempdir().map_err(|e| e.to_string())?;
            let src = work.path().join(format!("in.{ext}"));
            std::fs::write(&src, &data).map_err(|e| format!("{}: {e}", src.display()))?;
            let card = lib::VoiceCard { source: "file".into(), ..lib::guess_card(&name) };
            names.push(lib::import_audio(&dir, &name, &src, card)?);
        }
        Ok(names)
    })
    .await);
    let names = match res {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return bad(e),
        Err(r) => return r,
    };
    crate::app::publish(json!({ "changed": "voices" }));
    let mut jobs_started = Vec::new();
    for n in &names {
        if want_clean {
            jobs_started.push(job_id(enqueue(paths(&st), &st.jobs, jobs::JobKind::VoiceClean, n.clone(), want_transcribe).await).await);
        } else if want_transcribe {
            jobs_started.push(job_id(enqueue(paths(&st), &st.jobs, jobs::JobKind::VoiceTranscript, n.clone(), false).await).await);
        }
    }
    Json(json!({ "names": names, "jobs": jobs_started })).into_response()
}

async fn job_id(r: Response) -> Value {
    let status = r.status();
    let bytes = axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap_or_default();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if status.is_success() { json!({ "job_id": v["job_id"] }) } else { json!({ "error": v }) }
}

/// Архив выбранных голосов со всеми их файлами (звук, транскрипт, карточка, фото).
pub fn export_zip(dir: &Path, names: &[String]) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for name in names {
            lib::audio_path(dir, name).ok_or_else(|| t!("voice-missing", name = name.clone()))?;
            let exts: Vec<&str> = lib::AUDIO_EXTS.iter().chain(lib::PHOTO_EXTS.iter()).chain(["txt", "json", "words.json"].iter()).copied().collect();
            for ext in exts {
                let p = dir.join(format!("{name}.{ext}"));
                if !p.is_file() {
                    continue;
                }
                let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                zip.start_file(format!("{name}.{ext}"), opts).map_err(|e| e.to_string())?;
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

/// Голоса из архива экспорта: имя, занятое в библиотеке, получает номер; повторный импорт того же голоса
/// (тот же звук побайтно) распознаётся и пропускается.
pub fn import_zip(dir: &Path, bytes: &[u8]) -> Result<(Vec<String>, Vec<String>), String> {
    let (added, same) = import_zip_mapped(dir, bytes)?;
    Ok((added.into_iter().map(|(_, to)| to).collect(), same))
}

/// То же, что `import_zip`, с исходным именем каждого добавленного голоса: `(как звался в архиве, как назван здесь)`.
pub fn import_zip_mapped(dir: &Path, bytes: &[u8]) -> Result<(Vec<(String, String)>, Vec<String>), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut groups: std::collections::BTreeMap<String, Vec<(String, Vec<u8>)>> = Default::default();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let entry = Path::new(f.name()).file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let Some((stem, ext)) = entry.strip_suffix(".words.json").map(|stem| (stem, "words.json")).or_else(|| entry.rsplit_once('.')) else { continue };
        let name = lib::sanitize(stem);
        let ext = ext.to_ascii_lowercase();
        let known = lib::AUDIO_EXTS.contains(&ext.as_str()) || lib::PHOTO_EXTS.contains(&ext.as_str()) || ext == "txt" || ext == "json" || ext == "words.json";
        if name.is_empty() || !known {
            continue;
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data).map_err(|e| e.to_string())?;
        groups.entry(name).or_default().push((ext, data));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let (mut added, mut same) = (Vec::new(), Vec::new());
    for (name, files) in groups {
        let Some((audio_ext, audio)) = files.iter().find(|(e, _)| lib::AUDIO_EXTS.contains(&e.as_str())) else { continue };
        if let Some(existing) = lib::audio_path(dir, &name) {
            if std::fs::read(&existing).ok().as_deref() == Some(audio.as_slice()) {
                same.push(name);
                continue;
            }
        }
        let target = lib::free_name(dir, &name);
        std::fs::write(dir.join(format!("{target}.{audio_ext}")), audio).map_err(|e| format!("{target}: {e}"))?;
        for (ext, data) in files.iter().filter(|(e, _)| !lib::AUDIO_EXTS.contains(&e.as_str())) {
            if ext == "json" {
                let card: lib::VoiceCard = serde_json::from_slice(data).map_err(|e| format!("{name}.json: {e}"))?;
                lib::write_card(dir, &target, &card)?;
            } else {
                std::fs::write(dir.join(format!("{target}.{ext}")), data).map_err(|e| format!("{target}: {e}"))?;
            }
        }
        added.push((name, target));
    }
    Ok((added, same))
}

/// POST /voices/export {names, path?} — ZIP выбранных голосов: ответом или файлом по пути (для агента).
pub async fn export(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let names: Vec<String> = body.get("names").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(lib::sanitize).collect()).unwrap_or_default();
    if names.is_empty() {
        return bad("no voices chosen");
    }
    let target = body.get("path").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()).map(std::path::PathBuf::from);
    let dir = st.voices_dir.clone();
    let made = joined(tokio::task::spawn_blocking(move || export_zip(&dir, &names)).await);
    if let (Some(target), Ok(Ok(bytes))) = (target, &made) {
        return match std::fs::write(&target, bytes) {
            Ok(()) => Json(json!({ "path": target.to_string_lossy(), "bytes": bytes.len() })).into_response(),
            Err(e) => bad(format!("{}: {e}", target.display())),
        };
    }
    match made {
        Ok(Ok(bytes)) => (
            [
                (axum::http::header::CONTENT_TYPE, "application/zip".to_string()),
                (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"voices.zip\"".to_string()),
            ],
            bytes,
        )
            .into_response(),
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

/// POST /voices/import-zip (multipart: file) — голоса из архива экспорта.
pub async fn import_zip_route(State(st): State<AppState>, mut multipart: Multipart) -> Response {
    let mut data = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.bytes().await {
            Ok(b) => data = Some(b.to_vec()),
            Err(e) => return bad(e.to_string()),
        }
    }
    let Some(bytes) = data else { return bad("no archive") };
    let dir = st.voices_dir.clone();
    match joined(tokio::task::spawn_blocking(move || import_zip(&dir, &bytes)).await) {
        Ok(Ok((added, same))) => {
            crate::app::publish(json!({ "changed": "voices" }));
            Json(json!({ "added": added, "already": same })).into_response()
        }
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_exported_voice_comes_back_whole_and_a_second_import_is_recognised() {
        let a = tempfile::tempdir().unwrap();
        std::fs::write(a.path().join("Анна.wav"), b"RIFFaudio").unwrap();
        std::fs::write(a.path().join("Анна.txt"), "привет").unwrap();
        std::fs::write(a.path().join("Анна.jpg"), b"jpg").unwrap();
        lib::write_card(a.path(), "Анна", &lib::VoiceCard { favorite: true, gender: "female".into(), ..Default::default() }).unwrap();
        let zip = export_zip(a.path(), &["Анна".to_string()]).unwrap();

        let b = tempfile::tempdir().unwrap();
        let (added, same) = import_zip(b.path(), &zip).unwrap();
        assert_eq!((added, same.len()), (vec!["Анна".to_string()], 0));
        let v = lib::get(b.path(), "Анна").unwrap();
        assert!(v.card.favorite && v.transcript.as_deref() == Some("привет") && v.photo.as_deref() == Some("jpg"));

        let (added, same) = import_zip(b.path(), &zip).unwrap();
        assert!(added.is_empty() && same == ["Анна"]);

        std::fs::write(b.path().join("Анна.wav"), b"RIFFother").unwrap();
        let (added, _) = import_zip(b.path(), &zip).unwrap();
        assert_eq!(added, ["Анна 2"], "a different voice under a taken name gets a number");
    }

    #[test]
    fn exporting_a_voice_that_is_not_there_fails() {
        let a = tempfile::tempdir().unwrap();
        assert!(export_zip(a.path(), &["нет".to_string()]).is_err());
    }
}

/// POST /voices/ref-candidates {name} — куски рефа по словам распознавания с длиной, паузой, шумом и клиппингом.
pub async fn ref_candidates(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body_name(&body, "name");
    let p = paths(&st);
    match joined(tokio::task::spawn_blocking(move || voice_jobs::ref_candidates(&p, &name)).await) {
        Ok(Ok(v)) => Json(v).into_response(),
        Ok(Err(e)) => refuse(StatusCode::CONFLICT, "no_candidates", e),
        Err(r) => r,
    }
}

/// POST /voices/ref-apply {name, start, end} — оставить от рефа этот кусок (исходник сохраняется).
pub async fn ref_apply(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body_name(&body, "name");
    let (Some(start), Some(end)) = (body.get("start").and_then(Value::as_f64), body.get("end").and_then(Value::as_f64)) else {
        return bad("start and end are seconds");
    };
    let dir = st.voices_dir.clone();
    match joined(tokio::task::spawn_blocking(move || lib::apply_ref(&dir, &name, start, end).and_then(|_| lib::get(&dir, &name))).await) {
        Ok(Ok(v)) => {
            crate::app::publish(json!({ "changed": "voices" }));
            Json(json!({ "voice": v })).into_response()
        }
        Ok(Err(e)) => bad(e),
        Err(r) => r,
    }
}
