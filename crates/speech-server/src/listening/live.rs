//! Recognition while the microphone records: the recorder's sound (`record::tap`) is cut into utterances where the
//! speaker pauses (Silero VAD), and each is recognised as it ends by the recogniser a file of that language would get,
//! loaded once for the whole recording. The phrases become the recording's first transcript when it stops; full
//! recognition (speakers, exact words) runs as for any recording.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde_json::{json, Value};

use crate::app::Localize;
use crate::audio::wavio::to_16k;

/// A pause this long after speech ends the utterance.
const PAUSE_SECS: f64 = 0.7;
/// An utterance without a pause is cut at this length.
const LONGEST_SECS: f64 = 20.0;
/// Speech starts and ends this much wider than the detector heard it.
const EDGE_SECS: f64 = 0.15;
/// While a phrase goes on, what it says so far is recognised again this often.
const INTERIM_SECS: f64 = 1.5;
/// Under Auto, the language of the first speech is heard once it is this long (16 kHz samples), unless the phrase
/// ends sooner.
const HEAR_LANGUAGE_SAMPLES: usize = 2 * 16_000;

/// A phrase heard live: where it starts and ends in the recording, seconds, and what was said.
pub type Heard = (f64, f64, String);

/// A word of a phrase heard live: what it says, where it starts and ends in the recording, seconds.
pub type HeardWord = (String, f64, f64);

struct Live {
    lines: Vec<Heard>,
    /// The phrase being said: where it started and what it says so far, until a pause ends it.
    partial: Option<(f64, String)>,
    error: Option<String>,
    /// The person turned the microphone off for a while: what it hears is taken as silence, the session goes on.
    muted: bool,
    /// The places in `lines` of what the person typed: their own words, never the studio's voice heard back.
    typed: std::collections::HashSet<usize>,
    /// The places in `lines` the voice gate judged NOT the owner's voice (only when a gated conversation is armed).
    foreign: std::collections::HashSet<usize>,
    /// The places in `lines` heard while the studio's own phrase sounded and saying that phrase: its voice heard back.
    echo: std::collections::HashSet<usize>,
    /// The words of the lines whose recogniser timed them (text, start, end in the recording); a line without an entry
    /// has no word timings.
    words: std::collections::HashMap<usize, Vec<HeardWord>>,
    /// Seconds of the microphone the half-duplex took for the studio's own sound and held back.
    duplex_secs: f64,
    /// Why Smart Turn did not answer, the first time this conversation (each time goes to the log).
    turn_error: Option<String>,
    /// The moment the recording's second 0 was.
    zero: std::time::Instant,
    /// The recogniser that hears now, as the window names it, and the language asked of it.
    engine: String,
    lang: String,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

fn live() -> &'static Mutex<Option<Live>> {
    static L: OnceLock<Mutex<Option<Live>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(None))
}

fn with<R>(f: impl FnOnce(&mut Live) -> R) -> Option<R> {
    live().lock().unwrap_or_else(|p| p.into_inner()).as_mut().map(f)
}

/// Starts recognising the recording that has just started, in `lang`.
pub fn start(repo_root: &Path, models_root: &Path, lang: &str, engine: &str) -> Result<(), String> {
    let choice = crate::app::recognition::choice_of(repo_root, models_root, lang, &t!("listen-live-name"), engine)?;
    if !crate::engines::vad::installed(models_root) {
        return Err(t!("listen-live-no-vad"));
    }
    let stop = Arc::new(AtomicBool::new(false));
    *live().lock().unwrap_or_else(|p| p.into_inner()) = Some(Live {
        lines: Vec::new(),
        partial: None,
        error: None,
        muted: false,
        typed: Default::default(),
        foreign: Default::default(),
        echo: Default::default(),
        words: Default::default(), duplex_secs: 0.0, turn_error: None,
        zero: std::time::Instant::now(),
        engine: choice.label(),
        lang: lang.to_string(),
        stop: stop.clone(),
        worker: None,
    });
    run(choice, repo_root, models_root, lang, stop, 0.0);
    Ok(())
}

/// The recogniser of the running conversation is changed while the microphone goes on recording: the next phrase is
/// heard by `engine`, what was heard stays. Answers the new recogniser's name.
pub fn switch(repo_root: &Path, models_root: &Path, engine: &str) -> Result<String, String> {
    let lang = with(|l| l.lang.clone()).ok_or_else(|| t!("listen-live-not-running"))?;
    let choice = crate::app::recognition::choice_of(repo_root, models_root, &lang, &t!("listen-live-name"), engine)?;
    let (stop, worker) = with(|l| (l.stop.clone(), l.worker.take())).ok_or_else(|| t!("listen-live-not-running"))?;
    stop.store(true, Ordering::Relaxed);
    if let Some(w) = worker {
        let _ = w.join();
    }
    let at = crate::voices::record::in_use().map_or(0.0, |(_, on_for)| on_for.as_secs_f64());
    let stop = Arc::new(AtomicBool::new(false));
    let label = choice.label();
    with(|l| {
        l.stop = stop.clone();
        l.partial = None;
        l.engine = label.clone();
    });
    run(choice, repo_root, models_root, &lang, stop, at);
    Ok(label)
}

/// The recogniser hearing the running conversation, and its language; None with no conversation.
pub fn engine() -> Option<(String, String)> {
    with(|l| (l.engine.clone(), l.lang.clone()))
}

/// Recognises what the recorder hears from now on, its phrases placed `at` seconds into the recording.
fn run(choice: crate::engines::models::AsrChoice, repo_root: &Path, models_root: &Path, lang: &str, stop: Arc<AtomicBool>, at: f64) {
    let repo_root = repo_root.to_path_buf();
    let models_root = models_root.to_path_buf();
    let (to, from) = channel::<Vec<f32>>();
    crate::voices::record::tap(Some(to));
    std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(&models_root, "asr_backend"));
    let lang = if lang == "auto" { String::new() } else { lang.to_string() };
    let worker = std::thread::spawn(move || {
        if let Err(e) = listen(choice, &repo_root, &models_root, &lang, from, &stop, &crate::voices::record::rate, at) {
            tracing::error!("[ERROR] live recognition: {e}");
            with(|l| l.error = Some(e));
        }
    });
    with(|l| l.worker = Some(worker));
}

/// A piece of the microphone as recognition takes it: silence while the person has it muted.
fn heard_as(mut chunk: Vec<f32>) -> Vec<f32> {
    if with(|l| l.muted).unwrap_or(false) {
        chunk.fill(0.0);
    }
    chunk
}

/// 16 kHz mono samples as a PCM16 WAV.
fn write_16k(path: &Path, samples: &[f32]) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 1, sample_rate: 16_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("{}: {e}", path.display()))?;
    for s in samples {
        w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

/// While the studio speaks, the microphone is heard only when it is louder than the studio's own sound coming back
/// could be: this far under the sound sent to the window before the echo is measured, and at most this far once it is.
pub const DUPLEX_DB: f32 = -18.0;
/// The studio's sound comes back from the room this long after it was played.
pub const DUPLEX_TAIL: Duration = Duration::from_millis(200);
/// Once the person is heard over the studio, the microphone stays open this long, so their quieter sounds between
/// louder ones are not cut out of the phrase.
pub const DUPLEX_HOLD: Duration = Duration::from_millis(400);

/// The half-duplex of a conversation: whether the microphone is heard while the studio speaks. It measures how loud
/// the studio's voice comes back against the sound sent (`echo_db`) from the pieces it holds back, so the window's
/// volume, the room and the microphone's gain are in it.
#[derive(Default)]
pub struct Duplex {
    open_until: Option<std::time::Instant>,
    echo_db: Option<f32>,
}

impl Duplex {
    /// Whether a piece of the microphone as loud as `mic` (RMS) is the person, the studio's sound being `playback`
    /// (RMS, None: silent) then. The person is heard `ECHO_MARGIN_DB` over the echo measured, never needing more than
    /// `DUPLEX_DB`, which is also where it starts before it has heard the room.
    pub fn passes(&mut self, mic: f32, playback: Option<f32>, now: std::time::Instant) -> bool {
        let Some(p) = playback.filter(|p| *p > 0.0) else {
            return true;
        };
        let ratio = 20.0 * (mic.max(1e-9) / p).log10();
        let threshold = self.echo_db.map_or(DUPLEX_DB, |e| (e + ECHO_MARGIN_DB).min(DUPLEX_DB));
        if ratio >= threshold {
            self.open_until = Some(now + DUPLEX_HOLD);
            return true;
        }
        let open = self.open_until.is_some_and(|u| now < u);
        if !open && mic > 0.0 {
            self.echo_db = Some(self.echo_db.map_or(ratio, |e| e + ECHO_LEARN * (ratio - e)));
        }
        open
    }
}

/// The person is heard this much over the studio's voice coming back, which spreads around its measured level.
pub const ECHO_MARGIN_DB: f32 = 12.0;
/// How fast the measured echo follows each piece held back.
const ECHO_LEARN: f32 = 0.1;

/// A piece of the microphone the half-duplex took for the studio's own sound: heard as silence, and counted.
fn held_back(mut chunk: Vec<f32>, sr: u32) -> Vec<f32> {
    let secs = chunk.len() as f64 / sr.max(1) as f64;
    with(|l| l.duplex_secs += secs);
    chunk.fill(0.0);
    chunk
}

/// The microphone's pieces as the half-duplex hears them, on the recording's clock, and where in the recording
/// (seconds) it held pieces back.
struct Intake {
    duplex: Duplex,
    held: Vec<(f64, f64)>,
    zero: std::time::Instant,
    at: f64,
    taken: usize,
}

impl Intake {
    /// The microphone from its sample taken at `zero`, `at` seconds into the recording.
    fn new(zero: std::time::Instant, at: f64) -> Self {
        Intake { duplex: Duplex::default(), held: Vec::new(), zero, at, taken: 0 }
    }

    /// `chunk` as heard: the studio's own sound coming back held back when `guarded`. The piece is placed on the
    /// recording's clock by the samples taken before it, not by when the loop gets to it: the pieces pile up while the
    /// loop recognises a phrase.
    fn take(&mut self, chunk: Vec<f32>, sr: u32, guarded: bool) -> Vec<f32> {
        let sr = sr.max(1);
        let from = self.taken as f64 / sr as f64;
        self.taken += chunk.len();
        let to = self.taken as f64 / sr as f64;
        // a muted microphone is silence already: nothing of the studio's is held back in it
        if !guarded || chunk.iter().all(|s| *s == 0.0) {
            return chunk;
        }
        let (began, ended) = (self.zero + Duration::from_secs_f64(from), self.zero + Duration::from_secs_f64(to));
        let playback = crate::speech::playback_level(began, ended, DUPLEX_TAIL);
        if self.duplex.passes(crate::speech::rms(&chunk), playback, ended) {
            return chunk;
        }
        let (a, z) = (self.at + from, self.at + to);
        match self.held.last_mut() {
            Some(last) if (last.1 - a).abs() < 1e-9 => last.1 = z,
            _ => self.held.push((a, z)),
        }
        held_back(chunk, sr)
    }

    /// The samples of 16 kHz `audio` starting `start` seconds into the recording that were heard, not held back.
    fn heard_part(&self, audio: &[f32], start: f64) -> Vec<f32> {
        let end = start + audio.len() as f64 / 16_000.0;
        let spans: Vec<(usize, usize)> = self
            .held
            .iter()
            .filter(|(a, z)| *z > start && *a < end)
            .map(|(a, z)| ((((a - start) * 16_000.0).round().max(0.0)) as usize, (((z - start) * 16_000.0).round() as usize).min(audio.len())))
            .collect();
        audio.iter().enumerate().filter(|(i, _)| !spans.iter().any(|(a, z)| i >= a && i < z)).map(|(_, s)| *s).collect()
    }

    /// Forgets what was held back before `secs` of the recording: those samples are recognised already.
    fn forget_before(&mut self, secs: f64) {
        self.held.retain(|(_, z)| *z > secs);
    }
}

/// After the pause that ends a phrase, Smart Turn may still hear it unfinished; it is then given this much more silence
/// before it is ended anyway.
const TURN_EXTRA_SECS: f64 = 1.5;
/// The seconds before a phrase's end Smart Turn hears.
const TURN_HEARS_SECS: f64 = 8.0;
/// Smart Turn's probability over which a phrase is finished.
const TURN_THRESHOLD: f64 = 0.5;

/// Whether a phrase that paused is ended, by Smart Turn when it is chosen: asked once for each end of speech, the
/// answer kept until the person speaks again.
#[derive(Default)]
struct TurnEnd {
    asked: Option<(f64, bool)>,
}

impl TurnEnd {
    /// The phrase whose speech ends at `last` (seconds of the audio held, `secs` long) paused for at least the pause that
    /// ends a phrase: whether it ends now. `ask` hears it (finished or not); its failure ends the phrase by the pause and
    /// is answered in `failed`.
    fn ends(&mut self, secs: f64, last: f64, ask: impl FnOnce() -> Result<bool, String>, failed: &mut Option<String>) -> bool {
        if secs - last >= PAUSE_SECS + TURN_EXTRA_SECS {
            return true;
        }
        if let Some(finished) = self.asked.filter(|(at, _)| (*at - last).abs() < 1e-6).map(|(_, finished)| finished) {
            return finished;
        }
        let finished = match ask() {
            Ok(finished) => finished,
            Err(e) => {
                *failed = Some(e);
                true
            }
        };
        self.asked = Some((last, finished));
        finished
    }
}

/// Whether the running session's microphone is muted; None with no session.
pub fn muted() -> Option<bool> {
    with(|l| l.muted)
}

/// Mutes or unmutes the microphone of the running session.
pub fn mute(muted: bool) -> Result<(), String> {
    with(|l| l.muted = muted).ok_or_else(|| t!("listen-live-not-running"))
}

/// A line the person typed instead of saying it, put after what was heard last.
pub fn type_line(text: &str) -> Result<(), String> {
    with(|l| {
        let at = l.lines.last().map(|line| line.1).unwrap_or(0.0);
        l.typed.insert(l.lines.len());
        l.lines.push((at, at, text.to_string()));
    })
    .ok_or_else(|| t!("listen-live-not-running"))
}

/// What live recognition has heard so far, and why it stopped when it did; the lines without their word timings,
/// which the conversation's turns are polled for many times a second.
pub fn state() -> Value {
    state_of(false)
}

/// `state` with each line's word timings.
pub fn state_with_words() -> Value {
    state_of(true)
}

fn state_of(words: bool) -> Value {
    match live().lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        Some(l) => json!({
            "running": !l.stop.load(Ordering::Relaxed),
            "lines": l.lines.iter().enumerate().map(|(n, (a, z, text))| {
                let mut line = json!({ "start": a, "end": z, "text": text, "typed": l.typed.contains(&n), "owner": !l.foreign.contains(&n), "echo": l.echo.contains(&n), "words_source": if l.typed.contains(&n) { "typed" } else if l.words.contains_key(&n) { "asr" } else { "none" } });
                if words {
                    line["words"] = json!(l.words.get(&n).map(|ws| ws.iter().map(|(w, a, z)| json!({ "text": w, "start": a, "end": z })).collect::<Vec<_>>()));
                }
                line
            }).collect::<Vec<_>>(),
            "partial": l.partial.as_ref().map(|(a, text)| json!({ "start": a, "text": text })),
            "error": l.error,
            "muted": l.muted,
            "duplex_secs": (l.duplex_secs * 10.0).round() / 10.0,
            "turn_error": l.turn_error,
        }),
        None => json!({ "running": false, "lines": [], "partial": null, "error": null, "muted": false }),
    }
}

/// The moment `secs` of the running recording was heard; None with no conversation.
pub fn heard_at(secs: f64) -> Option<std::time::Instant> {
    with(|l| l.zero + Duration::from_secs_f64(secs.max(0.0)))
}

/// The second of the running recording `at` was (before its start: below 0); None with no conversation.
pub fn recording_secs(at: std::time::Instant) -> Option<f64> {
    with(|l| match at.checked_duration_since(l.zero) {
        Some(after) => after.as_secs_f64(),
        None => -l.zero.duration_since(at).as_secs_f64(),
    })
}

/// Serialises the tests that put a conversation of their own into the one live slot.
#[cfg(test)]
pub static TEST_SLOT: Mutex<()> = Mutex::new(());

/// A conversation that heard `lines` (start, end, text, typed, the owner's voice), its second 0 being `zero`, for tests.
#[cfg(test)]
pub fn conversation_for_test(zero: std::time::Instant, lines: &[(f64, f64, &str, bool, bool)]) {
    let mut l = Live { lines: Vec::new(), partial: None, error: None, muted: false, typed: Default::default(), foreign: Default::default(), echo: Default::default(), words: Default::default(), duplex_secs: 0.0, turn_error: None, zero, engine: String::new(), lang: "ru".into(), stop: Arc::new(AtomicBool::new(false)), worker: None };
    for (n, (a, z, text, typed, owner)) in lines.iter().enumerate() {
        if *typed {
            l.typed.insert(n);
        } else if crate::speech::is_echo(text, zero + Duration::from_secs_f64(*a), zero + Duration::from_secs_f64(*z)) {
            l.echo.insert(n);
        }
        if !*owner {
            l.foreign.insert(n);
        }
        l.lines.push((*a, *z, text.to_string()));
    }
    *live().lock().unwrap_or_else(|p| p.into_inner()) = Some(l);
}

/// The word timings of the running conversation's line `n` (from 0), for tests.
#[cfg(test)]
pub fn words_for_test(n: usize, words: &[(&str, f64, f64)]) {
    with(|l| l.words.insert(n, words.iter().map(|(w, a, z)| (w.to_string(), *a, *z)).collect()));
}

#[cfg(test)]
pub fn end_for_test() {
    *live().lock().unwrap_or_else(|p| p.into_inner()) = None;
}

pub fn finish() -> Result<Vec<Heard>, String> {
    crate::voices::record::tap(None);
    let (stop, worker) = match live().lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
        Some(l) => (l.stop.clone(), l.worker.take()),
        None => return Ok(Vec::new()),
    };
    stop.store(true, Ordering::Relaxed);
    let joined = worker.map(|w| w.join().is_ok()).unwrap_or(true);
    let Some(l) = live().lock().unwrap_or_else(|p| p.into_inner()).take() else {
        return Ok(Vec::new());
    };
    match (joined, l.error) {
        (false, _) => Err(t!("listen-live-broke")),
        (true, Some(e)) => Err(e),
        (true, None) => Ok(l.lines),
    }
}

fn listen(
    choice: crate::engines::models::AsrChoice,
    repo_root: &Path,
    models_root: &Path, lang: &str, from: Receiver<Vec<f32>>, stop: &AtomicBool, rate: &dyn Fn() -> u32, at: f64) -> Result<(), String> {
    let label = choice.label();
    let lang = if lang == "auto" { "" } else { lang };
    // Nemotron 3.5 is a streaming model: a phrase is recognised as it is spoken instead of again from its start
    let streaming = match &choice {
        crate::engines::models::AsrChoice::Ggml { models_root, build, model } | crate::engines::models::AsrChoice::Server { models_root, build, model, .. }
            if model.id == "nemotron" =>
        {
            Some((models_root.clone(), *build, *model))
        }
        _ => None,
    };
    let open_stream = |told: &str| -> Result<dub_asr::tcpp::TcppStream, String> {
        let (root, build, model) = streaming.as_ref().expect("a stream is opened for a streaming model");
        dub_asr::tcpp::TcppStream::new(&dub_asr::tcpp::engine_dir(root, *build), &dub_asr::tcpp::model_path(root, model), told)
    };
    // under Auto the stream opens once the language of the first speech is heard (Nemotron told none loses its first
    // word); until then nothing is shown of the phrase
    let mut stream = match &streaming {
        Some(_) if !lang.is_empty() => Some(open_stream(lang)?),
        _ => None,
    };
    // samples of `audio` the stream already has
    let mut streamed = 0usize;
    let mut asr = crate::engines::asr::build_engine(&choice, None)?;
    let work = tempfile::Builder::new().prefix("live-").tempdir().map_err(|e| e.to_string())?;
    let mut pending: Vec<f32> = Vec::new();
    let mut offset = at;
    let mut ended = false;
    let mut interim_at = 0.0f64;
    let wav = work.path().join("utterance.wav");
    let mut hear = |piece: &[f32]| -> Result<(String, Vec<HeardWord>), String> {
        let spec = hound::WavSpec { channels: 1, sample_rate: 16_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut w = hound::WavWriter::create(&wav, spec).map_err(|e| e.to_string())?;
        for s in piece {
            w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
        let heard = asr.transcribe(&wav, if lang.is_empty() { "auto" } else { lang }).map_err(|e| e.localize())?;
        // Parakeet times words in 80 ms frames: the last one may end up to a frame past the audio it was given
        let long = piece.len() as f64 / 16_000.0;
        let words: Vec<HeardWord> = heard.iter().flat_map(|s| s.words.iter()).map(|w| (w.word.trim().to_string(), w.start.min(long), w.end.min(long))).filter(|w| !w.0.is_empty()).collect();
        Ok((dub_asr::join_text(heard.iter().map(|s| s.text.as_str())), words))
    };
    let intake = std::cell::RefCell::new(Intake::new(heard_at(at).ok_or_else(|| t!("listen-live-not-running"))?, at));
    // the guard is read again now and then, so a change in the settings reaches a conversation under way
    let mut guard: Option<(std::time::Instant, crate::speech::floor_settings::Guard)> = None;
    let mut take = |chunk: Vec<f32>| -> Vec<f32> {
        let now = std::time::Instant::now();
        let mode = match guard {
            Some((at, g)) if now.duration_since(at) < Duration::from_secs(1) => g,
            before => {
                let g = crate::speech::floor_settings::guard(models_root);
                // the owner's-voice half of the guard follows a change of it as the half-duplex does
                if before.is_some_and(|(_, was)| was != g) && crate::voice_gate::live() {
                    crate::voice_gate::arm_live(models_root, g == crate::speech::floor_settings::Guard::Gate);
                }
                guard = Some((now, g));
                g
            }
        };
        intake.borrow_mut().take(heard_as(chunk), rate(), mode != crate::speech::floor_settings::Guard::Text)
    };
    let mut turn_end = TurnEnd::default();
    let mut turn_mode = crate::speech::floor_settings::turn(models_root);
    let mut turn_read = std::time::Instant::now();
    while !ended {
        if turn_read.elapsed() >= Duration::from_secs(1) {
            turn_mode = crate::speech::floor_settings::turn(models_root);
            turn_read = std::time::Instant::now();
        }
        match from.recv_timeout(Duration::from_millis(300)) {
            Ok(chunk) => pending.extend(take(chunk)),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => ended = true,
        }
        while let Ok(chunk) = from.try_recv() {
            pending.extend(take(chunk));
        }
        ended |= stop.load(Ordering::Relaxed);
        let sr = rate().max(1);
        let audio = to_16k(&pending, sr);
        let secs = audio.len() as f64 / 16_000.0;
        if secs < 1.0 && !ended {
            continue;
        }
        let probs = crate::engines::vad::probabilities(models_root, &audio)?;
        let spans = crate::engines::vad::speech(&probs, audio.len(), &crate::engines::vad::VadConfig::default());
        let Some(&(first, _)) = spans.first() else {
            // silence: keep only its last half second, where speech may be starting
            if secs > 5.0 {
                let keep = (0.5 * sr as f64) as usize;
                let drop = pending.len().saturating_sub(keep);
                pending.drain(..drop);
                offset += drop as f64 / sr as f64;
                intake.borrow_mut().forget_before(offset);
                streamed = streamed.saturating_sub(drop * 16_000 / sr as usize);
            }
            continue;
        };
        let last = spans.last().map(|s| s.1).unwrap_or(first);
        let paused = secs - last >= PAUSE_SECS;
        let turn_said = paused
            && !ended
            && turn_mode == crate::speech::floor_settings::Turn::SmartTurn
            && crate::engines::acpp::smart_turn_gguf(models_root).is_file();
        let ends = !turn_said || {
            let mut failed = None;
            let hear = || -> Result<bool, String> {
                let z = ((last + EDGE_SECS).min(secs) * 16_000.0) as usize;
                let piece = &audio[z.saturating_sub((TURN_HEARS_SECS * 16_000.0) as usize)..z.min(audio.len())];
                let wav = work.path().join("turn.wav");
                write_16k(&wav, piece)?;
                crate::engines::acpp::turn_finished(repo_root, models_root, &wav, TURN_THRESHOLD).map(|(finished, _)| finished)
            };
            let ends = turn_end.ends(secs, last, hear, &mut failed);
            if let Some(e) = failed {
                tracing::error!("[ERROR] live: Smart Turn did not answer, the phrase is ended by the pause: {e}");
                with(|l| {
                    l.turn_error.get_or_insert_with(|| t!("live-turn-failed", error = e));
                });
            }
            ends
        };
        let cut = if ended || (paused && ends) {
            Some((last + EDGE_SECS).min(secs))
        } else if last - first >= LONGEST_SECS {
            // the stream has heard up to the end already: a cut before it would leave those words to the next phrase too
            Some(if stream.is_some() { secs } else { last })
        } else {
            None
        };
        let a = (first - EDGE_SECS).max(0.0);
        let from_a = (a * 16_000.0) as usize;
        if let (Some((root, build, model)), true) = (&streaming, stream.is_none()) {
            let speech = &audio[from_a.min(audio.len())..];
            if speech.len() >= HEAR_LANGUAGE_SAMPLES || cut.is_some() {
                match dub_asr::tcpp::heard_language(root, *build, model, speech)? {
                    Some(told) => stream = Some(open_stream(&told)?),
                    None if speech.len() >= HEAR_LANGUAGE_SAMPLES => stream = Some(open_stream("")?),
                    None => {}
                }
            } else {
                continue;
            }
        }
        let Some(z) = cut else {
            // the phrase goes on: show what it says so far
            if let Some(s) = stream.as_mut() {
                let so_far = s.feed(&audio[streamed.max(from_a).min(audio.len())..])?;
                streamed = audio.len();
                with(|l| l.partial = Some((offset + a, so_far)));
            } else if secs - interim_at >= INTERIM_SECS && last - first >= INTERIM_SECS {
                interim_at = secs;
                let (so_far, _) = hear(&audio[from_a..])?;
                with(|l| l.partial = Some((offset + a, so_far)));
            }
            continue;
        };
        let to_z = ((z * 16_000.0) as usize).min(audio.len());
        let recognising = std::time::Instant::now();
        // the stream answers text only: its phrases carry no word timings
        let (text, words) = match stream.as_mut() {
            Some(s) => {
                s.feed(&audio[streamed.max(from_a).min(to_z)..to_z])?;
                streamed = 0;
                (s.finish()?, Vec::new())
            }
            None => hear(&audio[from_a..to_z])?,
        };
        with(|l| l.partial = None);
        interim_at = 0.0;
        let recognise_ms = recognising.elapsed().as_millis() as u64;
        // from the end of the person's speech to the text: the pause that ends a phrase, then recognising it
        let after_speech_ms = ((secs - last).max(0.0) * 1000.0) as u64 + recognise_ms;
        crate::app::note("asr", json!({
            "engine": label, "lang": if lang.is_empty() { "auto" } else { lang }, "speech_secs": ((z - a) * 100.0).round() / 100.0,
            "recognise_ms": recognise_ms, "after_speech_ms": after_speech_ms, "words": text.split_whitespace().count(),
        }));
        if !text.is_empty() {
            let line = (((offset + a) * 100.0).round() / 100.0, ((offset + z) * 100.0).round() / 100.0, text);
            // under an armed voice gate, judge whether this phrase reaches the agent (owner's voice and, if a wake
            // word is set, that it has woken the conversation), from the phrase's own 16 kHz audio
            let owner = crate::voice_gate::gate_line(models_root, &intake.borrow().heard_part(&audio[from_a.min(to_z)..to_z], offset + from_a.min(to_z) as f64 / 16_000.0));
            with(|l| {
                let n = l.lines.len();
                let (from, to) = (l.zero + Duration::from_secs_f64(line.0.max(0.0)), l.zero + Duration::from_secs_f64(line.1.max(0.0)));
                if crate::speech::is_echo(&line.2, from, to) {
                    l.echo.insert(n);
                }
                if !words.is_empty() {
                    let at = offset + a;
                    l.words.insert(n, words.iter().map(|(w, s, e)| (w.clone(), ((at + s) * 1000.0).round() / 1000.0, ((at + e) * 1000.0).round() / 1000.0)).collect());
                }
                l.lines.push(line);
                if owner == Some(false) {
                    l.foreign.insert(n);
                }
            });
        }
        let used = ((z * sr as f64) as usize).min(pending.len());
        pending.drain(..used);
        offset += used as f64 / sr as f64;
        intake.borrow_mut().forget_before(offset);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LIVE_WAV: a recording to feed as the microphone; SPEECH_STUDIO_ROOT: a studio root with its recognition model and
    /// Silero VAD; LIVE_LANG: the language to hear in (`ru` when not set, `auto` for Auto); LIVE_EXPECT: text a line
    /// must hold (`1945` when not set).
    #[test]
    #[ignore]
    fn a_recording_fed_as_the_microphone_is_heard_phrase_by_phrase() {
        let root = std::path::PathBuf::from(std::env::var("SPEECH_STUDIO_ROOT").expect("SPEECH_STUDIO_ROOT"));
        let models = root.join("models");
        // a test process has no job object of the studio: the engine servers it starts outlive it unless stopped here
        struct StopServers;
        impl Drop for StopServers {
            fn drop(&mut self) {
                acpp_server::SERVERS.stop_all();
            }
        }
        let _servers = StopServers;
        crate::set_ort_dylib_env(&root, &root).unwrap();
        let (x, sr) = crate::audio::wavio::read_mono_f32(Path::new(&std::env::var("LIVE_WAV").expect("LIVE_WAV"))).unwrap();
        let lang = std::env::var("LIVE_LANG").unwrap_or_else(|_| "ru".into());
        let choice = crate::app::recognition::choice(&root, &models, &lang, "test").unwrap();
        let streams = matches!(&choice, crate::engines::models::AsrChoice::Ggml { model, .. } | crate::engines::models::AsrChoice::Server { model, .. } if model.id == "nemotron");
        std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(&models, "asr_backend"));
        *live().lock().unwrap() = Some(Live { lines: Vec::new(), partial: None, error: None, muted: false, typed: Default::default(), foreign: Default::default(), echo: Default::default(), words: Default::default(), duplex_secs: 0.0, turn_error: None, zero: std::time::Instant::now(), engine: String::new(), lang: lang.clone(), stop: Arc::new(AtomicBool::new(false)), worker: None });
        let (to, from) = channel::<Vec<f32>>();
        // LIVE_SPEED: how many times faster than it was recorded the recording is fed (1 when not set)
        let speed: f64 = std::env::var("LIVE_SPEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
        let feeder = std::thread::spawn(move || {
            for chunk in x.chunks((sr / 10) as usize) {
                to.send(chunk.to_vec()).unwrap();
                std::thread::sleep(Duration::from_secs_f64(0.1 / speed.max(0.1)));
            }
        });
        let partials = std::thread::spawn(|| {
            let mut seen = Vec::new();
            for _ in 0..120 {
                if let Some(p) = live().lock().unwrap().as_ref().and_then(|l| l.partial.clone()) {
                    if seen.last() != Some(&p.1) {
                        seen.push(p.1);
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            seen
        });
        let stop = AtomicBool::new(false);
        listen(choice, &root, &models, &lang, from, &stop, &move || sr, 0.0).unwrap();
        feeder.join().unwrap();
        let seen = partials.join().unwrap();
        for p in &seen {
            eprintln!("… {p}");
        }
        assert!(!seen.is_empty(), "no text while the phrase went on");
        let heard = live().lock().unwrap().take().unwrap();
        let lines = heard.lines;
        eprintln!("{} lines heard; Smart Turn: {}", lines.len(), heard.turn_error.as_deref().unwrap_or("no failure"));
        for (n, l) in lines.iter().enumerate() {
            let words = heard.words.get(&n).map(|ws| ws.iter().map(|(w, a, z)| format!("{w}[{a:.2}-{z:.2}]")).collect::<Vec<_>>().join(" "));
            eprintln!("{:.2}-{:.2} {} | words: {}", l.0, l.1, l.2, words.as_deref().unwrap_or("none"));
        }
        if !streams {
            assert!((0..lines.len()).all(|n| heard.words.get(&n).is_some_and(|ws| ws.iter().all(|w| w.1 >= lines[n].0 - 0.01 && w.2 <= lines[n].1 + 0.01))), "every line of a recogniser that times its words carries them, inside the line");
        }
        let expect = std::env::var("LIVE_EXPECT").unwrap_or_else(|_| "1945".into());
        assert!(!lines.is_empty() && lines.iter().any(|l| l.2.contains(&expect)), "{lines:?}");
        assert!(lines.windows(2).all(|w| w[0].1 <= w[1].0 + 0.01));
    }

    #[test]
    fn resampling_to_16k_keeps_the_length_in_time() {
        let x: Vec<f32> = (0..48_000).map(|i| (i as f32 * 0.01).sin()).collect();
        let y = to_16k(&x, 48_000);
        assert_eq!(y.len(), 16_000);
        assert!((y[100] - x[300]).abs() < 1e-3);
        assert_eq!(to_16k(&x[..10], 16_000).len(), 10);
    }

    /// While the studio speaks its sound coming back is held back, the person
    /// speaking over it is heard at once, quieter sounds of theirs right after too; with headphones (no sound coming
    /// back) a quiet voice is heard at the same setting.
    #[test]
    fn the_half_duplex_holds_back_the_studio_and_lets_the_person_through() {
        let t0 = std::time::Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let mut d = Duplex::default();
        let studio = Some(0.1);
        assert!(!d.passes(0.008, studio, at(0)), "the studio heard back through the speakers");
        assert!(!d.passes(0.012, studio, at(100)), "still under its sound by 18 dB");
        assert!(d.passes(0.05, studio, at(200)), "the person speaking over it");
        assert!(d.passes(0.004, studio, at(450)), "their quieter sound right after is theirs");
        assert!(!d.passes(0.004, studio, at(700)), "once they stopped, the studio's sound is held back again");
        assert!(d.passes(0.004, None, at(800)), "the studio is silent: the microphone is heard");
        let mut headphones = Duplex::default();
        assert!(headphones.passes(0.02, studio, at(0)), "a quiet voice over headphones: nothing of the studio's comes back");
    }

    /// The microphone's pieces pile up while the loop recognises a phrase; a piece recorded while the studio spoke is
    /// held back when the loop gets to it later, and one recorded in silence passes though the studio speaks then: a
    /// piece is placed by the recording's clock, not by when it is taken.
    #[test]
    fn a_piece_taken_late_is_judged_by_when_it_was_recorded() {
        let _slot = crate::speech::FLOOR_SLOT.blocking_lock();
        crate::speech::unduck_for_test();
        let zero = std::time::Instant::now() - Duration::from_secs(20);
        crate::speech::level_for_test(zero + Duration::from_secs(1), zero + Duration::from_secs(2), 0.1);
        let mut intake = Intake::new(zero, 0.0);
        let echo = vec![0.008f32; 1_600];
        let heard: Vec<bool> = (0..20).map(|_| intake.take(echo.clone(), 16_000, true).iter().any(|s| *s != 0.0)).collect();
        assert!(heard[..9].iter().all(|h| *h), "recorded in silence: {heard:?}");
        assert!(heard[11..19].iter().all(|h| !*h), "recorded while the studio spoke: {heard:?}");
    }

    /// The studio's voice comes back from the room at its own level, far under the sound sent; once the half-duplex has heard it, a person speaking
    /// 30 dB under the sound sent is heard over it, though under the −18 dB it starts from — the window's volume and
    /// the microphone's gain are in what it measured.
    #[test]
    fn a_quiet_person_is_heard_over_the_echo_the_half_duplex_measured() {
        let t0 = std::time::Instant::now();
        let at = |ms: u64| t0 + Duration::from_millis(ms);
        let studio = Some(0.1f32);
        let level = |db: f32| 0.1 * 10f32.powf(db / 20.0);
        let mut d = Duplex::default();
        for k in 0..30 {
            assert!(!d.passes(level(-48.0 + (k % 5) as f32), studio, at(k * 100)), "the studio's voice coming back");
        }
        assert!(d.passes(level(-30.0), studio, at(3_000)), "a quiet person over it");
        assert!(!d.passes(level(-45.0), studio, at(3_500)), "its echo is still held back after the hold");
        let mut fresh = Duplex::default();
        assert!(!fresh.passes(level(-20.0), studio, at(0)), "before it has heard the room it starts from −18 dB");
    }

    /// A muted microphone is silence already: nothing of it is counted as the studio's sound held back.
    #[test]
    fn a_muted_microphone_is_not_counted_as_echo_held_back() {
        let _slot = crate::speech::FLOOR_SLOT.blocking_lock();
        crate::speech::unduck_for_test();
        let zero = std::time::Instant::now() - Duration::from_secs(25);
        crate::speech::level_for_test(zero, zero + Duration::from_secs(1), 0.1);
        let mut intake = Intake::new(zero, 0.0);
        for _ in 0..10 {
            intake.take(vec![0.0; 1_600], 16_000, true);
        }
        assert!(intake.held.is_empty(), "{:?}", intake.held);
    }

    /// With Smart Turn a paused phrase it hears unfinished waits for more speech or a longer silence; one it hears
    /// finished ends at the pause; it is asked once per end of speech; its failure ends the phrase by the pause and is told.
    #[test]
    fn smart_turn_holds_an_unfinished_phrase_and_lets_a_finished_one_end() {
        let mut turn = TurnEnd::default();
        let mut failed = None;
        let asked = std::cell::Cell::new(0);
        let unfinished = || { asked.set(asked.get() + 1); Ok(false) };
        assert!(!turn.ends(5.8, 5.0, unfinished, &mut failed), "heard unfinished: the phrase waits");
        assert!(!turn.ends(6.5, 5.0, || { asked.set(asked.get() + 1); Ok(false) }, &mut failed), "the answer is kept for this end of speech");
        assert_eq!(asked.get(), 1);
        assert!(turn.ends(5.0 + PAUSE_SECS + TURN_EXTRA_SECS, 5.0, || unreachable!(), &mut failed), "a long enough silence ends it anyway");
        assert!(turn.ends(9.8, 9.0, || Ok(true), &mut failed), "heard finished: ends at the pause");
        assert!(turn.ends(12.8, 12.0, || Err("the server is gone".into()), &mut failed), "a failure ends it by the pause");
        assert_eq!(failed.as_deref(), Some("the server is gone"));
    }

    /// The owner's voice gate hears the phrase's speech only: the pieces the half-duplex held back (silence put in
    /// their place) are left out, as silence was left out of the owner's print.
    #[test]
    fn the_voice_gate_gets_the_phrase_without_what_was_held_back() {
        let _slot = crate::speech::FLOOR_SLOT.blocking_lock();
        crate::speech::unduck_for_test();
        let zero = std::time::Instant::now() - Duration::from_secs(40);
        crate::speech::level_for_test(zero + Duration::from_secs(31), zero + Duration::from_secs(32), 0.1);
        let mut intake = Intake::new(zero + Duration::from_secs(30), 30.0);
        let mut audio: Vec<f32> = Vec::new();
        for k in 0..30 {
            audio.extend(intake.take(vec![if k < 8 || k >= 25 { 0.3 } else { 0.008 }; 1_600], 16_000, true));
        }
        let silenced = audio.iter().filter(|s| **s == 0.0).count();
        assert!(silenced >= 16_000, "the studio's second was held back: {silenced} samples");
        let heard = intake.heard_part(&audio, 30.0);
        assert_eq!(heard.len(), audio.len() - silenced, "every held back sample is left out, nothing else");
        assert!(heard.iter().all(|s| *s != 0.0));
    }

    /// The half-duplex measured on Live conversations as they were recorded (no model runs): 100 ms frames of the
    /// person's speech and of the studio's voice coming back, each against the level of the studio's sound sent to the
    /// window (its take's speech); what share of the person's frames a rule holds back and what share of the echo it
    /// lets through, the window at full volume and at 0.3. GATE_ROOT: a studio root; GATE_LIVE: the Listening projects
    /// of the recordings, comma separated; GATE_SAID: the say sessions' projects.
    #[test]
    #[ignore = "needs a studio root with recorded Live conversations"]
    fn the_half_duplex_measured_on_recorded_conversations() {
        let root = std::path::PathBuf::from(std::env::var("GATE_ROOT").expect("GATE_ROOT"));
        let workspace = root.join("workspace");
        let db = |x: f32| 20.0 * x.max(1e-9).log10();
        let speech_frames = |x: &[f32]| -> Vec<f32> {
            let rms: Vec<f32> = x.chunks(1_600).map(crate::speech::rms).collect();
            let loudest = rms.iter().copied().fold(0.0f32, f32::max);
            rms.into_iter().filter(|r| *r >= loudest * 0.05).collect()
        };
        let tmp = tempfile::tempdir().unwrap();
        let mut said: Vec<(Vec<String>, f32)> = Vec::new();
        for pid in std::env::var("GATE_SAID").unwrap_or_default().split(',').filter(|p| !p.is_empty()) {
            let p: Value = serde_json::from_slice(&std::fs::read(workspace.join(pid).join("project.json")).unwrap()).unwrap();
            for b in p["chapters"].as_array().unwrap().iter().flat_map(|c| c["blocks"].as_array().unwrap()) {
                let Some(f) = b["takes"].as_array().and_then(|t| t.iter().find(|t| t["n"] == b["active"])).and_then(|t| t["file"].as_str()) else { continue };
                let k = tmp.path().join(format!("{f}.wav"));
                crate::audio::media::to_16k_mono(&workspace.join(pid).join("takes").join(f), &k).unwrap();
                let frames = speech_frames(&crate::audio::wavio::read_mono_f32(&k).unwrap().0);
                let level = (frames.iter().map(|r| r * r).sum::<f32>() / frames.len().max(1) as f32).sqrt();
                said.push((crate::audio::qc::normalize(b["text"].as_str().unwrap_or_default()), level));
            }
        }
        let typical = {
            let mut l: Vec<f32> = said.iter().map(|s| s.1).filter(|l| *l > 0.0).collect();
            l.sort_by(f32::total_cmp);
            l[l.len() / 2]
        };
        let (mut person, mut echo): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
        for pid in std::env::var("GATE_LIVE").expect("GATE_LIVE").split(',').filter(|p| !p.is_empty()) {
            let dir = workspace.join(pid);
            let p: Value = serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
            let (x, sr) = crate::audio::wavio::read_mono_f32(&dir.join("source.wav")).unwrap();
            let x16 = to_16k(&x, sr);
            for l in p["lines"].as_array().unwrap() {
                let (a, z) = (l["start"].as_f64().unwrap(), l["end"].as_f64().unwrap());
                let piece = &x16[((a * 16_000.0) as usize).min(x16.len())..((z * 16_000.0) as usize).min(x16.len())];
                let words = crate::audio::qc::normalize(l["text"].as_str().unwrap_or_default());
                let studio = (!words.is_empty()).then(|| said.iter().find(|s| words.iter().filter(|w| s.0.contains(w)).count() * 3 >= words.len() * 2)).flatten();
                for f in speech_frames(piece) {
                    match studio {
                        Some(s) => echo.push(db(f) - db(s.1)),
                        None => person.push(db(f) - db(typical)),
                    }
                }
            }
        }
        let share = |xs: &[f32], f: &dyn Fn(f32) -> bool| 100.0 * xs.iter().filter(|x| f(**x)).count() as f64 / xs.len().max(1) as f64;
        let median = |xs: &[f32]| {
            let mut v = xs.to_vec();
            v.sort_by(f32::total_cmp);
            v.get(v.len() / 2).copied().unwrap_or(f32::NAN)
        };
        eprintln!("{} frames of the person's, {} of the studio's voice coming back; the studio's sound sent at {:.1} dBFS", person.len(), echo.len(), db(typical));
        eprintln!("person against the sound sent: median {:.1} dB; echo: median {:.1} dB", median(&person), median(&echo));
        let quantiles = |xs: &[f32]| {
            let mut v = xs.to_vec();
            v.sort_by(f32::total_cmp);
            [5, 10, 25, 50, 75, 90, 95].map(|q| format!("p{q} {:.1}", v.get(v.len() * q / 100).copied().unwrap_or(f32::NAN))).join(", ")
        };
        eprintln!("person: {}", quantiles(&person));
        eprintln!("echo: {}", quantiles(&echo));
        for volume in [1.0f32, 0.3] {
            let v = db(volume);
            let echo_v: Vec<f32> = echo.iter().map(|e| e + v).collect();
            eprintln!("window volume {volume}: fixed {DUPLEX_DB} dB — person held back {:.1} %, echo let through {:.1} %", share(&person, &|p| p < DUPLEX_DB), share(&echo_v, &|e| e >= DUPLEX_DB));
            let coupling = median(&echo_v);
            for margin in [6.0f32, 8.0, 10.0, 12.0] {
                let t = coupling + margin;
                eprintln!("  measured echo {coupling:.1} dB + {margin} dB = {t:.1} dB — person held back {:.1} %, echo let through {:.1} %", share(&person, &|p| p < t), share(&echo_v, &|e| e >= t));
            }
        }
    }
}
