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
use std::sync::{Mutex, OnceLock};

/// The recogniser works on 16 kHz mono audio.
pub const SR: u32 = 16_000;

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
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", vocab_json.display()))?;
        // the vocab is either a flat {symbol: id} map or a {"blank": 0, "vocab": {symbol: id}} object
        let obj = json.get("vocab").unwrap_or(&json).as_object().ok_or_else(|| format!("{}: no vocab object", vocab_json.display()))?;
        let map: HashMap<String, usize> = obj.iter().filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n as usize))).collect();
        let mut vocab = vec![String::new(); map.values().copied().max().map_or(0, |m| m + 1)];
        for (sym, id) in map {
            if id < vocab.len() {
                vocab[id] = sym;
            }
        }
        let mut builder = crate::app::ort_session()?;
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
                // drop the CTC blank and the special `<…>` tokens; keep real phones
                if let Some(sym) = self.vocab.get(best).filter(|s| !s.is_empty() && !s.starts_with('<')) {
                    phones.push(sym.clone());
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
    models_root.join("wake-word").join("vocab.json")
}

/// A length-normalized phone match of the heard phones against the target phrase's phones, 0..=1: the longest run of
/// the target found in sequence in what was heard, over the target's length. A simple, order-aware score to tune a
/// threshold against; the recogniser is not perfect, so an exact match is not required.
pub fn phone_overlap<T: PartialEq>(heard: &[T], target: &[T]) -> f32 {
    if target.is_empty() {
        return 0.0;
    }
    // longest common subsequence, order preserved
    let m = target.len();
    let mut dp = vec![0u16; m + 1];
    for h in heard {
        let mut diag = 0u16;
        for j in 0..m {
            let up = dp[j + 1];
            if *h == target[j] {
                dp[j + 1] = diag + 1;
            } else if dp[j] > dp[j + 1] {
                dp[j + 1] = dp[j];
            }
            diag = up;
        }
    }
    f32::from(dp[m]) / m as f32
}

/// Coarsen an IPA string to a plain phone-class char sequence for matching: drop combining diacritics, stress, length
/// and modifier letters, and map near-equivalent symbols to one letter. A G2P phrase and an acoustic recogniser spell
/// the same sounds with different IPA (eSpeak `kɭʌdʲ`, the model `klod`); coarse matching on classes bridges that —
/// the standard phone-class / feature matching for open-vocabulary keyword spotting, not exact symbol equality.
pub fn coarse(ipa: &str) -> Vec<char> {
    ipa.chars()
        .filter_map(|c| {
            if ('\u{0300}'..='\u{036f}').contains(&c) {
                return None; // combining diacritics, incl. the tie bar
            }
            let mapped = match c {
                'ɭ' | 'ɫ' | 'ʎ' => 'l',
                'ʌ' | 'ɒ' | 'ɔ' => 'o',
                'ɐ' | 'ə' | 'æ' => 'a',
                'ɪ' | 'ɨ' | 'ᵻ' => 'i',
                'ɯ' | 'ʊ' | 'ʉ' => 'u',
                'ɡ' | 'ɢ' => 'g',
                'ħ' | 'x' | 'χ' => 'h',
                'ɕ' | 'ʂ' | 'ʃ' | 'ɸ' | 'θ' => 's',
                'ʑ' | 'ʐ' | 'ʒ' | 'ð' => 'z',
                'ɴ' | 'ŋ' | 'ɲ' => 'n',
                'ɱ' => 'm',
                'ʁ' | 'ɾ' | 'ɹ' | 'ʀ' | 'ɻ' => 'r',
                'β' | 'ɓ' => 'b',
                'ɗ' | 'ɖ' => 'd',
                'ʈ' => 't',
                'ɟ' => 'j',
                'c' | 'q' => 'k',
                'ʔ' | 'ʲ' | 'ʰ' | 'ʷ' | 'ʼ' | 'ˠ' | 'ˤ' | 'ⁿ' | 'ˈ' | 'ˌ' | 'ː' | 'ˑ' | '▁' | ' ' | '\u{2009}' => return None,
                other => other,
            };
            Some(mapped.to_ascii_lowercase())
        })
        .collect()
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

impl PhoneRecognizer {
}

/// The bundled eSpeak-ng dictionaries, extracted once to a cache directory the G2P reads from. Without this the
/// convenience `espeak_ng::text_to_ipa` looks for data files that are not on disk and every phrase fails with
/// "dict not found" — so no wake phrase is ever recognised.
fn espeak_data() -> Result<&'static Path, String> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join("speech-studio-espeak-data");
        std::fs::create_dir_all(&dir).map_err(|e| format!("espeak data dir: {e}"))?;
        espeak_ng::install_bundled_languages(&dir, espeak_ng::bundled_languages()).map_err(|e| format!("espeak install: {e}"))?;
        Ok(dir)
    })
    .as_ref()
    .map(PathBuf::as_path)
    .map_err(|e| e.clone())
}

/// Grapheme-to-IPA for a phrase in the given eSpeak-ng voice, from the bundled dictionaries. The last result is cached:
/// a phrase does not change during a live check, so the recogniser-side work repeats but this does not.
fn to_ipa(lang: &str, phrase: &str) -> Result<String, String> {
    static CACHE: Mutex<Option<(String, String, String)>> = Mutex::new(None);
    if let Some((l, p, ipa)) = CACHE.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        if l == lang && p == phrase {
            return Ok(ipa.clone());
        }
    }
    let dir = espeak_data()?;
    let engine = espeak_ng::EspeakNg::with_data_dir(lang, dir).map_err(|e| format!("espeak {lang}: {e}"))?;
    let ipa = engine.text_to_phonemes(phrase).unwrap_or_default();
    *CACHE.lock().unwrap_or_else(|p| p.into_inner()) = Some((lang.to_string(), phrase.to_string(), ipa.clone()));
    Ok(ipa)
}

/// Whether the owner's set wake phrase is heard in these 16 kHz mono samples: the phone overlap of what was heard with
/// the phrase's phones is at or above `threshold`. False when no phrase is set or the recogniser is not installed.
pub fn says_wake(models_root: &Path, gpu: bool, lang: &str, phrase: &str, threshold: f32, samples: &[f32]) -> Result<bool, String> {
    let lang = if lang.trim().is_empty() { "en" } else { lang };
    let ipa = match to_ipa(lang, phrase) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };
    let target = coarse(&ipa);
    if target.is_empty() {
        return Ok(false);
    }
    with_recognizer(models_root, gpu, |r| {
        let heard = coarse(&r.phones(samples)?.concat());
        Ok(phone_overlap(&heard, &target) >= threshold)
    })
}


/// The live phone overlap of these 16 kHz mono samples with the wake phrase, 0..=1 — the same score `says_wake`
/// thresholds, returned raw so the settings can show it live and the person can try different voices. None when no
/// phrase is set, the recogniser is not installed, or the phrase has no phones.
pub fn overlap_score(models_root: &Path, gpu: bool, lang: &str, phrase: &str, samples: &[f32]) -> Result<Option<f32>, String> {
    if phrase.trim().is_empty() || !model_path(models_root).exists() {
        return Ok(None);
    }
    let lang = if lang.trim().is_empty() { "en" } else { lang };
    let ipa = match to_ipa(lang, phrase) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let target = coarse(&ipa);
    if target.is_empty() {
        return Ok(None);
    }
    with_recognizer(models_root, gpu, |r| {
        let heard = coarse(&r.phones(samples)?.concat());
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
    fn coarse_bridges_g2p_and_acoustic_spellings() {
        // eSpeak writes клодик as kɭʌdʲˈik, the acoustic model as lodik/lodzik: both coarsen close enough to match
        let target = coarse("k\u{26d}\u{28c}d\u{2b2}\u{2c8}ik");
        assert_eq!(target.iter().collect::<String>(), "klodik");
        assert!(phone_overlap(&coarse("lodzik"), &target) >= 0.8);
        assert!(phone_overlap(&coarse("p\u{28c}\u{261}\u{2c8}oda"), &target) < 0.6); // погода ≠ клодик
    }

    #[test]
    fn bundled_g2p_produces_ipa() {
        // the wake feature is dead without the bundled dictionaries: guard that they load and a phrase renders to IPA
        assert!(to_ipa("ru", "Клодик").map(|s| !s.is_empty()).unwrap_or(false));
        assert!(to_ipa("en", "hey claude").map(|s| !s.is_empty()).unwrap_or(false));
    }
}
