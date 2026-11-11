//! Voice gate for Live: the studio can listen passively and start a conversation only on the owner's voice (and,
//! when set, a chosen wake word). The owner enrols once by speaking a phrase; a speaker embedding (ReDimNet2-B3, SOTA
//! speaker verification, 192-d, run on ONNX Runtime) is kept, and later speech is let through only when its embedding
//! matches the enrolled one by cosine similarity. Off by default.

use crate::wake_word::says_wake;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The embedder works on 16 kHz mono audio.
pub const SR: u32 = 16_000;
/// Default cosine threshold for "the owner's voice": at or above it the same speaker. Tuneable in settings.
pub const DEFAULT_THRESHOLD: f32 = 0.5;
/// ReDimNet2-B3 embedding width.
pub const EMB_DIM: usize = 192;

/// ReDimNet2-B3 speaker embedder: the waveform-input ONNX graph (the mel frontend is inside it, opset 17, no STFT), so
/// it takes raw 16 kHz mono f32 in [-1, 1] of any length and returns a 192-d embedding. The onnxruntime dylib is set
/// for the process at start (`app::runtime`), so loading needs no further setup.
pub struct Embedder {
    session: Session,
}

impl Embedder {
    /// Load the model; `gpu` runs it on CUDA (a failure to reach CUDA is an error, not a silent fall back to CPU).
    pub fn load(onnx: &Path, gpu: bool) -> Result<Self, String> {
        let mut builder = crate::app::ort_session()?;
        if gpu {
            builder = builder
                .with_execution_providers([ort::ep::CUDA::default().build().error_on_failure()])
                .map_err(|e| format!("CUDA for {}: {e}", onnx.display()))?;
        }
        let session = builder.commit_from_file(onnx).map_err(|e| format!("commit_from_file {}: {e}", onnx.display()))?;
        Ok(Self { session })
    }

    /// A 192-d L2-normalized embedding of 16 kHz mono samples in [-1, 1]; the model takes the raw waveform of any
    /// length and does its own feature extraction.
    pub fn embed(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        if samples.is_empty() {
            return Err("voice gate: empty audio".into());
        }
        let dims: Vec<i64> = vec![1, samples.len() as i64];
        let tensor = TensorRef::from_array_view((dims, samples)).map_err(|e| format!("voice gate tensor: {e}"))?;
        let outputs = self.session.run(ort::inputs![tensor]).map_err(|e| format!("voice gate run: {e}"))?;
        let (_name, value) = outputs.iter().next().ok_or("voice gate: no output")?;
        let (_shape, data) = value.try_extract_tensor::<f32>().map_err(|e| format!("voice gate extract: {e}"))?;
        if data.len() != EMB_DIM {
            return Err(format!("voice gate: expected {EMB_DIM} dims, got {}", data.len()));
        }
        Ok(l2(data.to_vec()))
    }
}

/// Where the model lives under the models root (the download component unpacks it here).
pub fn model_path(models_root: &Path) -> PathBuf {
    models_root.join("voice-gate").join("redimnet2_b3_lm.onnx")
}

/// L2-normalize; a zero vector is left as is.
fn l2(mut v: Vec<f32>) -> Vec<f32> {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 1e-8 {
        for x in &mut v {
            *x /= n;
        }
    }
    v
}

/// Cosine similarity of two embeddings. Both are L2-normalized already, so this is their dot product, but it
/// normalizes defensively and returns 0 for a mismatched or empty pair.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-8 || nb < 1e-8 {
        0.0
    } else {
        dot / (na * nb)
    }
}

/// Phone-overlap at or above which the set wake phrase counts as heard. The recogniser is imperfect, so an exact
/// match is not required; tuned at the end-to-end test on real speech.
pub const WAKE_THRESHOLD: f32 = 0.6;

/// How the owner set the gate up. Off by default: the feature is opt-in.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// The gate is on: a conversation in Live starts only on the owner's voice.
    pub enabled: bool,
    /// Cosine at or above which the speech is the owner (conservative by default — fewer false activations).
    pub threshold: f32,
    /// A wake word that, when set, must be heard before the conversation starts; None for any speech (gated by voice).
    pub wake_word: Option<String>,
    /// The language of the wake phrase for grapheme-to-phoneme (an eSpeak-ng voice code such as "ru", "en", "cmn");
    /// None defaults to English. The settings pre-fill it with the interface language.
    #[serde(default)]
    pub wake_lang: Option<String>,
    /// Phone overlap at or above which the wake phrase counts as heard — its strictness, set by the UI slider.
    #[serde(default = "default_wake_threshold")]
    pub wake_threshold: f32,
    /// Start Speech Studio on login (per-OS). Off by default.
    pub autostart: bool,
}

fn default_wake_threshold() -> f32 {
    WAKE_THRESHOLD
}

impl Default for Config {
    fn default() -> Self {
        Self { enabled: false, threshold: DEFAULT_THRESHOLD, wake_word: None, wake_lang: None, wake_threshold: WAKE_THRESHOLD, autostart: false }
    }
}

/// Example wake phrases for the settings field (hints only — any phrase in any language works).
pub fn wake_words() -> &'static [&'static str] {
    &["Привет Анифса", "Эй Клод", "Hey Claude", "Okey Studio"]
}

/// The gate's own folder next to the model, where its settings and the owner's voice print live.
fn gate_dir(models_root: &Path) -> PathBuf {
    models_root.join("voice-gate")
}

/// The gate's settings; defaults (off) when none are saved.
pub fn config(models_root: &Path) -> Config {
    std::fs::read(gate_dir(models_root).join("config.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Save the gate's settings.
pub fn save_config(models_root: &Path, cfg: &Config) -> Result<(), String> {
    let dir = gate_dir(models_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join("config.json");
    std::fs::write(&path, serde_json::to_vec_pretty(cfg).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", path.display()))
}

/// The owner's voice print as it is kept: taken from the speech of the enrolment recording only, and how much of it.
#[derive(serde::Serialize, serde::Deserialize)]
struct Print {
    print: Vec<f32>,
    speech_secs: f64,
}

/// What the gate holds of the owner's voice.
enum Enrolment {
    None,
    /// A print an earlier version took from the whole recording, silence included: it does not recognise the owner.
    Stale,
    Ready(Vec<f32>),
}

fn enrolment(models_root: &Path) -> Enrolment {
    let Ok(body) = std::fs::read(gate_dir(models_root).join("owner.json")) else {
        return Enrolment::None;
    };
    if let Ok(p) = serde_json::from_slice::<Print>(&body) {
        if p.print.len() == EMB_DIM {
            return Enrolment::Ready(p.print);
        }
    }
    if serde_json::from_slice::<Vec<f32>>(&body).is_ok_and(|v| v.len() == EMB_DIM) {
        return Enrolment::Stale;
    }
    tracing::warn!("voice gate: {} is not a voice print", gate_dir(models_root).join("owner.json").display());
    Enrolment::None
}

/// The owner's enrolled voice print (192-d, L2-normalized), or None if no one has enrolled or the print is stale.
pub fn owner_print(models_root: &Path) -> Option<Vec<f32>> {
    match enrolment(models_root) {
        Enrolment::Ready(p) => Some(p),
        _ => None,
    }
}

/// Whether the print kept is an earlier version's, taken with the recording's silence: the owner enrols again.
pub fn stale(models_root: &Path) -> bool {
    matches!(enrolment(models_root), Enrolment::Stale)
}

/// Whether the owner has enrolled a voice.
pub fn enrolled(models_root: &Path) -> bool {
    owner_print(models_root).is_some()
}

/// Forget the owner's voice (the privacy delete).
pub fn forget_owner(models_root: &Path) -> Result<(), String> {
    let path = gate_dir(models_root).join("owner.json");
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// The embedder is loaded once and kept for the process.
fn with_embedder<T>(models_root: &Path, gpu: bool, f: impl FnOnce(&mut Embedder) -> Result<T, String>) -> Result<T, String> {
    static CACHE: Mutex<Option<Embedder>> = Mutex::new(None);
    let mut slot = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    if slot.is_none() {
        *slot = Some(Embedder::load(&model_path(models_root), gpu)?);
    }
    f(slot.as_mut().expect("just loaded"))
}

/// Enrol the owner from 16 kHz mono samples in [-1, 1]: keep the voice print of their speech, the silence and noise
/// around it left out (a print taken with them matches no phrase of the owner's). Returns the seconds of speech used.
pub fn enroll(models_root: &Path, gpu: bool, samples: &[f32]) -> Result<f64, String> {
    let probs = crate::engines::vad::probabilities(models_root, samples)?;
    let spans = crate::engines::vad::speech(&probs, samples.len(), &crate::engines::vad::VadConfig::default());
    let speech = speech_of(samples, &spans);
    let speech_secs = speech.len() as f64 / SR as f64;
    if speech_secs < 1.0 {
        return Err(t!("voice-gate-too-short"));
    }
    let print = with_embedder(models_root, gpu, |e| e.embed(&speech))?;
    let dir = gate_dir(models_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join("owner.json");
    std::fs::write(&path, serde_json::to_vec(&Print { print, speech_secs }).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(speech_secs)
}

/// The samples of `spans` (seconds) put one after another.
fn speech_of(samples: &[f32], spans: &[(f64, f64)]) -> Vec<f32> {
    spans
        .iter()
        .flat_map(|(a, z)| {
            let (a, z) = (((a * SR as f64) as usize).min(samples.len()), ((z * SR as f64) as usize).min(samples.len()));
            samples[a..z.max(a)].iter().copied()
        })
        .collect()
}

/// The cosine of this speech against the enrolled owner; None if no one is enrolled.
pub fn score(models_root: &Path, gpu: bool, samples: &[f32]) -> Result<Option<f32>, String> {
    let Some(owner) = owner_print(models_root) else {
        return Ok(None);
    };
    let print = with_embedder(models_root, gpu, |e| e.embed(samples))?;
    Ok(Some(cosine(&owner, &print)))
}


/// A rolling window of the most recent 16 kHz speech while the voice test is on, fed from the microphone tap, so the
/// live match against the enrolled voice can be shown and the owner can tune the sensitivity by it.
static TEST_WINDOW: Mutex<Vec<f32>> = Mutex::new(Vec::new());
/// Keep at most the last three seconds — enough for a stable match, recent enough to follow what is being said.
const TEST_WINDOW_SAMPLES: usize = 3 * SR as usize;

/// Feed 16 kHz mono samples into the rolling test window.
pub fn test_push(samples: &[f32]) {
    let mut w = TEST_WINDOW.lock().unwrap_or_else(|p| p.into_inner());
    w.extend_from_slice(samples);
    if w.len() > TEST_WINDOW_SAMPLES {
        let drop = w.len() - TEST_WINDOW_SAMPLES;
        w.drain(..drop);
    }
}

/// Empty the test window (the test stopped).
pub fn test_clear() {
    TEST_WINDOW.lock().unwrap_or_else(|p| p.into_inner()).clear();
}

/// The live match of the test window against the enrolled voice, 0..=1; None while there is under a second of audio or
/// no one is enrolled. CPU: a short window is light and keeps off the agent's GPU.
pub fn test_score(models_root: &Path) -> Result<Option<f32>, String> {
    let samples = TEST_WINDOW.lock().unwrap_or_else(|p| p.into_inner()).clone();
    if samples.len() < SR as usize {
        return Ok(None);
    }
    score(models_root, use_gpu(), &samples)
}


/// The live phone overlap of the test window with the configured wake phrase, 0..=1; None while there is under a second
/// of audio, no phrase is set, or the recogniser is not installed. Reuses the voice check's microphone tap and window.
pub fn wake_test_score(models_root: &Path) -> Result<Option<f32>, String> {
    let cfg = config(models_root);
    let phrase = match cfg.wake_word {
        Some(p) if !p.trim().is_empty() => p,
        _ => return Ok(None),
    };
    // empty when unset, so overlap_score guesses the language from the phrase's script (a hard "en" would misread a
    // non-Latin phrase and never match)
    let lang = cfg.wake_lang.unwrap_or_default();
    let full = TEST_WINDOW.lock().unwrap_or_else(|p| p.into_inner()).clone();
    if full.len() < SR as usize {
        return Ok(None);
    }
    // only the most recent ~1.5 s: the phrase is short, and a shorter clip is a faster recogniser pass, so the live
    // check tracks the voice without lag
    let recent = &full[full.len().saturating_sub((SR as usize) * 3 / 2)..];
    crate::wake_word::overlap_score(models_root, use_gpu(), &lang, &phrase, recent)
}

/// Whether this speech is the owner, at or above the configured threshold.
pub fn is_owner(models_root: &Path, gpu: bool, cfg: &Config, samples: &[f32]) -> Result<bool, String> {
    Ok(score(models_root, gpu, samples)?.is_some_and(|s| s >= cfg.threshold))
}

/// The gate is armed only for a Live conversation (set when one starts), so the live recogniser checks the owner's
/// voice there and not when a recording is simply being transcribed.
static ARMED: AtomicBool = AtomicBool::new(false);
/// With a wake word set, the conversation stays dormant until the phrase is heard; this turns true once it is, for
/// the rest of the Live session. Reset whenever the gate is (dis)armed, so every new conversation starts dormant.
static AWAKE: AtomicBool = AtomicBool::new(false);

/// Arm or disarm the gate for the Live conversation; either way the wake-word activation resets to dormant.
pub fn arm(on: bool) {
    ARMED.store(on, Ordering::Relaxed);
    AWAKE.store(false, Ordering::Relaxed);
}

/// Live keeps the studio from hearing itself by the owner's voice (`live_echo_guard` gate): the gate checks the owner's
/// voice in the conversation though it is not turned on for starting one.
static OWNER_GUARD: AtomicBool = AtomicBool::new(false);

/// The owner's voice keeps the studio from hearing itself in the conversation being armed, or no longer.
pub fn guard_owner(on: bool) {
    OWNER_GUARD.store(on, Ordering::Relaxed);
}

/// Arms the gate for a Live conversation, or no longer (`live_gate_on`).
pub fn arm_live(models_root: &Path, owner_guard: bool) {
    LIVE.store(true, Ordering::Relaxed);
    guard_owner(owner_guard);
    arm(live_gate_on(models_root, owner_guard));
}

/// The Live conversation ended: the gate is no longer its.
pub fn disarm_live() {
    LIVE.store(false, Ordering::Relaxed);
    arm(false);
}

/// Whether a Live conversation holds the gate.
pub fn live() -> bool {
    LIVE.load(Ordering::Relaxed)
}

static LIVE: AtomicBool = AtomicBool::new(false);

/// Whether a Live conversation is gated: the person turned on the owner's-voice gate, set a wake word, or the echo
/// guard keeps the studio from hearing itself by the owner's voice (`owner_guard`).
pub fn live_gate_on(models_root: &Path, owner_guard: bool) -> bool {
    let cfg = config(models_root);
    cfg.enabled || owner_guard || cfg.wake_word.as_deref().map(str::trim).is_some_and(|w| !w.is_empty())
}

/// Whether a gated Live conversation is under way.
pub fn is_armed() -> bool {
    ARMED.load(Ordering::Relaxed)
}

/// Whether the wake word has already woken this conversation (or none is set).
pub fn is_awake() -> bool {
    AWAKE.load(Ordering::Relaxed)
}

/// The device the voice gate runs its ONNX models on: the GPU when the card chosen in the settings can run the
/// studio's CUDA build, otherwise the CPU — the same decision the rest of the studio makes, never a hard-coded device.
fn use_gpu() -> bool {
    crate::engines::hw::gpu_report().cuda13_ok
}

/// For the live recogniser: whether a phrase reaches the agent. `None` means the gate does not apply here (not armed,
/// or nothing is configured) so the phrase is let through. `Some(false)` suppresses the phrase — it is not the owner,
/// or a wake word is set and has not been heard yet. `Some(true)` lets it through. The checks run on CPU: they fire
/// only on short spoken phrases, and the 582 MB wake recogniser runs only while dormant, so keeping both off VRAM
/// leaves the GPU free for the voicing and recognition that must be fast once the conversation is under way. On any
/// error the gate fails open (a slip never silences a real conversation).
pub fn gate_line(models_root: &Path, samples: &[f32]) -> Option<bool> {
    if !is_armed() {
        return None;
    }
    let cfg = config(models_root);
    let owner_gate = (cfg.enabled || OWNER_GUARD.load(Ordering::Relaxed)) && enrolled(models_root);
    let wake = cfg.wake_word.as_deref().map(str::trim).filter(|w| !w.is_empty());
    if !owner_gate && wake.is_none() {
        return None;
    }
    // the ONNX checks run on the GPU when the chosen card supports it, like the rest of the studio, else on the CPU
    let gpu = use_gpu();
    // speaker identity, when the owner gate is on; otherwise any voice counts as the owner for the wake step
    let is_owner_voice = if owner_gate {
        match is_owner(models_root, gpu, &cfg, samples) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("voice gate check: {e}");
                return None; // fail open
            }
        }
    } else {
        true
    };
    // with a wake word set, stay dormant until the phrase is heard (from the owner's voice, when the gate is on)
    if let Some(phrase) = wake {
        if !is_awake() {
            let lang = cfg.wake_lang.as_deref().unwrap_or("");
            let woke = is_owner_voice && says_wake(models_root, gpu, lang, phrase, cfg.wake_threshold, samples).unwrap_or(false);
            if woke {
                AWAKE.store(true, Ordering::Relaxed);
                return Some(true);
            }
            return Some(false);
        }
    }
    Some(is_owner_voice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_makes_a_unit_vector() {
        let v = l2(vec![3.0, 4.0]);
        let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_is_one_for_the_same_vector_and_zero_for_orthogonal() {
        let a = l2(vec![0.2, 0.1, 0.4, 0.3]);
        assert!((cosine(&a, &a) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[1.0, 0.0], &[1.0]), 0.0); // mismatched lengths
        assert_eq!(cosine(&[], &[]), 0.0);
    }

    /// A print taken with the recording's silence does not recognise the owner's phrases: it is told as stale and
    /// never used, a print of the speech alone is.
    #[test]
    fn a_print_taken_with_the_silence_is_stale_and_one_of_the_speech_is_used() {
        let dir = tempfile::tempdir().unwrap();
        let gate = dir.path().join("voice-gate");
        std::fs::create_dir_all(&gate).unwrap();
        assert!(!enrolled(dir.path()) && !stale(dir.path()));
        std::fs::write(gate.join("owner.json"), serde_json::to_vec(&vec![0.1f32; EMB_DIM]).unwrap()).unwrap();
        assert!(stale(dir.path()));
        assert!(!enrolled(dir.path()), "the gate does not use it");
        std::fs::write(gate.join("owner.json"), serde_json::to_vec(&Print { print: vec![0.1; EMB_DIM], speech_secs: 5.9 }).unwrap()).unwrap();
        assert!(enrolled(dir.path()) && !stale(dir.path()));
        let x: Vec<f32> = (0..32_000).map(|i| i as f32).collect();
        let speech = speech_of(&x, &[(0.5, 1.0), (1.5, 1.75)]);
        assert_eq!(speech.len(), 8_000 + 4_000);
        assert_eq!(speech[0], 8_000.0);
        assert_eq!(speech[8_000], 24_000.0);
    }

    fn tone(hz: f32, seconds: usize) -> Vec<f32> {
        let n = SR as usize * seconds;
        (0..n).map(|i| 0.3 * (2.0 * std::f32::consts::PI * hz * i as f32 / SR as f32).sin()).collect()
    }

    /// Runs the real model on this machine: set the model path and the onnxruntime dylib, e.g.
    /// SPEECH_VOICE_GATE_MODEL=D:\...\redimnet2_b3_lm.onnx ORT_DYLIB_PATH=...\onnxruntime-win-x64-1.31.1\lib\onnxruntime.dll
    #[test]
    #[ignore = "needs the ONNX model and onnxruntime dylib via env"]
    fn the_embedder_runs_on_ort_and_a_signal_matches_itself() {
        let path = std::env::var("SPEECH_VOICE_GATE_MODEL").expect("set SPEECH_VOICE_GATE_MODEL to the onnx file");
        let mut e = Embedder::load(std::path::Path::new(&path), false).expect("load");
        let a = e.embed(&tone(220.0, 3)).expect("embed a");
        assert_eq!(a.len(), EMB_DIM);
        assert!((cosine(&a, &a) - 1.0).abs() < 1e-5, "a voice matches itself");
        let b = e.embed(&tone(660.0, 3)).expect("embed b");
        assert!(cosine(&a, &b) < 0.999, "a different signal is not identical: {}", cosine(&a, &b));
    }

    /// The gate measured on Live conversations as they were recorded: each line of the recordings scored against the
    /// owner's voice print, labelled the studio's when its words are a phrase the studio said (the say sessions' blocks),
    /// the owner's otherwise; and how loud the studio's phrases came back through the microphone against their own
    /// sound, for the half-duplex. GATE_ROOT: a studio root (its models, voice print and workspace); GATE_LIVE: the
    /// Listening projects of the recordings, comma separated; GATE_SAID: the say sessions' projects. Runs on the CPU.
    #[test]
    #[ignore = "needs a studio root with recorded Live conversations"]
    fn the_gate_measured_on_recorded_conversations() {
        let root = PathBuf::from(std::env::var("GATE_ROOT").expect("GATE_ROOT"));
        let models = root.join("models");
        let workspace = root.join("workspace");
        crate::set_ort_dylib_env(&root);
        // the speech of a sound: its 30 ms frames no quieter than a twentieth of its loudest
        let speech_of = |x: &[f32]| -> Vec<f32> {
            let loudest = x.chunks(480).map(crate::speech::rms).fold(0.0f32, f32::max);
            x.chunks(480).filter(|f| crate::speech::rms(f) >= loudest * 0.05).flatten().copied().collect()
        };
        let tmp = tempfile::tempdir().unwrap();
        let mut said: Vec<(Vec<String>, Option<f32>)> = Vec::new();
        for pid in std::env::var("GATE_SAID").unwrap_or_default().split(',').filter(|p| !p.is_empty()) {
            let p: serde_json::Value = serde_json::from_slice(&std::fs::read(workspace.join(pid).join("project.json")).unwrap()).unwrap();
            for ch in p["chapters"].as_array().unwrap() {
                for b in ch["blocks"].as_array().unwrap() {
                    let take = b["takes"].as_array().and_then(|t| t.iter().find(|t| t["n"] == b["active"])).and_then(|t| t["file"].as_str());
                    let level = take.and_then(|f| {
                        let k = tmp.path().join(format!("{f}.wav"));
                        crate::audio::media::to_16k_mono(&workspace.join(pid).join("takes").join(f), &k).ok()?;
                        Some(crate::speech::rms(&speech_of(&crate::audio::wavio::read_mono_f32(&k).ok()?.0)))
                    });
                    said.push((crate::audio::qc::normalize(b["text"].as_str().unwrap_or_default()), level));
                }
            }
        }
        let studio_said = |words: &[String]| -> Option<Option<f32>> {
            if words.is_empty() {
                return None;
            }
            said.iter().find(|s| words.iter().filter(|w| s.0.contains(w)).count() * 3 >= words.len() * 2).map(|s| s.1)
        };
        let body = std::fs::read(models.join("voice-gate").join("owner.json")).unwrap();
        let owner: Vec<f32> = serde_json::from_slice::<Print>(&body).map(|p| p.print).or_else(|_| serde_json::from_slice::<Vec<f32>>(&body)).expect("an enrolled voice print");
        let mut e = Embedder::load(&model_path(&models), false).unwrap();
        // the enrolment recording again, its speech only: the print the gate would hold without the silence around it
        let (test, test_sr) = crate::audio::wavio::read_mono_f32(&workspace.join(".mic-test.wav")).unwrap();
        let test16 = crate::audio::wavio::to_16k(&test, test_sr);
        let speech = speech_of(&test16);
        let trimmed = e.embed(&speech).unwrap();
        eprintln!("enrolment: {:.1} s recorded, {:.1} s of speech; stored print against the trimmed one {:.3}", test16.len() as f64 / 16_000.0, speech.len() as f64 / 16_000.0, cosine(&owner, &trimmed));
        let mut rows: Vec<(bool, f64, f32, f32)> = Vec::new();
        let (mut echo_db, mut person_rms): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
        for pid in std::env::var("GATE_LIVE").expect("GATE_LIVE").split(',').filter(|p| !p.is_empty()) {
            let dir = workspace.join(pid);
            let p: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("project.json")).unwrap()).unwrap();
            let (x, sr) = crate::audio::wavio::read_mono_f32(&dir.join("source.wav")).unwrap();
            let x16 = crate::audio::wavio::to_16k(&x, sr);
            for l in p["lines"].as_array().unwrap() {
                let (a, z) = (l["start"].as_f64().unwrap(), l["end"].as_f64().unwrap());
                let piece = &x16[((a * 16_000.0) as usize).min(x16.len())..((z * 16_000.0) as usize).min(x16.len())];
                if piece.len() < 8_000 {
                    continue;
                }
                let print = e.embed(piece).unwrap();
                let text = l["text"].as_str().unwrap_or_default();
                let matched = studio_said(&crate::audio::qc::normalize(text));
                let heard = crate::speech::rms(&speech_of(piece));
                match matched {
                    Some(Some(sent)) if sent > 0.0 => echo_db.push(20.0 * (heard / sent).log10()),
                    None => person_rms.push(heard),
                    _ => {}
                }
                let (stored, again) = (cosine(&owner, &print), cosine(&trimmed, &print));
                eprintln!("{} {:>5.2}s stored {stored:.3} trimmed {again:.3} | {text}", if matched.is_some() { "STUDIO" } else { "OWNER " }, z - a);
                rows.push((matched.is_some(), z - a, stored, again));
            }
        }
        let owners: Vec<&(bool, f64, f32, f32)> = rows.iter().filter(|r| !r.0).collect();
        let studio: Vec<&(bool, f64, f32, f32)> = rows.iter().filter(|r| r.0).collect();
        eprintln!("lines: {} of the owner, {} of the studio", owners.len(), studio.len());
        for t in [0.30f32, 0.35, 0.40, 0.45, 0.50, 0.55, 0.61] {
            let rejected = |pick: fn(&(bool, f64, f32, f32)) -> f32| owners.iter().filter(|r| pick(r) < t).count() as f64 * 100.0 / owners.len().max(1) as f64;
            let passed = |pick: fn(&(bool, f64, f32, f32)) -> f32| studio.iter().filter(|r| pick(r) >= t).count() as f64 * 100.0 / studio.len().max(1) as f64;
            eprintln!(
                "threshold {t:.2}: stored print - owner rejected {:.1} %, studio passed {:.1} % | trimmed print - owner rejected {:.1} %, studio passed {:.1} %",
                rejected(|r| r.2), passed(|r| r.2), rejected(|r| r.3), passed(|r| r.3)
            );
        }
        let pct = |v: &mut Vec<f32>, q: f64| -> f32 {
            v.sort_by(f32::total_cmp);
            v.get(((v.len().max(1) - 1) as f64 * q).round() as usize).copied().unwrap_or(f32::NAN)
        };
        let sent: Vec<f32> = said.iter().filter_map(|s| s.1).collect();
        let mut sent_sorted = sent.clone();
        let median_sent = pct(&mut sent_sorted, 0.5);
        let mut person_db: Vec<f32> = person_rms.iter().map(|r| 20.0 * (r / median_sent).log10()).collect();
        eprintln!(
            "the studio heard back, dB under its own sound: n {} - median {:.1}, 90 % {:.1}, loudest {:.1} | the person, dB against the studio's median sound (rms {median_sent:.3}): n {} - quietest {:.1}, 10 % {:.1}, median {:.1}",
            echo_db.len(), pct(&mut echo_db, 0.5), pct(&mut echo_db, 0.9), pct(&mut echo_db, 1.0), person_db.len(), pct(&mut person_db, 0.0), pct(&mut person_db, 0.1), pct(&mut person_db, 0.5)
        );
    }
}
