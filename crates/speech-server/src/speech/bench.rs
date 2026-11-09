//! Live's benchmark: numbers a person and an agent can compare between two runs. `latency` — the waits of the last
//! conversation steps (from the end of the person's speech to its text, from a phrase's text to its first sound);
//! `floor` — phrases marked as an interruption or not against the rule that raises one; `asr` — recordings with their
//! reference text against the recogniser. Each run is written as a JSON file of its own in the studio's `bench` folder.

use std::path::{Path, PathBuf};

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::{refuse, AppState};

/// The steps `latency` looks at when no number is asked for.
const LATENCY_STEPS: usize = 200;

/// A phrase of the floor set: what the person says over the studio (`text`, or `audio` — a recording recognised first)
/// and whether it is an interruption.
#[derive(Deserialize)]
struct FloorPhrase {
    #[serde(default)]
    text: String,
    #[serde(default)]
    audio: Option<PathBuf>,
    #[serde(default)]
    lang: String,
    interrupt: bool,
}

/// A recording of the asr set with the text it says.
#[derive(Deserialize)]
struct Spoken {
    audio: PathBuf,
    text: String,
    #[serde(default)]
    lang: String,
}

/// The floor set the studio carries: backchannels, coughs and stray words that must not stop it, and turns that must.
const FLOOR_SET: &[(&str, bool)] = &[
    ("угу", false),
    ("ага", false),
    ("да", false),
    ("кхм", false),
    ("хм", false),
    ("ну да", false),
    ("ага угу", false),
    ("да да", false),
    ("ок", false),
    ("так", false),
    ("mhm", false),
    ("yeah", false),
    ("uh-huh", false),
    ("okay", false),
    ("right", false),
    ("ха ха", false),
    ("блин", false),
    ("подожди", false),
    ("стоп", false),
    ("подожди, а сколько это стоит", true),
    ("нет, давай по-другому сделаем", true),
    ("стой, я хотел спросить про другое", true),
    ("погоди, это не то, что я имел в виду", true),
    ("а можно ещё раз с самого начала", true),
    ("слушай, давай вернёмся к первому вопросу", true),
    ("я не согласен с этим утверждением", true),
    ("wait, that is not what I asked", true),
    ("hold on, let me say something first", true),
    ("no, I meant the other option", true),
    ("can you go back to the beginning", true),
];

fn stats(values: &mut Vec<f64>) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    values.sort_by(f64::total_cmp);
    let at = |q: f64| values[((values.len() - 1) as f64 * q).round() as usize];
    json!({ "n": values.len(), "median_ms": at(0.5), "p95_ms": at(0.95), "max_ms": values[values.len() - 1] })
}

fn latency(last: usize) -> Result<Value, String> {
    let steps = crate::app::recent_timings(None, if last == 0 { LATENCY_STEPS } else { last });
    let pick = |kind: &str, field: &str| -> Vec<f64> { steps.iter().filter(|s| s["kind"] == kind).filter_map(|s| s[field].as_f64()).collect() };
    let (mut after, mut first, mut heard, mut wait) = (pick("asr", "after_speech_ms"), pick("tts", "first_ms"), pick("tts", "heard_ms"), pick("tts", "queue_wait_ms"));
    if after.is_empty() && first.is_empty() && heard.is_empty() {
        return Err(t!("bench-no-timings"));
    }
    let gaps_over = wait.iter().filter(|w| **w > super::say::GAP_MS as f64).count();
    Ok(json!({
        "steps": steps.len(),
        "after_speech_ms": stats(&mut after),
        "first_ms": stats(&mut first),
        "heard_ms": stats(&mut heard),
        "queue_wait_ms": stats(&mut wait),
        "gaps_over_300_ms": gaps_over,
    }))
}

fn set_file(repo_root: &Path, name: &str) -> PathBuf {
    repo_root.join("bench").join(name)
}

fn read_set<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Vec<T>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn floor(repo_root: &Path, models_root: &Path) -> Result<Value, String> {
    let file = set_file(repo_root, "floor.json");
    let (phrases, source): (Vec<FloorPhrase>, String) = if file.is_file() {
        (read_set(&file)?, file.display().to_string())
    } else {
        (FLOOR_SET.iter().map(|(text, interrupt)| FloorPhrase { text: text.to_string(), audio: None, lang: String::new(), interrupt: *interrupt }).collect(), "built-in".into())
    };
    if phrases.is_empty() {
        return Err(t!("bench-set-empty", path = file.display().to_string()));
    }
    let (mut false_cuts, mut missed, mut quiet, mut turns) = (0usize, 0usize, 0usize, 0usize);
    let mut rows = Vec::new();
    for p in &phrases {
        let heard = match &p.audio {
            Some(audio) => crate::app::recognition::transcribe(repo_root, models_root, audio, if p.lang.is_empty() { "auto" } else { &p.lang }, "bench")?.text,
            None => p.text.clone(),
        };
        let raised = super::say::would_take_floor(&heard);
        if p.interrupt {
            turns += 1;
            missed += usize::from(!raised);
        } else {
            quiet += 1;
            false_cuts += usize::from(raised);
        }
        rows.push(json!({ "heard": heard, "interrupt": p.interrupt, "raised": raised, "right": raised == p.interrupt }));
    }
    let share = |n: usize, of: usize| if of == 0 { Value::Null } else { json!((n as f64 * 1000.0 / of as f64).round() / 10.0) };
    Ok(json!({
        "set": source,
        "phrases": phrases.len(),
        "not_interruptions": quiet,
        "interruptions": turns,
        "false_cuts": false_cuts,
        "false_cuts_pct": share(false_cuts, quiet),
        "missed": missed,
        "missed_pct": share(missed, turns),
        "rows": rows,
    }))
}

fn asr(repo_root: &Path, models_root: &Path, rounds: usize) -> Result<Value, String> {
    let file = set_file(repo_root, "asr.json");
    if !file.is_file() {
        return Err(t!("bench-no-asr-set", path = file.display().to_string()));
    }
    let set: Vec<Spoken> = read_set(&file)?;
    if set.is_empty() {
        return Err(t!("bench-set-empty", path = file.display().to_string()));
    }
    let (mut errors, mut words) = (0usize, 0usize);
    let mut times = Vec::new();
    let mut rows = Vec::new();
    for round in 0..rounds.max(1) {
        for s in &set {
            let lang = if s.lang.is_empty() { "auto" } else { &s.lang };
            let started = std::time::Instant::now();
            let heard = crate::app::recognition::transcribe(repo_root, models_root, &s.audio, lang, "bench")?.text;
            times.push(started.elapsed().as_secs_f64() * 1000.0);
            if round > 0 {
                continue;
            }
            let (expected, said) = crate::audio::qc::said_alike(&s.text, &heard, lang)?;
            let (expected, said) = (crate::audio::qc::normalize(&expected), crate::audio::qc::normalize(&said));
            let (inserted, deleted, substituted) = crate::audio::qc::word_ops(&expected, &said);
            errors += inserted + deleted + substituted;
            words += expected.len();
            rows.push(json!({ "audio": s.audio, "words": expected.len(), "inserted": inserted, "deleted": deleted, "substituted": substituted, "heard": heard }));
        }
    }
    if words == 0 {
        return Err(t!("bench-set-empty", path = file.display().to_string()));
    }
    Ok(json!({
        "set": file.display().to_string(),
        "recordings": set.len(),
        "rounds": rounds.max(1),
        "words": words,
        "wer_pct": (errors as f64 * 10_000.0 / words as f64).round() / 100.0,
        "recognise_ms": stats(&mut times),
        "rows": rows,
    }))
}

/// A file for this run that no earlier run has: the kind and the time, and a number when two runs fall in one second.
fn run_file(dir: &Path, kind: &str) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut path = dir.join(format!("live-{kind}-{stamp}.json"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("live-{kind}-{stamp}-{n}.json"));
        n += 1;
    }
    path
}

/// Runs the benchmark `kind` (`asr`, `latency` or `floor`) and writes it as a file of its own in the studio's `bench`
/// folder; the answer is the run with the file's path. `rounds`: how many times `asr` hears each recording, and how
/// many of the last steps `latency` looks at (0 — its usual 200).
pub fn live_bench(repo_root: &Path, kind: &str, rounds: usize) -> Result<Value, String> {
    let models_root = crate::app::models_root(repo_root);
    let result = match kind {
        "latency" => latency(rounds)?,
        "floor" => floor(repo_root, &models_root)?,
        "asr" => asr(repo_root, &models_root, rounds)?,
        other => return Err(t!("bench-kind", kind = other.to_string())),
    };
    let dir = repo_root.join("bench");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = run_file(&dir, kind);
    let run = json!({ "kind": kind, "at": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(), "version": env!("CARGO_PKG_VERSION"), "result": result, "file": file.display().to_string() });
    let body = serde_json::to_vec_pretty(&run).map_err(|e| e.to_string())?;
    std::fs::write(&file, body).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(run)
}

/// POST /speech/bench {kind, rounds} — `live_bench`.
pub async fn route(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let kind = body.get("kind").and_then(Value::as_str).unwrap_or_default().to_string();
    let rounds = body.get("rounds").and_then(Value::as_u64).unwrap_or(0) as usize;
    let root = st.repo_root.clone();
    match tokio::task::spawn_blocking(move || live_bench(&root, &kind, rounds)).await {
        Ok(Ok(run)) => Json(run).into_response(),
        Ok(Err(e)) => refuse(StatusCode::CONFLICT, "bench_failed", e),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule that raises an interruption, on the studio's own set: no backchannel, cough or stray word stops the
    /// studio, and every turn of the set raises one.
    #[test]
    fn the_floor_rule_passes_the_studio_s_own_set() {
        let dir = tempfile::tempdir().unwrap();
        let run = floor(dir.path(), &dir.path().join("models")).unwrap();
        assert_eq!(run["set"], "built-in");
        assert_eq!(run["false_cuts"], 0, "{run}");
        assert_eq!(run["missed"], 0, "{run}");
        assert!(run["interruptions"].as_u64().unwrap() >= 10 && run["not_interruptions"].as_u64().unwrap() >= 15);
    }

    /// A set of the person's own in the bench folder is the one measured, and its mistakes are counted by kind.
    #[test]
    fn a_set_in_the_bench_folder_is_measured_with_its_mistakes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("bench")).unwrap();
        std::fs::write(
            set_file(dir.path(), "floor.json"),
            r#"[{"text":"угу","interrupt":false},{"text":"подожди, а сколько это стоит","interrupt":false},{"text":"стоп","interrupt":true}]"#,
        )
        .unwrap();
        let run = floor(dir.path(), &dir.path().join("models")).unwrap();
        assert_eq!((run["false_cuts"].as_u64(), run["missed"].as_u64()), (Some(1), Some(1)), "{run}");
        assert_eq!(run["false_cuts_pct"], 50.0);
        assert_eq!(run["missed_pct"], 100.0);
    }

    /// Two runs are two files, and a kind the benchmark does not have is refused by name.
    #[test]
    fn each_run_is_a_file_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let first = live_bench(dir.path(), "floor", 1).unwrap();
        let second = live_bench(dir.path(), "floor", 1).unwrap();
        assert_ne!(first["file"], second["file"]);
        for run in [&first, &second] {
            let saved: Value = serde_json::from_slice(&std::fs::read(run["file"].as_str().unwrap()).unwrap()).unwrap();
            assert_eq!(saved["result"]["phrases"], FLOOR_SET.len());
        }
        assert!(live_bench(dir.path(), "speed", 1).is_err());
        assert!(live_bench(dir.path(), "asr", 1).is_err(), "no asr set in the folder: told, not skipped");
    }

    /// The waits of the steps noted, as median and 95th percentile; with no step noted the run says so.
    #[test]
    fn latency_is_measured_over_the_steps_noted() {
        let mut values: Vec<f64> = (1..=100).map(f64::from).collect();
        let s = stats(&mut values);
        assert_eq!((s["n"].as_u64(), s["median_ms"].as_f64(), s["p95_ms"].as_f64(), s["max_ms"].as_f64()), (Some(100), Some(51.0), Some(95.0), Some(100.0)));
        assert_eq!(stats(&mut Vec::new()), Value::Null);
    }
}
