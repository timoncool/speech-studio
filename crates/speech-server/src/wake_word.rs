//! Wake word for the Live voice gate: activate on a phrase the owner set, in any language.
//!
//! This is the established two-stage open-vocabulary KWS approach (DMA-KWS / DS-KWS: a CTC phoneme search finds what
//! was said, then a phoneme match verifies the keyword; the classic baseline is a CTC recogniser plus edit-distance
//! matching of the keyword's phones). A universal phone recogniser (PhoneticXeus, SOTA-2026 "An Empirical Recipe for
//! Universal Phone Recognition", ready int8 ONNX) turns 16 kHz speech into an IPA phone string; the set phrase is
//! turned into IPA by eSpeak-ng G2P; a phone match spots it. No per-word training and no fixed vocabulary — any phrase
//! in any language. Off unless a wake word is set, and paired with the speaker gate so it is the owner's own voice.

use ort::session::Session;
use ort::value::TensorRef;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The recogniser works on 16 kHz mono audio.
pub const SR: u32 = 16_000;
/// Ids 0..=3 of the vocab are `<blank>`, `<sos>`, `<eos>`, `<unk>`; real phones start at 4.
const FIRST_PHONE_ID: usize = 4;

/// PhoneticXeus universal phone recogniser: raw 16 kHz mono f32 waveform in, frame-level IPA CTC logits out. The
/// onnxruntime dylib is set for the process at start (`app::runtime`), so loading needs no further setup.
pub struct PhoneRecognizer {
    session: Session,
    /// id -> IPA symbol, from the model's vocab.
    vocab: Vec<String>,
}

impl PhoneRecognizer {
    /// Load the model and its IPA vocabulary (`{symbol: id}`); `gpu` runs it on CUDA.
    pub fn load(onnx: &Path, vocab_json: &Path, gpu: bool) -> Result<Self, String> {
        let text = std::fs::read_to_string(vocab_json).map_err(|e| format!("{}: {e}", vocab_json.display()))?;
        let map: HashMap<String, usize> = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", vocab_json.display()))?;
        let mut vocab = vec![String::new(); map.values().copied().max().map_or(0, |m| m + 1)];
        for (sym, id) in map {
            if id < vocab.len() {
                vocab[id] = sym;
            }
        }
        let mut builder = Session::builder().map_err(|e| format!("ort builder: {e}"))?;
        if gpu {
            builder = builder
                .with_execution_providers([ort::ep::CUDA::default().build().error_on_failure()])
                .map_err(|e| format!("CUDA for {}: {e}", onnx.display()))?;
        }
        let session = builder.commit_from_file(onnx).map_err(|e| format!("commit_from_file {}: {e}", onnx.display()))?;
        Ok(Self { session, vocab })
    }

    /// The IPA phones heard in 16 kHz mono samples (greedy CTC: the top id per frame, repeats merged, blank and the
    /// special ids dropped).
    pub fn phones(&mut self, samples: &[f32]) -> Result<Vec<String>, String> {
        if samples.is_empty() {
            return Ok(Vec::new());
        }
        let dims: Vec<i64> = vec![1, samples.len() as i64];
        let tensor = TensorRef::from_array_view((dims, samples)).map_err(|e| format!("wake tensor: {e}"))?;
        let outputs = self.session.run(ort::inputs![tensor]).map_err(|e| format!("wake run: {e}"))?;
        let (_name, value) = outputs.iter().next().ok_or("wake: no output")?;
        let (shape, data) = value.try_extract_tensor::<f32>().map_err(|e| format!("wake extract: {e}"))?;
        let classes = shape.last().copied().unwrap_or(0) as usize;
        if classes == 0 {
            return Ok(Vec::new());
        }
        let mut phones = Vec::new();
        let mut prev = usize::MAX;
        for frame in data.chunks_exact(classes) {
            let best = frame
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map_or(0, |(i, _)| i);
            if best != prev {
                if best >= FIRST_PHONE_ID {
                    if let Some(sym) = self.vocab.get(best).filter(|s| !s.is_empty()) {
                        phones.push(sym.clone());
                    }
                }
                prev = best;
            }
        }
        Ok(phones)
    }
}

/// Where the recogniser and its vocab live under the models root (the download component unpacks them here).
pub fn model_path(models_root: &Path) -> PathBuf {
    models_root.join("wake-word").join("model.int8.onnx")
}
pub fn vocab_path(models_root: &Path) -> PathBuf {
    models_root.join("wake-word").join("ipa_vocab.json")
}

/// A length-normalized phone match of the heard phones against the target phrase's phones, 0..=1: the longest run of
/// the target found in sequence in what was heard, over the target's length. A simple, order-aware score to tune a
/// threshold against; the recogniser is not perfect, so an exact match is not required.
pub fn phone_overlap(heard: &[String], target: &[String]) -> f32 {
    if target.is_empty() {
        return 0.0;
    }
    // longest common subsequence of phones, order preserved
    let (n, m) = (heard.len(), target.len());
    let mut dp = vec![0u16; m + 1];
    for h in heard.iter().take(n) {
        let mut diag = 0u16;
        for j in 0..m {
            let up = dp[j + 1];
            if h == &target[j] {
                dp[j + 1] = diag + 1;
            } else if dp[j] > dp[j + 1] {
                dp[j + 1] = dp[j];
            }
            diag = up;
        }
    }
    f32::from(dp[m]) / m as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phones(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_string()).collect()
    }

    #[test]
    fn phone_overlap_scores_the_target_run() {
        let target = phones(&["p", "r", "i", "v", "e", "t"]);
        assert_eq!(phone_overlap(&target, &target), 1.0); // exact
        assert_eq!(phone_overlap(&phones(&["x", "x"]), &target), 0.0); // nothing in common
        let heard = phones(&["a", "p", "r", "i", "v", "e", "t", "a"]); // the phrase inside other speech
        assert_eq!(phone_overlap(&heard, &target), 1.0);
        let partial = phones(&["p", "r", "i"]); // half the phones heard
        assert_eq!(phone_overlap(&partial, &target), 0.5);
    }
}
