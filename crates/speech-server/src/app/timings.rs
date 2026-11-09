//! How long each step of speaking and listening took, kept for the agent and the person to compare engines: a phrase
//! recognised (which recogniser, how long the speech was, how long recognising it took, how long after the person
//! stopped the text came), a phrase said aloud (which engine and voice, from the text to the first sound, how long the
//! engine took) and an engine loaded. Only the latest ones are kept, in memory.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use axum::extract::Query;
use axum::Json;
use serde_json::{json, Value};

const KEPT: usize = 1000;

static LOG: Mutex<VecDeque<Value>> = Mutex::new(VecDeque::new());
/// When the text of a block said aloud arrived, by block: the start of the wait the person hears.
static ARRIVED: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

/// Adds one step; `kind` is asr, tts or load.
pub fn note(kind: &str, mut step: Value) {
    if let Value::Object(fields) = &mut step {
        fields.insert("kind".into(), json!(kind));
        fields.insert("at".into(), json!(chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()));
    }
    let mut log = LOG.lock().unwrap_or_else(|p| p.into_inner());
    log.push_back(step);
    while log.len() > KEPT {
        log.pop_front();
    }
}

/// The text of `block` arrived to be said aloud now.
pub fn arrived(block: &str) {
    let mut at = ARRIVED.lock().unwrap_or_else(|p| p.into_inner());
    let at = at.get_or_insert_with(HashMap::new);
    at.retain(|_, t| t.elapsed().as_secs() < 3600);
    at.insert(block.to_string(), Instant::now());
}

/// Milliseconds from the arrival of `block`'s text until now, when it was said aloud.
pub fn since_arrival(block: &str) -> Option<u64> {
    ARRIVED.lock().unwrap_or_else(|p| p.into_inner()).as_ref()?.get(block).map(|t| t.elapsed().as_millis() as u64)
}

/// The last `last` steps noted, oldest first: of `kind`, or of every kind.
pub fn recent_timings(kind: Option<&str>, last: usize) -> Vec<Value> {
    let log = LOG.lock().unwrap_or_else(|p| p.into_inner());
    let picked: Vec<&Value> = log.iter().filter(|s| kind.is_none_or(|k| s["kind"] == k)).collect();
    picked[picked.len().saturating_sub(last)..].iter().map(|s| (*s).clone()).collect()
}

/// GET /timings?kind=&last= — the latest steps, oldest first: every kind, or asr, tts or load; the last `last` (100).
pub async fn route(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let kind = q.get("kind").map(String::as_str).filter(|k| !k.is_empty());
    let last = q.get("last").and_then(|n| n.parse::<usize>().ok()).unwrap_or(100);
    let log = LOG.lock().unwrap_or_else(|p| p.into_inner());
    let picked: Vec<&Value> = log.iter().filter(|s| kind.is_none_or(|k| s["kind"] == k)).collect();
    let from = picked.len().saturating_sub(last);
    Json(json!({ "steps": picked[from..] }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_latest_steps_of_a_kind_come_back_oldest_first() {
        note("asr", json!({ "engine": "test-a", "recognise_ms": 1 }));
        note("tts", json!({ "engine": "test-b" }));
        note("asr", json!({ "engine": "test-c", "recognise_ms": 2 }));
        let Json(got) = route(Query([("kind".to_string(), "asr".to_string()), ("last".to_string(), "2".to_string())].into())).await;
        let engines: Vec<&str> = got["steps"].as_array().unwrap().iter().map(|s| s["engine"].as_str().unwrap()).collect();
        assert_eq!(&engines[engines.len() - 2..], ["test-a", "test-c"]);
        arrived("blk-test");
        assert!(since_arrival("blk-test").is_some());
        assert!(since_arrival("blk-none").is_none());
    }
}
