//! OpenAI-compatible routes for other programs on this computer: `POST /v1/audio/speech`, `POST
//! /v1/audio/transcriptions`, `GET /v1/models`. `voice` is a voice name of the library, `model` an engine alias,
//! `instructions` the block direction; errors come in OpenAI's shape. The work is the agent's one-call tools, waited for
//! whole. With an API key (`SPEECH_STUDIO_API_TOKEN`, else the one made in Settings, API) a request needs
//! `Authorization: Bearer <key>`. Every request goes into a log the API page shows (the latest ones, since the start).

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Multipart, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

use crate::AppState;

/// How long a request waits for its audio or transcript.
const LONGEST: Duration = Duration::from_secs(30 * 60);
/// How many of the latest requests the log keeps.
const LOGGED: usize = 300;

/// Engine aliases: OpenAI's names map to the local engines.
const SPEECH_MODELS: &[(&str, &str)] = &[
    ("tts-1", "higgs"),
    ("tts-1-hd", "higgs"),
    ("gpt-4o-mini-tts", "higgs"),
    ("higgs", "higgs"),
    ("drama3", "drama3"),
];
const TRANSCRIBE_MODELS: &[&str] = &["whisper-1", "gpt-4o-transcribe", "gpt-4o-mini-transcribe", "parakeet"];

/// A request the API answered, for the log.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Logged {
    pub at: i64,
    pub endpoint: &'static str,
    pub model: String,
    pub voice: String,
    pub status: u16,
    pub ms: u64,
    /// Characters of text asked to voice, or bytes of audio sent to recognise.
    pub size: usize,
    pub error: Option<String>,
}

static LOG: Mutex<VecDeque<Logged>> = Mutex::new(VecDeque::new());

fn log(row: Logged) {
    let mut rows = LOG.lock().unwrap_or_else(|p| p.into_inner());
    rows.push_back(row);
    while rows.len() > LOGGED {
        rows.pop_front();
    }
}

/// What failed, in OpenAI's error kinds.
struct Failure {
    status: StatusCode,
    kind: &'static str,
    message: String,
}

fn invalid(message: impl Into<String>) -> Failure {
    Failure {
        status: StatusCode::BAD_REQUEST,
        kind: "invalid_request_error",
        message: message.into(),
    }
}

fn server(message: impl Into<String>) -> Failure {
    Failure {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        kind: "server_error",
        message: message.into(),
    }
}

fn error_response(f: &Failure) -> Response {
    (
        f.status,
        Json(json!({ "error": { "message": f.message, "type": f.kind, "param": Value::Null, "code": Value::Null } })),
    )
        .into_response()
}

/// Answers a request and writes it into the log.
fn answered(mut row: Logged, started: Instant, result: Result<Response, Failure>) -> Response {
    row.at = chrono::Utc::now().timestamp_millis();
    row.ms = started.elapsed().as_millis() as u64;
    let response = match result {
        Ok(response) => response,
        Err(f) => {
            row.error = Some(f.message.clone());
            error_response(&f)
        }
    };
    row.status = response.status().as_u16();
    log(row);
    response
}

fn authorised(headers: &HeaderMap) -> Result<(), Failure> {
    let token = crate::platform::credentials::api_token();
    authorised_token(headers, token.as_ref().map(|(token, _)| token.as_str()))
}

fn authorised_token(headers: &HeaderMap, token: Option<&str>) -> Result<(), Failure> {
    let Some(token) = token else { return Ok(()) };
    let given = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    if bool::from(subtle::ConstantTimeEq::ct_eq(given.as_bytes(), token.as_bytes())) {
        Ok(())
    } else {
        Err(Failure {
            status: StatusCode::UNAUTHORIZED,
            kind: "invalid_request_error",
            message: "Incorrect API key provided.".into(),
        })
    }
}

/// GET /v1/models
pub async fn models(headers: HeaderMap) -> Response {
    let started = Instant::now();
    let result = authorised(&headers).map(|()| {
        let ids = SPEECH_MODELS.iter().map(|(id, _)| *id).chain(TRANSCRIBE_MODELS.iter().copied());
        Json(json!({ "object": "list", "data": ids.map(|id| json!({ "id": id, "object": "model", "created": 0, "owned_by": "speech-studio" })).collect::<Vec<_>>() })).into_response()
    });
    answered(
        Logged {
            endpoint: "models",
            ..Default::default()
        },
        started,
        result,
    )
}

/// POST /v1/audio/speech {model, input, voice, response_format?, speed?, instructions?, seed?, temperature?, top_k?, top_p?,
/// max_tokens?, steps?, loudness?}
pub async fn speech(headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let started = Instant::now();
    let row = Logged {
        endpoint: "speech",
        model: body.get("model").and_then(Value::as_str).unwrap_or("tts-1").to_string(),
        voice: body.get("voice").and_then(Value::as_str).unwrap_or_default().to_string(),
        size: body.get("input").and_then(Value::as_str).map_or(0, |t| t.chars().count()),
        ..Default::default()
    };
    let result = speak(&headers, &body).await;
    answered(row, started, result)
}

async fn speak(headers: &HeaderMap, body: &Value) -> Result<Response, Failure> {
    speak_with(headers, body, |args| async move {
        crate::app::tools::one_call("speak_text", &args, Instant::now() + LONGEST).await
    })
    .await
}

async fn speak_with<F, Fut>(headers: &HeaderMap, body: &Value, call: F) -> Result<Response, Failure>
where
    F: FnOnce(Value) -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    authorised(headers)?;
    let input = body.get("input").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    if input.is_empty() {
        return Err(invalid("'input' is required."));
    }
    let model = body.get("model").and_then(Value::as_str).unwrap_or("tts-1");
    let Some((_, engine)) = SPEECH_MODELS.iter().find(|(id, _)| *id == model) else {
        return Err(invalid(format!("The model '{model}' does not exist: GET /v1/models lists them.")));
    };
    let asked = body.get("response_format").and_then(Value::as_str).unwrap_or("mp3");
    let (format, mime) = match asked {
        "mp3" => ("mp3", "audio/mpeg"),
        "wav" => ("wav", "audio/wav"),
        "flac" => ("flac", "audio/flac"),
        "aac" => ("m4a", "audio/aac"),
        "pcm" => ("wav", "audio/pcm"),
        other => {
            return Err(invalid(format!(
                "response_format '{other}' is not supported here: mp3, wav, flac, aac or pcm."
            )))
        }
    };
    let speed = body.get("speed").and_then(Value::as_f64).unwrap_or(1.0);
    if !(0.25..=4.0).contains(&speed) {
        return Err(invalid("speed is between 0.25 and 4.0."));
    }
    let text = if (speed - 1.0).abs() > 1e-3 {
        format!("[rate:{speed:.2}] {input}")
    } else {
        input
    };
    let mut args = json!({ "text": text, "engine": engine, "format": format });
    if let Some(voice) = body.get("voice").and_then(Value::as_str).filter(|v| !v.is_empty()) {
        args["voice"] = json!(voice);
    }
    if let Some(instructions) = body.get("instructions").and_then(Value::as_str).filter(|v| !v.trim().is_empty()) {
        args["direction"] = json!(instructions.trim());
    }
    // beyond OpenAI's fields: the project settings of the studio's own engines
    for setting in std::iter::once(&"seed").chain(crate::app::tools::ENGINE_SETTINGS) {
        if let Some(v) = body.get(*setting).filter(|v| !v.is_null()) {
            args[*setting] = v.clone();
        }
    }
    let done = match call(args).await {
        Ok(v) if v["done"] == true => v,
        Ok(_) => {
            return Err(Failure {
                status: StatusCode::GATEWAY_TIMEOUT,
                kind: "server_error",
                message: "The speech took longer than the request may wait.".into(),
            })
        }
        Err(e) => return Err(invalid(e)),
    };
    let path = done["file"].as_str().ok_or_else(|| server("the studio named no file"))?;
    let bytes = tokio::fs::read(path).await.map_err(|e| server(format!("{path}: {e}")))?;
    let bytes = if asked == "pcm" { pcm_of_wav(&bytes) } else { bytes };
    Ok(([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response())
}

/// Raw 16-bit samples of a WAV file (the `data` chunk).
fn pcm_of_wav(wav: &[u8]) -> Vec<u8> {
    let mut at = 12;
    while at + 8 <= wav.len() {
        let size = u32::from_le_bytes([wav[at + 4], wav[at + 5], wav[at + 6], wav[at + 7]]) as usize;
        if &wav[at..at + 4] == b"data" {
            return wav[at + 8..(at + 8 + size).min(wav.len())].to_vec();
        }
        at += 8 + size + (size & 1);
    }
    Vec::new()
}

/// POST /v1/audio/transcriptions (multipart: file, model, language?, response_format?)
pub async fn transcriptions(State(st): State<AppState>, headers: HeaderMap, multipart: Multipart) -> Response {
    let started = Instant::now();
    let mut row = Logged {
        endpoint: "transcriptions",
        model: "whisper-1".into(),
        ..Default::default()
    };
    let result = transcribe(&st, &headers, multipart, &mut row).await;
    answered(row, started, result)
}

async fn transcribe(st: &AppState, headers: &HeaderMap, multipart: Multipart, row: &mut Logged) -> Result<Response, Failure> {
    transcribe_with(st, headers, multipart, row, |args| async move {
        crate::app::tools::one_call("transcribe_file", &args, Instant::now() + LONGEST).await
    })
    .await
}

async fn transcribe_with<F, Fut>(
    st: &AppState,
    headers: &HeaderMap,
    mut multipart: Multipart,
    row: &mut Logged,
    call: F,
) -> Result<Response, Failure>
where
    F: FnOnce(Value) -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    authorised(headers)?;
    let (mut bytes, mut name, mut model, mut language, mut format) = (
        Vec::new(),
        String::from("audio.wav"),
        String::from("whisper-1"),
        String::from("auto"),
        String::from("json"),
    );
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(e) => return Err(invalid(e.to_string())),
        };
        let key = field.name().unwrap_or_default().to_string();
        if key == "file" {
            name = field.file_name().unwrap_or("audio.wav").to_string();
            bytes = field.bytes().await.map_err(|e| invalid(e.to_string()))?.to_vec();
        } else {
            let value = field.text().await.map_err(|e| invalid(e.to_string()))?;
            match key.as_str() {
                "model" => model = value,
                "language" if !value.trim().is_empty() => language = value.trim().to_lowercase(),
                "response_format" => format = value,
                _ => {}
            }
        }
    }
    row.model = model.clone();
    row.size = bytes.len();
    if bytes.is_empty() {
        return Err(invalid("'file' is required."));
    }
    if !TRANSCRIBE_MODELS.contains(&model.as_str()) {
        return Err(invalid(format!("The model '{model}' does not exist: GET /v1/models lists them.")));
    }
    let file_format = match format.as_str() {
        "json" | "text" | "verbose_json" => "json",
        "srt" => "srt",
        "vtt" => "vtt",
        other => {
            return Err(invalid(format!(
                "response_format '{other}' is not supported: json, text, srt, verbose_json or vtt."
            )))
        }
    };
    let ext = std::path::Path::new(&name)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("wav")
        .to_ascii_lowercase();
    let dir = st.workspace.join(".uploads");
    let path = dir.join(format!("{}.{ext}", blake3::hash(&bytes).to_hex()));
    if !path.is_file() {
        tokio::fs::create_dir_all(&dir)
            .await
            .and(tokio::fs::write(&path, &bytes).await)
            .map_err(|e| server(format!("{}: {e}", path.display())))?;
    }
    let args = json!({ "path": path, "format": file_format, "lang": language });
    let done = match call(args).await {
        Ok(v) if v["done"] == true => v,
        Ok(_) => {
            return Err(Failure {
                status: StatusCode::GATEWAY_TIMEOUT,
                kind: "server_error",
                message: "The recognition took longer than the request may wait.".into(),
            })
        }
        Err(e) => return Err(invalid(e)),
    };
    let body = done["transcript"].as_str().unwrap_or_default().to_string();
    if matches!(format.as_str(), "srt" | "vtt") {
        return Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response());
    }
    let parsed: Value = serde_json::from_str(&body).map_err(|e| server(format!("the transcript is not JSON: {e}")))?;
    Ok(transcript_answer(&format, &language, &parsed))
}

/// The transcript in the response_format OpenAI's clients read: json {text}, text, or verbose_json with segments and words.
fn transcript_answer(format: &str, language: &str, parsed: &Value) -> Response {
    let lines = parsed["lines"].as_array().cloned().unwrap_or_default();
    let text = lines.iter().filter_map(|l| l["text"].as_str()).collect::<Vec<_>>().join(" ");
    match format {
        "text" => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response(),
        "verbose_json" => Json(json!({
            "task": "transcribe", "language": language, "duration": parsed["duration"], "text": text,
            "segments": lines.iter().enumerate().map(|(i, l)| json!({ "id": i, "start": l["start"], "end": l["end"], "text": l["text"], "speaker": l["speaker"] })).collect::<Vec<_>>(),
            "words": lines.iter().flat_map(|l| l["words"].as_array().cloned().unwrap_or_default()).map(|w| json!({ "word": w["text"], "start": w["start"], "end": w["end"] })).collect::<Vec<_>>(),
        }))
        .into_response(),
        _ => Json(json!({ "text": text })).into_response(),
    }
}

// ---------------------------------------------------------------- the API page

fn key_state() -> Value {
    let source = crate::platform::credentials::api_token().map(|(_, source)| source);
    json!({ "configured": source.is_some(), "source": source, "environment_variable": crate::platform::credentials::API_TOKEN_ENV_VAR })
}

/// GET /openai/settings: whether a key guards the API and where it comes from; the key itself never comes back.
pub async fn settings() -> Response {
    Json(json!({ "key": key_state(), "speech_models": SPEECH_MODELS.iter().map(|(id, engine)| json!({ "id": id, "engine": engine })).collect::<Vec<_>>(), "transcribe_models": TRANSCRIBE_MODELS })).into_response()
}

/// POST /openai/key: makes a new key and answers it once, for the person to give their program.
pub async fn new_key() -> Response {
    let token = format!("sk-speech-{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    match crate::platform::credentials::store_api_token(Some(&token)) {
        Ok(_) => Json(json!({ "key": token, "state": key_state() })).into_response(),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "key_not_stored", "detail": format!("{e:#}") })),
        )
            .into_response(),
    }
}

/// DELETE /openai/key: the API is open to this computer again.
pub async fn delete_key() -> Response {
    match crate::platform::credentials::store_api_token(None) {
        Ok(_) => Json(key_state()).into_response(),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "key_not_removed", "detail": format!("{e:#}") })),
        )
            .into_response(),
    }
}

/// GET /openai/log: the latest requests, newest first.
pub async fn requests() -> Response {
    let rows: Vec<Logged> = LOG.lock().unwrap_or_else(|p| p.into_inner()).iter().rev().cloned().collect();
    Json(json!({ "requests": rows })).into_response()
}

/// DELETE /openai/log
pub async fn clear_requests() -> Response {
    LOG.lock().unwrap_or_else(|p| p.into_inner()).clear();
    StatusCode::NO_CONTENT.into_response()
}

#[cfg(test)]
mod boundary_tests;

#[cfg(test)]
mod tests {
    use super::*;

    async fn body_json(response: Response) -> Value {
        serde_json::from_slice(&axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    #[tokio::test]
    async fn models_list_in_openai_shape() {
        let listed = body_json(models(HeaderMap::new()).await).await;
        assert_eq!(listed["object"], "list");
        let ids: Vec<&str> = listed["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"tts-1") && ids.contains(&"whisper-1"));
        assert!(listed["data"].as_array().unwrap().iter().all(|m| m["object"] == "model"));
    }

    #[tokio::test]
    async fn a_bad_request_answers_in_openai_error_shape_and_is_logged() {
        let response = speech(HeaderMap::new(), Json(json!({ "model": "tts-1", "input": "  " }))).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error = body_json(response).await;
        assert_eq!(error["error"]["type"], "invalid_request_error");
        assert!(error["error"]["message"].as_str().unwrap().contains("input"));
        let response = speech(HeaderMap::new(), Json(json!({ "model": "nope", "input": "Привет" }))).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let rows = LOG.lock().unwrap().clone();
        assert!(rows.iter().any(|r| r.endpoint == "speech"
            && r.model == "nope"
            && r.status == 400
            && r.size == 6
            && r.error.as_deref().is_some_and(|e| e.contains("does not exist"))));
    }

    #[test]
    fn verbose_json_carries_segments_and_words() {
        let parsed = json!({ "duration": 2.0, "lines": [{ "text": "Привет мир", "start": 0.0, "end": 1.0, "speaker": "S1", "words": [{ "text": "Привет", "start": 0.0, "end": 0.5 }] }] });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let v = rt.block_on(body_json(transcript_answer("verbose_json", "ru", &parsed)));
        assert_eq!(v["text"], "Привет мир");
        assert_eq!(v["segments"][0]["end"], 1.0);
        assert_eq!(v["words"][0]["word"], "Привет");
        let j = rt.block_on(body_json(transcript_answer("json", "ru", &parsed)));
        assert_eq!(j, json!({ "text": "Привет мир" }));
    }

    #[test]
    fn pcm_is_the_data_chunk_of_the_wav() {
        let mut wav = b"RIFF\0\0\0\0WAVE".to_vec();
        wav.extend(b"fmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend([0u8; 16]);
        wav.extend(b"data");
        wav.extend(4u32.to_le_bytes());
        wav.extend([1, 2, 3, 4]);
        assert_eq!(pcm_of_wav(&wav), vec![1, 2, 3, 4]);
    }
}
