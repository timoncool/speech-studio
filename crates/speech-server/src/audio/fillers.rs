//! Filler words: what a speaker says to hold a pause rather than to mean something. Recognition often drops the
//! hesitation sounds (`э`, `um`), so a count of them is a lower bound.

use std::path::Path;

/// Russian fillers, each written as recognition gives it.
pub const RU: &[&str] = &[
    "э", "ээ", "эээ", "эм", "эмм", "мм", "ммм", "ну", "вот", "типа", "короче", "как бы", "это самое", "так сказать",
    "в общем", "собственно", "блин", "как его", "в принципе", "по сути", "на самом деле", "что называется",
];

/// English fillers.
pub const EN: &[&str] = &[
    "uh", "um", "uhm", "umm", "er", "erm", "hmm", "mm", "you know", "i mean", "sort of", "kind of", "basically", "actually",
    "literally", "you see",
];

/// The file in the studio's working folder whose `words` the person adds to the fillers counted.
pub const FILE: &str = "fillers.json";

/// The fillers counted, each as its normalised words, the longest first so `как бы` is taken before a single word.
pub fn list(workspace: &Path) -> Result<Vec<Vec<String>>, String> {
    let mut all: Vec<String> = RU.iter().chain(EN).map(|w| w.to_string()).collect();
    let own = workspace.join(FILE);
    if own.exists() {
        let body = std::fs::read(&own).map_err(|e| format!("{}: {e}", own.display()))?;
        let parsed: serde_json::Value = serde_json::from_slice(&body).map_err(|e| format!("{}: {e}", own.display()))?;
        let words = parsed["words"].as_array().ok_or_else(|| format!("{}: no \"words\" list", own.display()))?;
        for w in words {
            all.push(w.as_str().ok_or_else(|| format!("{}: {w} is not a word", own.display()))?.to_string());
        }
    }
    let mut out: Vec<Vec<String>> = all.iter().map(|w| crate::audio::qc::normalize(w)).filter(|w| !w.is_empty()).collect();
    out.sort_by_key(|w| std::cmp::Reverse(w.len()));
    out.dedup();
    Ok(out)
}

/// The fillers in `tokens` (normalised words with where each starts, seconds): each filler with how often it came and
/// where, a word counted in one filler only.
pub fn find(tokens: &[(String, f64)], fillers: &[Vec<String>]) -> Vec<(String, Vec<f64>)> {
    let mut used = vec![false; tokens.len()];
    let mut found: Vec<(String, Vec<f64>)> = Vec::new();
    for filler in fillers {
        let n = filler.len();
        let mut i = 0;
        while i + n <= tokens.len() {
            if (i..i + n).all(|k| !used[k] && tokens[k].0 == filler[k - i]) {
                used[i..i + n].iter_mut().for_each(|u| *u = true);
                let name = filler.join(" ");
                match found.iter_mut().find(|(w, _)| *w == name) {
                    Some((_, at)) => at.push(tokens[i].1),
                    None => found.push((name, vec![tokens[i].1])),
                }
                i += n;
            } else {
                i += 1;
            }
        }
    }
    found.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str) -> Vec<(String, f64)> {
        crate::audio::qc::normalize(text).into_iter().enumerate().map(|(i, w)| (w, i as f64)).collect()
    }

    #[test]
    fn a_two_word_filler_is_taken_whole_and_its_words_once() {
        let fillers = list(Path::new("no such folder")).unwrap();
        let found = find(&tokens("Ну, это как бы, ну, важно. Вот, как бы так"), &fillers);
        assert_eq!(found, vec![("как бы".to_string(), vec![2.0, 7.0]), ("ну".to_string(), vec![0.0, 4.0]), ("вот".to_string(), vec![6.0])]);
        assert!(find(&tokens("Это важная мысль"), &fillers).is_empty());
        assert_eq!(find(&tokens("Well, you know, um, it works"), &fillers).len(), 2);
    }

    #[test]
    fn the_persons_own_fillers_are_added_and_a_broken_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE), r#"{"words": ["значит"]}"#).unwrap();
        let fillers = list(dir.path()).unwrap();
        assert_eq!(find(&tokens("Значит, так"), &fillers), vec![("значит".to_string(), vec![0.0])]);
        std::fs::write(dir.path().join(FILE), "{").unwrap();
        assert!(list(dir.path()).is_err());
    }
}
