//! Voice gate for Live: the studio can listen passively and start a conversation only on the owner's voice (and,
//! when set, a chosen wake word). The owner enrols once by speaking a phrase; a speaker embedding (ReDimNet2-B3, SOTA
//! speaker verification, 192-d, run on ONNX Runtime) is kept, and later speech is let through only when its embedding
//! matches the enrolled one by cosine similarity. Off by default.

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
        let mut builder = Session::builder().map_err(|e| format!("ort builder: {e}"))?;
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

/// How the owner set the gate up. Off by default: the feature is opt-in.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// The gate is on: a conversation in Live starts only on the owner's voice.
    pub enabled: bool,
    /// Cosine at or above which the speech is the owner (conservative by default — fewer false activations).
    pub threshold: f32,
    /// A wake word that must also be heard, or None for any speech from the owner.
    pub wake_word: Option<String>,
    /// Start Speech Studio on login (per-OS). Off by default.
    pub autostart: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { enabled: false, threshold: DEFAULT_THRESHOLD, wake_word: None, autostart: false }
    }
}

/// The wake words on offer (filled when the wake-word model lands); empty for now, so the gate keys on the voice alone.
pub fn wake_words() -> &'static [&'static str] {
    &[]
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

/// The owner's enrolled voice print (192-d, L2-normalized), or None if no one has enrolled.
pub fn owner_print(models_root: &Path) -> Option<Vec<f32>> {
    std::fs::read(gate_dir(models_root).join("owner.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<f32>>(&b).ok())
        .filter(|v| v.len() == EMB_DIM)
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

/// Enrol the owner from 16 kHz mono samples in [-1, 1]: keep their voice print. Returns the seconds used.
pub fn enroll(models_root: &Path, gpu: bool, samples: &[f32]) -> Result<f64, String> {
    let print = with_embedder(models_root, gpu, |e| e.embed(samples))?;
    let dir = gate_dir(models_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join("owner.json");
    std::fs::write(&path, serde_json::to_vec(&print).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(samples.len() as f64 / SR as f64)
}

/// The cosine of this speech against the enrolled owner; None if no one is enrolled.
pub fn score(models_root: &Path, gpu: bool, samples: &[f32]) -> Result<Option<f32>, String> {
    let Some(owner) = owner_print(models_root) else {
        return Ok(None);
    };
    let print = with_embedder(models_root, gpu, |e| e.embed(samples))?;
    Ok(Some(cosine(&owner, &print)))
}

/// Whether this speech is the owner, at or above the configured threshold.
pub fn is_owner(models_root: &Path, gpu: bool, cfg: &Config, samples: &[f32]) -> Result<bool, String> {
    Ok(score(models_root, gpu, samples)?.is_some_and(|s| s >= cfg.threshold))
}

/// The gate is armed only for a Live conversation (set when one starts), so the live recogniser checks the owner's
/// voice there and not when a recording is simply being transcribed.
static ARMED: AtomicBool = AtomicBool::new(false);

/// Arm or disarm the gate for the Live conversation.
pub fn arm(on: bool) {
    ARMED.store(on, Ordering::Relaxed);
}

/// Whether a gated Live conversation is under way.
pub fn is_armed() -> bool {
    ARMED.load(Ordering::Relaxed)
}

/// For the live recogniser: whether a phrase is the owner. None means the gate does not apply here — it is not armed,
/// is off, no one has enrolled, or the embedding failed — and the phrase is then let through (fail-open, so a gate
/// slip never silences a real conversation). CPU, so a short phrase stays off the agent's GPU.
pub fn owner_check(models_root: &Path, samples: &[f32]) -> Option<bool> {
    if !is_armed() {
        return None;
    }
    let cfg = config(models_root);
    if !cfg.enabled || !enrolled(models_root) {
        return None;
    }
    match is_owner(models_root, false, &cfg, samples) {
        Ok(owner) => Some(owner),
        Err(e) => {
            tracing::warn!("voice gate check: {e}");
            None
        }
    }
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

    fn tone(hz: f32, seconds: usize) -> Vec<f32> {
        let n = SR as usize * seconds;
        (0..n).map(|i| 0.3 * (2.0 * std::f32::consts::PI * hz * i as f32 / SR as f32).sin()).collect()
    }

    /// Runs the real model on this machine: set the model path and the onnxruntime dylib, e.g.
    /// SPEECH_VOICE_GATE_MODEL=D:\...\redimnet2_b3_lm.onnx ORT_DYLIB_PATH=...\onnxruntime-1.28.dll
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
}
