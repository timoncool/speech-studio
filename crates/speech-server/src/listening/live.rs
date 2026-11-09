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
        zero: std::time::Instant::now(),
        engine: choice.label(),
        lang: lang.to_string(),
        stop: stop.clone(),
        worker: None,
    });
    run(choice, models_root, lang, stop, 0.0);
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
    run(choice, models_root, &lang, stop, at);
    Ok(label)
}

/// The recogniser hearing the running conversation, and its language; None with no conversation.
pub fn engine() -> Option<(String, String)> {
    with(|l| (l.engine.clone(), l.lang.clone()))
}

/// Recognises what the recorder hears from now on, its phrases placed `at` seconds into the recording.
fn run(choice: crate::engines::models::AsrChoice, models_root: &Path, lang: &str, stop: Arc<AtomicBool>, at: f64) {
    let models_root = models_root.to_path_buf();
    let (to, from) = channel::<Vec<f32>>();
    crate::voices::record::tap(Some(to));
    std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(&models_root, "asr_backend"));
    let lang = if lang == "auto" { String::new() } else { lang.to_string() };
    let worker = std::thread::spawn(move || {
        if let Err(e) = listen(choice, &models_root, &lang, from, &stop, &crate::voices::record::rate, at) {
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

/// What live recognition has heard so far, and why it stopped when it did.
pub fn state() -> Value {
    match live().lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        Some(l) => json!({
            "running": !l.stop.load(Ordering::Relaxed),
            "lines": l.lines.iter().enumerate().map(|(n, (a, z, text))| json!({ "start": a, "end": z, "text": text, "typed": l.typed.contains(&n), "owner": !l.foreign.contains(&n), "echo": l.echo.contains(&n) })).collect::<Vec<_>>(),
            "partial": l.partial.as_ref().map(|(a, text)| json!({ "start": a, "text": text })),
            "error": l.error,
            "muted": l.muted,
        }),
        None => json!({ "running": false, "lines": [], "partial": null, "error": null, "muted": false }),
    }
}

/// The moment `secs` of the running recording was heard; None with no conversation.
pub fn heard_at(secs: f64) -> Option<std::time::Instant> {
    with(|l| l.zero + Duration::from_secs_f64(secs.max(0.0)))
}

/// Serialises the tests that put a conversation of their own into the one live slot.
#[cfg(test)]
pub static TEST_SLOT: Mutex<()> = Mutex::new(());

/// A conversation that heard `lines` (start, end, text, typed, the owner's voice), its second 0 being `zero`, for tests.
#[cfg(test)]
pub fn conversation_for_test(zero: std::time::Instant, lines: &[(f64, f64, &str, bool, bool)]) {
    let mut l = Live { lines: Vec::new(), partial: None, error: None, muted: false, typed: Default::default(), foreign: Default::default(), echo: Default::default(), zero, engine: String::new(), lang: "ru".into(), stop: Arc::new(AtomicBool::new(false)), worker: None };
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

fn listen(choice: crate::engines::models::AsrChoice, models_root: &Path, lang: &str, from: Receiver<Vec<f32>>, stop: &AtomicBool, rate: &dyn Fn() -> u32, at: f64) -> Result<(), String> {
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
    let mut hear = |piece: &[f32]| -> Result<String, String> {
        let spec = hound::WavSpec { channels: 1, sample_rate: 16_000, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut w = hound::WavWriter::create(&wav, spec).map_err(|e| e.to_string())?;
        for s in piece {
            w.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
        let heard = asr.transcribe(&wav, if lang.is_empty() { "auto" } else { lang }).map_err(|e| e.localize())?;
        Ok(dub_asr::join_text(heard.iter().map(|s| s.text.as_str())))
    };
    while !ended {
        match from.recv_timeout(Duration::from_millis(300)) {
            Ok(chunk) => pending.extend(heard_as(chunk)),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => ended = true,
        }
        while let Ok(chunk) = from.try_recv() {
            pending.extend(heard_as(chunk));
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
                streamed = streamed.saturating_sub(drop * 16_000 / sr as usize);
            }
            continue;
        };
        let last = spans.last().map(|s| s.1).unwrap_or(first);
        let cut = if ended || secs - last >= PAUSE_SECS {
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
                let so_far = hear(&audio[from_a..])?;
                with(|l| l.partial = Some((offset + a, so_far)));
            }
            continue;
        };
        let to_z = ((z * 16_000.0) as usize).min(audio.len());
        let recognising = std::time::Instant::now();
        let text = match stream.as_mut() {
            Some(s) => {
                s.feed(&audio[streamed.max(from_a).min(to_z)..to_z])?;
                streamed = 0;
                s.finish()?
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
            let owner = crate::voice_gate::gate_line(models_root, &audio[from_a.min(to_z)..to_z]);
            with(|l| {
                let n = l.lines.len();
                let (from, to) = (l.zero + Duration::from_secs_f64(line.0.max(0.0)), l.zero + Duration::from_secs_f64(line.1.max(0.0)));
                if crate::speech::is_echo(&line.2, from, to) {
                    l.echo.insert(n);
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
        std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(&models, "asr_backend"));
        *live().lock().unwrap() = Some(Live { lines: Vec::new(), partial: None, error: None, muted: false, typed: Default::default(), foreign: Default::default(), echo: Default::default(), zero: std::time::Instant::now(), engine: String::new(), lang: lang.clone(), stop: Arc::new(AtomicBool::new(false)), worker: None });
        let (to, from) = channel::<Vec<f32>>();
        let feeder = std::thread::spawn(move || {
            for chunk in x.chunks((sr / 10) as usize) {
                to.send(chunk.to_vec()).unwrap();
                std::thread::sleep(Duration::from_millis(100));
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
        listen(choice, &models, &lang, from, &stop, &move || sr, 0.0).unwrap();
        feeder.join().unwrap();
        let seen = partials.join().unwrap();
        for p in &seen {
            eprintln!("… {p}");
        }
        assert!(!seen.is_empty(), "no text while the phrase went on");
        let lines = live().lock().unwrap().take().unwrap().lines;
        for l in &lines {
            eprintln!("{:.2}-{:.2} {}", l.0, l.1, l.2);
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
}
