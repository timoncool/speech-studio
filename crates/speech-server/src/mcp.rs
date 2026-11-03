//! MCP protocol adapter over the application's shared tools.

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::app::agent::{
    annotations, app_version, cut, guides, name_agent, resource, run_for_agent, seen, tools, Answer, LANGUAGES_URI, LIMIT, SKILL, SKILL_URI,
};

const STUDIO: &str = "Speech Studio";
const SERVER_NAME: &str = "speech-studio";

/// The protocol revisions the studio speaks: the stateless one, where every
/// request carries its version, and the handshake ones older clients open
/// with `initialize`.
const MODERN: &[&str] = &["2026-07-28"];
const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];
const META_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
/// How long a client may keep the tool list, the prompts and the resources:
/// they change only with the studio, but an update should show within minutes.
const LIST_TTL_MS: u64 = 300_000;
const HEADER_MISMATCH: i64 = -32020;
const UNSUPPORTED_VERSION: i64 = -32022;

fn server_info() -> Value {
    json!({ "name": SERVER_NAME, "title": STUDIO, "version": app_version() })
}

fn supported_versions() -> Vec<&'static str> {
    MODERN.iter().chain(LEGACY).copied().collect()
}

/// A complete result, signed with the server's identity.
fn rpc(id: Value, mut result: Value) -> Response {
    if let Some(fields) = result.as_object_mut() {
        fields.entry("resultType").or_insert_with(|| "complete".into());
        let meta = fields.entry("_meta").or_insert_with(|| json!({}));
        meta["io.modelcontextprotocol/serverInfo"] = server_info();
    }
    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })).into_response()
}

fn rpc_error(id: Value, code: i64, message: String) -> Response {
    rpc_failure(StatusCode::OK, id, code, message, None)
}

fn rpc_failure(status: StatusCode, id: Value, code: i64, message: String, data: Option<Value>) -> Response {
    let mut error = json!({ "code": code, "message": message });
    if let Some(data) = data {
        error["data"] = data;
    }
    (status, Json(json!({ "jsonrpc": "2.0", "id": id, "error": error }))).into_response()
}

/// A list or a read a client may cache: the same for everyone, fresh for five minutes.
fn cacheable(mut result: Value) -> Value {
    result["ttlMs"] = LIST_TTL_MS.into();
    result["cacheScope"] = "public".into();
    result
}

/// A header value, with the Base64 sentinel form decoded.
fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    use base64::Engine;
    let raw = headers.get(name)?.to_str().ok()?;
    match raw.strip_prefix("=?base64?").and_then(|inner| inner.strip_suffix("?=")) {
        Some(encoded) => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok()),
        None => Some(raw.to_string()),
    }
}

/// Why a stateless request is refused, if it is: a version the studio does
/// not speak, or headers that do not say what its body says.
fn refused(headers: &HeaderMap, method: &str, params: &Value, version: &str) -> Option<(i64, String, Option<Value>)> {
    if !MODERN.contains(&version) {
        return Some((
            UNSUPPORTED_VERSION,
            "Unsupported protocol version".into(),
            Some(json!({ "supported": supported_versions(), "requested": version })),
        ));
    }
    let mismatch = |what: String| Some((HEADER_MISMATCH, format!("Header mismatch: {what}"), None));
    match header_value(headers, "mcp-protocol-version") {
        Some(value) if value == version => {}
        Some(value) => {
            return mismatch(format!(
                "MCP-Protocol-Version header value '{value}' does not match body value '{version}'"
            ))
        }
        None => return mismatch("the MCP-Protocol-Version header is missing".into()),
    }
    match header_value(headers, "mcp-method") {
        Some(value) if value == method => {}
        Some(value) => return mismatch(format!("Mcp-Method header value '{value}' does not match body value '{method}'")),
        None => return mismatch("the Mcp-Method header is missing".into()),
    }
    let named = match method {
        "tools/call" | "prompts/get" => params.get("name"),
        "resources/read" => params.get("uri"),
        _ => return None,
    }
    .and_then(Value::as_str)
    .unwrap_or_default();
    match header_value(headers, "mcp-name") {
        Some(value) if value == named => None,
        Some(value) => mismatch(format!("Mcp-Name header value '{value}' does not match body value '{named}'")),
        None => mismatch("the Mcp-Name header is missing".into()),
    }
}

/// A tool's answer: the text an agent reads, and the same data structured
/// when it is JSON and whole.
fn tool_result(id: Value, text: String, structured: Option<Value>, error: bool) -> Response {
    let mut result = json!({ "content": [{ "type": "text", "text": text }], "isError": error });
    if let Some(structured) = structured.filter(|value| value.is_object() || value.is_array()) {
        result["structuredContent"] = structured;
    }
    rpc(id, result)
}

fn tool_json(id: Value, value: Value) -> Response {
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    if text.len() > LIMIT {
        tool_result(id, cut(text), None, false)
    } else {
        tool_result(id, text, Some(value), false)
    }
}

/// The window's answer as a tool's: a picture it took as an image the agent sees, text as text,
/// and anything else as structured content.
fn window_reply(id: Value, result: Value) -> Response {
    let mut content = Vec::new();
    if let Some(image) = result.get("image").and_then(Value::as_str) {
        let mime = result.get("mime").and_then(Value::as_str).unwrap_or("image/png");
        content.push(json!({ "type": "image", "data": image, "mimeType": mime }));
    }
    let text = match result.get("text").and_then(Value::as_str) {
        Some(text) => text.to_string(),
        None if result.is_null() || result.get("image").is_some() => "Done.".into(),
        None => serde_json::to_string_pretty(&result).unwrap_or_default(),
    };
    content.push(json!({ "type": "text", "text": cut(text) }));
    let mut answer = json!({ "content": content, "isError": false });
    if result.get("image").is_none() && result.get("text").is_none() && (result.is_object() || result.is_array()) {
        answer["structuredContent"] = result;
    }
    rpc(id, answer)
}

/// The name a tool shows the user: its words, the first capitalised.
fn tool_title(name: &str) -> String {
    let words = name.replace('_', " ");
    let mut letters = words.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default()
}

pub async fn handle(headers: HeaderMap, body: axum::body::Bytes) -> Response {
    // origin_guard (на всём роутере) уже сверил Host и источник; второй проверки здесь нет, чтобы локальный
    // агент со своим Origin и телом application/json доходил до MCP.
    let Ok(message) = serde_json::from_slice::<Value>(&body) else {
        return rpc_failure(StatusCode::BAD_REQUEST, Value::Null, -32700, "Parse error".into(), None);
    };
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        return rpc_failure(
            StatusCode::BAD_REQUEST,
            Value::Null,
            -32600,
            "Invalid request: one JSON-RPC request or notification per POST".into(),
            None,
        );
    };
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    // a notification is only acknowledged
    if message.get("id").is_none() {
        return StatusCode::ACCEPTED.into_response();
    }
    // A request of the stateless revision says its version in _meta and is
    // checked against its headers; one without it is of the handshake era.
    if let Some(version) = params.get("_meta").and_then(|meta| meta.get(META_VERSION)).and_then(Value::as_str) {
        if let Some((code, text, data)) = refused(&headers, method, &params, version) {
            return rpc_failure(StatusCode::BAD_REQUEST, id, code, text, data);
        }
    }
    if method != "tools/call" {
        seen(method);
    }
    match method {
        "initialize" => {
            if let Some(name) = params
                .pointer("/clientInfo/name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|n| !n.is_empty())
            {
                name_agent(name);
            }
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or_default();
            let protocol = LEGACY.iter().find(|version| **version == asked).copied().unwrap_or(LEGACY[0]);
            rpc(
                id,
                json!({
                    "protocolVersion": protocol,
                    "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
                    "serverInfo": server_info(),
                    "instructions": INSTRUCTIONS,
                }),
            )
        }
        "server/discover" => rpc(
            id,
            cacheable(json!({
                "supportedVersions": supported_versions(),
                "capabilities": { "tools": {}, "resources": {}, "prompts": {} },
                "instructions": INSTRUCTIONS,
            })),
        ),
        "ping" => rpc(id, json!({})),
        "resources/list" => {
            let mut list = vec![
                json!({ "uri": SKILL_URI, "name": "studio skill", "title": "How to drive the studio", "description": "How to drive Speech Studio: every tool by area, the ground rules, step-by-step recipes.", "mimeType": "text/markdown" }),
                json!({ "uri": LANGUAGES_URI, "name": "languages", "title": "Languages", "description": "The language codes a project is voiced or recognised in: lang.", "mimeType": "application/json" }),
            ];
            list.extend(guides::resources());
            rpc(id, cacheable(json!({ "resources": list })))
        }
        "resources/templates/list" => rpc(id, cacheable(json!({ "resourceTemplates": [] }))),
        "resources/read" => {
            let uri = params.get("uri").and_then(Value::as_str).unwrap_or_default();
            match resource(uri) {
                Some((mime, text)) => rpc(
                    id,
                    cacheable(json!({ "contents": [{ "uri": uri, "mimeType": mime, "text": text }] })),
                ),
                None => rpc_error(id, -32002, format!("Resource not found: {uri}")),
            }
        }
        "prompts/list" => {
            let mut list = vec![
                json!({ "name": "studio", "title": "Studio skill", "description": "Load the studio's skill: the tools, the ground rules and the recipes." }),
            ];
            list.extend(guides::prompts());
            rpc(id, cacheable(json!({ "prompts": list })))
        }
        "prompts/get" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
            let text = match name {
                "studio" => Some(SKILL.to_string()),
                other => guides::prompt(other, &arguments),
            };
            match text {
                Some(text) => rpc(
                    id,
                    json!({ "messages": [{ "role": "user", "content": { "type": "text", "text": text } }] }),
                ),
                None => rpc_error(id, -32602, format!("Unknown prompt: {name}")),
            }
        }
        "tools/list" => {
            let list: Vec<Value> = tools().iter().map(|tool| json!({ "name": tool.name, "title": tool_title(tool.name), "description": tool.description, "inputSchema": (tool.schema)(), "annotations": annotations(tool.name) })).collect();
            rpc(id, cacheable(json!({ "tools": list })))
        }
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            seen(name);
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            let Some(tool) = tools().iter().find(|tool| tool.name == name) else {
                return rpc_error(id, -32602, format!("Unknown tool: {name}; tools/list names them all."));
            };
            let answer = run_for_agent(tool, &args).await;
            match answer {
                Ok(Answer::Json(value)) => tool_json(id, value),
                Ok(Answer::Text(text)) => tool_result(id, text, None, false),
                Ok(Answer::Image { data, mime, text }) => rpc(
                    id,
                    json!({ "content": [{ "type": "image", "data": data, "mimeType": mime }, { "type": "text", "text": text }], "isError": false }),
                ),
                Ok(Answer::Window(result)) => window_reply(id, result),
                Err(problem) => tool_result(id, problem, None, true),
            }
        }
        _ => rpc_failure(StatusCode::NOT_FOUND, id, -32601, format!("Method not found: {method}"), None),
    }
}

/// What an agent is told when it connects.
const INSTRUCTIONS: &str = "You drive Speech Studio on this computer: it voices text (Speech) and recognises speech and checks a voicing (Listening). Every tool runs the same code as a control of the studio, through the routes its window calls, and every change to projects, recordings, voices, models, keys and settings is written into its activity journal. Start with studio_status. Long work - voicing, export, recognition, downloads and link fetches - is a job: start it, then studio_wait with its job_id instead of polling. Look ids up instead of guessing them: voices_list, models_status, projects_list, url_fetches_list. Files on this computer are passed by path, and a sound a tool answers comes back as a file path. Before writing text for an engine read studio_guide: tags (each engine reads its own), podcast, audiobook. To work in front of the user, use the ui_* tools (ui_navigate opens speech or listening), editor_* for the open project and player_* to play it to the user; they need the window open, the rest works without it. studio_batch runs several tools in one call. The whole guide is the resource studio://skill (prompt 'studio'); prompts write_podcast and make_audiobook start those tasks.";

#[cfg(test)]
mod tests {
    use super::*;

    async fn request(method: &str, params: Value) -> Value {
        let body = json!({"jsonrpc":"2.0","id":7,"method":method,"params":params});
        let response = handle(HeaderMap::new(), body.to_string().into()).await;
        assert_eq!(response.status(), StatusCode::OK);
        serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 1 << 22).await.unwrap()).unwrap()
    }

    #[tokio::test]
    async fn mcp_and_chat_use_the_same_catalog_and_executor() {
        let listed = request("tools/list", json!({})).await;
        let list = listed["result"]["tools"].as_array().unwrap();
        assert_eq!(list.len(), tools().len());
        for chat in crate::app::agent::chat_tools() {
            let tool = list.iter().find(|tool| tool["name"] == chat["function"]["name"]).unwrap();
            assert_eq!(tool["inputSchema"], chat["function"]["parameters"]);
            assert_eq!(tool["description"], chat["function"]["description"]);
        }
        let args = json!({"topic":"tags"});
        let called = request("tools/call", json!({"name":"studio_guide","arguments":args})).await;
        let chat = crate::app::agent::chat_call("studio_guide", &args).await.unwrap();
        assert_eq!(called["id"], 7);
        assert_eq!(called["result"]["isError"], false);
        assert_eq!(called["result"]["content"][0]["text"], chat.text);
        assert!(called["result"]["_meta"]["io.modelcontextprotocol/serverInfo"].is_object());
        assert!(crate::app::agent::chat_call("assistant_chat", &json!({})).await.is_err());
        let failed = request("tools/call", json!({"name":"studio_guide","arguments":{"topic":"missing"}})).await;
        assert_eq!(failed["result"]["isError"], true);
    }

    #[tokio::test]
    async fn discovery_resources_and_window_images_keep_the_protocol_envelope() {
        let init = request("initialize", json!({"protocolVersion":"2025-11-25"})).await;
        assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
        let skill = request("resources/read", json!({"uri":SKILL_URI})).await;
        assert_eq!(skill["result"]["contents"][0]["text"], SKILL);
        let unknown = request("tools/call", json!({"name":"unknown"})).await;
        assert_eq!(unknown["error"]["code"], -32602);
        let response = window_reply(json!(8), json!({"image":"YWJj","mime":"image/jpeg","text":"Screen"}));
        let image: Value = serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 1 << 20).await.unwrap()).unwrap();
        assert_eq!(
            image["result"]["content"][0],
            json!({"type":"image","data":"YWJj","mimeType":"image/jpeg"})
        );
        assert_eq!(image["result"]["content"][1]["text"], "Screen");
    }
}
