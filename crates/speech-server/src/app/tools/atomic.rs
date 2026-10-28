//! One-call tools: a file or a text in, the result out, without working in the studio. Each finds its project again
//! by a key of its inputs, so a call repeated with the same inputs answers from the finished work; when the work takes
//! longer than `seconds`, the answer is `done: false` with the running job, and the same call later goes on from there.

use std::time::{Duration, Instant};

use axum::http::{Method, StatusCode};
use serde_json::{json, Value};

use super::{call_route, compact_job, finished, object, segment, text, Call, Payload, Tool, WAIT_LONGEST};

pub(super) const PREFIX: &str = "composite:atomic:";
/// How long a call waits for its work by default before answering done false.
const WAIT: u64 = 45;

fn seconds_arg() -> Value {
    json!({ "type": "integer", "description": "how long to wait for the work in this call (default 45, at most 55); longer work answers done false with its job" })
}

fn composite(name: &str) -> Result<Call, String> {
    Ok(Call { method: Method::GET, path: format!("{PREFIX}{name}"), payload: Payload::None })
}

pub(super) fn tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "speak_text",
            description: "A text voiced into an audio file in one call: voice is a voice name of voices_list (none: the engine's base voice), engine an id of engines_list (default higgs), lang a code or auto, format wav, flac, mp3, ogg or m4a (default flac). Lines like \"Anna: ...\" give a dialogue with the cast of the voices named after the speakers. The first call makes a Speech project (project_id) and voices it; calling again with the same text and arguments answers with the file at once. The answer names the file on this computer.",
            schema: || object(json!({ "text": { "type": "string" }, "voice": { "type": "string" }, "engine": { "type": "string", "enum": crate::engines::all().iter().map(|e| e.id).collect::<Vec<_>>() }, "lang": { "type": "string" }, "format": { "type": "string", "enum": ["wav", "flac", "mp3", "ogg", "m4a"] }, "direction": { "type": "string", "description": "how it should sound (tone, pace), for the engines that take it" }, "seed": { "type": "integer", "minimum": 0, "description": "a fixed seed: another seed gives another take of the same text" }, "temperature": { "type": "number", "description": "sampling temperature of the engine (Higgs, Drama 3, Qwen3-TTS); the engine's default when left out" }, "top_k": { "type": "integer", "minimum": 0 }, "top_p": { "type": "number" }, "max_tokens": { "type": "integer", "minimum": 0, "description": "ceiling of the engine's answer in tokens; 0 sets it by the text's length" }, "steps": { "type": "integer", "minimum": 1, "description": "flow steps of Supertonic" }, "loudness": { "type": "string", "enum": ["off", "video", "podcast", "acx"], "description": "loudness preset of the export (default video)" }, "seconds": seconds_arg() }), &["text"]),
            call: |_| composite("speak_text"),
        },
        Tool {
            name: "transcribe_file",
            description: "The transcript of an audio or video file on this computer in one call: who speaks when and what they say. format: text (one line per phrase with its speaker and time, the default), srt, vtt, ass or json (lines with word timings). lang: the spoken language or auto; speakers: 0 to find them (default), 1 for one speaker, 2-8 for that many. The first call makes a Listening project of the file (project_id) and recognises it; calling again with the same file answers from it at once.",
            schema: || object(json!({ "path": { "type": "string" }, "format": { "type": "string", "enum": ["text", "srt", "vtt", "ass", "json"] }, "lang": { "type": "string" }, "speakers": { "type": "integer" }, "seconds": seconds_arg() }), &["path"]),
            call: |_| composite("transcribe_file"),
        },
        Tool {
            name: "verify_speech",
            description: "Check a voiced file against the text it should say, in one call: it is recognised and compared word by word; the answer has the word error rate and each missed, extra and replaced word. Use it on any audio, a take, an export or a recording.",
            schema: || object(json!({ "path": { "type": "string" }, "reference": { "type": "string" }, "lang": { "type": "string" }, "seconds": seconds_arg() }), &["path", "reference"]),
            call: |_| composite("verify_speech"),
        },
        Tool {
            name: "separate_file",
            description: "The voice of an audio or video file apart from music and noise (Mel-Band Roformer) in one call: two FLAC files, vocals and instrumental, on this computer. Calling again with the same file answers with them at once.",
            schema: || object(json!({ "path": { "type": "string" }, "seconds": seconds_arg() }), &["path"]),
            call: |_| composite("separate_file"),
        },
    ]
}

pub(super) async fn run(name: &str, args: &Value) -> Result<Value, String> {
    let seconds = args.get("seconds").and_then(Value::as_u64).unwrap_or(WAIT).clamp(1, WAIT_LONGEST);
    run_until(name, args, Instant::now() + Duration::from_secs(seconds)).await
}

/// The same work with a deadline of the caller's own (the OpenAI-compatible routes wait for the whole result).
pub(crate) async fn run_until(name: &str, args: &Value, until: Instant) -> Result<Value, String> {
    match name {
        "speak_text" => speak(args, until).await,
        "transcribe_file" => transcribe(args, until).await,
        "verify_speech" => verify(args, until).await,
        "separate_file" => separate(args, until).await,
        other => Err(format!("{other} is not a one-call tool")),
    }
}

// ---------------------------------------------------------------- routes and jobs

async fn request(method: Method, path: String, body: Option<Value>) -> Result<(StatusCode, Value), String> {
    let payload = body.map_or(Payload::None, Payload::Json);
    let (status, said) = call_route(Call { method, path, payload }).await?;
    Ok((status, serde_json::from_str(&said).unwrap_or(Value::String(said))))
}

fn refusal(asked: &str, status: StatusCode, said: &Value) -> String {
    let said = said.get("detail").and_then(Value::as_str).map_or_else(|| said.as_str().map_or_else(|| said.to_string(), str::to_string), str::to_string);
    format!("{asked}: {status} {said}")
}

async fn ok(method: Method, path: String, body: Option<Value>) -> Result<Value, String> {
    let asked = format!("{method} {}", path.split('?').next().unwrap_or_default());
    match request(method, path, body).await? {
        (status, said) if status.is_success() => Ok(said),
        (status, said) => Err(refusal(&asked, status, &said)),
    }
}

enum Waited {
    Done(Value),
    Failed(Value),
    Running(Value),
}

/// Waits for a job until it ends or the time is up, holding the studio's long poll.
async fn wait(id: &str, until: Instant) -> Result<Waited, String> {
    loop {
        let hold = until.saturating_duration_since(Instant::now()).as_secs().clamp(1, 20);
        let path = format!("/jobs/{}?wait={hold}", segment(id));
        let state = match request(Method::GET, path.clone(), None).await? {
            (StatusCode::NOT_FOUND, _) => return Ok(Waited::Done(Value::Null)),
            (status, said) if !status.is_success() => return Err(refusal(&format!("GET {path}"), status, &said)),
            (_, state) => state,
        };
        if finished(&state) {
            return Ok(if state["status"] == "done" { Waited::Done(state) } else { Waited::Failed(state) });
        }
        if Instant::now() >= until {
            return Ok(Waited::Running(state));
        }
    }
}

fn failure(job: &Value) -> String {
    let why = match &job["error"] {
        Value::String(error) => error.clone(),
        Value::Null => job["status"].as_str().unwrap_or("failed").to_string(),
        error => error.get("text").and_then(Value::as_str).map_or_else(|| error.to_string(), str::to_string),
    };
    format!("{} ({}): {why}", job["kind"].as_str().unwrap_or("the job"), job["status"].as_str().unwrap_or("error"))
}

fn pending(tool: &str, pid: Option<&str>, job: &Value) -> Value {
    let id = job.get("id").or_else(|| job.get("job_id")).and_then(Value::as_str).unwrap_or_default();
    json!({
        "done": false,
        "project_id": pid,
        "job": compact_job(job),
        "next": format!("The work is still going. studio_wait with job_id {id} until it is done, then call {tool} again with the same arguments: it goes on from the finished work."),
    })
}

/// Starts a job by its route and waits: its result when done, the running job when the time is up.
async fn job(route: String, body: Value, until: Instant) -> Result<Result<Value, Value>, String> {
    let started = ok(Method::POST, route, Some(body)).await?;
    let Some(id) = started["job_id"].as_str() else { return Ok(Ok(started)) };
    match wait(id, until).await? {
        Waited::Done(state) => Ok(Ok(state.get("result").cloned().unwrap_or(Value::Null))),
        Waited::Failed(state) => Err(failure(&state)),
        Waited::Running(state) => Ok(Err(state)),
    }
}

/// The job at work on a project, if any.
async fn running_on(pid: &str) -> Result<Option<Value>, String> {
    let listed = ok(Method::GET, format!("/jobs?pid={}", segment(pid)), None).await?;
    Ok(super::job_rows(&listed)?.into_iter().find(|j| j["pid"] == pid && !finished(j)))
}

fn local_path(args: &Value) -> Result<String, String> {
    let path = text(args, "path")?;
    if !std::path::Path::new(&path).is_file() {
        return Err(format!("{path} is not a file on this computer."));
    }
    Ok(path)
}

// ---------------------------------------------------------------- tools

/// The project settings a one-call voicing takes over the engine's defaults, as POST /projects names them.
pub(crate) const ENGINE_SETTINGS: &[&str] = &["temperature", "top_k", "top_p", "max_tokens", "steps", "loudness"];

async fn speak(args: &Value, until: Instant) -> Result<Value, String> {
    let said = text(args, "text")?;
    let format = args.get("format").and_then(Value::as_str).unwrap_or("flac").to_string();
    if !["wav", "flac", "mp3", "ogg", "m4a"].contains(&format.as_str()) {
        return Err(format!("format is wav, flac, mp3, ogg or m4a, not {format}"));
    }
    let field = |n: &str| args.get(n).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let seed = args.get("seed").and_then(Value::as_u64);
    let mut keyed = json!({ "text": said, "voice": field("voice"), "engine": field("engine"), "lang": field("lang"), "direction": field("direction") });
    if let Some(seed) = seed {
        keyed["seed"] = json!(seed);
    }
    for setting in ENGINE_SETTINGS {
        if let Some(v) = args.get(*setting).filter(|v| !v.is_null()) {
            keyed[*setting] = v.clone();
        }
    }
    let key = blake3::hash(keyed.to_string().as_bytes()).to_hex()[..16].to_string();
    let name: String = said.chars().take(40).collect();
    let mut asked = json!({ "name": name, "text": said, "voice": field("voice"), "engine": field("engine"), "lang": field("lang"), "direction": field("direction"), "seed": seed, "key": key });
    for setting in ENGINE_SETTINGS {
        asked[*setting] = keyed[*setting].clone();
    }
    let made = ok(Method::POST, "/projects".into(), Some(asked)).await?;
    let pid = made["project_id"].as_str().ok_or("POST /projects answered no project_id")?.to_string();
    let want = format!("{}.{format}", made["project"]["name"].as_str().unwrap_or("speech"));
    let exports = ok(Method::GET, format!("/projects/{}/exports", segment(&pid)), None).await?;
    if let Some(dir) = exports["dir"].as_str() {
        let ready = exports["files"].as_array().is_some_and(|files| files.iter().any(|f| f["file"].as_str().is_some_and(|n| n.ends_with(&format!(".{format}")))));
        if ready && made["existing"] == true {
            return Ok(json!({ "done": true, "project_id": pid, "file": std::path::Path::new(dir).join(&want) }));
        }
    }
    if let Some(active) = running_on(&pid).await? {
        match wait(active["id"].as_str().unwrap_or_default(), until).await? {
            Waited::Running(state) => return Ok(pending("speak_text", Some(&pid), &state)),
            Waited::Failed(state) => return Err(failure(&state)),
            Waited::Done(_) => {}
        }
    }
    let project = ok(Method::GET, format!("/projects/{}", segment(&pid)), None).await?;
    let unvoiced = project["chapters"].as_array().into_iter().flatten().flat_map(|c| c["blocks"].as_array().into_iter().flatten()).any(|b| b["status"] != "ready");
    if unvoiced {
        if let Err(state) = job(format!("/projects/{}/synth", segment(&pid)), json!({}), until).await? {
            return Ok(pending("speak_text", Some(&pid), &state));
        }
    }
    match job(format!("/projects/{}/export", segment(&pid)), json!({ "format": format }), until).await? {
        Err(state) => Ok(pending("speak_text", Some(&pid), &state)),
        Ok(result) => Ok(json!({ "done": true, "project_id": pid, "file": result["path"], "duration": result["duration"] })),
    }
}

/// The Listening project of a file, recognised: its id when done, or the pending answer.
async fn recognised(tool: &str, args: &Value, until: Instant) -> Result<Result<String, Value>, String> {
    let path = local_path(args)?;
    let made = ok(Method::POST, "/listen/from-path".into(), Some(json!({ "path": path }))).await?;
    let pid = made["pid"].as_str().ok_or("POST /listen/from-path answered no pid")?.to_string();
    if let Some(active) = running_on(&pid).await? {
        match wait(active["id"].as_str().unwrap_or_default(), until).await? {
            Waited::Running(state) => return Ok(Err(pending(tool, Some(&pid), &state))),
            Waited::Failed(state) => return Err(failure(&state)),
            Waited::Done(_) => {}
        }
    }
    let mut project = ok(Method::GET, format!("/listen/{}", segment(&pid)), None).await?;
    let lang = args.get("lang").and_then(Value::as_str).unwrap_or("auto");
    let speakers = args.get("speakers").and_then(Value::as_u64).unwrap_or(0);
    let wanted = project["settings"]["lang"] == lang && project["settings"]["speakers"] == speakers;
    if project["made"].is_null() || !wanted {
        project["settings"]["lang"] = json!(lang);
        project["settings"]["speakers"] = json!(speakers);
        ok(Method::PUT, format!("/listen/{}", segment(&pid)), Some(project)).await?;
        if let Err(state) = job(format!("/listen/{}/transcribe", segment(&pid)), json!({}), until).await? {
            return Ok(Err(pending(tool, Some(&pid), &state)));
        }
    }
    Ok(Ok(pid))
}

async fn transcribe(args: &Value, until: Instant) -> Result<Value, String> {
    let format = args.get("format").and_then(Value::as_str).unwrap_or("text");
    let file_format = if format == "text" { "txt" } else { format };
    if !["txt", "srt", "vtt", "ass", "json"].contains(&file_format) {
        return Err(format!("format is text, srt, vtt, ass or json, not {format}"));
    }
    let pid = match recognised("transcribe_file", args, until).await? {
        Ok(pid) => pid,
        Err(waiting) => return Ok(waiting),
    };
    let written = ok(Method::POST, format!("/listen/{}/export", segment(&pid)), Some(json!({ "format": file_format, "speakers": true, "times": true }))).await?;
    let full = written["text"].as_str().unwrap_or_default();
    const LIMIT: usize = 60_000;
    let shown: String = full.chars().take(LIMIT).collect();
    Ok(json!({ "done": true, "project_id": pid, "format": format, "transcript": shown, "cut": full.chars().count() > LIMIT, "file": written["file"] }))
}

async fn verify(args: &Value, until: Instant) -> Result<Value, String> {
    let reference = text(args, "reference")?;
    let pid = match recognised("verify_speech", args, until).await? {
        Ok(pid) => pid,
        Err(waiting) => return Ok(waiting),
    };
    let report = ok(Method::POST, format!("/listen/{}/check", segment(&pid)), Some(json!({ "reference": reference }))).await?;
    let project = ok(Method::GET, format!("/listen/{}", segment(&pid)), None).await?;
    let heard: Vec<&str> = project["lines"].as_array().into_iter().flatten().filter(|l| l["hidden"] != true).filter_map(|l| l["text"].as_str()).collect();
    let errors: Vec<&Value> = report["ops"].as_array().into_iter().flatten().filter(|o| o["op"] != "same").collect();
    Ok(json!({ "done": true, "project_id": pid, "wer": report["wer"], "words": report["words"], "heard": heard.join(" "), "differences": errors }))
}

async fn separate(args: &Value, until: Instant) -> Result<Value, String> {
    let path = local_path(args)?;
    match job("/separate".into(), json!({ "path": path }), until).await? {
        Err(state) => Ok(pending("separate_file", None, &state)),
        Ok(result) if result.is_null() => ok(Method::POST, "/separate".into(), Some(json!({ "path": path }))).await,
        Ok(result) => Ok(result),
    }
}
