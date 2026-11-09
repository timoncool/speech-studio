//! Roles for a Live conversation: ready ideas of who the agent plays for the person to practise with (an interviewer,
//! an investor, a sceptic). A role is a file `roles/<id>.json` in the studio's folder, a suggestion and not a frame:
//! the agent takes one as it is, changes any field of it for this conversation, speaks a role of its own that has no
//! file, or keeps a new one as a file. After the conversation its review is the person's numbers (pace, pauses,
//! fillers, how soon they answered) with what the agent noticed, kept beside the roles and compared with the review
//! before it of the same role.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use reqwest::StatusCode;
use serde_json::{json, Map, Value};

use crate::app::{refuse, AppState};

const FOLDER: &str = "roles";
const REVIEWS: &str = "reviews";

/// The roles the studio comes with; each is written into the folder once, where the person edits or removes it.
const STARTERS: [(&str, &str); 9] = [
    ("interviewer", include_str!("../../roles/interviewer.json")),
    ("investor", include_str!("../../roles/investor.json")),
    ("client", include_str!("../../roles/client.json")),
    ("skeptic", include_str!("../../roles/skeptic.json")),
    ("boss", include_str!("../../roles/boss.json")),
    ("difficult_colleague", include_str!("../../roles/difficult_colleague.json")),
    ("stage", include_str!("../../roles/stage.json")),
    ("language_partner", include_str!("../../roles/language_partner.json")),
    ("debate", include_str!("../../roles/debate.json")),
];
/// The starters already written into a folder are listed here, so one the person removed is not written again.
const WRITTEN: &str = ".starters.json";

const TURNS: u64 = 8;

/// What the agent is told with a role, in the language agents are addressed in.
const BRIEF: &str = "A role is a ready idea, not a frame: play it as written, change anything in it, or make one up with the person (live_role_set with role fields and no id; save true keeps it as a file) - any scenario you two can speak: a hard conversation, a language or diction drill, a speech exercise, an improvisation, a scene of a play with its cast and lines in fields of your own. A conversation role works best when you open with `opens`, ask one thing per turn in at most two sentences so the person does most of the talking, and stop after about `turns` turns. Afterwards live_role_review gives the person's numbers; add three of their phrases worth quoting, three things to change and one goal for the next round.";

/// The fields of a role that are text for the person, a string or a string per language.
const TEXTS: [&str; 4] = ["name", "goal", "persona", "opens"];
const LISTS: [&str; 2] = ["checks", "notes"];

struct Current {
    role: Value,
    /// The person's lines heard before the role was taken, and the second of the recording it was taken at.
    since_line: usize,
    since_secs: f64,
}

static CURRENT: Mutex<Option<Current>> = Mutex::new(None);

fn current() -> std::sync::MutexGuard<'static, Option<Current>> {
    CURRENT.lock().unwrap_or_else(|p| p.into_inner())
}

fn folder(root: &Path) -> PathBuf {
    root.join(FOLDER)
}

fn is_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// The starters the folder does not have yet are written into it; one written before and gone since was removed by
/// the person and stays removed, and a file of theirs is never written over.
fn write_starters(root: &Path) -> Result<(), String> {
    let dir = folder(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let list = dir.join(WRITTEN);
    let mut written: Vec<String> = match std::fs::read(&list) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", list.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("{}: {e}", list.display())),
    };
    let before = written.len();
    for (id, body) in STARTERS {
        if written.iter().any(|w| w == id) {
            continue;
        }
        let file = dir.join(format!("{id}.json"));
        if !file.exists() {
            crate::platform::fs::write_atomic(&file, body.as_bytes())?;
        }
        written.push(id.to_string());
    }
    if written.len() != before {
        let bytes = serde_json::to_vec(&written).map_err(|e| e.to_string())?;
        crate::platform::fs::write_atomic(&list, &bytes)?;
    }
    Ok(())
}

/// A text of a role in `lang`: the string itself, or of a string per language that language's, then English's, then
/// any.
fn local(v: &Value, lang: &str) -> Option<String> {
    let text = match v {
        Value::String(s) => Some(s.as_str()),
        Value::Object(by) => by.get(lang).or_else(|| by.get("en")).or_else(|| by.values().next()).and_then(Value::as_str),
        _ => None,
    };
    text.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

/// The role as it is spoken in `lang`: its texts in that language, its numbers within their bounds. A role needs only
/// a name; every other field is the author's to give.
fn resolved(raw: &Value, id: &str, lang: &str) -> Result<Value, String> {
    let fields = raw.as_object().ok_or_else(|| t!("role-not-object"))?;
    // a field the studio does not know (a situation, objections to raise) is the author's and goes to the agent as it is
    let mut out = fields.clone();
    out.insert("id".into(), json!(id));
    for key in TEXTS {
        out.insert(key.into(), json!(fields.get(key).and_then(|v| local(v, lang))));
    }
    if out["name"].is_null() {
        return Err(t!("role-no-name"));
    }
    for key in LISTS {
        let items: Vec<String> = fields.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(|v| local(v, lang)).collect()).unwrap_or_default();
        out.insert(key.into(), json!(items));
    }
    out.insert("turns".into(), json!(fields.get("turns").and_then(Value::as_u64).unwrap_or(TURNS).clamp(1, 100)));
    out.insert("difficulty".into(), json!(fields.get("difficulty").and_then(Value::as_u64).unwrap_or(1).clamp(1, 3)));
    let own_lang = fields.get("lang").and_then(Value::as_str).map(str::trim).filter(|l| !l.is_empty() && *l != "auto");
    out.insert("lang".into(), json!(own_lang.unwrap_or(lang)));
    out.insert("voice".into(), json!(fields.get("voice").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty())));
    Ok(Value::Object(out))
}

fn read_file(file: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", file.display()))
}

fn role_file(root: &Path, id: &str) -> PathBuf {
    folder(root).join(format!("{id}.json"))
}

/// The role `id` of the folder as its file has it.
fn load(root: &Path, id: &str) -> Result<Value, String> {
    if !is_id(id) {
        return Err(t!("role-bad-id", id = id.to_string()));
    }
    let file = role_file(root, id);
    if !file.exists() {
        return Err(t!("role-unknown", id = id.to_string()));
    }
    read_file(&file)
}

/// Every role of the folder in `lang`, by name; a file that cannot be read as a role is listed with its `problem`
/// instead of being left out.
pub fn list(root: &Path, lang: &str) -> Result<Vec<Value>, String> {
    write_starters(root)?;
    let dir = folder(root);
    let kept = review_files(root)?;
    let mut roles = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        let Some(id) = path.file_stem().and_then(|s| s.to_str()).filter(|_| path.extension().is_some_and(|x| x == "json")) else { continue };
        if id.starts_with('.') {
            continue;
        }
        let read = if is_id(id) { read_file(&path).and_then(|raw| resolved(&raw, id, lang)) } else { Err(t!("role-bad-id", id = id.to_string())) };
        let mut role = match read {
            Ok(role) => role,
            Err(problem) => json!({ "id": id, "name": id, "problem": problem }),
        };
        role["file"] = json!(path);
        role["starter"] = json!(STARTERS.iter().any(|(s, _)| *s == id));
        role["last_review"] = match last_review(&kept, id) {
            None => Value::Null,
            Some(Ok(r)) => json!({ "at": r["at"], "goal": r["goal"], "wpm": r["metrics"]["wpm"], "fillers_per_100_words": r["metrics"]["fillers_per_100_words"] }),
            Some(Err(problem)) => json!({ "problem": problem }),
        };
        roles.push(role);
    }
    roles.sort_by_key(|r| (r["starter"] != true, r["name"].as_str().unwrap_or_default().to_lowercase()));
    Ok(roles)
}

/// How many turns the studio took since `since`: its stretches of speech with a line of the person between them.
fn studio_turns(studio: &[(f64, f64)], person: &[f64], since: f64) -> usize {
    let mut turns = 0;
    let mut last: Option<f64> = None;
    for &(from, _) in studio.iter().filter(|(from, _)| *from >= since) {
        let answered = last.is_none_or(|before| person.iter().any(|&p| p > before && p < from));
        if answered {
            turns += 1;
        }
        last = Some(from);
    }
    turns
}

/// The person's lines of the conversation said under the role, and how many turns the studio took in it.
fn conversation(cur: &Current) -> (Vec<Value>, usize) {
    let state = crate::listening::recognition::state_with_words();
    let (all, _) = super::say::person_lines(&state);
    let studio = super::metrics::studio_speech();
    let starts: Vec<f64> = all.iter().filter(|l| l["echo"] != true).filter_map(|l| l["start"].as_f64()).collect();
    let turns = studio_turns(&studio, &starts, cur.since_secs);
    (all.into_iter().skip(cur.since_line).collect(), turns)
}

/// The role in play as the agent and the window see it, None without one.
pub fn playing() -> Option<Value> {
    let guard = current();
    let cur = guard.as_ref()?;
    let (lines, turns) = conversation(cur);
    let mut role = cur.role.clone();
    role["turns_taken"] = json!(turns);
    role["person_lines"] = json!(lines.len());
    role["brief"] = json!(BRIEF);
    Some(role)
}

/// A conversation that starts keeps the role and counts its lines and turns from the start.
pub fn new_conversation() {
    if let Some(cur) = current().as_mut() {
        cur.since_line = 0;
        cur.since_secs = 0.0;
    }
}

/// The role taken for the conversation: the file's `id` with `changes` over it, or `changes` alone as a role of the
/// agent's own. `keep` writes it into the folder under its id.
fn take(root: &Path, id: Option<&str>, changes: Option<&Value>, lang: &str, keep: bool) -> Result<Value, String> {
    write_starters(root)?;
    let mut raw = match id {
        Some(id) => load(root, id)?,
        None => json!({}),
    };
    if let Some(changes) = changes {
        let changes = changes.as_object().ok_or_else(|| t!("role-not-object"))?;
        let fields = raw.as_object_mut().ok_or_else(|| t!("role-not-object"))?;
        for (key, value) in changes {
            fields.insert(key.clone(), value.clone());
        }
    }
    let own_id = raw.get("id").and_then(Value::as_str).map(str::to_string).or_else(|| id.map(str::to_string)).unwrap_or_else(|| "custom".into());
    if !is_id(&own_id) {
        return Err(t!("role-bad-id", id = own_id));
    }
    raw["id"] = json!(own_id);
    let role = resolved(&raw, &own_id, lang)?;
    if keep {
        let bytes = serde_json::to_vec_pretty(&raw).map_err(|e| e.to_string())?;
        crate::platform::fs::write_atomic(&role_file(root, &own_id), &bytes)?;
    }
    let state = crate::listening::recognition::state_with_words();
    let since_line = super::say::person_lines(&state).0.len();
    let since_secs = crate::listening::recognition::recording_secs(std::time::Instant::now()).unwrap_or(0.0).max(0.0);
    *current() = Some(Current { role, since_line, since_secs });
    Ok(playing().expect("a role was just taken"))
}

/// The role's file goes to the system's bin; the role in play is ended when it is that one. Its reviews stay.
fn remove(root: &Path, id: &str) -> Result<(), String> {
    load(root, id)?;
    let file = role_file(root, id);
    trash::delete(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut cur = current();
    if cur.as_ref().is_some_and(|c| c.role["id"] == id) {
        *cur = None;
    }
    Ok(())
}

fn reviews(root: &Path) -> PathBuf {
    folder(root).join(REVIEWS)
}

/// Every review kept, by the name without its extension: the reviews of a role in the order they were made (the second
/// of one second, `-2`, after the first). No folder yet: none.
fn review_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = reviews(root);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", dir.display())),
    };
    let mut files: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort_by_key(|f| f.file_stem().map(|s| s.to_os_string()));
    Ok(files)
}

/// The latest review of role `id` among `files`; one that cannot be read is told, not passed over.
fn last_review(files: &[PathBuf], id: &str) -> Option<Result<Value, String>> {
    let prefix = format!("{id}-");
    files
        .iter()
        .rev()
        .find(|p| p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.strip_prefix(&prefix)).is_some_and(|stamp| stamp.starts_with(|c: char| c.is_ascii_digit())))
        .map(|f| read_file(f))
}

/// What is compared between two reviews of a role: the number's place in the metrics.
const COMPARED: [(&str, &[&str]); 6] = [
    ("wpm", &["wpm"]),
    ("fillers_per_100_words", &["fillers_per_100_words"]),
    ("pauses_500", &["pauses_500"]),
    ("pauses_1000", &["pauses_1000"]),
    ("start_latency_median_ms", &["start_latency", "median_ms"]),
    ("words", &["words"]),
];

fn change(now: &Value, before: &Value) -> Value {
    let at = |m: &Value, path: &[&str]| path.iter().fold(m.clone(), |v, key| v[*key].clone()).as_f64();
    let mut out = Map::new();
    for (name, path) in COMPARED {
        let (n, b) = (at(now, path), at(before, path));
        out.insert(name.into(), json!({ "now": n, "before": b, "delta": n.zip(b).map(|(n, b)| ((n - b) * 100.0).round() / 100.0) }));
    }
    Value::Object(out)
}

fn review_file(dir: &Path, id: &str) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut path = dir.join(format!("{id}-{stamp}.json"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{id}-{stamp}-{n}.json"));
        n += 1;
    }
    path
}

fn texts(body: &Value, key: &str) -> Vec<String> {
    body.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()).unwrap_or_default()
}

/// The review of the conversation held under the role in play: the person's numbers over their lines, their lines to
/// quote, what the agent noticed (`quotes`, `fixes`, `goal`), and the change against the review before it of the same
/// role. Kept as a file of its own beside the roles.
pub fn review(root: &Path, workspace: &Path, noticed: &Value) -> Result<Value, String> {
    let (role, lines, turns) = {
        let guard = current();
        let cur = guard.as_ref().ok_or_else(|| t!("role-none"))?;
        let (lines, turns) = conversation(cur);
        (cur.role.clone(), lines, turns)
    };
    if !lines.iter().any(|l| l["echo"] != true) {
        return Err(t!("role-review-no-lines"));
    }
    let fillers = crate::audio::fillers::list(workspace)?;
    let metrics = super::metrics::measure(&lines, &super::metrics::studio_speech(), &fillers);
    let id = role["id"].as_str().unwrap_or("custom").to_string();
    let dir = reviews(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = review_file(&dir, &id);
    let before = last_review(&review_files(root)?, &id).transpose()?;
    let said: Vec<Value> = lines.iter().filter(|l| l["echo"] != true).map(|l| json!({ "n": l["n"], "text": l["text"], "start": l["start"] })).collect();
    let mut out = json!({
        "role": id,
        "name": role["name"],
        "at": chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
        "turns_taken": turns,
        "turns": role["turns"],
        "checks": role["checks"],
        "metrics": metrics,
        "lines": said,
        "quotes": texts(noticed, "quotes"),
        "fixes": texts(noticed, "fixes"),
        "goal": noticed.get("goal").and_then(Value::as_str).map(str::trim).filter(|g| !g.is_empty()),
        "before": before.as_ref().map(|b| json!({ "at": b["at"], "goal": b["goal"], "change": change(&metrics, &b["metrics"]) })),
    });
    let bytes = serde_json::to_vec_pretty(&out).map_err(|e| e.to_string())?;
    crate::platform::fs::write_atomic(&file, &bytes)?;
    out["file"] = json!(file);
    Ok(out)
}

fn window_lang() -> String {
    crate::i18n::language().language.as_str().to_string()
}

/// GET /speech/roles — `live_role_list`: the roles of the studio's folder in the window's language (`?lang=` another),
/// and the one in play.
pub async fn roles(State(st): State<AppState>, axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>) -> Response {
    let lang = q.get("lang").filter(|l| !l.is_empty()).cloned().unwrap_or_else(window_lang);
    let root = st.repo_root.clone();
    match tokio::task::spawn_blocking(move || list(&root, &lang)).await {
        Ok(Ok(roles)) => Json(json!({ "folder": folder(&st.repo_root), "roles": roles, "playing": playing(), "brief": BRIEF })).into_response(),
        Ok(Err(e)) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "bad_roles", e),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
    }
}

/// POST /speech/role {id?, role?, lang?, save?, remove?} — `live_role_set`: the role `id` of the folder, with the fields
/// of `role` over it; `role` alone is a role of the caller's own; neither ends the role in play; `remove` with an `id`
/// puts that role's file into the bin.
pub async fn role_set(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let id = body.get("id").and_then(Value::as_str).map(str::trim).filter(|i| !i.is_empty()).map(str::to_string);
    let changes = body.get("role").filter(|r| !r.is_null()).cloned();
    if let Some(id) = id.clone().filter(|_| body.get("remove").and_then(Value::as_bool) == Some(true)) {
        let root = st.repo_root.clone();
        return match tokio::task::spawn_blocking(move || remove(&root, &id)).await {
            Ok(Ok(())) => {
                crate::app::publish(json!({ "changed": "role" }));
                Json(json!({ "removed": true, "playing": playing() })).into_response()
            }
            Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "bad_role", e),
            Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
        };
    }
    if id.is_none() && changes.is_none() {
        *current() = None;
        crate::app::publish(json!({ "changed": "role" }));
        return Json(json!({ "playing": Value::Null })).into_response();
    }
    let lang = body.get("lang").and_then(Value::as_str).filter(|l| !l.is_empty()).map(str::to_string).unwrap_or_else(window_lang);
    let keep = body.get("save").and_then(Value::as_bool).unwrap_or(false);
    let root = st.repo_root.clone();
    match tokio::task::spawn_blocking(move || take(&root, id.as_deref(), changes.as_ref(), &lang, keep)).await {
        Ok(Ok(role)) => {
            crate::app::publish(json!({ "changed": "role" }));
            Json(json!({ "playing": role })).into_response()
        }
        Ok(Err(e)) => refuse(StatusCode::BAD_REQUEST, "bad_role", e),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
    }
}

/// POST /speech/role/review {quotes?, fixes?, goal?} — `live_role_review`.
pub async fn role_review(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    let (root, workspace) = (st.repo_root.clone(), st.workspace.clone());
    match tokio::task::spawn_blocking(move || review(&root, &workspace, &body)).await {
        Ok(Ok(made)) => {
            crate::app::publish(json!({ "changed": "role" }));
            Json(made).into_response()
        }
        Ok(Err(e)) => refuse(StatusCode::CONFLICT, "no_review", e),
        Err(e) => refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nine roles come with the studio, each complete in every language of the window: a name, a goal, a first line,
    /// what counts as success and what to avoid.
    #[test]
    fn the_studio_comes_with_nine_roles_in_every_language() {
        let _lang = crate::i18n::test_language("en");
        let dir = tempfile::tempdir().unwrap();
        for lang in crate::i18n::LANGUAGES {
            let roles = list(dir.path(), lang).unwrap();
            assert_eq!(roles.len(), 9);
            for role in &roles {
                assert!(role["problem"].is_null(), "{role}");
                let raw = read_file(Path::new(role["file"].as_str().unwrap())).unwrap();
                for key in ["name", "goal", "opens"] {
                    assert!(raw[key][lang].as_str().is_some_and(|s| !s.is_empty()), "{} has no {key} in {lang}", role["id"]);
                    assert!(role[key].as_str().is_some_and(|s| !s.is_empty()));
                }
                for key in LISTS {
                    assert!(raw[key].as_array().unwrap().iter().all(|item| item[lang].is_string()), "{} {key} in {lang}", role["id"]);
                    assert!(role[key].as_array().unwrap().len() >= 2);
                }
                assert!(role["persona"].is_string() && (1..=3).contains(&role["difficulty"].as_u64().unwrap()));
            }
        }
        let ids: Vec<&str> = STARTERS.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, ["interviewer", "investor", "client", "skeptic", "boss", "difficult_colleague", "stage", "language_partner", "debate"]);
    }

    /// The folder is the person's: a role they changed is not written over, one they removed does not come back, one
    /// they added is listed, and a file that is not a role is listed with what is wrong with it.
    #[test]
    fn the_roles_folder_is_the_person_s() {
        let _lang = crate::i18n::test_language("en");
        let dir = tempfile::tempdir().unwrap();
        list(dir.path(), "en").unwrap();
        std::fs::write(role_file(dir.path(), "boss"), r#"{"name":"My boss","turns":3}"#).unwrap();
        std::fs::remove_file(role_file(dir.path(), "debate")).unwrap();
        std::fs::write(role_file(dir.path(), "dentist"), r#"{"name":{"ru":"Стоматолог","en":"Dentist"},"goal":"Explain the bill."}"#).unwrap();
        std::fs::write(role_file(dir.path(), "broken"), "{").unwrap();
        let roles = list(dir.path(), "ru").unwrap();
        let by = |id: &str| roles.iter().find(|r| r["id"] == id);
        assert_eq!(by("boss").unwrap()["name"], "My boss");
        assert_eq!(by("boss").unwrap()["turns"], 3);
        assert!(by("debate").is_none());
        assert_eq!(by("dentist").unwrap()["name"], "Стоматолог");
        assert_eq!(by("dentist").unwrap()["goal"], "Explain the bill.");
        assert!(by("broken").unwrap()["problem"].is_string());
    }

    /// A role is a suggestion: the agent takes one with its own changes over it, speaks one of its own with no file,
    /// keeps it as a file, and ends it.
    #[test]
    fn a_role_is_taken_changed_made_up_and_kept() {
        let _lang = crate::i18n::test_language("en");
        let _slot = crate::listening::recognition::TEST_SLOT.lock().unwrap_or_else(|p| p.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let taken = take(dir.path(), Some("investor"), Some(&json!({ "turns": 3, "opens": "Pitch me." })), "ru", false).unwrap();
        assert_eq!((taken["id"].as_str(), taken["turns"].as_u64(), taken["opens"].as_str(), taken["name"].as_str()), (Some("investor"), Some(3), Some("Pitch me."), Some("Инвестор")));
        assert_eq!(taken["turns_taken"], 0);
        assert!(taken["brief"].as_str().unwrap().contains("not a frame"));
        assert_eq!(read_file(&role_file(dir.path(), "investor")).unwrap()["turns"], 6, "the file is as it was");

        let own = take(dir.path(), None, Some(&json!({ "id": "landlord", "name": "Landlord", "goal": "Raise the rent." })), "en", true).unwrap();
        assert_eq!((own["id"].as_str(), own["turns"].as_u64(), own["opens"].as_str()), (Some("landlord"), Some(TURNS), None));
        assert!(list(dir.path(), "en").unwrap().iter().any(|r| r["id"] == "landlord"));
        assert_eq!(playing().unwrap()["id"], "landlord");

        assert!(take(dir.path(), Some("nobody"), None, "en", false).is_err());
        assert!(take(dir.path(), None, Some(&json!({ "goal": "No name." })), "en", false).is_err());
        assert!(take(dir.path(), None, Some(&json!({ "id": "../up", "name": "Up" })), "en", true).is_err());
        assert_eq!(playing().unwrap()["id"], "landlord", "a role refused leaves the one in play");
        *current() = None;
    }

    /// The studio's turns are its stretches of speech with the person's line between them: two phrases in a row are one
    /// turn, and what was said before the role was taken is not counted.
    #[test]
    fn the_studio_s_turns_are_counted_between_the_person_s_lines() {
        let studio = [(1.0, 3.0), (3.2, 5.0), (9.0, 11.0), (15.0, 16.0), (16.5, 18.0)];
        let person = [6.0, 12.0];
        assert_eq!(studio_turns(&studio, &person, 0.0), 3);
        assert_eq!(studio_turns(&studio, &person, 8.0), 2);
        assert_eq!(studio_turns(&[], &person, 0.0), 0);
    }

    /// Two reviews of one role are two files, and the second says how the person's numbers moved against the first.
    #[test]
    fn a_review_is_compared_with_the_one_before_it() {
        let now = json!({ "wpm": 140.0, "fillers_per_100_words": 2.0, "pauses_500": 3, "pauses_1000": 1, "words": 200, "start_latency": { "median_ms": 900 } });
        let before = json!({ "wpm": 120.0, "fillers_per_100_words": 4.5, "pauses_500": 5, "pauses_1000": 1, "words": 150, "start_latency": { "median_ms": null } });
        let moved = change(&now, &before);
        assert_eq!(moved["wpm"]["delta"], 20.0);
        assert_eq!(moved["fillers_per_100_words"]["delta"], -2.5);
        assert_eq!(moved["pauses_1000"]["delta"], 0.0);
        assert!(moved["start_latency_median_ms"]["delta"].is_null(), "no number before: no change named");

        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(reviews(dir.path())).unwrap();
        let first = review_file(&reviews(dir.path()), "boss");
        std::fs::write(&first, r#"{"at":"first","metrics":{}}"#).unwrap();
        let second = review_file(&reviews(dir.path()), "boss");
        assert_ne!(first, second);
        std::fs::write(&second, r#"{"at":"second","metrics":{}}"#).unwrap();
        std::fs::write(reviews(dir.path()).join("boss-of-all-20260101-000000.json"), r#"{"at":"another role"}"#).unwrap();
        let kept = review_files(dir.path()).unwrap();
        assert_eq!(last_review(&kept, "boss").unwrap().unwrap()["at"], "second");
        assert_eq!(last_review(&kept[..kept.iter().position(|f| f == &second).unwrap()], "boss").unwrap().unwrap()["at"], "first");
        assert!(last_review(&kept, "stage").is_none());
        std::fs::write(review_file(&reviews(dir.path()), "boss"), "{").unwrap();
        assert!(last_review(&review_files(dir.path()).unwrap(), "boss").unwrap().is_err(), "a review that cannot be read is told");
        assert!(review_files(&dir.path().join("nowhere")).unwrap().is_empty());
    }

    /// A review without a role in play, or with nothing the person said, is refused with the reason.
    #[test]
    fn a_review_needs_a_role_and_the_person_s_lines() {
        let _lang = crate::i18n::test_language("en");
        let _slot = crate::listening::recognition::TEST_SLOT.lock().unwrap_or_else(|p| p.into_inner());
        let dir = tempfile::tempdir().unwrap();
        *current() = None;
        assert_eq!(review(dir.path(), dir.path(), &json!({})).unwrap_err(), t!("role-none"));
        take(dir.path(), Some("stage"), None, "en", false).unwrap();
        assert_eq!(review(dir.path(), dir.path(), &json!({})).unwrap_err(), t!("role-review-no-lines"));
        *current() = None;
    }
}
