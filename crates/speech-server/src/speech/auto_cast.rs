//! Voices from the library for the characters a project has no voice for, by what is known of each: a voice of the
//! other gender is never offered, then the age and the project's language weigh; each voice goes to one character, and
//! the voices the project already speaks with are left out.

use crate::speech::project::{Character, SpeechProject};
use crate::voices::Voice;

const AGE_ORDER: [&str; 4] = ["child", "teen", "adult", "senior"];

fn age_score(want: &str, have: &str) -> i32 {
    let (Some(a), Some(b)) = (AGE_ORDER.iter().position(|x| *x == want), AGE_ORDER.iter().position(|x| *x == have)) else { return 0 };
    match a.abs_diff(b) {
        0 => 3,
        1 => 1,
        _ => -2,
    }
}

fn lang_score(project: &str, voice: &str) -> i32 {
    let main = |l: &str| l.split(['-', '_']).next().unwrap_or("").to_lowercase();
    if project.is_empty() || project == "auto" || voice.is_empty() {
        0
    } else if main(project) == main(voice) {
        2
    } else {
        -3
    }
}

/// How well a library voice fits a character; `None` when it cannot be that character's.
fn fit(c: &Character, v: &Voice, lang: &str) -> Option<i32> {
    if !c.gender.is_empty() && !v.card.gender.is_empty() && c.gender != v.card.gender {
        return None;
    }
    let gender = if !c.gender.is_empty() && c.gender == v.card.gender { 4 } else { 0 };
    Some(gender + age_score(&c.age, &v.card.age) + lang_score(lang, &v.card.lang) + i32::from(v.card.favorite))
}

/// The voice offered to each speaker with no voice in the cast, the narrator aside, in the order the speakers first
/// speak; `None` when no free voice of the library fits. A project in `auto` written mostly in Cyrillic counts as
/// Russian. `library` comes sorted by name, which settles ties.
pub fn propose(project: &SpeechProject, library: &[Voice], narrator: &str) -> Vec<(String, Option<String>)> {
    let mut taken: Vec<String> = project.cast.values().chain(project.settings.voice.iter()).filter(|v| !v.is_empty()).cloned().collect();
    let mut speakers: Vec<&str> = Vec::new();
    for b in project.blocks() {
        if b.speaker != narrator && !speakers.contains(&b.speaker.as_str()) && project.cast.get(&b.speaker).is_none_or(|v| v.is_empty()) {
            speakers.push(&b.speaker);
        }
    }
    let text = project.blocks().map(|b| b.text.as_str()).collect::<Vec<_>>().join(" ");
    let lang = if project.settings.lang == "auto" && crate::engines::stress::is_russian(&text, "auto") { "ru" } else { project.settings.lang.as_str() };
    let unknown = Character::default();
    speakers
        .into_iter()
        .map(|s| {
            let c = project.characters.get(s).unwrap_or(&unknown);
            let best = library
                .iter()
                .filter(|v| v.card.variant_of.is_empty() && !taken.contains(&v.name))
                .filter_map(|v| fit(c, v, lang).map(|score| (score, v)))
                .fold(None::<(i32, &Voice)>, |best, (score, v)| match best {
                    Some((b, _)) if b >= score => best,
                    _ => Some((score, v)),
                })
                .map(|(_, v)| v.name.clone());
            if let Some(v) = &best {
                taken.push(v.clone());
            }
            (s.to_string(), best)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speech::project::Block;
    use crate::speech::project::Chapter;
    use crate::voices::VoiceCard;

    fn voice(name: &str, gender: &str, age: &str, lang: &str) -> Voice {
        Voice { name: name.into(), audio: String::new(), size: 0, transcript: None, photo: None, original: false, quiet: false, card: VoiceCard { gender: gender.into(), age: age.into(), lang: lang.into(), ..Default::default() } }
    }

    #[test]
    fn each_character_gets_a_free_voice_of_its_gender_and_nearest_age() {
        let mut p = SpeechProject::default();
        p.settings.lang = "ru".into();
        p.settings.voice = Some("Диктор".into());
        let block = |s: &str| Block { id: s.into(), speaker: s.into(), text: "Да.".into(), ..Default::default() };
        p.chapters.push(Chapter { id: "c".into(), title: String::new(), blocks: vec![block("Рассказчик"), block("Анна"), block("Дед"), block("Маша"), block("Олег")] });
        p.characters.insert("Анна".into(), Character { gender: "female".into(), age: "adult".into() });
        p.characters.insert("Дед".into(), Character { gender: "male".into(), age: "senior".into() });
        p.characters.insert("Маша".into(), Character { gender: "female".into(), age: "child".into() });
        p.cast.insert("Олег".into(), "Свой".into());
        let library = vec![
            voice("Ася", "female", "teen", "ru"),
            voice("Вера", "female", "adult", "ru"),
            voice("Диктор", "male", "adult", "ru"),
            voice("Кейт", "female", "adult", "en"),
            voice("Пётр", "male", "senior", "ru"),
        ];
        let picks = propose(&p, &library, "Рассказчик");
        assert_eq!(picks, vec![("Анна".into(), Some("Вера".into())), ("Дед".into(), Some("Пётр".into())), ("Маша".into(), Some("Ася".into()))]);
    }

    #[test]
    fn no_voice_of_the_other_gender_is_offered() {
        let mut p = SpeechProject::default();
        p.chapters.push(Chapter { id: "c".into(), title: String::new(), blocks: vec![Block { id: "b".into(), speaker: "Анна".into(), text: "Да.".into(), ..Default::default() }] });
        p.characters.insert("Анна".into(), Character { gender: "female".into(), age: String::new() });
        assert_eq!(propose(&p, &[voice("Пётр", "male", "adult", "ru")], "Рассказчик"), vec![("Анна".into(), None)]);
    }
}
