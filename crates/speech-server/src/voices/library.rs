//! Голоса-персоны. Голос — файл `voices/<имя>.<wav|mp3|flac|m4a|ogg>` с транскриптом рефа `<имя>.txt` рядом (формат
//! Dub Studio и войспака: такие папки читаются как есть). Карточка персоны — `<имя>.json`, фото — `<имя>.<jpg|png|webp>`.
//! Голоса без карточки получают её по имени (`RU_Female_…` войспака: язык и пол).
//! Исходник голоса до очистки хранится в `voices/.originals/`, чтобы очистку можно было отменить.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::platform::fs::write_atomic;

pub const AUDIO_EXTS: [&str; 5] = ["wav", "mp3", "flac", "m4a", "ogg"];
pub const PHOTO_EXTS: [&str; 3] = ["jpg", "png", "webp"];
const ORIGINALS: &str = ".originals";
const WORK: &str = ".work";

/// Временный WAV для звука, который пишет ffmpeg: формат он берёт из расширения, поэтому `.wav` в конце, а папка
/// служебная, чтобы недописанный файл не попал в список голосов.
fn staging_wav(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let work = dir.join(WORK);
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    Ok(work.join(format!("{name}-{}.wav", uuid::Uuid::new_v4().simple())))
}
pub const GENDERS: [&str; 3] = ["", "female", "male"];
pub const AGES: [&str; 5] = ["", "child", "teen", "adult", "senior"];
pub const CONSENTS: [&str; 4] = ["", "own", "permitted", "public"];
pub const KINDS: [&str; 3] = ["clone", "preset", "design"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct VoiceCard {
    pub gender: String,
    pub age: String,
    pub lang: String,
    pub tags: Vec<String>,
    pub source: String,
    pub consent: String,
    pub favorite: bool,
    pub kind: String,
    pub transcript_auto: bool,
    pub cleaned: bool,
    /// The voice this one is a variant of (a performance of the same character: angry, whispering); empty — its own.
    pub variant_of: String,
    /// The variant's name, shown where a block picks it.
    pub variant: String,
    /// What the voice may be used for: an SPDX id such as CC-BY-4.0, or the terms in words.
    pub license: String,
    /// A long reference gives each new take its next good piece instead of always the best one.
    pub rotate: bool,
    /// The name its source gives the voice (the cloud catalogue's «Алана Хасанова»), shown instead of the file's.
    pub display: String,
    /// Who the voice is, as its source describes it (the cloud catalogue's line).
    pub description: String,
    /// The speaker's year of birth and the recording's length in seconds, as its source gives them.
    pub birth_year: Option<u32>,
    pub duration_s: Option<f64>,
}

impl Default for VoiceCard {
    fn default() -> Self {
        VoiceCard {
            gender: String::new(),
            age: String::new(),
            lang: String::new(),
            tags: Vec::new(),
            source: String::new(),
            consent: String::new(),
            favorite: false,
            kind: "clone".into(),
            transcript_auto: false,
            cleaned: false,
            variant_of: String::new(),
            variant: String::new(),
            license: String::new(),
            rotate: false,
            display: String::new(),
            description: String::new(),
            birth_year: None,
            duration_s: None,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Voice {
    pub name: String,
    pub audio: String,
    pub size: u64,
    pub transcript: Option<String>,
    pub photo: Option<String>,
    pub original: bool,
    pub card: VoiceCard,
}

/// Имя голоса, безопасное для файла: буквы, цифры, пробел, `-`, `_`, до 48 знаков.
pub fn sanitize(s: &str) -> String {
    let n: String = s.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_').take(48).collect();
    n.trim().to_string()
}

fn with_ext(dir: &Path, name: &str, exts: &[&str]) -> Option<(PathBuf, String)> {
    exts.iter().map(|e| (dir.join(format!("{name}.{e}")), e.to_string())).find(|(p, _)| p.is_file())
}

pub fn audio_path(dir: &Path, name: &str) -> Option<PathBuf> {
    with_ext(dir, name, &AUDIO_EXTS).map(|(p, _)| p)
}

pub fn photo_path(dir: &Path, name: &str) -> Option<PathBuf> {
    with_ext(dir, name, &PHOTO_EXTS).map(|(p, _)| p)
}

pub fn original_path(dir: &Path, name: &str) -> Option<PathBuf> {
    with_ext(&dir.join(ORIGINALS), name, &AUDIO_EXTS).map(|(p, _)| p)
}

const LANG_WORDS: [(&str, &str); 11] = [
    ("arabic", "ar"), ("chinese", "zh"), ("english", "en"), ("french", "fr"), ("german", "de"), ("italian", "it"),
    ("japanese", "ja"), ("korean", "ko"), ("portuguese", "pt"), ("russian", "ru"), ("spanish", "es"),
];

fn gender_word(w: &str) -> Option<&'static str> {
    match w.to_ascii_lowercase().trim_end_matches(|c: char| c.is_ascii_digit()) {
        "female" | "famale" => Some("female"),
        "male" => Some("male"),
        _ => None,
    }
}

/// Карточка по имени из войспака: `RU_Female_…` (регистр любой, бывает и `Famale`) и `English_Female2` — язык и пол.
pub fn guess_card(name: &str) -> VoiceCard {
    let mut card = VoiceCard::default();
    let parts: Vec<&str> = name.splitn(3, '_').collect();
    if parts.len() == 3 && parts[0].len() == 2 && parts[0].chars().all(|c| c.is_ascii_alphabetic()) {
        if let Some(g) = gender_word(parts[1]) {
            card.lang = parts[0].to_ascii_lowercase();
            card.gender = g.into();
            card.source = "pack".into();
        }
    } else if parts.len() == 2 {
        let lang = LANG_WORDS.iter().find(|(w, _)| parts[0].eq_ignore_ascii_case(w)).map(|(_, c)| *c);
        if let (Some(lang), Some(g)) = (lang, gender_word(parts[1])) {
            card.lang = lang.into();
            card.gender = g.into();
            card.source = "pack".into();
        }
    }
    card
}

pub fn read_card(dir: &Path, name: &str) -> Result<VoiceCard, String> {
    let p = dir.join(format!("{name}.json"));
    match std::fs::read_to_string(&p) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(guess_card(name)),
        Err(e) => Err(format!("{}: {e}", p.display())),
    }
}

pub fn write_card(dir: &Path, name: &str, card: &VoiceCard) -> Result<(), String> {
    let text = serde_json::to_string_pretty(card).map_err(|e| e.to_string())?;
    write_atomic(&dir.join(format!("{name}.json")), text.as_bytes())
}

fn one_of(field: &str, value: &Value, allowed: &[&str]) -> Result<String, String> {
    let v = value.as_str().ok_or_else(|| format!("{field} must be a string"))?;
    if allowed.contains(&v) {
        Ok(v.to_string())
    } else {
        Err(format!("{field} must be one of: {}", allowed.iter().filter(|a| !a.is_empty()).cloned().collect::<Vec<_>>().join(", ")))
    }
}

/// Поля карточки, которые меняет пользователь: незнакомое поле или значение — ошибка, а не тихий пропуск.
pub fn apply_patch(card: &mut VoiceCard, patch: &Value) -> Result<(), String> {
    let obj = patch.as_object().ok_or("the card change must be an object")?;
    for (k, v) in obj {
        match k.as_str() {
            "gender" => card.gender = one_of(k, v, &GENDERS)?,
            "age" => card.age = one_of(k, v, &AGES)?,
            "consent" => card.consent = one_of(k, v, &CONSENTS)?,
            "kind" => card.kind = one_of(k, v, &KINDS)?,
            "lang" => {
                let l = v.as_str().ok_or("lang must be a string")?.trim().to_ascii_lowercase();
                if !(l.is_empty() || (2..=3).contains(&l.len()) && l.chars().all(|c| c.is_ascii_lowercase())) {
                    return Err("lang must be a language code such as ru or en".into());
                }
                card.lang = l;
            }
            "source" => card.source = v.as_str().ok_or("source must be a string")?.trim().chars().take(200).collect(),
            "favorite" => card.favorite = v.as_bool().ok_or("favorite must be true or false")?,
            "variant_of" => card.variant_of = v.as_str().ok_or("variant_of must be a voice name")?.trim().chars().take(200).collect(),
            "variant" => card.variant = v.as_str().ok_or("variant must be a string")?.trim().chars().take(32).collect(),
            "rotate" => card.rotate = v.as_bool().ok_or("rotate must be true or false")?,
            "license" => card.license = v.as_str().ok_or("license must be a string")?.trim().chars().take(120).collect(),
            "tags" => {
                let arr = v.as_array().ok_or("tags must be a list")?;
                let mut tags: Vec<String> = Vec::new();
                for t in arr {
                    let t = t.as_str().ok_or("every tag must be a string")?.trim().to_lowercase();
                    if !t.is_empty() && !tags.contains(&t) {
                        tags.push(t.chars().take(32).collect());
                    }
                }
                card.tags = tags;
            }
            other => return Err(format!("unknown card field: {other}")),
        }
    }
    Ok(())
}

/// Транскрипт рефа: `<имя>.txt`, а у голосов войспака бывает и `<имя>.MP3.txt` и `<имя>.lab`.
pub fn read_transcript(dir: &Path, name: &str) -> Option<String> {
    let direct = std::iter::once(dir.join(format!("{name}.txt")));
    let doubled = AUDIO_EXTS.iter().flat_map(|e| [dir.join(format!("{name}.{e}.txt")), dir.join(format!("{name}.{}.txt", e.to_ascii_uppercase()))]);
    let lab = std::iter::once(dir.join(format!("{name}.lab")));
    direct.chain(doubled).chain(lab).find_map(|p| std::fs::read_to_string(p).ok()).map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

/// Транскрипт рефа: пустой текст убирает файл (движок клонирует без текста рефа) вместе с `.lab` войспака, который
/// иначе вернул бы старый текст.
pub fn write_transcript(dir: &Path, name: &str, text: &str) -> Result<(), String> {
    let p = dir.join(format!("{name}.txt"));
    let text = text.trim();
    if text.is_empty() {
        for p in [p, dir.join(format!("{name}.lab"))] {
            match std::fs::remove_file(&p) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(format!("{}: {e}", p.display())),
                _ => {}
            }
        }
        return Ok(());
    }
    write_atomic(&p, text.as_bytes())
}

pub fn get(dir: &Path, name: &str) -> Result<Voice, String> {
    let (audio, ext) = with_ext(dir, name, &AUDIO_EXTS).ok_or_else(|| format!("no voice named {name}"))?;
    Ok(Voice {
        name: name.to_string(),
        audio: ext,
        size: std::fs::metadata(&audio).map(|m| m.len()).unwrap_or(0),
        transcript: read_transcript(dir, name),
        photo: with_ext(dir, name, &PHOTO_EXTS).map(|(_, e)| e),
        original: original_path(dir, name).is_some(),
        card: read_card(dir, name)?,
    })
}

/// Все голоса папки по имени; голос с испорченной карточкой попадает в `broken` с причиной.
pub fn list(dir: &Path) -> (Vec<Voice>, Vec<(String, String)>) {
    let mut names = std::collections::BTreeSet::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if p.is_file() && AUDIO_EXTS.contains(&ext.as_str()) {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    names.insert(stem.to_string());
                }
            }
        }
    }
    let mut voices = Vec::new();
    let mut broken = Vec::new();
    for n in names {
        match get(dir, &n) {
            Ok(v) => voices.push(v),
            Err(e) => broken.push((n, e)),
        }
    }
    (voices, broken)
}

/// Файлы голоса и их суффиксы после имени (`.wav`, `.words.json`, `.MP3.txt` …), в папке и в `.originals`.
fn sidecars(dir: &Path, name: &str) -> Vec<(PathBuf, String)> {
    let mut suffixes: Vec<String> = AUDIO_EXTS.iter().chain(PHOTO_EXTS.iter()).chain(["txt", "lab", "json", "words.json"].iter()).map(|e| format!(".{e}")).collect();
    suffixes.extend(AUDIO_EXTS.iter().flat_map(|e| [format!(".{e}.txt"), format!(".{}.txt", e.to_ascii_uppercase())]));
    let mut all = Vec::new();
    for folder in [dir.to_path_buf(), dir.join(ORIGINALS)] {
        for suffix in &suffixes {
            let p = folder.join(format!("{name}{suffix}"));
            if p.is_file() {
                all.push((p, suffix.clone()));
            }
        }
    }
    all
}

pub fn rename(dir: &Path, from: &str, to: &str) -> Result<(), String> {
    if to.is_empty() {
        return Err("the new name is empty".into());
    }
    if from == to {
        return Ok(());
    }
    audio_path(dir, from).ok_or_else(|| format!("no voice named {from}"))?;
    if audio_path(dir, to).is_some() {
        return Err(format!("a voice named {to} already exists"));
    }
    for (p, suffix) in sidecars(dir, from) {
        let target = p.with_file_name(format!("{to}{suffix}"));
        std::fs::rename(&p, &target).map_err(|e| format!("{} -> {}: {e}", p.display(), target.display()))?;
    }
    relink_variants(dir, from, Some(to))
}

/// The variants of `parent` follow it to its new name, or become voices of their own when it is gone.
fn relink_variants(dir: &Path, parent: &str, to: Option<&str>) -> Result<(), String> {
    let (voices, _) = list(dir);
    for v in voices.iter().filter(|v| v.card.variant_of == parent) {
        let mut card = v.card.clone();
        card.variant_of = to.unwrap_or_default().to_string();
        if to.is_none() {
            card.variant.clear();
        }
        write_card(dir, &v.name, &card)?;
    }
    Ok(())
}

/// A variant points at a voice of its own that exists, and a voice with variants is not one itself.
pub fn check_variant(dir: &Path, name: &str, card: &VoiceCard) -> Result<(), String> {
    if card.variant_of.is_empty() {
        return Ok(());
    }
    if card.variant_of == name {
        return Err("a voice cannot be a variant of itself".into());
    }
    audio_path(dir, &card.variant_of).ok_or_else(|| format!("no voice named {}", card.variant_of))?;
    if !read_card(dir, &card.variant_of)?.variant_of.is_empty() {
        return Err(format!("{} is itself a variant: name the voice it belongs to", card.variant_of));
    }
    let (voices, _) = list(dir);
    if voices.iter().any(|v| v.card.variant_of == name) {
        return Err(format!("{name} has variants of its own, so it cannot be one"));
    }
    if card.variant.is_empty() {
        return Err("a variant needs a name, such as angry or whisper".into());
    }
    Ok(())
}

pub fn delete(dir: &Path, name: &str) -> Result<(), String> {
    audio_path(dir, name).ok_or_else(|| format!("no voice named {name}"))?;
    for (p, _) in sidecars(dir, name) {
        std::fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    relink_variants(dir, name, None)
}

pub fn set_photo(dir: &Path, name: &str, bytes: &[u8], ext: &str) -> Result<(), String> {
    let ext = ext.to_ascii_lowercase();
    let ext = if ext == "jpeg" { "jpg".to_string() } else { ext };
    if !PHOTO_EXTS.contains(&ext.as_str()) {
        return Err("the photo must be JPG, PNG or WebP".into());
    }
    audio_path(dir, name).ok_or_else(|| format!("no voice named {name}"))?;
    for e in PHOTO_EXTS {
        let p = dir.join(format!("{name}.{e}"));
        if p.is_file() {
            std::fs::remove_file(&p).map_err(|er| format!("{}: {er}", p.display()))?;
        }
    }
    write_atomic(&dir.join(format!("{name}.{ext}")), bytes)
}

/// Имя, свободное в папке: `base`, затем `base 2`, `base 3`…
pub fn free_name(dir: &Path, base: &str) -> String {
    let base = if base.is_empty() { "Voice".to_string() } else { base.to_string() };
    if audio_path(dir, &base).is_none() {
        return base;
    }
    (2..).map(|i| format!("{base} {i}")).find(|n| audio_path(dir, n).is_none()).expect("an unused name")
}

/// Новый голос из звукового файла: WAV с громкостью рефа -20 LUFS (пик -2 dBTP), под свободным именем.
pub fn import_audio(dir: &Path, base: &str, src: &Path, card: VoiceCard) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = free_name(dir, base);
    let target = dir.join(format!("{name}.wav"));
    let part = staging_wav(dir, &name)?;
    let made = crate::audio::media::loudnorm(src, &part, -20.0, -2.0, 11.0).and_then(|_| std::fs::rename(&part, &target).map_err(|e| format!("{}: {e}", target.display())));
    if let Err(e) = made {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    write_card(dir, &name, &card)?;
    Ok(name)
}

/// Заменить звук голоса очищенным, сохранив исходник в `.originals` (только первый: повторная очистка не
/// затирает настоящий исходник уже очищенным звуком).
pub fn replace_with_cleaned(dir: &Path, name: &str, cleaned: &Path) -> Result<(), String> {
    let audio = audio_path(dir, name).ok_or_else(|| format!("no voice named {name}"))?;
    let originals = dir.join(ORIGINALS);
    std::fs::create_dir_all(&originals).map_err(|e| format!("{}: {e}", originals.display()))?;
    if original_path(dir, name).is_none() {
        let keep = originals.join(audio.file_name().expect("a file name"));
        std::fs::copy(&audio, &keep).map_err(|e| format!("{}: {e}", keep.display()))?;
    }
    let target = dir.join(format!("{name}.wav"));
    let part = staging_wav(dir, name)?;
    if let Err(e) = crate::audio::media::loudnorm(cleaned, &part, -20.0, -2.0, 11.0) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    if audio != target {
        std::fs::remove_file(&audio).map_err(|e| format!("{}: {e}", audio.display()))?;
    }
    std::fs::rename(&part, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    let mut card = read_card(dir, name)?;
    card.cleaned = true;
    write_card(dir, name, &card)
}

pub fn read_words(dir: &Path, name: &str) -> Option<Vec<crate::voices::ref_pick::Word>> {
    std::fs::read_to_string(dir.join(format!("{name}.words.json"))).ok().and_then(|t| serde_json::from_str(&t).ok())
}

pub fn write_words(dir: &Path, name: &str, words: &[crate::voices::ref_pick::Word]) -> Result<(), String> {
    let text = serde_json::to_string(words).map_err(|e| e.to_string())?;
    write_atomic(&dir.join(format!("{name}.words.json")), text.as_bytes())
}

/// Сохранить текст и тайминги до правки рефа в `.originals` (только первые, как и звук); текст войспака из `.lab`
/// сохраняется как `.txt`.
fn keep_original_text(dir: &Path, name: &str) -> Result<(), String> {
    let originals = dir.join(ORIGINALS);
    std::fs::create_dir_all(&originals).map_err(|e| format!("{}: {e}", originals.display()))?;
    for suffix in [".txt", ".words.json"] {
        let mut src = dir.join(format!("{name}{suffix}"));
        if suffix == ".txt" && !src.is_file() {
            src = dir.join(format!("{name}.lab"));
        }
        let keep = originals.join(format!("{name}{suffix}"));
        if src.is_file() && !keep.is_file() {
            std::fs::copy(&src, &keep).map_err(|e| format!("{}: {e}", keep.display()))?;
        }
    }
    Ok(())
}

/// Оставить от рефа кусок `start..end`: звук обрезается, текст и тайминги — по словам куска; исходник со всем
/// его текстом остаётся в `.originals`.
pub fn apply_ref(dir: &Path, name: &str, start: f64, end: f64) -> Result<(), String> {
    if !(end > start && start >= 0.0) {
        return Err(format!("bad window {start}..{end}"));
    }
    let audio = audio_path(dir, name).ok_or_else(|| format!("no voice named {name}"))?;
    let words = read_words(dir, name).ok_or_else(|| format!("{name} has no word timings: recognise its text first"))?;
    let originals = dir.join(ORIGINALS);
    std::fs::create_dir_all(&originals).map_err(|e| format!("{}: {e}", originals.display()))?;
    if original_path(dir, name).is_none() {
        let keep = originals.join(audio.file_name().expect("a file name"));
        std::fs::copy(&audio, &keep).map_err(|e| format!("{}: {e}", keep.display()))?;
    }
    keep_original_text(dir, name)?;
    let part = staging_wav(dir, name)?;
    if let Err(e) = crate::audio::media::trim_ref(&audio, &part, start, end) {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    let target = dir.join(format!("{name}.wav"));
    if audio != target {
        std::fs::remove_file(&audio).map_err(|e| format!("{}: {e}", audio.display()))?;
    }
    std::fs::rename(&part, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    let inside = crate::voices::ref_pick::words_within(&words, start, end);
    write_words(dir, name, &inside)?;
    let text = inside.iter().map(|w| w.word.trim()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
    write_transcript(dir, name, &text)
}

/// Вернуть исходник вместо очищенного звука.
pub fn restore_original(dir: &Path, name: &str) -> Result<(), String> {
    let orig = original_path(dir, name).ok_or_else(|| format!("{name} has no original kept"))?;
    if let Some(cur) = audio_path(dir, name) {
        std::fs::remove_file(&cur).map_err(|e| format!("{}: {e}", cur.display()))?;
    }
    let target = dir.join(orig.file_name().expect("a file name"));
    std::fs::rename(&orig, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    for suffix in [".txt", ".words.json"] {
        let kept = dir.join(ORIGINALS).join(format!("{name}{suffix}"));
        if kept.is_file() {
            let back = dir.join(format!("{name}{suffix}"));
            std::fs::rename(&kept, &back).map_err(|e| format!("{}: {e}", back.display()))?;
        }
    }
    let mut card = read_card(dir, name)?;
    card.cleaned = false;
    write_card(dir, name, &card)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn voice(dir: &Path, name: &str, ext: &str) {
        std::fs::write(dir.join(format!("{name}.{ext}")), b"RIFF").unwrap();
    }

    #[test]
    fn a_pack_voice_gets_its_language_and_gender_from_the_name() {
        let c = guess_card("RU_Female_abramova_oljga");
        assert_eq!((c.lang.as_str(), c.gender.as_str(), c.source.as_str()), ("ru", "female", "pack"));
        assert_eq!(guess_card("Мой голос"), VoiceCard::default());
        let typo = guess_card("RU_Famale_ALEKSANDRA_NAZAROVA");
        assert_eq!((typo.lang.as_str(), typo.gender.as_str()), ("ru", "female"));
        assert_eq!(guess_card("Ru_Male_KlimZhukov").gender, "male");
        assert_eq!(guess_card("RU_MALE_Zadornov(koncert)").gender, "male");
        let worded = guess_card("English_Female2");
        assert_eq!((worded.lang.as_str(), worded.gender.as_str()), ("en", "female"));
        assert_eq!(guess_card("Spanish_Male").lang, "es");
        assert_eq!(guess_card("Some_Thing"), VoiceCard::default());
    }

    #[test]
    fn the_list_carries_transcript_photo_and_card_and_skips_other_files() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "A", "mp3");
        std::fs::write(d.path().join("A.txt"), " привет \n").unwrap();
        std::fs::write(d.path().join("A.png"), b"png").unwrap();
        voice(d.path(), "B", "wav");
        std::fs::write(d.path().join("notes.txt"), "x").unwrap();
        std::fs::create_dir(d.path().join(".originals")).unwrap();
        voice(&d.path().join(".originals"), "B", "mp3");
        let (voices, broken) = list(d.path());
        assert!(broken.is_empty());
        assert_eq!(voices.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!((voices[0].transcript.as_deref(), voices[0].photo.as_deref()), (Some("привет"), Some("png")));
        assert!(voices[1].original && !voices[0].original);
    }

    #[test]
    fn a_broken_card_is_reported_not_hidden() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "A", "wav");
        std::fs::write(d.path().join("A.json"), "{not json").unwrap();
        let (voices, broken) = list(d.path());
        assert!(voices.is_empty());
        assert_eq!(broken[0].0, "A");
    }

    #[test]
    fn a_patch_takes_known_fields_and_refuses_the_rest() {
        let mut c = VoiceCard::default();
        apply_patch(&mut c, &json!({ "gender": "male", "age": "senior", "lang": "EN", "tags": ["Calm", "calm", " deep "], "favorite": true, "consent": "own" })).unwrap();
        assert_eq!((c.gender.as_str(), c.age.as_str(), c.lang.as_str(), c.favorite, c.consent.as_str()), ("male", "senior", "en", true, "own"));
        assert_eq!(c.tags, ["calm", "deep"]);
        assert!(apply_patch(&mut c, &json!({ "gender": "robot" })).is_err());
        assert!(apply_patch(&mut c, &json!({ "colour": "red" })).unwrap_err().contains("unknown card field"));
        assert!(apply_patch(&mut c, &json!({ "lang": "russian" })).is_err());
        assert_eq!(c.gender, "male", "a refused change leaves the card as it was");
    }

    #[test]
    fn rename_moves_every_file_of_the_voice_and_refuses_a_taken_name() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "A", "mp3");
        std::fs::write(d.path().join("A.txt"), "t").unwrap();
        write_card(d.path(), "A", &VoiceCard { favorite: true, ..Default::default() }).unwrap();
        std::fs::create_dir(d.path().join(".originals")).unwrap();
        voice(&d.path().join(".originals"), "A", "wav");
        voice(d.path(), "B", "wav");
        assert!(rename(d.path(), "A", "B").unwrap_err().contains("already exists"));
        rename(d.path(), "A", "C").unwrap();
        let c = get(d.path(), "C").unwrap();
        assert!(c.card.favorite && c.transcript.is_some() && c.original);
        assert!(get(d.path(), "A").is_err());
    }

    #[test]
    fn a_variant_belongs_to_a_voice_of_its_own_and_follows_it() {
        let d = tempfile::tempdir().unwrap();
        for n in ["Oleg", "Oleg angry", "Anna"] {
            voice(d.path(), n, "wav");
        }
        let variant = |of: &str, name: &str| VoiceCard { variant_of: of.into(), variant: name.into(), ..Default::default() };
        assert!(check_variant(d.path(), "Oleg angry", &variant("Nobody", "angry")).is_err());
        assert!(check_variant(d.path(), "Oleg", &variant("Oleg", "angry")).is_err());
        assert!(check_variant(d.path(), "Oleg angry", &variant("Oleg", "")).is_err());
        check_variant(d.path(), "Oleg angry", &variant("Oleg", "angry")).unwrap();
        write_card(d.path(), "Oleg angry", &variant("Oleg", "angry")).unwrap();
        assert!(check_variant(d.path(), "Anna", &variant("Oleg angry", "x")).unwrap_err().contains("itself a variant"));
        assert!(check_variant(d.path(), "Oleg", &variant("Anna", "x")).unwrap_err().contains("has variants"));
        rename(d.path(), "Oleg", "Oleg K").unwrap();
        assert_eq!(read_card(d.path(), "Oleg angry").unwrap().variant_of, "Oleg K");
        delete(d.path(), "Oleg K").unwrap();
        let left = read_card(d.path(), "Oleg angry").unwrap();
        assert!(left.variant_of.is_empty() && left.variant.is_empty());
    }

    #[test]
    fn rename_keeps_the_word_timings_apart_from_the_card() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "A", "wav");
        write_card(d.path(), "A", &VoiceCard { favorite: true, ..Default::default() }).unwrap();
        write_words(d.path(), "A", &[crate::voices::ref_pick::Word { word: "x".into(), start: 0.0, end: 0.5 }]).unwrap();
        rename(d.path(), "A", "B").unwrap();
        assert!(read_card(d.path(), "B").unwrap().favorite);
        assert_eq!(read_words(d.path(), "B").unwrap().len(), 1);
    }

    #[test]
    fn restoring_brings_back_the_text_kept_with_the_original() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "A", "wav");
        write_transcript(d.path(), "A", "весь текст").unwrap();
        keep_original_text(d.path(), "A").unwrap();
        std::fs::create_dir_all(d.path().join(".originals")).unwrap();
        voice(&d.path().join(".originals"), "A", "mp3");
        write_transcript(d.path(), "A", "кусок").unwrap();
        restore_original(d.path(), "A").unwrap();
        assert_eq!(read_transcript(d.path(), "A").as_deref(), Some("весь текст"));
        assert!(audio_path(d.path(), "A").unwrap().ends_with("A.mp3"));
    }

    #[test]
    fn delete_removes_the_voice_with_its_files() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "A", "wav");
        std::fs::write(d.path().join("A.txt"), "t").unwrap();
        std::fs::write(d.path().join("A.jpg"), b"j").unwrap();
        delete(d.path(), "A").unwrap();
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
        assert!(delete(d.path(), "A").is_err());
    }

    #[test]
    fn a_new_photo_replaces_the_old_one_and_a_wrong_type_is_refused() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "A", "wav");
        set_photo(d.path(), "A", b"1", "PNG").unwrap();
        set_photo(d.path(), "A", b"2", "jpeg").unwrap();
        assert_eq!(photo_path(d.path(), "A").unwrap(), d.path().join("A.jpg"));
        assert!(!d.path().join("A.png").exists());
        assert!(set_photo(d.path(), "A", b"3", "gif").is_err());
        assert!(set_photo(d.path(), "Z", b"3", "png").is_err());
    }

    #[test]
    fn a_pack_transcript_named_after_the_audio_file_is_found() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "RU_Female_Cheburashka", "mp3");
        std::fs::write(d.path().join("RU_Female_Cheburashka.MP3.txt"), "Я Чебурашка").unwrap();
        assert_eq!(read_transcript(d.path(), "RU_Female_Cheburashka").as_deref(), Some("Я Чебурашка"));
    }

    #[test]
    fn a_pack_transcript_in_a_lab_file_is_found_and_follows_the_voice() {
        let d = tempfile::tempdir().unwrap();
        voice(d.path(), "English_Female", "wav");
        std::fs::write(d.path().join("English_Female.lab"), "In the ancient land of Eldoria\n").unwrap();
        assert_eq!(read_transcript(d.path(), "English_Female").as_deref(), Some("In the ancient land of Eldoria"));
        write_transcript(d.path(), "English_Female", "Edited").unwrap();
        assert_eq!(read_transcript(d.path(), "English_Female").as_deref(), Some("Edited"), "the person's text comes first");
        rename(d.path(), "English_Female", "Narrator").unwrap();
        assert!(d.path().join("Narrator.lab").is_file());
        write_transcript(d.path(), "Narrator", "").unwrap();
        assert_eq!(read_transcript(d.path(), "Narrator"), None, "a cleared text does not come back from the pack's file");
    }

    #[test]
    fn an_empty_transcript_removes_the_file() {
        let d = tempfile::tempdir().unwrap();
        write_transcript(d.path(), "A", "  слова  ").unwrap();
        assert_eq!(read_transcript(d.path(), "A").as_deref(), Some("слова"));
        write_transcript(d.path(), "A", " ").unwrap();
        assert!(!d.path().join("A.txt").exists());
        write_transcript(d.path(), "A", "").unwrap();
    }

    #[test]
    fn free_names_count_up() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(free_name(d.path(), "Голос"), "Голос");
        voice(d.path(), "Голос", "wav");
        voice(d.path(), "Голос 2", "mp3");
        assert_eq!(free_name(d.path(), "Голос"), "Голос 3");
    }
}
