//! Speech said aloud as it is written: an agent sends its text piece by piece while it writes it, each finished phrase
//! becomes a block of the session's project and is voiced at once (`aloud`), the window playing the sound as it comes.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use axum::extract::{Path as AxPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::app::refuse;
use crate::speech::project as sp;
use crate::{AppState, jobs};
use super::SpeechPaths;

/// A phrase longer than this is said up to its last comma rather than waiting for its full stop.
const LONG_PHRASE: usize = 300;

struct Session {
    buf: String,
    direction: String,
    /// Grows with every piece of text, so a later flush can tell no text came since.
    pieces: u64,
    /// The agent named no engine and no voice: each phrase takes the voice the person has chosen in Live by then.
    follows: bool,
}

/// A text stopped at the end of a phrase this long is taken as finished: an agent's last sentence has nothing after it.
const SETTLE_MS: u64 = 600;

/// The blocks a person's speaking cut off before they were said, and when.
static INTERRUPTED: LazyLock<Mutex<HashMap<String, std::time::Instant>>> = LazyLock::new(Default::default);

/// How long an interrupted block is still told as interrupted rather than waiting.
const INTERRUPTED_SECS: u64 = 3600;

static SESSIONS: LazyLock<Mutex<HashMap<String, Session>>> = LazyLock::new(Default::default);

/// What the studio said aloud lately: when, in which session, as which block and its text. A microphone hearing it back
/// is the studio's own voice, not a person, and a window may still be playing it after its session has ended.
static SAID: LazyLock<Mutex<Vec<(std::time::Instant, String, String, String)>>> = LazyLock::new(Default::default);

/// How long after its text came a session's phrase may still be playing in a window.
const ECHO_SECS: u64 = 30;

/// How long the text of a phrase said aloud is kept to tell its echo: a queue may sound it well after it came.
const SAID_KEEP_SECS: u64 = 120;

/// How far around the time the studio's phrase sounded a line heard may be its echo: the room sends it back a moment
/// late, and the recording's clock and the sound's agree only as closely as the window got the pieces.
const ECHO_SLACK: std::time::Duration = std::time::Duration::from_millis(1500);

/// Whether `heard`, said from `from` to `to`, is the studio's own speech coming back: it was heard while a phrase of the
/// studio sounded, and most of its words are that phrase's, in its order. A person repeating the studio's words after
/// it has finished is not its echo.
pub fn is_echo(heard: &str, from: std::time::Instant, to: std::time::Instant) -> bool {
    let words = crate::audio::qc::normalize(heard);
    if words.is_empty() {
        return false;
    }
    let texts: HashMap<String, String> = SAID.lock().unwrap_or_else(|p| p.into_inner()).iter().map(|(_, _, block, text)| (block.clone(), text.clone())).collect();
    let spoken: Vec<String> = super::live::aired_blocks()
        .into_iter()
        .filter(|a| a.from <= to + ECHO_SLACK && a.to + ECHO_SLACK >= from)
        .filter_map(|a| texts.get(&a.block).map(|t| crate::audio::qc::normalize(t)))
        .flatten()
        .collect();
    in_order(&words, &spoken) * 3 >= words.len() * 2
}

/// How many of `a`'s words `b` has in the same order (the longest common subsequence).
fn in_order(a: &[String], b: &[String]) -> usize {
    let mut row = vec![0usize; b.len() + 1];
    for x in a {
        let mut diagonal = 0;
        for (j, y) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if x == y { diagonal + 1 } else { above.max(row[j]) };
            diagonal = above;
        }
    }
    row[b.len()]
}

/// Words a listener says without taking the floor (backchannels): heard alone, they do not stop the studio's speech.
const BACKCHANNEL: &[&str] = &[
    "угу", "ага", "ага-ага", "да", "ну", "мм", "ммм", "хм", "ок", "окей", "так", "mm", "mmm", "mhm", "hmm", "uh", "um", "huh", "uh-huh",
    "yeah", "yes", "ok", "okay", "right", "good", "ха", "хах", "хаха", "хе", "хи", "кхм", "кхе", "кх", "ha", "haha", "he", "hehe",
    "ahem",
];

/// Whether what the person is saying over the studio is a turn of their own, as voice agents tell it apart (Pipecat's
/// `min_words`, LiveKit's `min_words`): two words at least, and not only backchannels. A cough, a laugh or the studio's
/// own non-speech sounds come out of recognition as a word or none and do not cut its speech off.
pub fn takes_the_floor(heard: &str) -> bool {
    let words = crate::audio::qc::normalize(heard);
    words.len() >= 2 && !words.iter().all(|w| BACKCHANNEL.contains(&w.as_str()))
}

/// A person started talking: every session stops speaking, as people stop when interrupted, the ended ones whose
/// sound may still play included. Answers whether there was a session to stop.
pub fn interrupt(workspace: &std::path::Path, queue: &jobs::JobQueue) -> bool {
    // the open sessions, and the ended ones whose last phrases a window may still be playing
    let mut open: Vec<String> = sessions().keys().cloned().collect();
    {
        let said = SAID.lock().unwrap_or_else(|p| p.into_inner());
        for (_, sid, _, _) in said.iter().filter(|(at, _, _, _)| at.elapsed().as_secs() < ECHO_SECS) {
            if !open.contains(sid) {
                open.push(sid.clone());
            }
        }
    }
    let speaking = !open.is_empty();
    for sid in open.iter() {
        mark_cut_off(workspace, sid);
        crate::speech::jobs::unqueue(sid, None);
        crate::speech::live::silence(sid);
    }
    let jobs = queue.clone();
    tokio::spawn(async move {
        for sid in open {
            if let Some((job, crate::jobs::JobKind::Synth)) = jobs.active_for(&sid).await {
                let _ = jobs.cancel(&job).await;
            }
        }
    });
    speaking
}

/// The phrases of session `sid` cut off now: those not voiced yet and the one whose sound is still playing.
fn mark_cut_off(workspace: &std::path::Path, sid: &str) {
    let now = std::time::Instant::now();
    let sounding: std::collections::HashSet<String> = super::live::aired_blocks().into_iter().filter(|a| a.project == sid && a.to > now).map(|a| a.block).collect();
    let mut cut = INTERRUPTED.lock().unwrap_or_else(|p| p.into_inner());
    cut.retain(|_, at| at.elapsed().as_secs() < INTERRUPTED_SECS);
    match crate::speech::load(workspace, sid) {
        Ok(project) => cut.extend(project.blocks().filter(|b| (b.takes.is_empty() && b.error.is_none()) || sounding.contains(&b.id)).map(|b| (b.id.clone(), now))),
        Err(e) => tracing::warn!("say: the phrases cut off in {sid} are not marked: {e}"),
    }
}

/// Whether the studio is saying something aloud now or has phrases on their way to be said: said lately, not played
/// yet, not cut off, and still to sound — voiced and a window there to play it, or being voiced (no take and no error
/// yet, its session open or its voicing under way). A phrase that failed has nothing to sound.
fn speaking(workspace: &std::path::Path) -> bool {
    if super::live::airing_until().is_some() {
        return true;
    }
    if !super::live::has_player() {
        return false;
    }
    let aired: std::collections::HashSet<String> = super::live::aired_blocks().into_iter().map(|a| a.block).collect();
    let on_way: Vec<(String, String)> = {
        let cut = INTERRUPTED.lock().unwrap_or_else(|p| p.into_inner());
        SAID.lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .filter(|(at, _, block, _)| at.elapsed().as_secs() < ECHO_SECS && !aired.contains(block) && !cut.contains_key(block))
            .map(|(_, sid, block, _)| (sid.clone(), block.clone()))
            .collect()
    };
    let mut projects: HashMap<String, Option<sp::SpeechProject>> = HashMap::new();
    on_way.iter().any(|(sid, block)| {
        let project = projects.entry(sid.clone()).or_insert_with(|| crate::speech::load(workspace, sid).ok());
        project.as_ref().and_then(|p| p.blocks().find(|b| &b.id == block)).is_some_and(|b| {
            b.error.is_none() && (!b.takes.is_empty() || sessions().contains_key(sid) || crate::speech::jobs::voicing(sid))
        })
    })
}

/// Whether `sid` is a session said aloud, open or lately ended.
pub(super) fn is_session(sid: &str) -> bool {
    sessions().contains_key(sid) || SAID.lock().unwrap_or_else(|p| p.into_inner()).iter().any(|(_, s, _, _)| s == sid)
}

fn sessions() -> std::sync::MutexGuard<'static, HashMap<String, Session>> {
    SESSIONS.lock().unwrap_or_else(|p| p.into_inner())
}

/// The phrases of `buf` that are finished, taken out of it: up to a full stop, a question or an exclamation mark, an
/// ellipsis or a line break followed by more, a long phrase up to its last comma, and with `end` whatever is left.
pub fn cut(buf: &mut String, end: bool) -> Vec<String> {
    let mut out = Vec::new();
    loop {
        let chars: Vec<(usize, char)> = buf.char_indices().collect();
        let mut at = None;
        for (i, &(pos, c)) in chars.iter().enumerate() {
            let next = chars.get(i + 1).map(|&(_, n)| n);
            let stop = matches!(c, '。' | '！' | '？') || (matches!(c, '.' | '!' | '?' | '…') && next.is_some_and(|n| n.is_whitespace() || matches!(n, '"' | '»' | ')' | '”')));
            if c == '\n' || stop {
                let close = chars[i + 1..].iter().take_while(|&&(_, n)| matches!(n, '"' | '»' | ')' | '”')).count();
                at = Some(chars.get(i + 1 + close).map_or(buf.len(), |&(p, _)| p).max(pos + c.len_utf8()));
                break;
            }
        }
        if at.is_none() && buf.chars().count() > LONG_PHRASE {
            at = buf.rfind([',', ';', ':', '，', '；']).map(|p| p + buf[p..].chars().next().map_or(1, char::len_utf8));
        }
        let Some(at) = at else { break };
        let phrase = buf[..at].trim().to_string();
        buf.replace_range(..at, "");
        if !phrase.is_empty() {
            out.push(phrase);
        }
    }
    if end && !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
        buf.clear();
    }
    out
}

/// Whether the text ends where a phrase does: its stop mark, maybe a closing quote, then nothing.
fn ends_a_phrase(buf: &str) -> bool {
    let t = buf.trim_end().trim_end_matches(['"', '»', ')', '”']);
    t.ends_with(['.', '!', '?', '…', '。', '！', '？'])
}

/// The voice the person chose for what agents say aloud (Live's sound window): engine, voice and sampling. An agent's
/// say_start takes each one it does not give itself.
#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct TalkVoice {
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub params: Option<crate::engines::tts_host::Params>,
}

fn talk_voice_path(workspace: &std::path::Path) -> std::path::PathBuf {
    workspace.join(".talk-voice.json")
}

fn talk_voice(workspace: &std::path::Path) -> TalkVoice {
    std::fs::read(talk_voice_path(workspace)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// GET /speech/say/voice — the voice chosen for speaking aloud; empty fields: the studio's default.
pub async fn voice_get(State(st): State<AppState>) -> Json<TalkVoice> {
    Json(talk_voice(&st.workspace))
}

/// POST /speech/say/voice {engine, voice?, params?} — choose the voice agents speak aloud with when they give none.
pub async fn voice_set(State(st): State<AppState>, headers: HeaderMap, Json(chosen): Json<TalkVoice>) -> Response {
    if !chosen.engine.is_empty() && crate::engines::get(&chosen.engine).is_none() {
        return refuse(StatusCode::BAD_REQUEST, "bad_engine", t!("speech-engine-unknown", engine = chosen.engine.clone()));
    }
    if let Some(p) = &chosen.params {
        if let Err(e) = p.checked() {
            return refuse(StatusCode::BAD_REQUEST, "bad_params", e);
        }
    }
    let body = match serde_json::to_vec_pretty(&chosen) {
        Ok(b) => b,
        Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "save_failed", e.to_string()),
    };
    if let Err(e) = std::fs::write(talk_voice_path(&st.workspace), body) {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, "save_failed", e.to_string());
    }
    crate::speech::live::voice_chosen();
    // the voice the person picks in the window is the one heard, whatever voice an agent opened its session with
    if !crate::app::agent::by_agent(&headers) {
        for session in sessions().values_mut() {
            session.follows = true;
        }
    }
    // a conversation going on takes the new voice at its next phrase: the engine loads now, not in that phrase
    if !chosen.engine.is_empty() && sessions().values().any(|s| s.follows) {
        let (paths, engine, voice) = (crate::speech::api::paths(&st), chosen.engine.clone(), chosen.voice.clone());
        tokio::task::spawn_blocking(move || {
            crate::speech::live::voice_loading(Some(&engine));
            if let Err(e) = crate::speech::jobs::warm(&paths, &engine, voice.as_deref(), "auto") {
                tracing::warn!("say: the new voice's engine was not warmed: {e}");
            }
            crate::speech::live::voice_loading(None);
        });
    }
    Json(chosen).into_response()
}

/// POST /speech/say {voice?, engine?, lang?, direction?, name?} — a session said aloud as its text comes; refused when no
/// window listens, there being nothing to play it on.
pub async fn start(State(st): State<AppState>, Json(body): Json<Value>) -> Response {
    if !crate::speech::live::listening() {
        return refuse(StatusCode::CONFLICT, "no_window", t!("speak-aloud-no-window"));
    }
    let field = |n: &str| body.get(n).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string);
    let chosen = talk_voice(&st.workspace);
    let own_engine = field("engine");
    let engine = own_engine.clone().or_else(|| Some(chosen.engine.clone()).filter(|e| !e.is_empty())).unwrap_or_else(|| crate::engines::qwen_tts::ENGINE.to_string());
    let Some(manifest) = crate::engines::get(&engine) else {
        return refuse(StatusCode::BAD_REQUEST, "bad_engine", t!("speech-engine-unknown", engine = engine));
    };
    let name = field("name").unwrap_or_else(|| t!("say-session-name", time = chrono::Local::now().format("%H:%M").to_string()));
    let mut project = sp::from_text(&name, "", &t!("speech-narrator"));
    project.chapters[0].blocks.clear();
    // the person's voice and sampling go with the person's engine: an agent naming another engine brings its own
    let theirs = own_engine.is_none() || own_engine.as_deref() == Some(chosen.engine.as_str());
    project.settings.voice = field("voice").or_else(|| if theirs { chosen.voice.clone() } else { None });
    project.settings.lang = field("lang").unwrap_or_else(|| "auto".into());
    project.settings.params.engine_defaults(&manifest);
    if let Some(p) = chosen.params.clone().filter(|_| theirs) {
        project.settings.params = p;
    }
    project.settings.engine = engine;
    project.settings.qc.asr_check = false;
    project.settings.qc.takes = 1;
    project.settings.qc.similarity = false;
    // a phrase said aloud is heard as it is made: no check after it, no second take of it
    project.settings.qc.heuristics = Vec::new();
    let pid = sp::new_id();
    if let Err(e) = crate::speech::save(&st.workspace, &pid, &project) {
        return refuse(StatusCode::INTERNAL_SERVER_ERROR, "save_failed", e);
    }
    let follows = own_engine.is_none() && field("voice").is_none();
    sessions().insert(pid.clone(), Session { buf: String::new(), direction: field("direction").unwrap_or_default(), pieces: 0, follows });
    let (paths, engine, voice, lang) = (crate::speech::api::paths(&st), project.settings.engine.clone(), project.settings.voice.clone(), project.settings.lang.clone());
    tokio::task::spawn_blocking(move || {
        crate::speech::live::voice_loading(Some(&engine));
        if let Err(e) = crate::speech::jobs::warm(&paths, &engine, voice.as_deref(), &lang) {
            tracing::warn!("say: the engine was not warmed before the first phrase: {e}");
        }
        crate::speech::live::voice_loading(None);
    });
    Json(json!({ "session": pid, "engine": project.settings.engine, "voice": project.settings.voice })).into_response()
}

/// POST /speech/say/{session} {text, end?} — more of the text; each finished phrase is voiced at once, `end` says the
/// rest and closes the session.
pub async fn text(State(st): State<AppState>, AxPath(sid): AxPath<String>, Json(body): Json<Value>) -> Response {
    if hushed() {
        return refuse(StatusCode::CONFLICT, "hushed", t!("say-hushed"));
    }
    let piece = body.get("text").and_then(Value::as_str).unwrap_or_default();
    let end = body.get("end").and_then(Value::as_bool).unwrap_or(false);
    let (phrases, direction, pieces, follows) = {
        let mut all = sessions();
        let Some(session) = all.get_mut(&sid) else {
            return refuse(StatusCode::NOT_FOUND, "no_session", t!("say-no-session", session = sid));
        };
        session.buf.push_str(piece);
        session.pieces += 1;
        let phrases = cut(&mut session.buf, end);
        let direction = session.direction.clone();
        let pieces = session.pieces;
        let follows = session.follows;
        if end {
            all.remove(&sid);
        }
        (phrases, direction, pieces, follows)
    };
    if !end {
        let (paths2, queue2, sid2) = (super::api::paths(&st), st.jobs.clone(), sid.clone());
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(SETTLE_MS)).await;
            let settled = {
                let mut all = sessions();
                match all.get_mut(&sid2) {
                    Some(s) if s.pieces == pieces && ends_a_phrase(&s.buf) => Some((cut(&mut s.buf, true), s.direction.clone())),
                    _ => None,
                }
            };
            if let Some((phrases, direction)) = settled {
                let _ = voice(paths2, queue2, sid2, phrases, direction, follows).await;
            }
        });
    }
    voice(super::api::paths(&st), st.jobs.clone(), sid, phrases, direction, follows).await
}

/// Puts the phrases into the session's project and voices them aloud; a session that follows the person's choice takes
/// the voice chosen in Live now.
async fn voice(paths: SpeechPaths, queue: jobs::JobQueue, sid: String, phrases: Vec<String>, direction: String, follows: bool) -> Response {
    if phrases.is_empty() {
        return Json(json!({ "voicing": 0 })).into_response();
    }
    let chosen = follows.then(|| talk_voice(&paths.workspace)).filter(|c| !c.engine.is_empty());
    let manifest = chosen.as_ref().and_then(|c| crate::engines::get(&c.engine));
    let speaker = t!("speech-narrator");
    let added = crate::speech::edit(&paths.workspace, &sid, |project| {
        if let (Some(c), Some(m)) = (&chosen, &manifest) {
            let s = &mut project.settings;
            if s.engine != c.engine || s.voice != c.voice {
                s.engine = c.engine.clone();
                s.voice = c.voice.clone();
                s.params.engine_defaults(m);
                if let Some(p) = c.params.clone() {
                    s.params = p;
                }
            }
        }
        phrases
            .iter()
            .map(|phrase| {
                let id = sp::new_id();
                crate::app::arrived(&id);
                project.chapters[0].blocks.push(sp::Block { id: id.clone(), speaker: speaker.clone(), text: phrase.clone(), direction: direction.clone(), ..Default::default() });
                id
            })
            .collect::<Vec<String>>()
    });
    let ids = match added {
        Ok(ids) => ids,
        Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "save_failed", e),
    };
    {
        let mut said = SAID.lock().unwrap_or_else(|p| p.into_inner());
        said.retain(|(at, _, _, _)| at.elapsed().as_secs() < SAID_KEEP_SECS);
        said.extend(ids.iter().zip(&phrases).map(|(id, p)| (std::time::Instant::now(), sid.clone(), id.clone(), p.clone())));
    }
    let started = super::synthesis::enqueue(&paths, &queue, sid.clone(), json!({ "ids": ids, "aloud": true })).await;
    if !started.status().is_success() {
        return started;
    }
    let job: Value = match axum::body::to_bytes(started.into_body(), usize::MAX).await.map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string())) {
        Ok(v) => v,
        Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e),
    };
    if let Some(job_id) = job["job_id"].as_str().map(str::to_string) {
        tokio::spawn(failed_job_marks_phrases(paths.workspace, queue, sid, job_id, ids.clone()));
    }
    Json(json!({ "voicing": ids.len(), "blocks": ids })).into_response()
}

/// A voicing job that fails as a whole (an engine without the language, a missing model) leaves its phrases unsaid with
/// its reason, so `state` says they failed instead of waiting for ever.
async fn failed_job_marks_phrases(workspace: std::path::PathBuf, queue: jobs::JobQueue, sid: String, job_id: String, ids: Vec<String>) {
    let end = loop {
        match queue.wait(&job_id, 300).await {
            Some(snap) if matches!(snap["status"].as_str(), Some("running" | "queued")) => continue,
            other => break other,
        }
    };
    let Some(error) = end.filter(|s| s["status"] == "error").and_then(|s| s["error"].as_str().map(str::to_string)) else { return };
    let marked = crate::speech::edit(&workspace, &sid, |project| {
        for id in &ids {
            if let Some(b) = project.block_mut(id).filter(|b| b.takes.is_empty() && b.error.is_none()) {
                b.error = Some(sp::BlockError { key: String::new(), text: error.clone() });
            }
        }
    });
    if let Err(e) = marked {
        tracing::error!("[ERROR] say: the failed phrases of {sid} were not marked: {e}");
    }
}

/// DELETE /speech/say — every session stops speaking now, as when the person interrupts.
pub async fn stop_all(State(st): State<AppState>, headers: HeaderMap) -> Json<Value> {
    if !crate::app::agent::by_agent(&headers) {
        *HUSHED_AT.lock().unwrap_or_else(|p| p.into_inner()) = Some(heard_count());
    }
    super::floor::agent_decides("stop");
    Json(json!({ "stopped": interrupt(&st.workspace, &st.jobs) }))
}

/// DELETE /speech/say/{session} — stop now: what is queued is dropped, the phrase being voiced is cancelled, the window
/// stops its sound; the person speaking over the studio is told as stopped.
pub async fn stop(State(st): State<AppState>, AxPath(sid): AxPath<String>) -> Response {
    super::floor::agent_decides("stop");
    sessions().remove(&sid);
    mark_cut_off(&st.workspace, &sid);
    crate::speech::jobs::unqueue(&sid, None);
    if let Some((job, crate::jobs::JobKind::Synth)) = st.jobs.active_for(&sid).await {
        let _ = st.jobs.cancel(&job).await;
    }
    crate::speech::live::silence(&sid);
    Json(json!({ "stopped": sid })).into_response()
}

/// GET /speech/say/{session}[?last=N] — what was said and how soon each phrase was heard, the silence heard before a
/// phrase that was not ready in time (`queue_wait_ms`), what waits, whether it is open; with `last`, only the last N
/// phrases.
pub async fn state(State(st): State<AppState>, AxPath(sid): AxPath<String>, axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>) -> Response {
    let project = match crate::speech::load(&st.workspace, &sid) {
        Ok(p) => p,
        Err(e) => return refuse(StatusCode::NOT_FOUND, "no_project", e),
    };
    let pending = sessions().get(&sid).map(|s| s.buf.clone());
    let last = q.get("last").and_then(|n| n.parse::<usize>().ok()).unwrap_or(usize::MAX);
    let skip = project.blocks().count().saturating_sub(last);
    let phrases: Vec<Value> = project
        .blocks()
        .skip(skip)
        .map(|b| {
            let take = b.takes.iter().find(|t| Some(t.n) == b.active);
            json!({ "id": b.id, "text": b.text, "state": phrase_state(b), "error": b.error.as_ref().map(|e| e.text.clone()), "first_ms": take.and_then(|t| t.first_ms), "heard_ms": take.and_then(|t| t.heard_ms), "queue_wait_ms": super::live::queue_wait_ms(&b.id), "seconds": take.map(|t| t.duration) })
        })
        .collect();
    let waits: Vec<u64> = phrases.iter().filter_map(|p| p["queue_wait_ms"].as_u64()).collect();
    Json(json!({
        "session": sid, "open": pending.is_some(), "waiting_text": pending.unwrap_or_default(), "phrases": phrases,
        "queue_wait_ms": waits.iter().max(), "gaps_over_ms": GAP_MS, "gaps_over": waits.iter().filter(|w| **w > GAP_MS).count(),
    }))
    .into_response()
}

/// What became of a phrase: failed (nothing of it sounds, a stop or not), interrupted, said or waiting.
fn phrase_state(b: &sp::Block) -> &'static str {
    let take = b.takes.iter().find(|t| Some(t.n) == b.active);
    if take.is_none() && b.error.is_some() {
        "failed"
    } else if INTERRUPTED.lock().unwrap_or_else(|p| p.into_inner()).contains_key(&b.id) {
        "interrupted"
    } else if take.is_some() {
        "said"
    } else {
        "waiting"
    }
}

/// A silence between two phrases longer than this is heard as the studio stopping mid-thought.
pub const GAP_MS: u64 = 300;

static LISTENING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The person stopped the speech in a window, when this many phrases had been heard: what an agent says next waits for
/// the person's next phrase, said or typed.
static HUSHED_AT: Mutex<Option<usize>> = Mutex::new(None);

/// How many phrases the person has said or typed in the running conversation, the studio's own voice heard back left out.
fn heard_count() -> usize {
    person_lines(&crate::listening::recognition::state()).0.iter().filter(|l| l["echo"] != true).count()
}

/// Whether the person stopped the speech and has not spoken or typed since. Fewer phrases than then: the conversation
/// was started again, and a new one does not keep the old one's stop.
fn hushed() -> bool {
    let mut at = HUSHED_AT.lock().unwrap_or_else(|p| p.into_inner());
    match *at {
        Some(n) if heard_count() == n => true,
        Some(_) => {
            *at = None;
            false
        }
        None => false,
    }
}

pub async fn listen_start(State(st): State<AppState>, headers: HeaderMap, body: Option<Json<Value>>) -> Response {
    let body = body.map(|Json(v)| v).unwrap_or(Value::Null);
    let lang = body.get("lang").and_then(Value::as_str).unwrap_or("auto");
    // the voice gate is armed while a Live conversation runs, when the person turned on the owner's-voice gate or set
    // a wake word; the live recogniser then keys each phrase on it
    let gate_cfg = crate::voice_gate::config(&st.models_root);
    // the echo guard `gate` keeps the studio from hearing itself by the owner's voice: the gate is armed for that too
    let guard = crate::speech::floor_settings::guard(&st.models_root);
    let owner_guard = guard == crate::speech::floor_settings::Guard::Gate;
    let gate_on = gate_cfg.enabled || owner_guard || gate_cfg.wake_word.as_deref().map(str::trim).is_some_and(|w| !w.is_empty());
    let guarding = json!({
        "mode": guard.as_str(),
        "owner_gate": owner_guard && crate::voice_gate::enrolled(&st.models_root),
        "problem": (owner_guard && !crate::voice_gate::enrolled(&st.models_root)).then(|| t!("echo-guard-no-owner")),
    });
    crate::voice_gate::guard_owner(owner_guard);
    // the microphone is on already (the person turned it on): this conversation listens to it too, and a recogniser
    // named here hears its next phrase while the microphone goes on recording
    if let Some(pid) = crate::listening::recording().filter(|_| crate::listening::recognition::state()["running"] == true) {
        let engine = body.get("engine").and_then(Value::as_str).filter(|e| !e.is_empty()).map(str::to_string);
        if let Some(engine) = engine {
            if !matches!(engine.as_str(), "parakeet" | "nemotron" | "whisper") {
                return refuse(StatusCode::BAD_REQUEST, "bad_engine", t!("listen-live-engine", engine = engine));
            }
            let (root, models, asked) = (st.repo_root.clone(), st.models_root.clone(), engine.clone());
            match tokio::task::spawn_blocking(move || crate::listening::recognition::switch(&root, &models, &asked)).await {
                Ok(Ok(label)) => crate::speech::live::asr_switched(&engine, &label),
                Ok(Err(e)) => return refuse(StatusCode::CONFLICT, "asr_switch_failed", e),
                Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e.to_string()),
            }
        }
        watch_interruptions(st.workspace, st.models_root, st.jobs);
        crate::voice_gate::arm(gate_on);
        let (asr, lang) = crate::listening::recognition::engine().unwrap_or_default();
        return Json(json!({ "pid": pid, "live": true, "joined": true, "asr": asr, "lang": lang, "gate": gate_on, "echo_guard": guarding })).into_response();
    }
    // the microphone is the person's: an agent listens to a conversation the person started and never turns it on
    if crate::app::agent::by_agent(&headers) {
        return refuse(StatusCode::CONFLICT, "mic_off", t!("say-mic-off"));
    }
    let started = crate::listening::http::record_start(State(st.clone()), HeaderMap::new(), Some(Json(json!({ "live": true, "lang": lang, "device": body.get("device"), "engine": body.get("engine") })))).await;
    if !started.status().is_success() {
        return started;
    }
    *HUSHED_AT.lock().unwrap_or_else(|p| p.into_inner()) = None;
    super::floor::new_conversation();
    watch_interruptions(st.workspace, st.models_root, st.jobs);
    crate::voice_gate::arm(gate_on);
    started
}

/// A one-shot partial this long is already clearly a turn, not a cough or a stray word, and cuts the studio off at once.
const FLOOR_WORDS_STRONG: usize = 4;

/// Whether a floor-taking phrase is confirmed ongoing speech rather than a brief cough or a one-off mis-recognition.
/// `base` is the word count when the phrase was first seen this turn (None on the first sighting). A short 2–3 word
/// phrase must GROW to count — a cough comes out as a short static partial that never grows and so is ignored — while a
/// phrase of `FLOOR_WORDS_STRONG` words or more is clearly a turn straight away.
fn floor_grew(base: Option<usize>, words: usize) -> bool {
    words >= FLOOR_WORDS_STRONG || matches!(base, Some(b) if words > b)
}

/// While a Live conversation runs, tells each time the person takes the floor over the studio's speech (`floor`): the
/// setting says whether the studio stops at once, the agent decides with the studio quieter meanwhile, or nobody stops it.
fn watch_interruptions(workspace: std::path::PathBuf, models_root: std::path::PathBuf, queue: jobs::JobQueue) {
    if LISTENING.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    tokio::spawn(async move {
        // the word count a floor-taking phrase was first seen at this turn, and whether this turn was told already
        let mut base: Option<usize> = None;
        let mut told = false;
        while LISTENING.load(std::sync::atomic::Ordering::Relaxed) {
            let state = crate::listening::recognition::state();
            let partial = state["partial"]["text"].as_str().unwrap_or_default().to_string();
            let now = std::time::Instant::now();
            let began = state["partial"]["start"].as_f64().and_then(crate::listening::recognition::heard_at).unwrap_or(now);
            if takes_the_floor(&partial) && !is_echo(&partial, began, now) {
                let words = crate::audio::qc::normalize(&partial).len();
                // a turn only once the person is really speaking over the studio: a cough is a short static partial
                // that never grows
                if !told && floor_grew(base, words) {
                    told = true;
                    if speaking(&workspace) {
                        let mode = super::floor::mode(&models_root);
                        let n = super::floor::raise(&partial, words, state["partial"]["start"].as_f64(), mode);
                        match mode {
                            super::floor::Mode::Always => {
                                interrupt(&workspace, &queue);
                            }
                            super::floor::Mode::Agent => {
                                let (w, q) = (workspace.clone(), queue.clone());
                                let wait = std::time::Duration::from_millis(super::floor::wait_ms(&models_root));
                                let ws = workspace.clone();
                                super::floor::hold(n, wait, move || speaking(&ws), move || {
                                    interrupt(&w, &q);
                                });
                            }
                            super::floor::Mode::Never => {}
                        }
                    }
                }
                if base.is_none() {
                    base = Some(words);
                }
            } else {
                // silence, a backchannel or a brief noise: the turn is over, arm again for the next one
                base = None;
                told = false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        }
    });
}

/// POST /speech/listen/mute {muted | toggle} — the microphone muted or back on (toggle: the other way round); the
/// conversation and what was heard go on. An agent only mutes it: turning it back on is the person's.
pub async fn listen_mute(headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let muted = match body.get("muted").and_then(Value::as_bool) {
        Some(m) => m,
        None if body.get("toggle").and_then(Value::as_bool) == Some(true) => !crate::listening::recognition::muted().unwrap_or(false),
        None => true,
    };
    if !muted && crate::app::agent::by_agent(&headers) {
        return refuse(StatusCode::FORBIDDEN, "mic_only_person", t!("mic-only-person"));
    }
    match crate::listening::recognition::mute(muted) {
        Ok(()) => Json(json!({ "muted": muted })).into_response(),
        Err(e) => refuse(StatusCode::CONFLICT, "not_listening", e),
    }
}

/// POST /speech/listen/text {text} — a phrase the person typed, given to the agent like one they said.
pub async fn listen_text(Json(body): Json<Value>) -> Response {
    let text = body.get("text").and_then(Value::as_str).map(str::trim).unwrap_or_default();
    if text.is_empty() {
        return refuse(StatusCode::BAD_REQUEST, "no_text", t!("say-no-text"));
    }
    match crate::listening::recognition::type_line(text) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => refuse(StatusCode::CONFLICT, "not_listening", e),
    }
}

/// The lines the agent gets from the recogniser's `state`, numbered from 1 in that list, and how many the voice gate held
/// back: every line but those the gate judged another voice (or heard before the wake word); what the person typed is
/// always theirs. The studio's own voice heard back stays in the list, marked `echo`. The list only grows, so a number
/// once given always names the same line.
pub(super) fn person_lines(state: &Value) -> (Vec<Value>, usize) {
    let all = state["lines"].as_array().map(Vec::as_slice).unwrap_or_default();
    let mut gated = 0;
    let mut lines = Vec::new();
    for l in all {
        if l["typed"] != true && l["owner"] == false {
            gated += 1;
            continue;
        }
        lines.push(json!({ "n": lines.len() + 1, "text": l["text"], "start": l["start"], "end": l["end"], "echo": l["echo"] == true, "typed": l["typed"] == true, "words": l["words"], "words_source": l["words_source"] }));
    }
    (lines, gated)
}

/// GET /speech/listen/next?after=N&seconds=S — the person's lines after the first N, waiting up to S seconds (at most 55)
/// for one; `heard` is how many lines there are to get, so the next call asks after it.
pub async fn listen_next(axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>) -> Json<Value> {
    let after: usize = q.get("after").and_then(|v| v.parse().ok()).unwrap_or(0);
    let seconds: u64 = q.get("seconds").and_then(|v| v.parse().ok()).unwrap_or(30).min(55);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    loop {
        let state = crate::listening::recognition::state();
        let (lines, gated) = person_lines(&state);
        let heard = lines.len();
        // word timings are voice_lines' to give: a turn of the conversation needs the words only
        let fresh: Vec<Value> = lines
            .into_iter()
            .skip(after)
            .map(|mut l| {
                if let Some(o) = l.as_object_mut() {
                    o.remove("words");
                }
                l
            })
            .collect();
        let running = state["running"].as_bool().unwrap_or(false);
        // a recogniser that broke hears nothing more: the agent learns it now, not after the wait
        let broken = !state["error"].is_null();
        if !fresh.is_empty() || !running || broken || std::time::Instant::now() >= until {
            return Json(json!({ "lines": fresh, "heard": heard, "gated": gated, "partial": state["partial"], "listening": running, "error": state["error"] }));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// DELETE /speech/listen — the microphone off; the conversation's recording and what was heard stay a Listening project.
/// An agent may turn it off too: only turning it on is the person's.
pub async fn listen_stop(State(st): State<AppState>) -> Response {
    LISTENING.store(false, std::sync::atomic::Ordering::Relaxed);
    crate::voice_gate::arm(false);
    crate::voice_gate::guard_owner(false);
    let stopped = crate::listening::http::record_stop(State(st)).await;
    if !stopped.status().is_success() {
        return stopped;
    }
    // the recording is a Listening project now: its id and how much was heard, not the whole project
    let made: Value = match axum::body::to_bytes(stopped.into_body(), usize::MAX).await.map_err(|e| e.to_string()).and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string())) {
        Ok(v) => v,
        Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "internal", e),
    };
    Json(json!({ "pid": made["pid"], "lines": made["project"]["lines"].as_array().map_or(0, Vec::len) })).into_response()
}

/// What the microphone does now, for the window's indicator: on or not, for what (live, listening, voice, test), for
/// how many seconds, and whether the conversation is muted.
pub fn mic_state() -> Value {
    let Some((test, on_for)) = crate::voices::record::in_use() else {
        return json!({ "on": false });
    };
    let listening = crate::listening::recording().is_some();
    let what = if listening && LISTENING.load(std::sync::atomic::Ordering::Relaxed) {
        "live"
    } else if listening {
        "listening"
    } else if test {
        "test"
    } else {
        "voice"
    };
    let (asr, lang) = crate::listening::recognition::engine().unzip();
    json!({ "on": true, "what": what, "seconds": on_for.as_secs(), "muted": crate::listening::recognition::muted().unwrap_or(false), "asr": asr, "lang": lang })
}

/// GET /mic/state — see `mic_state`.
pub async fn mic_state_route() -> Json<Value> {
    Json(mic_state())
}

/// The microphone stays on with no window of the studio open this long at most: nobody would see it is on.
const NO_WINDOW_SECS: u32 = 5;
static GUARDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// While the microphone is on, turns it off once no window of the studio has been open for `NO_WINDOW_SECS`.
pub fn guard_mic(workspace: std::path::PathBuf) {
    if GUARDED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    tokio::spawn(async move {
        let mut gone = 0;
        while crate::voices::record::in_use().is_some() {
            gone = if crate::speech::live::listening() { 0 } else { gone + 1 };
            if gone >= NO_WINDOW_SECS {
                tracing::warn!("no window of the studio is open: the microphone is turned off");
                LISTENING.store(false, std::sync::atomic::Ordering::Relaxed);
                if crate::listening::recording().is_some() {
                    let _ = crate::listening::stop_recording(&workspace).await;
                } else {
                    let _ = tokio::task::spawn_blocking(crate::voices::record::stop).await;
                }
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        GUARDED.store(false, std::sync::atomic::Ordering::Relaxed);
        if crate::voices::record::in_use().is_some() {
            guard_mic(workspace);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn finished_phrases_are_cut_and_the_rest_waits() {
        let mut buf = "Привет! Это первая фраза. А это ещё не".to_string();
        assert_eq!(cut(&mut buf, false), ["Привет!", "Это первая фраза."]);
        assert_eq!(buf, " А это ещё не");
        buf.push_str(" кончилась");
        assert!(cut(&mut buf, false).is_empty());
        assert_eq!(cut(&mut buf, true), ["А это ещё не кончилась"]);
        assert!(buf.is_empty());
    }

    #[test]
    fn a_cough_a_laugh_or_a_backchannel_does_not_cut_the_studio_off() {
        assert!(!takes_the_floor("Mm."));
        assert!(!takes_the_floor("Good."));
        assert!(!takes_the_floor("Ха"));
        assert!(!takes_the_floor("угу, ага"));
        assert!(!takes_the_floor("Ха-ха-ха!"));
        assert!(!takes_the_floor("Кхм, кхм."));
        assert!(!takes_the_floor(""));
        assert!(takes_the_floor("подожди, стоп"));
        assert!(takes_the_floor("Что случилось?"));
    }

    #[test]
    fn a_short_floor_taking_phrase_must_grow_to_cut_the_studio_off() {
        // a cough recognised as two static words never grows, so it does not interrupt — the bug the person hit
        assert!(!floor_grew(None, 2)); // first sighting of a short phrase: wait and see
        assert!(!floor_grew(Some(2), 2)); // still two words a poll later: a cough, left alone
        assert!(!floor_grew(Some(3), 3)); // three static words: still not confirmed
        assert!(floor_grew(Some(2), 3)); // it grew: the person is speaking over the studio
        assert!(floor_grew(None, FLOOR_WORDS_STRONG)); // a long phrase in one shot is clearly a turn at once
    }

    #[tokio::test]
    async fn an_agent_mutes_the_microphone_but_never_turns_it_back_on() {
        let _slot = crate::listening::recognition::TEST_SLOT.lock().unwrap_or_else(|p| p.into_inner());
        let mut agent = HeaderMap::new();
        agent.insert("x-speech-agent", "1".parse().unwrap());
        let unmute = listen_mute(agent.clone(), Json(json!({ "muted": false }))).await;
        assert_eq!(unmute.status(), StatusCode::FORBIDDEN);
        let mute = listen_mute(agent, Json(json!({ "muted": true }))).await;
        assert_eq!(mute.status(), StatusCode::CONFLICT, "muting is allowed: refused only because no conversation runs");
        assert_eq!(mic_state(), json!({ "on": false }));
    }

    #[test]
    fn a_number_with_a_dot_and_a_long_phrase_without_a_stop() {
        let mut buf = "Встреча в 10.30 у входа. Дальше".to_string();
        assert_eq!(cut(&mut buf, false), ["Встреча в 10.30 у входа."]);
        let mut whole = format!("{}, и ещё немного", "слово ".repeat(25));
        assert!(cut(&mut whole, false).is_empty(), "a sentence of 150 characters waits for its stop and is said whole");
        let mut long = format!("{}, и ещё немного", "слово ".repeat(60));
        let said = cut(&mut long, false);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].ends_with(','));
        assert_eq!(long.trim(), "и ещё немного");
    }

    #[test]
    fn the_studio_hearing_its_own_phrase_is_echo_and_a_person_is_not() {
        let now = Instant::now();
        let ago = |s: u64| now - Duration::from_secs(s);
        SAID.lock().unwrap().push((ago(12), "s-castle".into(), "b-castle".into(), "Старинный замок построили в тысяча восемьсот восемьдесят седьмом году.".into()));
        super::super::live::air_for_test("s-castle", "b-castle", ago(10), ago(5));
        assert!(is_echo("замок построили в тысяча восемьсот", ago(9), ago(6)), "heard while the phrase sounded, in its words");
        assert!(!is_echo("подожди, а кто его построил", ago(9), ago(6)), "the person speaking over the phrase");
        assert!(!is_echo("замок построили в тысяча восемьсот", ago(1), now), "the person repeating the studio's words after it");
        assert!(!is_echo("", ago(9), ago(6)));
    }

    /// Lines heard in a conversation whose second 0 was a minute ago, while the studio said a phrase from its 20th to
    /// its 26th second (scenarios 4 and 9 of TZ-LIVE-TRAINER).
    fn eight_phrases(now: Instant) -> Instant {
        let zero = now - Duration::from_secs(60);
        SAID.lock().unwrap().push((zero + Duration::from_secs(19), "s-eight".into(), "b-eight".into(), "Сегодня мы поговорим о том, как устроен голосовой тренажёр.".into()));
        super::super::live::air_for_test("s-eight", "b-eight", zero + Duration::from_secs(20), zero + Duration::from_secs(26));
        crate::listening::recognition::conversation_for_test(
            zero,
            &[
                (1.0, 3.0, "Привет, как дела", false, true),
                (4.0, 6.0, "Расскажи про тренажёр", false, true),
                (21.0, 24.0, "мы поговорим о том как устроен голосовой тренажёр", false, true),
                (22.0, 24.5, "подожди, а сколько это стоит", false, true),
                (30.0, 32.0, "Как устроен голосовой тренажёр", false, true),
                (33.0, 35.0, "это телевизор за стеной", false, false),
                (36.0, 38.0, "да, понятно", false, true),
                (39.0, 41.0, "А что дальше", false, true),
                (41.0, 41.0, "напечатал вопрос", true, true),
            ],
        );
        zero
    }

    async fn next(after: usize) -> Value {
        let q = HashMap::from([("after".to_string(), after.to_string()), ("seconds".to_string(), "0".to_string())]);
        listen_next(axum::extract::Query(q)).await.0
    }

    #[tokio::test]
    async fn every_line_heard_reaches_the_agent_and_echo_is_a_mark_not_a_gap() {
        let _slot = crate::listening::recognition::TEST_SLOT.lock().unwrap_or_else(|p| p.into_inner());
        eight_phrases(Instant::now());
        let all = next(0).await;
        let lines = all["lines"].as_array().unwrap();
        assert_eq!(all["heard"], lines.len(), "heard counts the lines there are to get: {all}");
        assert_eq!(lines.len(), 8, "every line but the one the voice gate held back: {all}");
        assert_eq!(all["gated"], 1);
        let numbers: Vec<u64> = lines.iter().map(|l| l["n"].as_u64().unwrap()).collect();
        assert_eq!(numbers, (1..=8).collect::<Vec<u64>>());
        let echo: Vec<bool> = lines.iter().map(|l| l["echo"] == true).collect();
        assert_eq!(echo, [false, false, true, false, false, false, false, false], "only the studio's phrase heard back while it sounded: {all}");
        assert_eq!(lines[4]["text"], "Как устроен голосовой тренажёр", "the person repeating the studio's words after it is the person");
        assert_eq!(lines[7]["typed"], true);
        for after in 0..=8 {
            let got = next(after).await;
            assert_eq!(got["heard"], 8);
            assert_eq!(got["lines"].as_array().unwrap().len(), 8 - after, "after {after}: {got}");
        }
        crate::listening::recognition::end_for_test();
    }

    #[tokio::test]
    async fn a_stop_in_the_window_holds_until_the_person_speaks_and_ends_with_the_conversation() {
        let _slot = crate::listening::recognition::TEST_SLOT.lock().unwrap_or_else(|p| p.into_inner());
        let zero = eight_phrases(Instant::now());
        assert_eq!(heard_count(), 7, "the person's lines, the studio's echo left out");
        *HUSHED_AT.lock().unwrap() = Some(heard_count());
        assert!(hushed(), "the person stopped the speech and said nothing since");
        // the microphone turned off and on: a new conversation, nothing heard in it yet
        crate::listening::recognition::conversation_for_test(zero, &[]);
        assert!(!hushed(), "a new conversation does not keep the old one's stop");
        assert_eq!(*HUSHED_AT.lock().unwrap(), None);
        *HUSHED_AT.lock().unwrap() = Some(heard_count());
        crate::listening::recognition::conversation_for_test(zero, &[(1.0, 2.0, "дальше", false, true)]);
        assert!(!hushed(), "the person's next phrase lets the agent speak");
        crate::listening::recognition::end_for_test();
    }

    /// Scenario 1: a stop cuts off the phrase whose sound still plays (voiced already) and the ones not voiced yet; the
    /// phrase heard to its end stays said.
    #[test]
    fn a_stop_marks_the_phrase_still_sounding_as_interrupted() {
        let dir = tempfile::tempdir().unwrap();
        let mut project = sp::from_text("talk", "", "n");
        project.chapters[0].blocks = ["Первая фраза.", "Вторая фраза.", "Третья фраза."].iter().map(|t| sp::Block { id: sp::new_id(), text: t.to_string(), ..Default::default() }).collect();
        let ids: Vec<String> = project.blocks().map(|b| b.id.clone()).collect();
        assert_eq!(ids.len(), 3, "{ids:?}");
        for id in &ids[..2] {
            let b = project.block_mut(id).unwrap();
            b.takes.push(sp::Take { n: 1, ..Default::default() });
            b.active = Some(1);
        }
        let sid = sp::new_id();
        crate::speech::save(dir.path(), &sid, &project).unwrap();
        let now = Instant::now();
        super::super::live::air_for_test(&sid, &ids[0], now - Duration::from_secs(9), now - Duration::from_secs(4));
        super::super::live::air_for_test(&sid, &ids[1], now - Duration::from_secs(4), now + Duration::from_secs(6));
        mark_cut_off(dir.path(), &sid);
        let cut = INTERRUPTED.lock().unwrap();
        assert!(!cut.contains_key(&ids[0]), "heard to its end");
        assert!(cut.contains_key(&ids[1]), "its sound was still playing");
        assert!(cut.contains_key(&ids[2]), "not voiced yet");
    }

    /// A phrase whose voicing failed has nothing to sound: a stop does not mark it interrupted, and it is told failed.
    #[test]
    fn a_failed_phrase_stays_failed_after_a_stop() {
        let dir = tempfile::tempdir().unwrap();
        let mut project = sp::from_text("talk", "", "n");
        let failed = sp::Block { id: sp::new_id(), text: "Не вышло.".into(), error: Some(sp::BlockError { key: "k".into(), text: "engine fell".into() }), ..Default::default() };
        let waiting = sp::Block { id: sp::new_id(), text: "Ещё не озвучена.".into(), ..Default::default() };
        project.chapters[0].blocks = vec![failed.clone(), waiting.clone()];
        let sid = sp::new_id();
        crate::speech::save(dir.path(), &sid, &project).unwrap();
        mark_cut_off(dir.path(), &sid);
        assert!(!INTERRUPTED.lock().unwrap().contains_key(&failed.id), "nothing of it sounds or will");
        assert!(INTERRUPTED.lock().unwrap().contains_key(&waiting.id));
        INTERRUPTED.lock().unwrap().insert(failed.id.clone(), Instant::now());
        assert_eq!(phrase_state(&failed), "failed", "failed is told before interrupted");
        assert_eq!(phrase_state(&waiting), "interrupted");
    }

    /// The studio is not «speaking» because of a phrase that failed or that no window is there to play: no false turn,
    /// ducking or automatic stop in the silence.
    #[test]
    fn a_failed_phrase_or_no_window_is_not_speech_on_its_way() {
        let _air = super::super::live::AIR_TEST.lock().unwrap_or_else(|p| p.into_inner());
        super::super::live::quiet_window();
        let dir = tempfile::tempdir().unwrap();
        let mut project = sp::from_text("talk", "", "n");
        let failed = sp::Block { id: sp::new_id(), text: "Не вышло.".into(), error: Some(sp::BlockError { key: "k".into(), text: "engine fell".into() }), ..Default::default() };
        project.chapters[0].blocks = vec![failed.clone()];
        let sid = sp::new_id();
        crate::speech::save(dir.path(), &sid, &project).unwrap();
        SAID.lock().unwrap().push((Instant::now(), sid.clone(), failed.id.clone(), failed.text.clone()));
        let window = super::super::live::player_for_test();
        assert!(!speaking(dir.path()), "a failed phrase is not on its way");
        let mut voiced = sp::Block { id: sp::new_id(), text: "Готова.".into(), ..Default::default() };
        voiced.takes.push(sp::Take { n: 1, ..Default::default() });
        voiced.active = Some(1);
        project.chapters[0].blocks.push(voiced.clone());
        crate::speech::save(dir.path(), &sid, &project).unwrap();
        SAID.lock().unwrap().push((Instant::now(), sid.clone(), voiced.id.clone(), voiced.text.clone()));
        assert!(speaking(dir.path()), "a voiced phrase the window has not played yet is on its way");
        drop(window);
        assert!(!speaking(dir.path()), "no window to play it");
        SAID.lock().unwrap().retain(|(_, s, _, _)| *s != sid);
    }

    #[test]
    fn chinese_phrases_end_at_their_own_marks() {
        let mut buf = "你好。今天天气很好！".to_string();
        assert_eq!(cut(&mut buf, false), ["你好。", "今天天气很好！"]);
    }

    #[test]
    fn a_quote_closing_the_phrase_stays_with_it() {
        let mut buf = "Он сказал: «Иду!» Потом ушёл".to_string();
        assert_eq!(cut(&mut buf, false), ["Он сказал: «Иду!»"]);
    }
}
