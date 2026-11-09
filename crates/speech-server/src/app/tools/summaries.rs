//! Compact status, job waiting and result shaping for application tools.

use super::*;

// ---------------------------------------------------------------- jobs

/// A job that ended, one way or the other; any other status is still at work.
pub(super) const FINISHED: &[&str] = &["done", "error", "failed", "cancelled", "abandoned", "interrupted"];

pub(super) fn finished(job: &Value) -> bool {
    job["status"].as_str().is_some_and(|status| FINISHED.contains(&status))
}

/// The rows of GET /jobs: its list, bare or under "jobs".
pub(super) fn job_rows(listed: &Value) -> Result<Vec<Value>, String> {
    listed
        .as_array()
        .or_else(|| listed.get("jobs").and_then(Value::as_array))
        .cloned()
        .ok_or_else(|| {
            format!(
                "GET /jobs answered no list of jobs: {}",
                listed.to_string().chars().take(200).collect::<String>()
            )
        })
}

/// A job as an agent follows it: what it is, where it got, what it made. A
/// result that is a whole project is left to project_get.
pub(super) fn compact_job(job: &Value) -> Value {
    let result = &job["result"];
    let result = if result.get("segments").is_some() {
        json!("the project, updated: project_get reads it")
    } else {
        result.clone()
    };
    let mut row = json!({
        "id": job.get("id").or_else(|| job.get("job_id")).cloned().unwrap_or(Value::Null),
        "kind": job["kind"], "pid": job["pid"], "status": job["status"],
        "stage": job["stage"], "msg": job["msg"], "pct": job["pct"],
    });
    if !job["position"].is_null() {
        row["position"] = job["position"].clone();
    }
    if !result.is_null() {
        row["result"] = result;
    }
    if !job["error"].is_null() {
        row["error"] = job["error"].clone();
    }
    row
}

/// Where a running job is, in one line.
pub(super) fn describe_job(job: &Value) -> String {
    let text = |key: &str| job[key].as_str().map(str::to_string).unwrap_or_default();
    let pct = job["pct"].as_f64().map(|pct| format!(" {pct:.0}%")).unwrap_or_default();
    format!(
        "{} {} of project {} ({}{pct})",
        text("status"),
        job["kind"].as_str().unwrap_or("job"),
        text("pid"),
        text("stage")
    )
    .replace(" ()", "")
}

/// The required models still missing and whether the set is ready.
pub(super) fn compact_setup(setup: &Value) -> Value {
    let missing: Vec<Value> = setup["components"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|component| component["installed"] == false && component["requirement"] == "required")
        .map(|component| json!({ "id": component["id"], "name": component["name"], "size": component["size"] }))
        .collect();
    json!({ "ready": setup["ready"], "driver_ok": setup["driverOk"], "download_pending_bytes": setup["downloadPending"], "missing_required": missing })
}

/// Every component in one line: what it is, whether it is there, what it takes.
pub(super) fn compact_models(setup: &Value) -> Value {
    let components: Vec<Value> = setup["components"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|component| {
            json!({
                "id": component["id"], "name": component["name"], "purpose": component["purpose"], "requirement": component["requirement"],
                "installed": component["installed"], "size": component["size"], "on_disk": component["bytesOnDisk"], "vram": component["vram"],
            })
        })
        .collect();
    json!({ "ready": setup["ready"], "driver_ok": setup["driverOk"], "download_pending_bytes": setup["downloadPending"], "components": components })
}

// ---------------------------------------------------------------- projects

/// The OpenRouter key never leaves the studio, and a proxy's password is hidden.
pub(super) fn redact(value: Value) -> Value {
    match value {
        Value::Object(fields) => {
            let mut clean = serde_json::Map::new();
            for (key, value) in fields {
                match key.as_str() {
                    "or_key" => {
                        let set = value.as_str().is_some_and(|key| !key.trim().is_empty());
                        clean.insert("or_key_set".into(), set.into());
                    }
                    "proxy_url" => {
                        let shown = value
                            .as_str()
                            .map(|url| Value::String(crate::engines::models::split_proxy_password(url).0))
                            .unwrap_or(value);
                        clean.insert(key, shown);
                    }
                    _ => {
                        clean.insert(key, redact(value));
                    }
                }
            }
            Value::Object(clean)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(redact).collect()),
        other => other,
    }
}

/// What an agent is answered: the parts it acts on, unless it asked for
/// every field with response_format detailed.
pub(super) fn shape(name: &str, args: &Value, value: Value) -> Value {
    let detailed = args.get("response_format").and_then(Value::as_str) == Some("detailed");
    match name {
        "url_probe" => compact_probe(value),
        _ if detailed => value,
        "jobs_list" => match job_rows(&value) {
            Ok(rows) => {
                let mut listed = json!({ "jobs": rows.iter().map(compact_job).collect::<Vec<_>>() });
                if !value["project_job"].is_null() {
                    listed["project_job"] = value["project_job"].clone();
                }
                listed
            }
            Err(problem) => json!({ "error": problem }),
        },
        "job_get" => compact_job(&value),
        "models_status" => compact_models(&value),
        "settings_get" => json!({
            "settings": value["selection"],
            "choices": {
                "asr_engine": value["asr_engines"],
                "llama_ubatch": value["llama_ubatches"], "higgs_ref_secs": value["higgs_ref_secs_opts"],
                "local_backend": ["auto", "gpu", "vulkan", "cpu"], "tts_backend": ["auto", "gpu", "vulkan", "cpu"], "sep_backend": ["auto", "gpu", "vulkan", "cpu"], "diar_backend": ["auto", "gpu", "vulkan", "cpu"], "asr_backend": ["auto", "gpu", "vulkan", "cpu"],
            },
        }),
        "projects_list" => {
            let query = args.get("query").and_then(Value::as_str).unwrap_or_default().to_lowercase();
            let bound = |name: &str, end: bool| match args.get(name).and_then(Value::as_str) {
                Some(text) => moment(text, end).map(Some),
                None => Ok(None),
            };
            let (since, until) = match (bound("since", false), bound("until", true)) {
                (Ok(since), Ok(until)) => (since, until),
                (Err(problem), _) | (_, Err(problem)) => return json!({ "error": problem }),
            };
            let projects = value["projects"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|project| {
                    let edited = project["mtime"].as_i64().unwrap_or_default();
                    (query.is_empty() || project["name"].as_str().unwrap_or_default().to_lowercase().contains(&query))
                        && since.is_none_or(|since| edited >= since)
                        && until.is_none_or(|until| edited <= until)
                });
            Value::Array(projects.collect())
        }
        _ => value,
    }
}

/// A probe as an agent chooses from it, whatever response_format says: the preview picture's base64
/// is the window's, the automatic subtitles are only their languages, and no track names its file
/// formats, which the download picks itself.
pub(super) fn compact_probe(mut value: Value) -> Value {
    if let Value::Object(fields) = &mut value {
        fields.remove("thumbnail_data");
        fields.remove("thumbnail_error");
        if let Some(Value::Array(tracks)) = fields.get_mut("subtitles") {
            for track in tracks.iter_mut() {
                *track = json!({ "lang": track["lang"], "name": track["name"] });
            }
        }
        if let Some(Value::Array(tracks)) = fields.get_mut("auto_subtitles") {
            for track in tracks.iter_mut() {
                *track = track["lang"].take();
            }
        }
    }
    value
}

// ---------------------------------------------------------------- waiting

pub(super) async fn status_summary() -> Value {
    let (jobs, setup, by_link) = tokio::join!(fetch("/jobs"), fetch("/setup/status"), fetch("/url/fetches"));
    let mut summary = json!({});
    if let Some(role) = crate::speech::role_playing() {
        summary["live_role"] = role;
    }
    match jobs.and_then(|jobs| job_rows(&jobs)) {
        Ok(rows) => {
            let (done, working): (Vec<Value>, Vec<Value>) = rows.into_iter().partition(finished);
            summary["jobs"] = Value::Array(working.iter().map(compact_job).collect());
            summary["finished_jobs"] = Value::Array(done.iter().take(5).map(compact_job).collect());
        }
        Err(problem) => summary["jobs_error"] = problem.into(),
    }
    match setup {
        Ok(setup) => {
            if let Some(download) = download_row(&setup) {
                if let Some(jobs) = summary["jobs"].as_array_mut() {
                    jobs.push(download);
                }
            }
            summary["models"] = compact_setup(&setup);
        }
        Err(problem) => summary["models_error"] = problem.into(),
    }
    match by_link {
        Ok(listed) => {
            let rows = listed["fetches"].as_array().into_iter().flatten().map(fetch_row);
            let (done, working): (Vec<Value>, Vec<Value>) = rows.partition(finished);
            if let Some(jobs) = summary["jobs"].as_array_mut() {
                jobs.extend(working);
            }
            if let Some(ended) = summary["finished_jobs"].as_array_mut() {
                ended.extend(done.into_iter().take(3));
            }
        }
        Err(problem) => summary["url_downloads_error"] = problem.into(),
    }
    summary
}

/// A download by link runs beside the job queue like the models download: it is shown and waited for as work of
/// kind download, and its result names the project it made.
pub(super) fn fetch_row(fetch: &Value) -> Value {
    let status = match fetch["status"].as_str() {
        Some("downloading") => "running",
        Some("completed") => "done",
        Some("failed") => "error",
        Some(other) => other,
        None => "unknown",
    };
    let pct = match (fetch["downloaded"].as_f64(), fetch["total"].as_f64()) {
        (Some(done), Some(total)) if total > 0.0 => Value::from((done / total * 100.0).min(100.0).round()),
        _ => Value::Null,
    };
    let mut row = json!({ "id": fetch["id"], "kind": "download", "pid": fetch["pid"], "status": status, "stage": fetch["phase"], "msg": fetch["title"], "pct": pct, "url": fetch["url"] });
    if fetch["status"] == "completed" {
        row["result"] = json!({ "project_id": fetch["pid"], "subs_imported": fetch["subsImported"] });
    }
    if !fetch["warning"].is_null() {
        row["warning"] = json!({ "code": fetch["warning"], "detail": fetch["warningDetail"] });
    }
    if !fetch["errorCode"].is_null() {
        row["error"] = json!({ "code": fetch["errorCode"], "detail": fetch["error"], "hint": fetch["hint"] });
    }
    row
}

/// The models download runs beside the job queue: while it goes it is shown and waited for as work of
/// kind download.
pub(super) fn download_row(setup: &Value) -> Option<Value> {
    let active = setup.get("active")?;
    if active["status"] != "downloading" {
        return None;
    }
    let (done, total) = (
        active["downloaded"].as_f64().unwrap_or(0.0),
        active["total"].as_f64().unwrap_or(0.0),
    );
    Some(json!({
        "id": active["id"], "kind": "download", "status": "running", "stage": active["phase"],
        "pct": if total > 0.0 { Value::from((done / total * 100.0).round()) } else { Value::Null },
        "components": active["ids"], "waiting_seconds": active["waitingS"],
    }))
}

/// The kinds of work a status summary still has running or waiting.
pub(super) fn busy(summary: &Value) -> Result<Vec<String>, String> {
    if let Some(problem) = summary.get("jobs_error").and_then(Value::as_str) {
        return Err(format!("The studio's jobs cannot be read: {problem}"));
    }
    Ok(summary["jobs"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|job| job["kind"].as_str().unwrap_or("unknown").to_string())
        .collect())
}

/// The id of a download by link starts so; a job id is hex and never does.
pub(super) const DOWNLOAD_BY_LINK: &str = "url";

/// How long one wait holds a call: clients give up on a tool call after about
/// a minute, so a wait answers before that and the agent calls it again.
pub(super) const WAIT_DEFAULT: u64 = 30;
pub(super) const WAIT_LONGEST: u64 = 55;

/// Waits for a job or for one kind of work or everything, a slice at a time.
pub(super) async fn wait_for(args: &Value) -> Result<Value, String> {
    let seconds = args
        .get("seconds")
        .and_then(Value::as_u64)
        .unwrap_or(WAIT_DEFAULT)
        .clamp(2, WAIT_LONGEST);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let job = args.get("job_id").and_then(Value::as_str).map(str::to_string);
    let until = args.get("until").and_then(Value::as_str).unwrap_or("idle").to_string();
    loop {
        let (done, now) = match &job {
            Some(job) if job.starts_with(DOWNLOAD_BY_LINK) => {
                let state = fetch(&format!("/url/fetches/{}", segment(job))).await.map_err(|why| {
                    format!(
                        "No download by link {job} ({why}): its id is fetch.id of project_create_from_url; url_fetches_list shows them."
                    )
                })?;
                let row = fetch_row(&state);
                (finished(&row), row)
            }
            Some(job) => {
                let state = fetch(&format!("/jobs/{}", segment(job)))
                    .await
                    .map_err(|why| format!("No job {job} ({why}): job_id is what project_synth, project_export, project_cast_characters, podcast_write, listen_transcribe, listen_cut, voice_transcribe, voice_clean, voices_download_pack or a one-call tool answering done false returned, or the fetch.id of a download by link (a models download is waited for with until download). Wait for other work with until."))?;
                if state.get("status").and_then(Value::as_str).is_none() {
                    return Err(format!("The job {job} has no status: {state}"));
                }
                (finished(&state), compact_job(&state))
            }
            None => {
                let summary = status_summary().await;
                let working = busy(&summary)?;
                let done = if until == "idle" {
                    working.is_empty()
                } else {
                    !working.contains(&until)
                };
                (done, summary)
            }
        };
        if done {
            return Ok(json!({ "done": true, "state": now }));
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(json!({ "done": false, "note": "still running; call studio_wait again to keep waiting", "state": now }));
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Tools whose route waits in the graphics card's queue: while a job runs they
/// would wait behind it for minutes, longer than a client holds a call.
pub(super) const QUEUED_BEHIND_JOBS: &[&str] = &["project_frame"];

pub(super) async fn graphics_card_free() -> Result<(), String> {
    let jobs = fetch("/jobs")
        .await
        .map_err(|why| format!("The studio's jobs cannot be read, so a frame could wait behind a render for minutes: {why}"))?;
    let working: Vec<String> = job_rows(&jobs)?.iter().filter(|job| !finished(job)).map(describe_job).collect();
    if working.is_empty() {
        return Ok(());
    }
    Err(format!(
        "The graphics card is busy: {}. A frame waits behind that work; call studio_wait until idle, then project_frame again.",
        working.join("; ")
    ))
}
