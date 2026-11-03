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
use std::sync::Mutex;

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

/// The recogniser is loaded once and kept for the process (it is large; a reload per phrase would be wasteful).
fn with_recognizer<T>(models_root: &Path, gpu: bool, f: impl FnOnce(&mut PhoneRecognizer) -> Result<T, String>) -> Result<T, String> {
    static CACHE: Mutex<Option<PhoneRecognizer>> = Mutex::new(None);
    let mut slot = CACHE.lock().unwrap_or_else(|p| p.into_inner());
    if slot.is_none() {
        *slot = Some(PhoneRecognizer::load(&model_path(models_root), &vocab_path(models_root), gpu)?);
    }
    f(slot.as_mut().expect("just loaded"))
}

/// Greedy longest-match tokenization of an IPA string into a known phone inventory, so a G2P'd phrase and the
/// recogniser's own output land in one token space (eSpeak writes `t͡ʃ` as three code points, the model's vocab holds
/// `t͡ʃʰ` as one phone; a char split would never line them up). Whitespace and stress marks are skipped; a code point
/// that starts no known phone is dropped so the rest keeps aligning. `symbols` must be ordered longest phone first.
pub fn tokenize_ipa(symbols: &[Vec<char>], ipa: &str) -> Vec<String> {
    let chars: Vec<char> = ipa.chars().filter(|c| !c.is_whitespace() && *c != '\u{02c8}' && *c != '\u{02cc}').collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let hit = symbols.iter().find(|sym| !sym.is_empty() && i + sym.len() <= chars.len() && chars[i..i + sym.len()] == sym[..]);
        match hit {
            Some(sym) => {
                out.push(sym.iter().collect());
                i += sym.len();
            }
            None => i += 1,
        }
    }
    out
}

impl PhoneRecognizer {
    /// The phone inventory (ids `FIRST_PHONE_ID`..), each as its code points, ordered longest first for greedy matching.
    fn phone_symbols(&self) -> Vec<Vec<char>> {
        let mut syms: Vec<Vec<char>> = self.vocab.iter().skip(FIRST_PHONE_ID).filter(|s| !s.is_empty()).map(|s| s.chars().collect()).collect();
        syms.sort_by(|a, b| b.len().cmp(&a.len()));
        syms
    }
}

/// Whether the owner's set wake phrase is heard in these 16 kHz mono samples: the phone overlap of what was heard with
/// the phrase's phones is at or above `threshold`. False when no phrase is set or the recogniser is not installed.
pub fn says_wake(models_root: &Path, gpu: bool, lang: &str, phrase: &str, threshold: f32, samples: &[f32]) -> Result<bool, String> {
    let lang = if lang.trim().is_empty() { "en" } else { lang };
    let ipa = match espeak_ng::text_to_ipa(lang, phrase) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    with_recognizer(models_root, gpu, |r| {
        let target = tokenize_ipa(&r.phone_symbols(), &ipa);
        if target.is_empty() {
            return Ok(false);
        }
        let heard = r.phones(samples)?;
        Ok(phone_overlap(&heard, &target) >= threshold)
    })
}


/// The live phone overlap of these 16 kHz mono samples with the wake phrase, 0..=1 — the same score `says_wake`
/// thresholds, returned raw so the settings can show it live and the person can try different voices. None when no
/// phrase is set, the recogniser is not installed, or the phrase has no phones.
pub fn overlap_score(models_root: &Path, gpu: bool, lang: &str, phrase: &str, samples: &[f32]) -> Result<Option<f32>, String> {
    if phrase.trim().is_empty() {
        return Ok(None);
    }
    let lang = if lang.trim().is_empty() { "en" } else { lang };
    let ipa = match espeak_ng::text_to_ipa(lang, phrase) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    with_recognizer(models_root, gpu, |r| {
        let target = tokenize_ipa(&r.phone_symbols(), &ipa);
        if target.is_empty() {
            return Ok(None);
        }
        let heard = r.phones(samples)?;
        Ok(Some(phone_overlap(&heard, &target)))
    })
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

    #[test]
    fn tokenize_ipa_prefers_the_longest_known_phone() {
        // inventory holds a multi-code-point affricate and its prefix; longest must win
        let inv: Vec<Vec<char>> = ["t\u{361}\u{283}\u{2b0}", "t", "\u{283}", "a"].iter().map(|s| s.chars().collect()).collect();
        let mut inv = inv;
        inv.sort_by(|a, b| b.len().cmp(&a.len()));
        // "t͡ʃʰa" is the affricate then "a", not t + separate pieces
        assert_eq!(tokenize_ipa(&inv, "t\u{361}\u{283}\u{2b0}a"), vec!["t\u{361}\u{283}\u{2b0}".to_string(), "a".to_string()]);
        // an unknown leading mark is dropped, the rest still aligns
        assert_eq!(tokenize_ipa(&inv, "\u{2c8}ta"), vec!["t".to_string(), "a".to_string()]);
    }
}
