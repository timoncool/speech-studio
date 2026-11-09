//! A book project rebuilt from an edited source without losing what was voiced: each paragraph of the new text that
//! reads as a run of the project's blocks (the same letters, whatever markup, stress marks or ё were added, however
//! the director split it among speakers) keeps those blocks with their takes; any other paragraph becomes new blocks.

use std::collections::HashSet;

use crate::speech::project as sp;
use crate::speech::project::{Block, Chapter, SpeechProject};

/// What a rebuild keeps and changes.
#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub struct Stats {
    /// Blocks kept as they were, takes and all.
    pub kept: usize,
    /// Blocks made from new or changed paragraphs.
    pub added: usize,
    /// Blocks whose text is gone from the book, and how many of them had takes.
    pub dropped: usize,
    pub dropped_voiced: usize,
}

/// One new chapter: its title and text.
pub struct NewChapter {
    pub title: String,
    pub text: String,
}

fn key(text: &str) -> String {
    crate::speech::text::visible(text).chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).map(|c| if c == 'ё' { 'е' } else { c }).collect()
}

/// The first run of unused old blocks, in their order, whose letters make `want`.
fn run_for(want: &str, keys: &[String], used: &[bool]) -> Option<(usize, usize)> {
    if want.is_empty() {
        return None;
    }
    for start in 0..keys.len() {
        if used[start] || keys[start].is_empty() || !want.starts_with(keys[start].as_str()) {
            continue;
        }
        let mut have = 0;
        for end in start..keys.len() {
            if used[end] {
                break;
            }
            if !want[have..].starts_with(keys[end].as_str()) {
                break;
            }
            have += keys[end].len();
            if have == want.len() {
                return Some((start, end + 1));
            }
        }
    }
    None
}

/// The project with its chapters made anew from `chapters`, the blocks of unchanged paragraphs kept. A chapter keeps
/// its id when the old project has one of the same title, or an untitled one at the same place. `lines` puts direct speech of new paragraphs under its own
/// speaker, as at import.
pub fn rebuild(old: &SpeechProject, chapters: &[NewChapter], narrator: &str, lines: Option<&str>) -> (SpeechProject, Stats) {
    let blocks: Vec<&Block> = old.blocks().collect();
    let keys: Vec<String> = blocks.iter().map(|b| key(&b.text)).collect();
    let mut used = vec![false; blocks.len()];
    let mut stats = Stats::default();
    let mut titles_taken: HashSet<String> = HashSet::new();
    let mut out = old.clone();
    out.chapters = chapters
        .iter()
        .enumerate()
        .map(|(n, c)| {
            let free = |o: &&Chapter| !titles_taken.contains(&o.id);
            let id = old
                .chapters
                .iter()
                .filter(free)
                .find(|o| !o.title.is_empty() && o.title == c.title)
                .or_else(|| old.chapters.get(n).filter(|o| o.title.is_empty() && c.title.is_empty()).filter(free))
                .map(|o| o.id.clone())
                .unwrap_or_else(sp::new_id);
            titles_taken.insert(id.clone());
            let mut made: Vec<Block> = Vec::new();
            for para in c.text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
                match run_for(&key(para), &keys, &used) {
                    Some((a, b)) => {
                        for i in a..b {
                            used[i] = true;
                            made.push(blocks[i].clone());
                        }
                        stats.kept += b - a;
                    }
                    None if sp::says_nothing(para) => {
                        if let Some(prev) = made.last_mut() {
                            prev.pause_ms = Some(prev.pause_ms.unwrap_or(0).max(sp::SCENE_BREAK_MS));
                        }
                    }
                    None => {
                        let fresh: Vec<Block> = sp::blocks_of(para, narrator, lines);
                        stats.added += fresh.len();
                        made.extend(fresh);
                    }
                }
            }
            Chapter { id, title: c.title.clone(), blocks: made }
        })
        .collect();
    for b in blocks.iter().zip(&used).filter(|(_, u)| !**u).map(|(b, _)| b) {
        stats.dropped += 1;
        if !b.takes.is_empty() {
            stats.dropped_voiced += 1;
        }
    }
    (out, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(speaker: &str, text: &str, takes: usize) -> Block {
        let mut b = Block { id: sp::new_id(), speaker: speaker.into(), text: text.into(), ..Default::default() };
        b.takes = (1..=takes as u32).map(|n| sp::Take { n, ..Default::default() }).collect();
        b
    }

    #[test]
    fn unchanged_paragraphs_keep_their_voiced_blocks_however_they_were_split() {
        let mut old = SpeechProject::default();
        let anna = block("Анна", "<|emotion:anger|>Ты опять опоздал,", 2);
        let said = block("Рассказчик", "— сказала Анна.", 1);
        let morning = block("Рассказчик", "Утро было т+ихим.", 1);
        let gone = block("Рассказчик", "Этого абзаца больше нет.", 1);
        old.chapters.push(Chapter { id: "c1".into(), title: "Глава 1".into(), blocks: vec![morning.clone(), anna.clone(), said.clone(), gone] });
        let text = "Утро было тихим.\n\n— Ты опять опоздал, — сказала Анна.\n\nНовый абзац.".to_string();
        let (new, stats) = rebuild(&old, &[NewChapter { title: "Глава 1".into(), text }], "Рассказчик", None);
        assert_eq!(stats, Stats { kept: 3, added: 1, dropped: 1, dropped_voiced: 1 });
        let ids: Vec<&str> = new.chapters[0].blocks.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(&ids[..3], &[morning.id.as_str(), anna.id.as_str(), said.id.as_str()]);
        assert_eq!(new.chapters[0].id, "c1");
        assert_eq!(new.chapters[0].blocks[1].takes.len(), 2);
        assert_eq!(new.chapters[0].blocks[3].text, "Новый абзац.");
    }

    #[test]
    fn a_changed_word_makes_the_paragraph_new() {
        let mut old = SpeechProject::default();
        old.chapters.push(Chapter { id: "c".into(), title: String::new(), blocks: vec![block("Рассказчик", "Утро было тихим.", 1)] });
        let (new, stats) = rebuild(&old, &[NewChapter { title: String::new(), text: "Утро было ясным.".into() }], "Рассказчик", None);
        assert_eq!(stats, Stats { kept: 0, added: 1, dropped: 1, dropped_voiced: 1 });
        assert_eq!(new.chapters[0].id, "c", "an untitled chapter keeps its id by place");
    }
}
