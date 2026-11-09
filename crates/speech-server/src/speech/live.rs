//! A block's audio while it is being voiced: the pieces the engine streams go to `GET /speech/live` (SSE).

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use serde_json::{json, Value};
use tokio::sync::broadcast;

fn channel() -> &'static broadcast::Sender<Value> {
    static TX: OnceLock<broadcast::Sender<Value>> = OnceLock::new();
    TX.get_or_init(|| broadcast::channel(1024).0)
}

/// The projects whose voicing the window plays aloud wherever it is, as it comes (an agent asked to say something).
fn aloud_set() -> &'static Mutex<std::collections::HashSet<String>> {
    static SET: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    SET.get_or_init(Default::default)
}

/// Plays `project`'s voicing aloud until the guard is dropped; refused when no window listens, there being nothing
/// to play it on.
pub fn aloud(project: &str) -> Result<Aloud, String> {
    if channel().receiver_count() == 0 {
        return Err(t!("speak-aloud-no-window"));
    }
    aloud_set().lock().unwrap_or_else(|p| p.into_inner()).insert(project.to_string());
    Ok(Aloud(project.to_string()))
}

pub struct Aloud(String);

impl Drop for Aloud {
    fn drop(&mut self) {
        aloud_set().lock().unwrap_or_else(|p| p.into_inner()).remove(&self.0);
    }
}

/// Whether a window listens to the voicing, and so can play it.
pub fn listening() -> bool {
    channel().receiver_count() > 0
}

/// The voice of what is said aloud is loading (its engine), or is ready (`None`): the window says so meanwhile.
pub fn voice_loading(engine: Option<&str>) {
    let _ = channel().send(json!({ "kind": "voice", "loading": engine, "aloud": true }));
}

/// The voice for speaking aloud was chosen (by an agent or another window): the windows show it.
pub fn voice_chosen() {
    let _ = channel().send(json!({ "kind": "chosen", "aloud": true }));
}

/// An agent changed the conversation's recogniser (`engine`, named `label`): the window shows it as chosen.
pub fn asr_switched(engine: &str, label: &str) {
    let _ = channel().send(json!({ "kind": "asr", "engine": engine, "label": label, "aloud": true }));
}

/// The window stops playing `project` now and drops what it queued.
pub fn silence(project: &str) {
    cut_airing(project);
    let _ = channel().send(json!({ "kind": "stop", "project": project, "aloud": true }));
}

/// How long the sound said aloud stays on record after it was heard: a whole conversation, whose metrics place the
/// studio's speech among the person's lines.
const AIRED_SECS: u64 = 6 * 3600;

/// When a block said aloud sounds from the window, as the server sent it: the window plays the pieces one after
/// another, so a piece sounds from the later of its sending and the end of the piece before it.
#[derive(Clone)]
pub struct Aired {
    pub project: String,
    pub block: String,
    pub from: Instant,
    pub to: Instant,
    /// The silence heard before this phrase because it was not ready: from the later of the end of the session's
    /// phrase before it and the arrival of its own text, to its first sound. None for a session's first phrase.
    pub wait_ms: Option<u64>,
}

struct Airing {
    blocks: Vec<Aired>,
    /// Where the sound the window has queued ends.
    until: Option<Instant>,
    /// How loud each piece sent to the window is (RMS of its samples) and when it sounds.
    levels: std::collections::VecDeque<(Instant, Instant, f32, String)>,
}

/// How long the loudness of the sound said aloud is kept.
const LEVEL_SECS: u64 = 30;

fn airing() -> std::sync::MutexGuard<'static, Airing> {
    static AIR: OnceLock<Mutex<Airing>> = OnceLock::new();
    AIR.get_or_init(|| Mutex::new(Airing { blocks: Vec::new(), until: None, levels: Default::default() })).lock().unwrap_or_else(|p| p.into_inner())
}

/// Root mean square of `samples`.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// How loud the sound said aloud is from `from` to `to` (the loudest of its pieces then, as the window plays it now:
/// quieter while it is ducked), `tail` longer after each piece: a room sends the sound back a moment late. None when
/// nothing sounded then.
pub fn playback_level(from: Instant, to: Instant, tail: std::time::Duration) -> Option<f32> {
    let ducked = 10f32.powf(f32::from_bits(DUCKED_DB.load(Ordering::Relaxed)) / 20.0);
    airing().levels.iter().filter(|(a, z, _, _)| *a <= to && *z + tail >= from).map(|l| l.2 * ducked).reduce(f32::max)
}

/// `secs` more of `block`'s sound sent to the window to play now.
fn aired(project: &str, block: &str, secs: f64, level: f32) {
    let now = Instant::now();
    let mut air = airing();
    let from = air.until.filter(|u| *u > now).unwrap_or(now);
    let to = from + std::time::Duration::from_secs_f64(secs.max(0.0));
    air.until = Some(to);
    while air.levels.front().is_some_and(|l| now.saturating_duration_since(l.1).as_secs() >= LEVEL_SECS) {
        air.levels.pop_front();
    }
    air.levels.push_back((from, to, level, project.to_string()));
    air.blocks.retain(|a| now.saturating_duration_since(a.to).as_secs() < AIRED_SECS);
    match air.blocks.iter_mut().rev().find(|a| a.block == block) {
        Some(a) => a.to = to,
        None => {
            let before = air.blocks.iter().filter(|a| a.project == project).map(|a| a.to).max();
            let arrived = crate::app::since_arrival(block).and_then(|ms| now.checked_sub(std::time::Duration::from_millis(ms)));
            let wait_ms = before.map(|end| from.saturating_duration_since(arrived.map_or(end, |t| t.max(end))).as_millis() as u64);
            air.blocks.push(Aired { project: project.into(), block: block.into(), from, to, wait_ms });
        }
    }
}

/// The silence heard before `block` because it was not ready (see `Aired::wait_ms`).
pub fn queue_wait_ms(block: &str) -> Option<u64> {
    airing().blocks.iter().rev().find(|a| a.block == block).and_then(|a| a.wait_ms)
}

/// The window dropped `project`'s sound: what it had queued does not sound, and is not heard back.
fn cut_airing(project: &str) {
    let now = Instant::now();
    let mut air = airing();
    for a in air.blocks.iter_mut().filter(|a| a.project == project) {
        a.to = a.to.min(now);
        a.from = a.from.min(a.to);
    }
    for l in air.levels.iter_mut().filter(|l| l.3 == project) {
        l.1 = l.1.min(now);
        l.0 = l.0.min(l.1);
    }
    let latest = air.blocks.iter().map(|a| a.to).max();
    air.until = latest.filter(|u| *u > now);
}

/// When the sound said aloud ends: None when nothing sounds now.
pub fn airing_until() -> Option<Instant> {
    airing().until.filter(|u| *u > Instant::now())
}

/// The window plays what is said aloud `db` quieter than its volume (0: as loud as set).
pub fn duck(db: f64) {
    DUCKED_DB.store((db as f32).to_bits(), Ordering::Relaxed);
    let _ = channel().send(json!({ "kind": "duck", "db": db, "aloud": true }));
}

/// How much quieter than sent the window plays what is said aloud now (`duck`), bits of an f32 in decibels.
static DUCKED_DB: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The blocks said aloud lately with when each sounded, the earliest first.
pub fn aired_blocks() -> Vec<Aired> {
    let mut blocks = airing().blocks.clone();
    blocks.sort_by_key(|a| a.from);
    blocks
}

#[cfg(test)]
pub fn air_for_test(project: &str, block: &str, from: Instant, to: Instant) {
    let mut air = airing();
    air.blocks.push(Aired { project: project.into(), block: block.into(), from, to, wait_ms: None });
}

/// Sound as loud as `level` (RMS) said aloud from `from` to `to`, for tests.
#[cfg(test)]
pub fn level_for_test(from: Instant, to: Instant, level: f32) {
    airing().levels.push_back((from, to, level, String::new()));
}

/// The window plays as loud as set, for tests.
#[cfg(test)]
pub fn unduck_for_test() {
    DUCKED_DB.store(0f32.to_bits(), Ordering::Relaxed);
}

/// The windows that play what is said aloud, by connection, in the order they opened: only the first plays it, so a
/// second window does not sound the same phrase again a moment later.
fn players() -> &'static Mutex<std::collections::BTreeSet<u64>> {
    static SET: OnceLock<Mutex<std::collections::BTreeSet<u64>>> = OnceLock::new();
    SET.get_or_init(Default::default)
}

/// A window's connection while it plays aloud; leaving the set when the connection closes.
struct Player(u64);

impl Player {
    fn join() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        players().lock().unwrap_or_else(|p| p.into_inner()).insert(id);
        Player(id)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        players().lock().unwrap_or_else(|p| p.into_inner()).remove(&self.0);
    }
}

/// The window that plays what is said aloud now.
fn player() -> Option<u64> {
    players().lock().unwrap_or_else(|p| p.into_inner()).first().copied()
}

/// Whether a window is there to play what is said aloud.
pub fn has_player() -> bool {
    player().is_some()
}

/// A window playing aloud while the guard lives, for tests.
#[cfg(test)]
pub fn player_for_test() -> impl Drop {
    Player::join()
}

fn is_aloud(project: &str) -> bool {
    aloud_set().lock().unwrap_or_else(|p| p.into_inner()).contains(project)
}

/// One block being voiced: its pieces in order, a restart when a defective take is said again, its end.
pub struct Phrase {
    project: String,
    block: String,
    started: Instant,
    first_ms: Mutex<Option<u64>>,
    heard_ms: Mutex<Option<u64>>,
    ended: AtomicBool,
    gate: Mutex<crate::audio::trim::PauseGate>,
}

impl Drop for Phrase {
    fn drop(&mut self) {
        if !self.ended.load(Ordering::Relaxed) {
            self.end(false);
        }
    }
}

impl Phrase {
    pub fn begin(project: &str, block: &str) -> Self {
        let phrase = Self {
            project: project.into(),
            block: block.into(),
            started: Instant::now(),
            first_ms: Mutex::new(None),
            heard_ms: Mutex::new(None),
            ended: AtomicBool::new(false),
            gate: Mutex::new(Default::default()),
        };
        phrase.send("start", json!({}));
        phrase
    }

    fn send(&self, kind: &str, extra: Value) {
        let tx = channel();
        if tx.receiver_count() == 0 {
            return;
        }
        let aloud = is_aloud(&self.project);
        let mut ev = json!({ "kind": kind, "project": self.project, "block": self.block, "aloud": aloud });
        if aloud {
            ev["player"] = json!(player());
        }
        if let (Some(ev), Value::Object(extra)) = (ev.as_object_mut(), extra) {
            ev.extend(extra);
        }
        let _ = tx.send(ev);
    }

    /// Mono samples at `sr`, as the engine decoded them.
    pub fn chunk(&self, samples: &[f32], sr: u32) {
        if samples.is_empty() {
            return;
        }
        if let Ok(mut first) = self.first_ms.lock() {
            if first.is_none() {
                *first = Some(self.started.elapsed().as_millis() as u64);
                *self.heard_ms.lock().unwrap_or_else(|p| p.into_inner()) = crate::app::since_arrival(&self.block);
            }
        }
        // a phrase said aloud is heard without the long pauses an engine leaves inside it, as an export is
        if self.is_aloud() {
            let kept = self.gate.lock().unwrap_or_else(|p| p.into_inner()).pass(samples, sr);
            self.play(&kept, sr);
        } else {
            self.play(samples, sr);
        }
    }

    fn play(&self, samples: &[f32], sr: u32) {
        if channel().receiver_count() == 0 {
            return;
        }
        if self.is_aloud() && player().is_some() {
            aired(&self.project, &self.block, samples.len() as f64 / sr.max(1) as f64, rms(samples));
        }
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        self.send("chunk", json!({ "sr": sr, "data": base64::engine::general_purpose::STANDARD.encode(bytes) }));
    }

    /// Silence the block's text asks for between its pieces; it is not the block's first sound.
    pub fn pause(&self, ms: u32, sr: u32) {
        if ms > 0 {
            self.play(&vec![0.0; (sr as u64 * ms as u64 / 1000) as usize], sr);
        }
    }

    /// The take was defective and is said again: what was played of it is dropped.
    pub fn restart(&self) {
        self.send("restart", json!({}));
    }

    /// Whether the phrase is heard aloud as it comes (an agent speaking), not only in a project's player.
    pub fn is_aloud(&self) -> bool {
        is_aloud(&self.project)
    }

    /// Milliseconds from the start of the block to its first sound.
    pub fn first_ms(&self) -> Option<u64> {
        self.first_ms.lock().ok().and_then(|f| *f)
    }

    /// A phrase said aloud: milliseconds from its text arriving to its first sound.
    pub fn heard_ms(&self) -> Option<u64> {
        *self.heard_ms.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The block's id.
    pub fn block(&self) -> &str {
        &self.block
    }

    pub fn end(&self, ok: bool) {
        self.ended.store(true, Ordering::Relaxed);
        self.send("end", json!({ "ok": ok, "first_ms": self.first_ms() }));
    }
}

/// GET /speech/live[?aloud=1] — every block voiced from now on, while it is voiced; a window that plays what is said
/// aloud asks with `aloud=1`, learns its number from the first event and plays an aloud phrase only when it is that
/// phrase's `player`.
pub async fn events(axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>) -> Response {
    let mut rx = channel().subscribe();
    let player = (q.get("aloud").map(String::as_str) == Some("1")).then(Player::join);
    let stream = async_stream::stream! {
        let player = player;
        if let Some(p) = &player {
            yield Ok::<_, Infallible>(Event::default().data(json!({ "kind": "hello", "id": p.0 }).to_string()));
        }
        loop {
            match rx.recv().await {
                Ok(ev) => yield Ok::<_, Infallible>(Event::default().data(ev.to_string())),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    yield Ok(Event::default().data(json!({ "kind": "lagged", "missed": n }).to_string()));
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

/// The window's one queue of sound and its windows are shared: the tests that queue sound into it, open windows or
/// read whether the studio speaks take turns.
#[cfg(test)]
pub static AIR_TEST: Mutex<()> = Mutex::new(());

/// Waits until the window's queue of sound has played out.
#[cfg(test)]
pub fn quiet_window() {
    while airing_until().is_some() {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_open_window_plays_what_is_said_aloud() {
        let _air = AIR_TEST.lock().unwrap_or_else(|p| p.into_inner());
        let first = Player::join();
        let second = Player::join();
        assert_eq!(player(), Some(first.0));
        drop(first);
        assert_eq!(player(), Some(second.0), "the next window plays once the first closes");
    }

    /// Sound the window dropped on a stop is not heard back: the half-duplex does not hold the microphone for it.
    #[test]
    fn a_dropped_queue_is_not_heard_back() {
        let _air = AIR_TEST.lock().unwrap_or_else(|p| p.into_inner());
        quiet_window();
        let p = "p-dropped";
        aired(p, "dr-1", 5.0, 0.2);
        let later = Instant::now() + std::time::Duration::from_secs(2);
        assert!(playback_level(later, later + std::time::Duration::from_millis(100), std::time::Duration::ZERO).is_some());
        silence(p);
        assert_eq!(playback_level(later, later + std::time::Duration::from_millis(100), std::time::Duration::ZERO), None);
    }

    /// Scenario of TZ-LIVE-TRAINER F: a phrase not ready when the one before it ended is told by the silence heard
    /// before it; a phrase ready in time, the session's first and one whose own text came late wait nothing of the queue.
    #[test]
    fn the_silence_before_a_phrase_that_was_not_ready_is_measured() {
        let _air = AIR_TEST.lock().unwrap_or_else(|p| p.into_inner());
        quiet_window();
        let p = "p-queue-wait";
        crate::app::arrived("qw-1");
        aired(p, "qw-1", 0.05, 0.1);
        assert_eq!(queue_wait_ms("qw-1"), None, "the session's first phrase");
        crate::app::arrived("qw-2");
        aired(p, "qw-2", 0.05, 0.1);
        assert_eq!(queue_wait_ms("qw-2"), Some(0), "queued behind the first while it still played");
        crate::app::arrived("qw-3");
        std::thread::sleep(std::time::Duration::from_millis(250));
        aired(p, "qw-3", 0.05, 0.1);
        let wait = queue_wait_ms("qw-3").unwrap();
        assert!((120..=260).contains(&wait), "ready only well after the phrase before it ended: {wait}");
        std::thread::sleep(std::time::Duration::from_millis(200));
        crate::app::arrived("qw-4");
        aired(p, "qw-4", 0.05, 0.1);
        assert!(queue_wait_ms("qw-4").unwrap() < 50, "its text came late, not the engine: {:?}", queue_wait_ms("qw-4"));
    }

    #[test]
    fn the_studio_s_sound_is_loud_while_it_plays_and_a_moment_after() {
        let _air = AIR_TEST.lock().unwrap_or_else(|p| p.into_inner());
        quiet_window();
        let p = "p-level";
        aired(p, "lv-1", 0.3, 0.2);
        let now = Instant::now();
        let tail = std::time::Duration::from_millis(200);
        assert_eq!(playback_level(now, now + std::time::Duration::from_millis(100), tail), Some(0.2));
        assert!(playback_level(now + std::time::Duration::from_millis(450), now + std::time::Duration::from_millis(480), tail).is_some(), "within the tail");
        assert_eq!(playback_level(now + std::time::Duration::from_millis(700), now + std::time::Duration::from_millis(800), tail), None, "after the tail");
        assert!((rms(&[0.5, -0.5]) - 0.5).abs() < 1e-6);
    }

    #[tokio::test]
    async fn a_listener_gets_the_pieces_in_order_and_the_first_sound() {
        let mut rx = channel().subscribe();
        let phrase = Phrase::begin("p1", "b1");
        phrase.pause(10, 24_000);
        assert_eq!(phrase.first_ms(), None, "a pause is no first sound");
        phrase.chunk(&[0.5, -0.5], 24_000);
        phrase.restart();
        phrase.end(true);
        let kinds: Vec<String> = (0..5).map(|_| rx.try_recv().unwrap()["kind"].as_str().unwrap().to_string()).collect();
        assert_eq!(kinds, ["start", "chunk", "chunk", "restart", "end"]);
        assert!(phrase.first_ms().is_some());
        let mut again = channel().subscribe();
        phrase.chunk(&[0.25], 24_000);
        let ev = again.try_recv().unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(ev["data"].as_str().unwrap()).unwrap();
        assert_eq!(f32::from_le_bytes(bytes[..4].try_into().unwrap()), 0.25);
        assert_eq!(ev["sr"], 24_000);
    }
}
