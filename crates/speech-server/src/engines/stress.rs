use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use ort::session::Session;
use ort::value::TensorRef;

pub use crate::text::stress::{is_russian, Stressed};
use crate::text::stress::{is_vowel, Inference, Rules};

pub const COMPONENT: &str = "ruaccent";
pub const DIR: &str = "ruaccent";
pub const OMOGRAPH_BIG: &str = "nn/nn_omograph/big_poetry";
// The accent network's 40-token training limit includes BOS and EOS.
const ACCENT_MAX_CHARS: usize = 38;
const ACCENT_MIN_SCORE: f32 = 0.55;

pub struct Stresser {
    rules: Rules,
    model: Model,
}

struct Model {
    chars: HashMap<char, i64>,
    accent: Mutex<Session>,
    omograph: Mutex<Session>,
    omo_tok: tokenizers::Tokenizer,
    omo_dir: PathBuf,
    omo_types: bool,
}

pub fn dir(models_root: &Path) -> PathBuf {
    models_root.join(DIR)
}

pub fn omograph_model(models_root: &Path) -> String {
    omograph_dir(&dir(models_root))
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn omograph_dir(d: &Path) -> PathBuf {
    let big = d.join(OMOGRAPH_BIG);
    if big.join("model.onnx").is_file() && big.join("tokenizer.json").is_file() {
        big
    } else {
        d.join("nn/nn_omograph/tiny2")
    }
}

pub fn installed(repo_root: &Path) -> bool {
    crate::setup::manifest()
        .iter()
        .find(|c| c.id == COMPONENT)
        .is_some_and(|c| crate::setup::component_status(repo_root, c).installed)
}

static LOADED: LazyLock<Mutex<Option<Arc<Stresser>>>> = LazyLock::new(|| Mutex::new(None));

pub fn shared(models_root: &Path) -> Result<Arc<Stresser>, String> {
    // The slot is replaced only after a complete load succeeds.
    let mut slot = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = slot.as_ref().filter(|s| s.model.omo_dir == omograph_dir(&dir(models_root))) {
        return Ok(s.clone());
    }
    let s = Arc::new(Stresser::load(&dir(models_root))?);
    *slot = Some(s.clone());
    Ok(s)
}

impl Stresser {
    pub fn load(d: &Path) -> Result<Self, String> {
        // ort panics when its dynamic runtime cannot be loaded.
        std::panic::catch_unwind(|| {
            let rules = Rules::load(d)?;
            let model = Model::load(d)?;
            Ok(Self { rules, model })
        })
        .map_err(|p| {
            let why = p
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown runtime panic".into());
            format!("{}: {why}", d.display())
        })?
    }

    pub fn knows(&self, word: &str) -> bool {
        self.rules.knows(word)
    }

    pub fn annotate(&self, text: &str) -> Result<Stressed, String> {
        self.rules.annotate(text, &self.model)
    }
}

fn session(path: &Path, threads: usize) -> Result<Session, String> {
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    crate::app::ort_session()?
        .with_intra_threads(threads)
        .map_err(|e| err(&e))?
        .commit_from_file(path)
        .map_err(|e| err(&e))
}

impl Model {
    fn load(d: &Path) -> Result<Self, String> {
        let vocab_path = d.join("nn/nn_accent/vocab.txt");
        let vocab = std::fs::read_to_string(&vocab_path).map_err(|e| format!("{}: {e}", vocab_path.display()))?;
        let chars = vocab
            .lines()
            .enumerate()
            .filter_map(|(i, l)| {
                let mut it = l.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Some((c, i as i64)),
                    _ => None,
                }
            })
            .collect();
        let omo_dir = omograph_dir(d);
        let omograph = session(&omo_dir.join("model.onnx"), if omo_dir.ends_with("tiny2") { 2 } else { 4 })?;
        let tokenizer_path = omo_dir.join("tokenizer.json");
        let mut omo_tok = tokenizers::Tokenizer::from_file(&tokenizer_path).map_err(|e| format!("{}: {e}", tokenizer_path.display()))?;
        omo_tok
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: 512,
                ..Default::default()
            }))
            .map_err(|e| format!("{}: {e}", tokenizer_path.display()))?;
        omo_tok.with_padding(None);
        Ok(Self {
            chars,
            accent: Mutex::new(session(&d.join("nn/nn_accent/model.onnx"), 2)?),
            omo_types: omograph.inputs().iter().any(|i| i.name() == "token_type_ids"),
            omograph: Mutex::new(omograph),
            omo_tok,
            omo_dir,
        })
    }
}

impl Inference for Model {
    fn accent_position(&self, word: &str) -> Result<Option<usize>, String> {
        let chars: Vec<char> = word.chars().collect();
        let Some(ids) = accent_ids(&chars, &self.chars) else {
            return Ok(None);
        };
        let n = ids.len();
        let mask = vec![1i64; n];
        let types = vec![0i64; n];
        let shape = vec![1i64, n as i64];
        let mut s = self.accent.lock().map_err(|_| "accent lock".to_string())?;
        let out = s
            .run(ort::inputs![
                "input_ids" => TensorRef::from_array_view((shape.clone(), ids.as_slice())).map_err(|e| e.to_string())?,
                "attention_mask" => TensorRef::from_array_view((shape.clone(), mask.as_slice())).map_err(|e| e.to_string())?,
                "token_type_ids" => TensorRef::from_array_view((shape, types.as_slice())).map_err(|e| e.to_string())?,
            ])
            .map_err(|e| format!("accent: {e}"))?;
        let logits = out.get("logits").ok_or("accent: missing logits output")?;
        let (shape, logits) = logits.try_extract_tensor::<f32>().map_err(|e| format!("accent: {e}"))?;
        accent_position(&chars, shape, logits)
    }

    fn omograph_probability(&self, context: &str, hypothesis: &str) -> Result<f32, String> {
        let enc = self.omo_tok.encode((context, hypothesis), true).map_err(|e| e.to_string())?;
        let ids: Vec<i64> = enc.get_ids().iter().map(|v| *v as i64).collect();
        let mask: Vec<i64> = enc.get_attention_mask().iter().map(|v| *v as i64).collect();
        let types: Vec<i64> = enc.get_type_ids().iter().map(|v| *v as i64).collect();
        let shape = vec![1i64, ids.len() as i64];
        let mut s = self.omograph.lock().map_err(|_| "omograph lock".to_string())?;
        let mut inputs = ort::inputs![
            "input_ids" => TensorRef::from_array_view((shape.clone(), ids.as_slice())).map_err(|e| e.to_string())?,
            "attention_mask" => TensorRef::from_array_view((shape.clone(), mask.as_slice())).map_err(|e| e.to_string())?,
        ];
        if self.omo_types {
            inputs.push((
                "token_type_ids".into(),
                TensorRef::from_array_view((shape, types.as_slice()))
                    .map_err(|e| e.to_string())?
                    .into(),
            ));
        }
        let out = s.run(inputs).map_err(|e| format!("omograph: {e}"))?;
        let logits = out.get("logits").ok_or("omograph: missing logits output")?;
        let (shape, logits) = logits.try_extract_tensor::<f32>().map_err(|e| format!("omograph: {e}"))?;
        omograph_probability(shape, logits)
    }
}

fn accent_ids(chars: &[char], vocab: &HashMap<char, i64>) -> Option<Vec<i64>> {
    if chars.len() > ACCENT_MAX_CHARS {
        return None;
    }
    let mut ids = vec![2];
    ids.extend(chars.iter().map(|c| *vocab.get(c).unwrap_or(&1)));
    ids.push(3);
    Some(ids)
}

fn accent_position(chars: &[char], shape: &[i64], logits: &[f32]) -> Result<Option<usize>, String> {
    let n = chars.len() + 2;
    let labels = shape.get(2).and_then(|v| usize::try_from(*v).ok()).filter(|v| *v >= 2);
    let labels = labels
        .filter(|labels| shape.len() == 3 && shape[0] == 1 && shape[1] == n as i64 && n.checked_mul(*labels) == Some(logits.len()))
        .ok_or_else(|| format!("accent: invalid logits shape {shape:?} ({} values, {n} tokens)", logits.len()))?;
    if logits.iter().any(|v| !v.is_finite()) {
        return Err("accent: non-finite logits".into());
    }
    let mut best: Option<(usize, f32)> = None;
    for (i, c) in chars.iter().enumerate() {
        let row = &logits[(i + 1) * labels..(i + 2) * labels];
        let p = softmax(row, "accent")?;
        let (arg, score) = p
            .iter()
            .enumerate()
            .fold((0, f32::MIN), |m, (k, v)| if *v > m.1 { (k, *v) } else { m });
        if arg == 1 && score >= ACCENT_MIN_SCORE && is_vowel(*c) && best.is_none_or(|b| score > b.1) {
            best = Some((i, score));
        }
    }
    Ok(best.map(|b| b.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homograph_selection_requires_both_files_before_using_big_poetry() {
        let root = tempfile::tempdir().unwrap();
        let d = dir(root.path());
        let big = d.join(OMOGRAPH_BIG);
        let tiny = d.join("nn/nn_omograph/tiny2");
        assert_eq!(omograph_dir(&d), tiny);
        assert_eq!(omograph_model(root.path()), "tiny2");
        std::fs::create_dir_all(&big).unwrap();
        std::fs::write(big.join("model.onnx"), b"model").unwrap();
        assert_eq!(omograph_dir(&d), tiny);
        std::fs::write(big.join("tokenizer.json"), b"tokenizer").unwrap();
        assert_eq!(omograph_dir(&d), big);
        assert_eq!(omograph_model(root.path()), "big_poetry");
        std::fs::remove_file(big.join("model.onnx")).unwrap();
        assert_eq!(omograph_dir(&d), tiny);
    }

    #[test]
    fn accent_input_keeps_unicode_character_ids_and_the_training_length_limit() {
        let vocab = HashMap::from([('а', 12), ('я', 44)]);
        assert_eq!(accent_ids(&['а', 'ё', 'я'], &vocab).unwrap(), [2, 12, 1, 44, 3]);
        assert_eq!(accent_ids(&['а'; 38], &vocab).unwrap().len(), 40);
        assert!(accent_ids(&['а'; 39], &vocab).is_none());
    }

    fn accent_rows(rows: &[[f32; 3]]) -> (Vec<char>, Vec<i64>, Vec<f32>) {
        let chars: Vec<_> = "абуе".chars().take(rows.len()).collect();
        let mut logits = vec![0.0; 3];
        logits.extend(rows.iter().flatten());
        logits.extend([0.0; 3]);
        let shape = vec![1, (chars.len() + 2) as i64, 3];
        (chars, shape, logits)
    }

    #[test]
    fn accent_decoding_selects_a_confident_primary_stress_on_a_vowel() {
        let (chars, shape, logits) = accent_rows(&[[0.0, 2.0, 0.0], [0.0, 9.0, 0.0], [0.0, 3.0, 0.0], [0.0, 1.0, 5.0]]);
        assert_eq!(accent_position(&chars, &shape, &logits).unwrap(), Some(2));
        let (chars, shape, logits) = accent_rows(&[[0.0, 0.1, 0.0]]);
        assert_eq!(accent_position(&chars, &shape, &logits).unwrap(), None);
        let (chars, shape, logits) = accent_rows(&[[0.0, 2.0, 0.0], [0.0; 3], [0.0, 2.0, 0.0]]);
        assert_eq!(accent_position(&chars, &shape, &logits).unwrap(), Some(0));
    }

    #[test]
    fn invalid_accent_output_dimensions_and_lengths_are_errors() {
        for (shape, logits) in [
            (vec![], vec![]),
            (vec![1, 3], vec![0.0; 3]),
            (vec![1, 3, 0], vec![]),
            (vec![1, 3, 1], vec![0.0; 3]),
            (vec![2, 3, 3], vec![0.0; 18]),
            (vec![1, 4, 3], vec![0.0; 12]),
            (vec![1, 3, 3], vec![0.0; 8]),
            (vec![1, 3, 3], vec![0.0; 10]),
            (vec![1, 3, -1], vec![]),
            (vec![1, 3, i64::MAX], vec![]),
            (vec![1, 3, 3, 1], vec![0.0; 9]),
        ] {
            let error = accent_position(&['а'], &shape, &logits).unwrap_err();
            assert!(error.contains("accent") && error.contains("shape"), "{error}");
        }
    }

    #[test]
    fn homograph_output_is_a_two_class_distribution() {
        let expected = 2.0f32.exp() / (1.0 + 2.0f32.exp());
        assert!((omograph_probability(&[1, 2], &[0.0, 2.0]).unwrap() - expected).abs() < 1e-6);
        for (shape, logits) in [
            (vec![], vec![]),
            (vec![1, 1], vec![1.0]),
            (vec![1, 3], vec![1.0; 3]),
            (vec![2, 1], vec![1.0; 2]),
            (vec![1, 2], vec![1.0]),
            (vec![1, 2], vec![1.0; 3]),
        ] {
            assert!(omograph_probability(&shape, &logits).unwrap_err().contains("omograph"));
        }
    }

    #[test]
    fn non_finite_model_outputs_are_errors_instead_of_missing_stress() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut logits = vec![0.0; 9];
            logits[4] = value;
            assert!(accent_position(&['а'], &[1, 3, 3], &logits).unwrap_err().contains("non-finite"));
            assert!(omograph_probability(&[1, 2], &[0.0, value]).unwrap_err().contains("non-finite"));
        }
    }
}

fn omograph_probability(shape: &[i64], logits: &[f32]) -> Result<f32, String> {
    if shape != [1, 2] || logits.len() != 2 {
        return Err(format!(
            "omograph: invalid logits shape {shape:?} ({} values, expected [1, 2])",
            logits.len()
        ));
    }
    Ok(softmax(logits, "omograph")?[1])
}

fn softmax(x: &[f32], stage: &str) -> Result<Vec<f32>, String> {
    if x.is_empty() || x.iter().any(|v| !v.is_finite()) {
        return Err(format!("{stage}: empty or non-finite logits"));
    }
    let m = x.iter().cloned().fold(f32::MIN, f32::max);
    let e: Vec<f32> = x.iter().map(|v| (v - m).exp()).collect();
    let s: f32 = e.iter().sum();
    Ok(e.into_iter().map(|v| v / s).collect())
}

#[cfg(test)]
mod live {
    use super::*;

    #[test]
    #[ignore = "needs a complete ruaccent directory: STRESS_DIR and ORT_DYLIB_PATH"]
    fn a_shared_model_is_reused_and_a_failed_reload_keeps_the_previous_model() {
        let d = PathBuf::from(std::env::var("STRESS_DIR").expect("STRESS_DIR"));
        assert_eq!(d.file_name().unwrap(), DIR);
        let models_root = d.parent().unwrap();
        let first = shared(models_root).unwrap();
        assert!(Arc::ptr_eq(&first, &shared(models_root).unwrap()));
        let missing = tempfile::tempdir().unwrap();
        let error = shared(missing.path()).err().expect("missing dictionaries");
        assert!(error.contains(&missing.path().display().to_string()), "{error}");
        assert!(Arc::ptr_eq(&first, &shared(models_root).unwrap()));
        assert_eq!(first.annotate("Молоко").unwrap().text, "Молок+о");
    }

    /// Прогон на настоящих моделях: `STRESS_DIR` — папка RUAccent, `ORT_DYLIB_PATH` — onnxruntime 1.28.
    #[test]
    #[ignore]
    fn real_models_read_context_and_dictionary() {
        let dir = std::env::var("STRESS_DIR").expect("STRESS_DIR");
        let t0 = std::time::Instant::now();
        let s = Stresser::load(Path::new(&dir)).expect("load");
        eprintln!("load {:.2}s", t0.elapsed().as_secs_f64());
        let mut reports = Vec::new();
        for text in [
            "Старинный замок стоял на холме.",
            "Закрой дверь на замок, пожалуйста.",
            "Из этой муки пекут хлеб. Адские муки грешников.",
            "Елка стояла в углу, все дети пели.",
            "Он пришел домой и включил телевизор.",
            "Это з+амок, а не <|emotion:awe|> замок. [pause 300ms] Белки бегали по деревьям.",
            "Кто-то сказал: «Вышлите документы в среду».",
            "Необычные квакозябрики и нейрофлюксаторы придумали глокую куздру.",
            "Ёлка, ЕЛКА, вс+е и вс+ё. [[молоко]] <[коса]> [rate:1.5] молоко.",
        ] {
            let t = std::time::Instant::now();
            let r = s.annotate(text).expect("annotate");
            eprintln!("{:.0}ms | {}", t.elapsed().as_secs_f64() * 1000.0, r.text);
            for w in r
                .words
                .iter()
                .filter(|w| w.doubt || w.yo_choice.is_some() || !w.variants.is_empty())
            {
                eprintln!(
                    "    {} {} {:?} doubt={} yo={:?}",
                    w.word, w.source, w.variants, w.doubt, w.yo_choice
                );
            }
            reports.push(r);
        }
        if let Some(path) = std::env::var_os("STRESS_SNAPSHOT") {
            std::fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
        }
    }
}
