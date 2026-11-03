//! Распознавание одного файла выбранным в настройках ASR: Parakeet, а язык, которого он не знает, — Whisper.
//! Общее для текста рефа голоса и проверки озвучки.

use std::path::Path;

use serde_json::json;

use crate::app::localize::Localize;
use crate::voices::ref_pick::Word;

pub struct Heard {
    pub text: String,
    pub words: Vec<Word>,
}

/// Many files at once, each with its language and what it is (named when no recogniser knows the language): a language's
/// files go to its recogniser in one batch (transcribe.cpp runs them in one pass, CrisperWhisper one by one in its server). Every file gets
/// its own answer.
pub fn transcribe_many(
    repo_root: &Path,
    models_root: &Path,
    files: &[(std::path::PathBuf, String, String)],
) -> Result<std::collections::HashMap<std::path::PathBuf, Result<Heard, String>>, String> {
    let mut by_lang: std::collections::BTreeMap<&str, Vec<(&std::path::PathBuf, &str)>> = Default::default();
    for (file, lang, what) in files {
        by_lang
            .entry(if lang == "auto" { "" } else { lang.as_str() })
            .or_default()
            .push((file, what.as_str()));
    }
    let work = tempfile::Builder::new().prefix("asr-many-").tempdir().map_err(|e| e.to_string())?;
    std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(models_root, "asr_backend"));
    let mut out = std::collections::HashMap::new();
    for (lang, group) in by_lang {
        let choice = match choice(repo_root, models_root, lang, group[0].1) {
            Ok(c) => c,
            Err(e) => {
                for (file, _) in group {
                    out.insert(file.clone(), Err(e.clone()));
                }
                continue;
            }
        };
        let mut wavs = Vec::new();
        let mut ready = Vec::new();
        for (k, (file, _)) in group.iter().enumerate() {
            let wav = work.path().join(format!("{lang}-{k}.wav"));
            match crate::audio::media::to_16k_mono(file, &wav) {
                Ok(()) => {
                    wavs.push(wav);
                    ready.push(*file);
                }
                Err(e) => {
                    out.insert((*file).clone(), Err(e));
                }
            }
        }
        let mut engine = match crate::engines::asr::build_engine(&choice, None) {
            Ok(e) => e,
            Err(e) => {
                for file in ready {
                    out.insert(file.clone(), Err(e.clone()));
                }
                continue;
            }
        };
        let heard = engine.transcribe_many_words(&wavs, if lang.is_empty() { "auto" } else { lang });
        for (file, words) in ready.into_iter().zip(heard) {
            let heard = words.map_err(|e| e.localize()).map(|words| Heard {
                text: dub_asr::join_text(words.iter().map(|w| w.word.as_str())),
                words: words
                    .iter()
                    .map(|w| Word {
                        word: w.word.clone(),
                        start: w.start,
                        end: w.end,
                    })
                    .collect(),
            });
            out.insert(file.clone(), heard);
        }
    }
    Ok(out)
}

pub fn transcribe(repo_root: &Path, models_root: &Path, audio: &Path, lang: &str, what: &str) -> Result<Heard, String> {
    let choice = choice(repo_root, models_root, lang, what)?;
    run(&choice, models_root, audio, if lang == "auto" { "" } else { lang })
}

/// The recogniser for `lang`: the one chosen in the settings, Whisper for a language it does not know.
pub fn choice(repo_root: &Path, models_root: &Path, lang: &str, what: &str) -> Result<crate::engines::models::AsrChoice, String> {
    choice_of(repo_root, models_root, lang, what, "")
}

/// The recogniser for `lang` with `engine` (parakeet, nemotron, whisper) in place of the settings' one; empty
/// keeps it. A recogniser that was asked for but is not downloaded, or does not know the language, is an error, not
/// another recogniser; only Parakeet, the default, gives way to Whisper for a language it lacks.
pub fn choice_of(repo_root: &Path, models_root: &Path, lang: &str, what: &str, engine: &str) -> Result<crate::engines::models::AsrChoice, String> {
    let mut sel = crate::engines::models::load_selection(models_root);
    if !engine.is_empty() {
        if let Some(obj) = sel.as_object_mut() {
            obj.insert("asr_engine".into(), json!(engine));
        }
    }
    let asked = sel.get("asr_engine").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
    let choice = crate::engines::models::resolve_asr_choice(repo_root, models_root, &sel);
    if asked == "whisper" && !matches!(choice, crate::engines::models::AsrChoice::CrisperWhisper { .. }) {
        return Err(t!("listen-whisper-missing"));
    }
    let lang = if lang.is_empty() || lang == "auto" { "" } else { lang };
    if !choice.knows(lang) {
        if !matches!(asked.as_str(), "" | "parakeet") {
            let name = match &choice {
                crate::engines::models::AsrChoice::Ggml { model, .. } | crate::engines::models::AsrChoice::Server { model, .. } => model.name,
                crate::engines::models::AsrChoice::CrisperWhisper { .. } => "CrisperWhisper",
            };
            return Err(t!("listen-gguf-lang", engine = name, lang = lang.to_string()));
        }
        if let Some(obj) = sel.as_object_mut() {
            obj.insert("asr_engine".into(), json!("whisper"));
        }
        let whisper = crate::engines::models::resolve_asr_choice(repo_root, models_root, &sel);
        if !matches!(whisper, crate::engines::models::AsrChoice::CrisperWhisper { .. }) {
            return Err(t!("voice-lang-needs-whisper", name = what.to_string(), lang = lang.to_string()));
        }
        return Ok(whisper);
    }
    match &choice {
        crate::engines::models::AsrChoice::Ggml { build, model, .. } if !crate::engines::tcpp_asr::installed(models_root, *build, model) => {
            return Err(t!("listen-gguf-missing", engine = model.name));
        }
        crate::engines::models::AsrChoice::Server { model, .. } if !dub_asr::tcpp::model_path(models_root, model).is_file() => {
            return Err(t!("listen-gguf-missing", engine = model.name));
        }
        _ => {}
    }
    Ok(choice)
}

/// What the recogniser `engine` (parakeet, nemotron, whisper; empty: the settings' one) needs and this computer does
/// not have yet: its model and its engine, with their CUDA libraries when recognition runs on the card.
pub fn missing_parts(repo_root: &Path, models_root: &Path, engine: &str) -> Vec<String> {
    let mut sel = crate::engines::models::load_selection(models_root);
    if !engine.is_empty() {
        if let Some(obj) = sel.as_object_mut() {
            obj.insert("asr_engine".into(), json!(engine));
        }
    }
    let mut ids: Vec<String> = Vec::new();
    if sel.get("asr_engine").and_then(serde_json::Value::as_str) == Some("whisper") {
        // its weights, and the language id (VoxLingua107) with the transcribe.cpp build it runs on for «auto»
        if !crate::engines::acpp::crisperwhisper_installed(models_root) {
            ids.push("crisperwhisper-turbo".to_string());
        }
        ids.extend(["langid".to_string(), crate::engines::models::tcpp_build(models_root).dir().to_string()]);
        if let Ok(build) = crate::engines::acpp::build_for(crate::engines::models::server_choice(models_root, "asr_backend")) {
            ids.push(crate::engines::acpp::component(build).to_string());
            ids.extend(crate::engines::acpp::runtime_component(build).map(str::to_string));
        }
    } else {
        match crate::engines::models::resolve_asr_choice(repo_root, models_root, &sel) {
            crate::engines::models::AsrChoice::Ggml { build, model, .. } => {
                ids.extend([build.dir().to_string(), model.component.to_string()]);
                if matches!(build, crate::engines::tcpp_asr::Build::Cuda) {
                    ids.push("cuda-runtime".to_string());
                }
            }
            // the model, the server build of the recognition device and its CUDA runtime
            crate::engines::models::AsrChoice::Server { model, choice, .. } => {
                ids.push(model.component.to_string());
                if let Ok(build) = crate::engines::acpp::build_for(choice) {
                    ids.push(crate::engines::acpp::component(build).to_string());
                    ids.extend(crate::engines::acpp::runtime_component(build).map(str::to_string));
                }
            }
            crate::engines::models::AsrChoice::CrisperWhisper { .. } => {}
        }
    }
    let manifest = crate::setup::manifest();
    ids.retain(|id| {
        manifest
            .iter()
            .find(|c| c.id == id.as_str())
            .is_some_and(|c| !crate::setup::component_status(repo_root, c).installed)
    });
    ids
}

/// Downloads in the running job what the recogniser `engine` lacks (see `missing_parts`).
pub fn ensure(repo_root: &Path, models_root: &Path, engine: &str, progress: &crate::setup::ProgressCb) -> Result<(), String> {
    let ids = missing_parts(repo_root, models_root, engine);
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    super::engine_setup::ensure_components(repo_root, &ids, progress)
}

/// Whisper when `transcribe` runs Parakeet or Nemotron, Parakeet when it runs Whisper, if that one is on this computer
/// and knows the language: a second opinion on a take its sound made suspect (Game Dub's checks heard a repeat one of
/// them missed). `None` when there is no such model.
pub fn second_opinion(repo_root: &Path, models_root: &Path, audio: &Path, lang: &str) -> Result<Option<Heard>, String> {
    let mut sel = crate::engines::models::load_selection(models_root);
    let first = crate::engines::models::resolve_asr_choice(repo_root, models_root, &sel);
    let lang = if lang.is_empty() || lang == "auto" { "" } else { lang };
    let first_whisper = matches!(first, crate::engines::models::AsrChoice::CrisperWhisper { .. });
    if let Some(obj) = sel.as_object_mut() {
        obj.insert("asr_engine".into(), json!(if first_whisper { "parakeet" } else { "whisper" }));
    }
    let second = crate::engines::models::resolve_asr_choice(repo_root, models_root, &sel);
    if matches!(second, crate::engines::models::AsrChoice::CrisperWhisper { .. }) == first_whisper || !second.knows(lang) {
        return Ok(None);
    }
    run(&second, models_root, audio, lang).map(Some)
}

/// Below this share of the probability in a model's own languages the recording is taken to be in another language.
/// VoxLingua107 mixes close languages (Russian FLEURS clips came out Belarusian 0.60 / Russian 0.40), so a share this
/// size still means the model's language; Chinese gives its share near zero.
const OWN_LANGUAGES_MIN: f32 = 0.1;

/// The language of 16 kHz mono speech by VoxLingua107 (its first 30 s), for a model that knows `known` (its own
/// languages): its likeliest own language while those hold a noticeable share, else the likeliest language a recogniser
/// here can read, so «auto» never routes a recording to a model that cannot read it (a Chinese reference recognised by
/// Parakeet, which does not know it, came back empty and was refused as silent). None for a clip too short to hear.
/// Needs the langid helper and the tcpp build on disk.
pub(crate) fn spoken_language(models_root: &Path, wav: &Path, known: &[&str]) -> Result<Option<String>, String> {
    let (pcm, _) = crate::audio::wavio::read_mono_f32_head(wav, 30 * 16_000)?;
    if pcm.len() < dub_asr::tcpp::LANGID_MIN_SAMPLES {
        return Ok(None);
    }
    let pcm = pcm.as_slice();
    let engine = dub_asr::tcpp::engine_dir(models_root, crate::engines::models::tcpp_build(models_root));
    let model = dub_asr::tcpp::helper_path(models_root, &dub_asr::tcpp::LANGID);
    let own = dub_asr::tcpp::detect_language(&engine, &model, pcm, known)?;
    if own.allowed_mass >= OWN_LANGUAGES_MIN {
        return Ok(own.ranked.into_iter().next().map(|(code, _)| code));
    }
    // Outside the model's own languages the recording goes to CrisperWhisper, so the language must be one it knows:
    // VoxLingua107 also names languages Whisper has no token for (Esperanto, Guarani, Interlingua, …), which the
    // recogniser would then refuse by name. The likeliest language a recogniser here can read is taken instead.
    let all = dub_asr::tcpp::detect_language(&engine, &model, pcm, &[])?;
    Ok(recognisable_language(&all.ranked))
}

/// From language candidates ranked by likelihood, the likeliest one a recogniser here can read (CrisperWhisper's
/// languages); the likeliest overall when none of them can, so a recording in a language no recogniser knows is still
/// named and refused by name rather than mis-sent.
fn recognisable_language(ranked: &[(String, f32)]) -> Option<String> {
    ranked
        .iter()
        .find(|(code, _)| acpp_server::language::whisper_code(code).is_ok())
        .or_else(|| ranked.first())
        .map(|(code, _)| code.clone())
}

fn run(choice: &crate::engines::models::AsrChoice, models_root: &Path, audio: &Path, lang: &str) -> Result<Heard, String> {
    let work = tempfile::Builder::new().prefix("asr-").tempdir().map_err(|e| e.to_string())?;
    let wav = work.path().join("in-16k.wav");
    crate::audio::media::to_16k_mono(audio, &wav)?;
    std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(models_root, "asr_backend"));
    let mut asr = crate::engines::asr::build_engine(choice, None)?;
    let segments = asr.transcribe(&wav, if lang.is_empty() { "auto" } else { lang }).map_err(|e| e.localize())?;
    let text = dub_asr::join_text(segments.iter().map(|s| s.text.as_str()));
    let words = segments.iter().flat_map(|s| s.words.iter()).map(|w| Word { word: w.word.clone(), start: w.start, end: w.end }).collect();
    Ok(Heard { text, words })
}

#[cfg(test)]
mod tests;
