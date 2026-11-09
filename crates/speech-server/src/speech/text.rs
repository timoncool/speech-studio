//! Speech selects the text format for its engine; parsing and adaptation live in shared text modules.

use crate::text::adapter::Adapter;
pub(crate) use crate::text::adapter::Ignored;
pub(crate) use crate::text::markup::{has_stress, markup_problems, phrases, pieces, sentences, visible, Piece, WORD_GAP_MS};

pub(super) fn adapter(engine: &str) -> Adapter {
    match engine {
        crate::engines::models::DRAMA_TTS => Adapter::Drama,
        crate::engines::supertonic::ENGINE => Adapter::Plain {
            acute: true,
            own: crate::text::adapter::SUPERTONIC_TAGS[0].1,
        },
        crate::engines::qwen_tts::ENGINE_STRESS => Adapter::Plain { acute: true, own: &[] },
        crate::engines::qwen_tts::ENGINE => Adapter::Plain { acute: false, own: &[] },
        crate::engines::audio8::ENGINE => Adapter::Plain { acute: false, own: &[] },
        _ => Adapter::Higgs,
    }
}

pub fn for_engine(say: &str, lang: &str, engine: &str) -> (String, Ignored) {
    adapter(engine).apply(say, lang)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supertonic_keeps_its_own_tags_and_drops_the_rest() {
        let (s, ig) = for_engine(
            "Ну <laugh> хватит <|emotion:happy|> [laughs] <whisper>уже</whisper>.",
            "ru",
            crate::engines::supertonic::ENGINE,
        );
        assert!(
            s.contains("<laugh>") && !s.contains("<|") && !s.contains("[laughs]") && !s.contains("<whisper>"),
            "{s}"
        );
        assert_eq!(ig.unknown_tags.len(), 4, "{ig:?}");
    }

    #[test]
    fn engines_without_own_markup_get_plain_text() {
        for engine in [crate::engines::qwen_tts::ENGINE] {
            let (s, ignored) = for_engine("Он зашёл в з+амок <|emotion:happy|> [laughs] сразу.", "ru", engine);
            assert!(
                s.contains("замок") && !s.contains('\u{301}') && !s.contains('+') && !s.contains("<|") && !s.contains("[laughs]"),
                "{engine}: {s}"
            );
            assert!(ignored.stress && ignored.unknown_tags.len() == 2, "{engine}: {ignored:?}");
        }
    }

    #[test]
    fn supertonic_and_the_stress_adapter_read_an_acute_after_the_vowel() {
        for engine in [crate::engines::qwen_tts::ENGINE_STRESS, crate::engines::supertonic::ENGINE] {
            let (q, ignored) = for_engine("Он зашёл в з+амок <|emotion:happy|> [laughs] сразу.", "ru", engine);
            assert!(q.contains("за\u{301}мок"), "{engine}: {q}");
            assert!(
                !q.contains('+') && !q.contains("<|") && !q.contains("[laughs]") && !q.contains('\u{E002}'),
                "{engine}: {q}"
            );
            assert!(!ignored.stress && ignored.unknown_tags.len() == 2, "{engine}: {ignored:?}");
        }
    }

    #[test]
    fn audio8_is_given_the_text_without_stress_marks_or_tags() {
        let (q, ignored) = for_engine("Он зашёл в з+амок <|emotion:happy|> [laughs] сразу.", "ru", crate::engines::audio8::ENGINE);
        assert!(q.contains("замок"), "{q}");
        assert!(!q.contains('+') && !q.contains('\u{301}') && !q.contains("<|") && !q.contains("[laughs]"), "{q}");
        assert!(ignored.stress && ignored.unknown_tags.len() == 2, "{ignored:?}");
    }
}
