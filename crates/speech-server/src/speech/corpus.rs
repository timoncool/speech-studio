//! Hard Russian phrases kept as a regression: homographs read by their context, «ё» the text leaves out, the stress
//! people get wrong, numbers, dates, centuries and abbreviations. Each line pins how the studio's normalisation and
//! stress marking read the phrase; a project made of them, voiced with the check by recognition on, shows in the
//! quality report how an engine says them.

/// kind, phrase, how it is read (normalised and stress-marked).
const CORPUS: &str = include_str!("../../data/ru_hard_corpus.tsv");

#[cfg(test)]
pub const KINDS: [&str; 6] = ["homograph", "yo", "stress", "number", "abbr", "mixed"];

pub struct Phrase {
    #[cfg_attr(not(test), allow(dead_code))]
    pub kind: &'static str,
    pub text: &'static str,
    #[cfg_attr(not(test), allow(dead_code))]
    pub read: &'static str,
}

pub fn phrases() -> Vec<Phrase> {
    CORPUS
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut it = l.split('\t');
            Phrase { kind: it.next().unwrap_or_default(), text: it.next().unwrap_or_default(), read: it.next().unwrap_or_default() }
        })
        .collect()
}

/// A Speech project of the phrases, a block each, in Russian with stress marking, normalisation and the check by
/// recognition on: voiced by an engine, its quality report shows which phrases the engine says wrong.
pub fn project(name: &str, narrator: &str) -> crate::speech::project::SpeechProject {
    let mut p = crate::speech::project::from_text(name, "", narrator);
    p.chapters[0].blocks = phrases()
        .iter()
        .map(|ph| crate::speech::project::Block { id: crate::speech::project::new_id(), speaker: narrator.to_string(), text: ph.text.to_string(), ..Default::default() })
        .collect();
    p.settings.lang = "ru".into();
    p.settings.auto_stress = true;
    p.settings.normalize = true;
    p.settings.qc.asr_check = true;
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn the_project_has_a_block_a_phrase_and_checks_by_recognition() {
        let p = project("Corpus", "Narrator");
        assert_eq!(p.chapters[0].blocks.len(), phrases().len());
        assert!(p.chapters[0].blocks.iter().all(|b| b.speaker == "Narrator"));
        assert!(p.settings.qc.asr_check && p.settings.auto_stress && p.settings.normalize && p.settings.lang == "ru");
    }

    #[test]
    fn every_phrase_has_a_known_kind_and_its_reading() {
        let all = phrases();
        assert!(all.len() >= 25);
        for p in &all {
            assert!(KINDS.contains(&p.kind), "{}: kind {}", p.text, p.kind);
            assert!(!p.text.trim().is_empty() && !p.read.trim().is_empty(), "{}: no reading", p.text);
        }
    }

    /// SPEECH_STUDIO_ROOT: a studio root with RUAccent and onnxruntime.
    #[test]
    #[ignore]
    fn the_hard_phrases_read_as_pinned() {
        let root = PathBuf::from(std::env::var("SPEECH_STUDIO_ROOT").expect("SPEECH_STUDIO_ROOT"));
        crate::set_ort_dylib_env(&root);
        let s = crate::engines::stress::Stresser::load(&crate::engines::stress::dir(Path::new(&root.join("models")))).expect("RUAccent");
        let mut wrong = Vec::new();
        for p in phrases() {
            let got = s.annotate(&crate::text::normalize_ru::normalize(p.text).text).expect("annotate").text;
            eprintln!("{}\t{}\t{}", p.kind, p.text, got);
            if got != p.read {
                wrong.push(format!("{}\n  want: {}\n  got:  {}", p.text, p.read, got));
            }
        }
        assert!(wrong.is_empty(), "{} of {}:\n{}", wrong.len(), phrases().len(), wrong.join("\n"));
    }
}
