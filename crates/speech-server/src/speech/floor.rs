//! The person speaking over the studio: an event the agent decides on (stop or go on), the studio's voice turned down
//! while it decides, and the setting that says who decides at all.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::{Path as AxPath, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::app::refuse;

/// Who stops the studio when the person speaks over it: the studio at once, the agent (the studio on its own only
/// when the agent says nothing for a while), or nobody.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Always,
    Agent,
    Never,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Always => "always",
            Mode::Agent => "agent",
            Mode::Never => "never",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "always" => Some(Mode::Always),
            "agent" => Some(Mode::Agent),
            "never" => Some(Mode::Never),
            _ => None,
        }
    }
}

/// The setting of who stops the studio, and of how long the agent has to decide.
pub const MODE_KEY: &str = "live_interrupt";
pub const WAIT_KEY: &str = "live_interrupt_ms";
pub const DEFAULT_MODE: Mode = Mode::Agent;
pub const DEFAULT_WAIT_MS: u64 = 1200;
/// The shortest and the longest wait the setting takes.
pub const WAIT_MS: std::ops::RangeInclusive<u64> = 200..=10_000;
/// How far down the studio's voice goes while the agent decides: the person hears they were heard and goes on.
pub const DUCK_DB: f64 = -18.0;

/// How Live keeps the studio from hearing itself: the owner's voice gate on top of the two below (gate), the microphone
/// not heard while the studio's sound is louder than it (duplex) on top of the text check, or the text check alone
/// (text: a line heard while a phrase of the studio sounded and saying it is marked echo).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Guard {
    Gate,
    Duplex,
    Text,
}

impl Guard {
    pub fn as_str(self) -> &'static str {
        match self {
            Guard::Gate => "gate",
            Guard::Duplex => "duplex",
            Guard::Text => "text",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "gate" => Some(Guard::Gate),
            "duplex" => Some(Guard::Duplex),
            "text" => Some(Guard::Text),
            _ => None,
        }
    }
}

pub const GUARD_KEY: &str = "live_echo_guard";
/// The owner's voice gate is not the default: on recorded conversations it has not yet shown it lets the owner's
/// phrases through often enough.
pub const DEFAULT_GUARD: Guard = Guard::Duplex;

pub fn guard(models_root: &Path) -> Guard {
    chosen(models_root, GUARD_KEY).as_deref().and_then(Guard::parse).unwrap_or(DEFAULT_GUARD)
}

pub fn is_value(key: &str, value: &str) -> Option<bool> {
    match key {
        GUARD_KEY => Some(Guard::parse(value).is_some()),
        MODE_KEY => Some(Mode::parse(value).is_some()),
        WAIT_KEY => Some(value.parse::<u64>().is_ok_and(|ms| WAIT_MS.contains(&ms))),
        _ => None,
    }
}

fn chosen(models_root: &Path, key: &str) -> Option<String> {
    crate::engines::models::load_selection(models_root).get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string)
}

pub fn mode(models_root: &Path) -> Mode {
    chosen(models_root, MODE_KEY).as_deref().and_then(Mode::parse).unwrap_or(DEFAULT_MODE)
}

pub fn wait_ms(models_root: &Path) -> u64 {
    chosen(models_root, WAIT_KEY).and_then(|v| v.parse().ok()).filter(|ms| WAIT_MS.contains(ms)).unwrap_or(DEFAULT_WAIT_MS)
}

/// The settings as the window shows them: the defaults where the person chose none; nothing is written.
pub fn with_defaults(mut sel: Value) -> Value {
    if let Some(obj) = sel.as_object_mut() {
        for (k, v) in [(MODE_KEY, DEFAULT_MODE.as_str().to_string()), (WAIT_KEY, DEFAULT_WAIT_MS.to_string()), (GUARD_KEY, DEFAULT_GUARD.as_str().to_string())] {
            if obj.get(k).and_then(Value::as_str).is_none_or(|x| x.trim().is_empty()) {
                obj.insert(k.into(), v.into());
            }
        }
    }
    sel
}

/// One time the person spoke over the studio.
struct Floor {
    n: u64,
    partial: String,
    words: usize,
    /// Where it started, seconds of the recording.
    at: Option<f64>,
    time: i64,
    mode: Mode,
    raised: Instant,
    /// stop, continue, ended (the studio had finished by itself), or none (nobody stops it: mode never).
    decided: Option<(&'static str, Instant)>,
    /// The studio decided on its own: at once (always) or when the agent said nothing in time (agent).
    auto: bool,
}

impl Floor {
    fn json(&self) -> Value {
        json!({
            "n": self.n, "floor": true, "partial": self.partial, "words": self.words, "at": self.at, "time": self.time, "mode": self.mode.as_str(),
            "decided": self.decided.map(|d| d.0), "auto": self.auto, "decided_ms": self.decided.map(|d| d.1.duration_since(self.raised).as_millis() as u64),
        })
    }
}

static FLOORS: Mutex<Vec<Floor>> = Mutex::new(Vec::new());

fn floors() -> std::sync::MutexGuard<'static, Vec<Floor>> {
    FLOORS.lock().unwrap_or_else(|p| p.into_inner())
}

/// The person took the floor saying `partial` (`words` words, from `at`): the event, decided already unless the agent
/// decides it. Answers its number, which no event of an earlier conversation had: a wait still running from one cannot
/// decide an event of the next.
pub fn raise(partial: &str, words: usize, at: Option<f64>, mode: Mode) -> u64 {
    let mut all = floors();
    let n = LAST.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let now = Instant::now();
    let decided = match mode {
        Mode::Always => Some(("stop", now)),
        Mode::Never => Some(("none", now)),
        Mode::Agent => None,
    };
    all.push(Floor { n, partial: partial.to_string(), words, at, time: chrono::Utc::now().timestamp_millis(), mode, raised: now, decided, auto: mode == Mode::Always });
    n
}

/// The number of the last event raised.
static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A new conversation starts with no events, the ones of the conversation before belonging to it, and the studio's
/// voice at its own level: a wait of the old one does not leave it quieter.
pub fn new_conversation() {
    floors().clear();
    super::live::duck(0.0);
}

/// The event waiting for the agent's word, if one does.
#[cfg(test)]
pub fn pending() -> Option<u64> {
    floors().iter().rev().find(|f| f.decided.is_none()).map(|f| f.n)
}

/// Decides event `n` as `how`; false when it was decided already, so only the first word counts.
pub fn decide(n: u64, how: &'static str, auto: bool) -> bool {
    let mut all = floors();
    match all.iter_mut().find(|f| f.n == n) {
        Some(f) if f.decided.is_none() => {
            f.decided = Some((how, Instant::now()));
            f.auto = auto;
            true
        }
        _ => false,
    }
}

/// The agent decides every waiting event: the studio's voice comes back up, and `how` is told. Answers the latest one
/// decided; None with no event waiting. A word of the agent's answers all of them, so an earlier one's wait does not
/// cut the studio off after the agent went on.
pub fn agent_decides(how: &'static str) -> Option<u64> {
    let now = Instant::now();
    let decided = floors().iter_mut().filter(|f| f.decided.is_none()).fold(None, |_, f| {
        f.decided = Some((how, now));
        f.auto = false;
        Some(f.n)
    });
    if decided.is_some() {
        super::live::duck(0.0);
    }
    decided
}

/// While event `n` waits for the agent the studio is quieter; once `wait` passes with no word, `cut` stops it (or,
/// when it has stopped by itself, `speaking` says so and the event is told as ended).
pub fn hold(n: u64, wait: Duration, speaking: impl Fn() -> bool + Send + 'static, cut: impl FnOnce() + Send + 'static) {
    super::live::duck(DUCK_DB);
    tokio::spawn(async move {
        tokio::time::sleep(wait).await;
        let still = speaking();
        if decide(n, if still { "stop" } else { "ended" }, still) {
            super::live::duck(0.0);
            if still {
                cut();
            }
        }
    });
}

/// GET /speech/listen/floor?after=N&seconds=S — the times the person spoke over the studio after the first N, waiting up
/// to S seconds (at most 55) for one; `heard` is the number of the last event, so the next call asks after it.
pub async fn voice_floor(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let after: u64 = q.get("after").and_then(|v| v.parse().ok()).unwrap_or(0);
    let seconds: u64 = q.get("seconds").and_then(|v| v.parse().ok()).unwrap_or(30).min(55);
    let until = Instant::now() + Duration::from_secs(seconds);
    loop {
        let (events, heard, waiting) = {
            let all = floors();
            (all.iter().filter(|f| f.n > after).map(Floor::json).collect::<Vec<_>>(), LAST.load(std::sync::atomic::Ordering::Relaxed), all.iter().rev().find(|f| f.decided.is_none()).map(|f| f.n))
        };
        if !events.is_empty() || Instant::now() >= until {
            return Json(json!({ "events": events, "heard": heard, "pending": waiting }));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// POST /speech/say/{session}/continue — the person spoke over the studio and the agent goes on: the studio's voice comes
/// back up and the event is told as continued.
pub async fn say_continue(AxPath(sid): AxPath<String>) -> Response {
    if !super::say::is_session(&sid) {
        return refuse(StatusCode::NOT_FOUND, "no_session", t!("say-no-session", session = sid));
    }
    match agent_decides("continue") {
        Some(n) => Json(json!({ "continued": n })).into_response(),
        None => refuse(StatusCode::CONFLICT, "no_floor", t!("say-no-floor")),
    }
}

#[cfg(test)]
pub fn clear_for_test() {
    floors().clear();
    LAST.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub static TEST_SLOT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    async fn events(after: u64) -> Value {
        voice_floor(Query(HashMap::from([("after".to_string(), after.to_string()), ("seconds".to_string(), "0".to_string())]))).await.0
    }

    /// In agent mode the event reaches the agent and the studio goes on until it stops it.
    #[tokio::test]
    async fn the_agent_hears_the_person_speak_over_it_and_stops_the_studio_itself() {
        let _slot = TEST_SLOT.lock().await;
        clear_for_test();
        let cut = Arc::new(AtomicBool::new(false));
        let n = raise("подожди, а сколько это стоит", 5, Some(12.5), Mode::Agent);
        let c = cut.clone();
        hold(n, Duration::from_millis(300), || true, move || c.store(true, Ordering::Relaxed));
        let got = events(0).await;
        assert_eq!(got["pending"], n, "{got}");
        assert_eq!(got["events"][0]["decided"], Value::Null);
        assert_eq!(got["events"][0]["words"], 5);
        assert!(!cut.load(Ordering::Relaxed), "the studio goes on while the agent decides");
        assert_eq!(agent_decides("stop"), Some(n));
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert!(!cut.load(Ordering::Relaxed), "the agent decided: the backstop does not cut again");
        let got = events(0).await;
        assert_eq!(got["events"][0]["decided"], "stop");
        assert_eq!(got["events"][0]["auto"], false);
        assert_eq!(got["pending"], Value::Null);
        assert_eq!(agent_decides("continue"), None, "nothing waits any more");
    }

    /// In agent mode with no word from the agent the studio stops itself after the wait, told as auto.
    #[tokio::test]
    async fn with_no_word_from_the_agent_the_studio_stops_after_the_wait() {
        let _slot = TEST_SLOT.lock().await;
        clear_for_test();
        let cut = Arc::new(AtomicBool::new(false));
        let n = raise("нет, стоп, давай про другое", 5, None, Mode::Agent);
        let c = cut.clone();
        hold(n, Duration::from_millis(100), || true, move || c.store(true, Ordering::Relaxed));
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(cut.load(Ordering::Relaxed));
        let got = events(n - 1).await;
        assert_eq!(got["events"][0]["decided"], "stop");
        assert_eq!(got["events"][0]["auto"], true);
        assert!(got["events"][0]["decided_ms"].as_u64().unwrap() >= 100);
        // the studio finished by itself before the wait ran out: nothing to cut
        let ended = raise("ну и ещё одно", 4, None, Mode::Agent);
        let c = Arc::new(AtomicBool::new(false));
        let c2 = c.clone();
        hold(ended, Duration::from_millis(50), || false, move || c2.store(true, Ordering::Relaxed));
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(!c.load(Ordering::Relaxed));
        assert_eq!(events(ended - 1).await["events"][0]["decided"], "ended");
    }

    #[tokio::test]
    async fn always_and_never_are_decided_at_once_and_the_agent_goes_on() {
        let _slot = TEST_SLOT.lock().await;
        clear_for_test();
        let a = raise("стоп стоп стоп стоп", 4, None, Mode::Always);
        let v = raise("стоп стоп стоп стоп", 4, None, Mode::Never);
        let got = events(0).await;
        assert_eq!(got["events"][0]["decided"], "stop");
        assert_eq!(got["events"][0]["auto"], true);
        assert_eq!(got["events"][1]["decided"], "none");
        assert_eq!((a, v), (1, 2));
        assert_eq!(pending(), None);
        let n = raise("да-да, продолжай, только короче", 5, None, Mode::Agent);
        assert_eq!(agent_decides("continue"), Some(n));
        assert_eq!(events(n - 1).await["events"][0]["decided"], "continue");
    }

    /// The person spoke over the studio twice before the agent's word: «go on» answers both, and the earlier one's wait
    /// does not cut the studio off afterwards.
    #[tokio::test]
    async fn the_agent_s_word_answers_every_waiting_event() {
        let _slot = TEST_SLOT.lock().await;
        clear_for_test();
        let cut = Arc::new(AtomicBool::new(false));
        let first = raise("подожди", 1, None, Mode::Agent);
        let c = cut.clone();
        hold(first, Duration::from_millis(150), || true, move || c.store(true, Ordering::Relaxed));
        let second = raise("подожди, а сколько это стоит", 5, None, Mode::Agent);
        let c = cut.clone();
        hold(second, Duration::from_millis(150), || true, move || c.store(true, Ordering::Relaxed));
        assert_eq!(agent_decides("continue"), Some(second));
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(!cut.load(Ordering::Relaxed), "the agent went on: nothing cuts the studio off");
        let got = events(0).await;
        assert_eq!(got["events"][0]["decided"], "continue", "{got}");
        assert_eq!(got["events"][1]["decided"], "continue", "{got}");
        assert_eq!(got["pending"], Value::Null);
    }

    /// A new conversation does not show the events of the one before.
    #[tokio::test]
    async fn a_new_conversation_starts_with_no_events() {
        let _slot = TEST_SLOT.lock().await;
        clear_for_test();
        let cut = Arc::new(AtomicBool::new(false));
        let old = raise("подожди", 1, None, Mode::Agent);
        let c = cut.clone();
        hold(old, Duration::from_millis(100), || true, move || c.store(true, Ordering::Relaxed));
        new_conversation();
        let got = events(0).await;
        assert_eq!(got["events"], json!([]), "{got}");
        assert_eq!(got["heard"], old, "the next call asks after the last number");
        let new = raise("а теперь вот что", 4, None, Mode::Agent);
        assert!(new > old, "an event of the new conversation is not one the old wait can decide");
        let after_heard = events(got["heard"].as_u64().unwrap()).await;
        assert_eq!(after_heard["events"].as_array().unwrap().len(), 1, "{after_heard}");
        assert_eq!(after_heard["heard"], new);
        assert_eq!(events(new).await["events"], json!([]), "asked after the last one heard, nothing comes again");
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(!cut.load(Ordering::Relaxed), "the old wait cut nothing");
        assert_eq!(events(0).await["events"][0]["decided"], Value::Null, "the new event waits for the agent");
    }

    #[test]
    fn the_setting_takes_its_own_values_only() {
        assert_eq!(is_value(MODE_KEY, "agent"), Some(true));
        assert_eq!(is_value(MODE_KEY, "sometimes"), Some(false));
        assert_eq!(is_value(WAIT_KEY, "1200"), Some(true));
        assert_eq!(is_value(WAIT_KEY, "50"), Some(false));
        assert_eq!(is_value("tts", "x"), None);
        let shown = with_defaults(json!({}));
        assert_eq!(shown[MODE_KEY], "agent");
        assert_eq!(shown[WAIT_KEY], "1200");
    }
}
