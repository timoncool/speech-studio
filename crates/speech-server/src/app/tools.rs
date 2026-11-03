//! Shared application tools, route execution, journaling and window/agent coordination.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde_json::{json, Value};
use tower::ServiceExt;

mod activity;
pub(crate) mod assistant;
mod atomic;
mod catalog;
pub(crate) mod guides;
mod routing;
mod speech_tools;
mod studio_tools;
mod summaries;
mod window;

use activity::*;
pub(crate) use activity::{agent_present, name_agent, seen, status};
pub(crate) use atomic::ENGINE_SETTINGS;
use catalog::*;
pub(crate) use catalog::{annotations, resource};
use routing::*;
use summaries::*;

pub(crate) use assistant::{agent_llm, ask_agent, follow_agent_steps};
pub(crate) use window::{track, REV_HEADER};
pub use window::{window_events, window_focus, window_result};

/// The studio's API router, set once the service has built it.
static API: OnceLock<Router> = OnceLock::new();

/// Takes the API router before the Origin/Host guard is layered: the guard
/// wraps this router together with `/mcp` and `/mcp/status` from outside, so
/// the MCP endpoints are guarded and the tools' own calls do not go through it.
/// The tools' requests still carry `Host: 127.0.0.1`, so they pass the guard
/// should it be put inside this router.
pub fn install(api: Router) {
    let _ = API.set(api);
}

/// The skill an agent reads, served as a resource and a prompt.
pub(crate) const SKILL: &str = include_str!("../../../../docs/mcp-skill.md");
pub(crate) const SKILL_URI: &str = "studio://skill";
pub(crate) const LANGUAGES_URI: &str = "studio://languages";
/// Answers longer than this are cut: a transcript of hundreds of lines is more
/// than an agent reads in one call.
pub(crate) const LIMIT: usize = 60_000;

/// What a tool sends to its route.
enum Payload {
    None,
    Json(Value),
    /// Multipart form fields and files, as the page uploads them.
    Form {
        fields: Vec<(String, String)>,
        files: Vec<(String, PathBuf, String)>,
    },
    /// A command for the studio's window: what is on screen and its controls. Answered by
    /// the page itself.
    Window {
        command: &'static str,
        args: Value,
        seconds: u64,
    },
    /// A stream the window follows (server-sent events), read for `seconds` or until it ends.
    Stream {
        body: Option<Value>,
        seconds: u64,
    },
}

struct Call {
    method: Method,
    path: String,
    payload: Payload,
}

pub(crate) struct Tool {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) schema: fn() -> Value,
    call: fn(&Value) -> Result<Call, String>,
}

#[derive(Clone, Copy)]
enum Caller {
    Agent,
    Chat,
}

impl Caller {
    fn allows(self, name: &str) -> bool {
        matches!(self, Self::Agent) || !name.starts_with("assistant_")
    }
}

fn get(path: String) -> Result<Call, String> {
    Ok(Call {
        method: Method::GET,
        path,
        payload: Payload::None,
    })
}

/// Whether a request is an agent's tool call rather than the person's own control in a window.
pub(crate) fn by_agent(headers: &axum::http::HeaderMap) -> bool {
    headers.contains_key(window::AGENT_HEADER)
}

fn post(path: String, body: Value) -> Result<Call, String> {
    Ok(Call {
        method: Method::POST,
        path,
        payload: Payload::Json(body),
    })
}

fn send(method: Method, path: String, body: Value) -> Result<Call, String> {
    Ok(Call {
        method,
        path,
        payload: Payload::Json(body),
    })
}

fn composite(kind: &'static str) -> Result<Call, String> {
    Ok(Call {
        method: Method::GET,
        path: format!("composite:{kind}"),
        payload: Payload::None,
    })
}

/// A one-call tool run from elsewhere in the studio (the OpenAI-compatible routes), waited for until `until`.
pub(crate) async fn one_call(name: &str, args: &Value, until: std::time::Instant) -> Result<Value, String> {
    atomic::run_until(name, args, until).await
}

/// A GET inside the process, as JSON; a route that refuses says why.
async fn fetch(path: &str) -> Result<Value, String> {
    let (status, text) = call_route(Call {
        method: Method::GET,
        path: path.into(),
        payload: Payload::None,
    })
    .await?;
    if !status.is_success() {
        return Err(format!("GET {path}: {status} {text}"));
    }
    serde_json::from_str(&text).map_err(|_| format!("GET {path} did not answer JSON: {}", text.chars().take(200).collect::<String>()))
}

/// The app's version: tauri.conf.json is its single source, as for the window.
pub(crate) fn app_version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        let conf: Value =
            serde_json::from_str(include_str!("../../../../desktop/src-tauri/tauri.conf.json")).expect("tauri.conf.json is JSON");
        conf["version"]
            .as_str()
            .expect("tauri.conf.json names the app's version")
            .to_string()
    })
}

// ---------------------------------------------------------------- tools

pub(crate) fn tools() -> &'static [Tool] {
    catalog::tools()
}

/// Cuts an answer to what an agent reads in one go, and says how to get the rest.
pub(crate) fn cut(mut text: String) -> String {
    if text.len() > LIMIT {
        let mut end = LIMIT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n... (cut: ask for a part, e.g. project_get with from and to seconds or ids)");
    }
    text
}

/// What a tool answered: JSON, text, a picture, or the window's own answer.
pub(crate) enum Answer {
    Json(Value),
    Text(String),
    Image { data: String, mime: String, text: String },
    Window(Value),
}

/// A route answered with a file, not with data: audio, video, an archive.
fn is_file_answer(mime: &str) -> bool {
    let mime = mime.split(';').next().unwrap_or_default().trim();
    mime.starts_with("audio/") || mime.starts_with("video/") || matches!(mime, "application/octet-stream" | "application/zip")
}

/// The studio's `agent-files` folder, where the files tools answer with are kept.
pub(super) async fn agent_files() -> Result<PathBuf, String> {
    let paths = fetch("/app/paths").await?;
    let data = paths["data_dir"].as_str().ok_or("the studio named no data folder")?;
    let folder = Path::new(data).join("agent-files");
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|e| format!("{}: {e}", folder.display()))?;
    // an agent reads such a file right after the call: a day later it is only taking room
    let mut old = tokio::fs::read_dir(&folder)
        .await
        .map_err(|e| format!("{}: {e}", folder.display()))?;
    while let Some(entry) = old.next_entry().await.map_err(|e| format!("{}: {e}", folder.display()))? {
        let stale = entry
            .metadata()
            .await
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > Duration::from_secs(24 * 3600));
        if stale {
            if let Err(e) = tokio::fs::remove_file(entry.path()).await {
                tracing::warn!("{}: {e}", entry.path().display());
            }
        }
    }
    Ok(folder)
}

/// The file a route answered with, written as it comes into the `agent-files` folder: the agent gets its path, size
/// and type. A type that names no format keeps the extension of the file the route was asked for.
async fn keep_file(name: &str, asked: &str, response: Response) -> Result<Value, String> {
    let mime = mime_of(&response).split(';').next().unwrap_or_default().trim().to_string();
    let ext = match mime.as_str() {
        "audio/wav" | "audio/x-wav" | "audio/wave" | "audio/pcm" => "wav",
        "audio/flac" => "flac",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        "audio/mp4" | "audio/aac" | "audio/x-m4a" => "m4a",
        "video/mp4" => "mp4",
        "application/zip" => "zip",
        "application/octet-stream" => Path::new(asked.split('?').next().unwrap_or_default())
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("bin"),
        other => other.rsplit('/').next().unwrap_or("bin"),
    };
    let file = agent_files().await?.join(format!("{name}-{}.{ext}", chrono_stamp()));
    let written = async {
        let mut out = tokio::fs::File::create(&file).await.map_err(|e| e.to_string())?;
        let mut body = response.into_body().into_data_stream();
        let mut bytes = 0u64;
        while let Some(chunk) = futures_util::StreamExt::next(&mut body).await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            bytes += chunk.len() as u64;
            tokio::io::AsyncWriteExt::write_all(&mut out, &chunk)
                .await
                .map_err(|e| e.to_string())?;
        }
        tokio::io::AsyncWriteExt::flush(&mut out).await.map_err(|e| e.to_string())?;
        Ok::<u64, String>(bytes)
    }
    .await;
    match written {
        Ok(bytes) => Ok(json!({ "file": file.to_string_lossy(), "mime": mime, "bytes": bytes })),
        Err(error) => {
            let _ = tokio::fs::remove_file(&file).await;
            Err(format!("{}: {error}", file.display()))
        }
    }
}

/// A stream the window follows, read for `seconds` or until it ends: its events in order.
async fn read_stream(name: &str, call: Call, seconds: u64) -> Result<Value, String> {
    let response = respond(call).await?;
    if !response.status().is_success() {
        return Err(reply_text(&reply_of(response).await?));
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let mut body = response.into_body().into_data_stream();
    let mut pending: Vec<u8> = Vec::new();
    let mut events = Vec::new();
    let ended = loop {
        match tokio::time::timeout_at(until, futures_util::StreamExt::next(&mut body)).await {
            Err(_) => break false,
            Ok(None) => break true,
            Ok(Some(Err(error))) => return Err(format!("{name}: {error}")),
            Ok(Some(Ok(chunk))) => {
                pending.extend(chunk.iter().filter(|byte| **byte != b'\r'));
                while let Some(at) = pending.windows(2).position(|pair| pair == b"\n\n") {
                    let event: Vec<u8> = pending.drain(..at + 2).collect();
                    let event = String::from_utf8_lossy(&event);
                    let data: Vec<&str> = event
                        .lines()
                        .filter_map(|line| line.strip_prefix("data:"))
                        .map(|data| data.strip_prefix(' ').unwrap_or(data))
                        .collect();
                    if !data.is_empty() {
                        let data = data.join("\n");
                        events.push(serde_json::from_str(&data).unwrap_or(Value::String(data)));
                    }
                }
            }
        }
    };
    Ok(json!({ "events": events, "ended": ended }))
}

/// Several tools in one call, run as `tools/call` runs each. Its future is boxed: it runs tools, and a tool runs it.
fn batch(args: &Value, caller: Caller) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send + '_>> {
    Box::pin(async move {
        let calls = args
            .get("calls")
            .and_then(Value::as_array)
            .ok_or("'calls' must be a list of {name, arguments}")?;
        let keep_going = args.get("keep_going").and_then(Value::as_bool).unwrap_or(false);
        let mut done = Vec::new();
        for call in calls {
            let name = call.get("name").and_then(Value::as_str).unwrap_or_default();
            let arguments = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let answer = match tools().iter().find(|tool| tool.name == name) {
                None => Err(format!("Unknown tool: {name}; tools/list names them all.")),
                Some(tool) if tool.name == "studio_batch" => Err("studio_batch does not run inside itself.".into()),
                Some(tool) => run_tool(tool, &arguments, caller).await,
            };
            let failed = answer.is_err();
            done.push(match answer {
                Ok(Answer::Json(value)) | Ok(Answer::Window(value)) => json!({ "name": name, "result": value }),
                Ok(Answer::Text(text)) => json!({ "name": name, "result": text }),
                Ok(Answer::Image { mime, .. }) => {
                    json!({ "name": name, "result": format!("a picture ({mime}): call {name} alone to see it") })
                }
                Err(error) => json!({ "name": name, "error": error }),
            });
            if failed && !keep_going {
                break;
            }
        }
        Ok(json!({ "calls": done }))
    })
}

/// A name part that keeps files of one tool apart: milliseconds since 1970.
fn chrono_stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Runs a tool as `tools/call` runs one, for an agent and for the Assistant panel alike, and journals its change.
async fn run_tool(tool: &'static Tool, args: &Value, caller: Caller) -> Result<Answer, String> {
    let name = tool.name;
    if !caller.allows(name) {
        return Err(format!("Unknown tool: {name}"));
    }
    let target = if JOURNAL.iter().any(|(tool, _, _)| *tool == name) {
        journal_target(args)
    } else {
        String::new()
    };
    // a tool may look at a file on disk before its call: off the runtime
    let prepare = tool.call;
    let call = {
        let args = args.clone();
        tokio::task::spawn_blocking(move || prepare(&args))
            .await
            .unwrap_or_else(|error| Err(format!("{name} failed: {error}")))?
    };
    Ok(match call {
        Call {
            payload: Payload::Window { command, args, seconds },
            ..
        } => Answer::Window(window::ask_window(command, args, seconds).await?),
        call if call.path == "composite:status" => Answer::Json(status_summary().await),
        call if call.path == "composite:wait" => Answer::Json(wait_for(args).await?),
        call if call.path == "composite:questions" => Answer::Json(assistant::wait_for_questions(args).await),
        call if call.path == "composite:answer" => Answer::Json(assistant::answer_question(args)?),
        call if call.path == "composite:agent" => Answer::Json(status().await.0),
        call if call.path == "composite:windows" => Answer::Json(window::watch(args).await),
        call if call.path == "composite:window_focus" => Answer::Json(window::focus(args)?),
        call if call.path == "composite:window_drop" => Answer::Json(window::drop_command(args)?),
        call if call.path == "composite:batch" => Answer::Json(batch(args, caller).await?),
        call if call.path == "composite:guide" => {
            let topic = text(args, "topic")?;
            Answer::Text(guides::guide(&topic).ok_or_else(|| {
                format!(
                    "No guide {topic}: {}.",
                    guides::TOPICS.iter().map(|(topic, _)| *topic).collect::<Vec<_>>().join(", ")
                )
            })?)
        }
        call @ Call {
            payload: Payload::Stream { .. },
            ..
        } => {
            let seconds = if let Payload::Stream { seconds, .. } = &call.payload {
                *seconds
            } else {
                WAIT_DEFAULT
            };
            let read = read_stream(name, call, seconds).await?;
            Answer::Json(if name == "speech_live_watch" {
                studio_tools::live_files(read).await?
            } else {
                read
            })
        }
        call if call.path.starts_with(atomic::PREFIX) => {
            let state = atomic::run(&call.path[atomic::PREFIX.len()..], args).await?;
            journal_call(name, &target, Some(&state));
            Answer::Json(state)
        }
        call => {
            if QUEUED_BEHIND_JOBS.contains(&name) {
                graphics_card_free().await?;
            }
            let asked = call.path.clone();
            let response = respond(call).await?;
            if response.status().is_success() && is_file_answer(&mime_of(&response)) {
                let file = keep_file(name, &asked, response).await?;
                journal_call(name, &target, None);
                return Ok(Answer::Json(file));
            }
            let reply = reply_of(response).await?;
            if !reply.status.is_success() {
                return Err(reply_text(&reply));
            }
            if reply.mime.starts_with("image/") {
                use base64::Engine;
                let mime = reply.mime.split(';').next().unwrap_or_default().trim().to_string();
                Answer::Image {
                    data: base64::engine::general_purpose::STANDARD.encode(&reply.bytes),
                    mime,
                    text: format!("The picture {name} answered with."),
                }
            } else {
                let text = reply_text(&reply);
                match serde_json::from_str::<Value>(&text) {
                    Ok(value) => {
                        journal_call(name, &target, Some(&value));
                        Answer::Json(shape(name, args, redact(value)))
                    }
                    Err(_) => {
                        journal_call(name, &target, None);
                        Answer::Text(text)
                    }
                }
            }
        }
    })
}

/// The tools for the Assistant panel, as OpenAI-style functions: the same ones an agent over MCP has, but the
/// assistant's own request tools.
pub(crate) fn chat_tools() -> Vec<Value> {
    tools().iter().filter(|tool| Caller::Chat.allows(tool.name)).map(|tool| json!({ "type": "function", "function": { "name": tool.name, "description": tool.description, "parameters": (tool.schema)() } })).collect()
}

/// A tool's answer for the Assistant panel's model: its text, and a picture it answered with as a data URL.
pub(crate) struct ChatAnswer {
    pub text: String,
    pub image: Option<String>,
}

/// Runs a tool for the Assistant panel as `tools/call` runs one for an agent, journal line included.
pub(crate) async fn chat_call(name: &str, args: &Value) -> Result<ChatAnswer, String> {
    let tool = tools()
        .iter()
        .find(|tool| tool.name == name)
        .ok_or_else(|| format!("Unknown tool: {name}"))?;
    Ok(match run_tool(tool, args, Caller::Chat).await? {
        Answer::Json(value) => ChatAnswer {
            text: cut(serde_json::to_string(&value).unwrap_or_default()),
            image: None,
        },
        Answer::Text(text) => ChatAnswer {
            text: cut(text),
            image: None,
        },
        Answer::Image { data, mime, text } => ChatAnswer {
            text,
            image: Some(format!("data:{mime};base64,{data}")),
        },
        Answer::Window(result) => {
            let image = result.get("image").and_then(Value::as_str).map(|data| {
                format!(
                    "data:{};base64,{data}",
                    result.get("mime").and_then(Value::as_str).unwrap_or("image/png")
                )
            });
            let text = match result.get("text").and_then(Value::as_str) {
                Some(text) => text.to_string(),
                None if image.is_some() || result.is_null() => "Done.".into(),
                None => serde_json::to_string(&result).unwrap_or_default(),
            };
            ChatAnswer { text: cut(text), image }
        }
    })
}

#[cfg(test)]
mod tests;

pub(crate) async fn run_for_agent(tool: &'static Tool, args: &Value) -> Result<Answer, String> {
    let step = assistant::step_of(tool.name);
    if let Some(step) = &step {
        assistant::chat_step(json!({ "type": "tool", "id": step, "name": tool.name, "args": args }));
    }
    let answer = run_tool(tool, args, Caller::Agent).await;
    if let Some(step) = step {
        let (ok, summary) = match &answer {
            Ok(Answer::Json(value)) | Ok(Answer::Window(value)) => (true, serde_json::to_string(value).unwrap_or_default()),
            Ok(Answer::Text(text)) | Ok(Answer::Image { text, .. }) => (true, text.clone()),
            Err(problem) => (false, problem.clone()),
        };
        assistant::chat_step(
            json!({ "type": "tool_done", "id": step, "ok": ok, "summary": summary.chars().take(240).collect::<String>() }),
        );
    }
    answer
}
