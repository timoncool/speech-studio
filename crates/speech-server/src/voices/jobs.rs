//! Работа над голосом, которая идёт задачей очереди: транскрипт рефа (выбранный локальный ASR) и очистка голоса
//! от музыки и шума (Mel-Band Roformer).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::app::Localize;
use crate::voices::library as lib;

pub struct VoicePaths {
    pub repo_root: PathBuf,
    pub models_root: PathBuf,
    pub voices_dir: PathBuf,
}

fn work_dir(voices: &Path, name: &str, what: &str) -> Result<tempfile::TempDir, String> {
    let base = voices.join(".work");
    std::fs::create_dir_all(&base).map_err(|e| format!("{}: {e}", base.display()))?;
    tempfile::Builder::new().prefix(&format!("{what}-")).tempdir_in(&base).map_err(|e| format!("{name}: {e}"))
}

/// Recognise a voice reference in the right language, with the recogniser that language needs. Under «Auto» the
/// language is the one VoxLingua107 hears, so a reference the default recogniser (Parakeet) does not know — Chinese and
/// the like — goes to CrisperWhisper instead of coming back empty and being refused as silent, and the components of
/// whichever recogniser handles it are fetched before it runs.
fn recognise_reference(p: &VoicePaths, audio: &Path, card_lang: &str, name: &str, progress: &dyn Fn(Value)) -> Result<crate::app::recognition::Heard, String> {
    let lang = reference_language(p, audio, card_lang, progress);
    let engine = match crate::app::recognition::choice_of(&p.repo_root, &p.models_root, &lang, name, "")? {
        crate::engines::models::AsrChoice::CrisperWhisper { .. } => "whisper",
        _ => "",
    };
    crate::app::recognition::ensure(&p.repo_root, &p.models_root, engine, progress)?;
    crate::app::recognition::transcribe(&p.repo_root, &p.models_root, audio, &lang, name)
}

/// The language a reference is recognised in: the one set on its card, or — under «Auto» — the language VoxLingua107
/// hears in it. «auto» when it cannot be heard, or when the selected recogniser (CrisperWhisper) reads every language
/// itself and needs no language id.
fn reference_language(p: &VoicePaths, audio: &Path, card_lang: &str, progress: &dyn Fn(Value)) -> String {
    let lang = card_lang.trim();
    if !lang.is_empty() && lang != "auto" {
        return lang.to_string();
    }
    match heard_reference_language(p, audio, progress) {
        Ok(Some(code)) => code,
        Ok(None) => "auto".to_string(),
        Err(e) => {
            tracing::warn!("voice reference language id: {e}");
            "auto".to_string()
        }
    }
}

fn heard_reference_language(p: &VoicePaths, audio: &Path, progress: &dyn Fn(Value)) -> Result<Option<String>, String> {
    let sel = crate::engines::models::load_selection(&p.models_root);
    // CrisperWhisper reads «auto» itself across its languages, so it needs no language id
    let known: &[&str] = match crate::engines::models::resolve_asr_choice(&p.repo_root, &p.models_root, &sel) {
        crate::engines::models::AsrChoice::Ggml { model, .. } | crate::engines::models::AsrChoice::Server { model, .. } => model.languages,
        crate::engines::models::AsrChoice::CrisperWhisper { .. } => return Ok(None),
    };
    crate::app::ensure_components(&p.repo_root, &[crate::engines::models::tcpp_build(&p.models_root).dir(), "langid"], progress)?;
    let work = work_dir(&p.voices_dir, "langid", "langid")?;
    let wav = work.path().join("ref16k.wav");
    crate::audio::media::to_16k_mono(audio, &wav)?;
    crate::app::recognition::spoken_language(&p.models_root, &wav, known)
}

/// Транскрипт рефа тем ASR, что выбран в настройках; пишет `<имя>.txt` и помечает транскрипт как распознанный.
pub fn transcribe(p: &VoicePaths, name: &str, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let audio = lib::audio_path(&p.voices_dir, name).ok_or_else(|| t!("voice-missing", name = name.to_string()))?;
    let card = lib::read_card(&p.voices_dir, name)?;
    progress(json!({ "type": "progress", "stage": "voice_transcript", "msg": t!("voice-transcribing", name = name.to_string()) }));
    crate::engines::tts_host::make_room();
    crate::app::ensure_job_components(&p.repo_root, &p.models_root, false, false, &|ev| progress(ev))?;
    let heard = recognise_reference(p, &audio, &card.lang, name, &|ev| progress(ev))?;
    let text = heard.text;
    if text.is_empty() {
        return Err(t!("voice-no-speech", name = name.to_string()));
    }
    lib::write_transcript(&p.voices_dir, name, &text)?;
    lib::write_words(&p.voices_dir, name, &heard.words)?;
    let mut card = lib::read_card(&p.voices_dir, name)?;
    card.transcript_auto = true;
    lib::write_card(&p.voices_dir, name, &card)?;
    Ok(json!({ "name": name, "transcript": text }))
}

/// Тайминги слов рефа длиннее `limit`, если их нет: по ним из длинного рефа берётся кусок с его текстом, без них —
/// первые `limit` секунд без текста, а Drama 3 без текста не клонирует. Расшифровку голоса не трогает.
pub fn ensure_words(p: &VoicePaths, name: &str, limit: f64) -> Result<(), String> {
    if lib::read_words(&p.voices_dir, name).is_some() {
        return Ok(());
    }
    let audio = lib::audio_path(&p.voices_dir, name).ok_or_else(|| t!("voice-missing", name = name.to_string()))?;
    if crate::audio::media::duration(&audio)? <= limit + 0.5 {
        return Ok(());
    }
    let card = lib::read_card(&p.voices_dir, name)?;
    // распознавание рядом с целой моделью озвучки не входит и в 24 ГБ
    crate::engines::tts_host::unload();
    crate::app::ensure_job_components(&p.repo_root, &p.models_root, false, false, &|_| {})?;
    let heard = recognise_reference(p, &audio, &card.lang, name, &|_| {})?;
    if heard.words.is_empty() {
        return Err(t!("voice-no-speech", name = name.to_string()));
    }
    lib::write_words(&p.voices_dir, name, &heard.words)
}

/// Голос без музыки и шума: Roformer отделяет вокал, он заменяет звук голоса, исходник остаётся в `.originals`.
pub fn clean(p: &VoicePaths, name: &str, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let audio = lib::audio_path(&p.voices_dir, name).ok_or_else(|| t!("voice-missing", name = name.to_string()))?;
    crate::engines::tts_host::make_room();
    crate::app::ensure_job_components(&p.repo_root, &p.models_root, false, false, &|ev| progress(ev))?;
    crate::app::ensure_components(&p.repo_root, &crate::engines::models::sep_components(&p.models_root)?, &|ev| progress(ev))?;
    let sel = crate::engines::models::load_selection(&p.models_root);
    let cli = crate::engines::models::sep_cli(&p.repo_root, &p.models_root)?;
    let model = crate::engines::models::resolve_sep(&p.models_root, &sel);
    if !cli.is_file() || !model.is_file() {
        return Err(t!("voice-clean-missing"));
    }
    let work = work_dir(&p.voices_dir, name, "clean")?;
    let wav = work.path().join("mix.wav");
    progress(json!({ "type": "progress", "stage": "voice_clean", "msg": t!("voice-cleaning", name = name.to_string()) }));
    // the separator takes 44.1 kHz only and refuses a voice recorded at 48 or 24 kHz
    crate::audio::media::extract_audio(&audio, &wav, 44_100, 2)?;
    let stems = work.path().join("stems");
    tracing::info!("separation: {} ({})", cli.display(), crate::engines::models::server_choice(&p.models_root, "sep_backend"));
    let res = dub_sep::separate(&wav, &stems, &cli, &model).map_err(|e| e.localize())?;
    lib::replace_with_cleaned(&p.voices_dir, name, &res.vocals)?;
    Ok(json!({ "name": name, "cleaned": true }))
}

/// Голос без шума (DPDFNet): `limit_db` — предел подавления, `None` — без предела. Исходник остаётся в `.originals`.
pub fn denoise(p: &VoicePaths, name: &str, limit_db: Option<f32>, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let audio = lib::audio_path(&p.voices_dir, name).ok_or_else(|| t!("voice-missing", name = name.to_string()))?;
    crate::app::ensure_components(&p.repo_root, &["dpdfnet"], &|ev| progress(ev))?;
    let work = work_dir(&p.voices_dir, name, "denoise")?;
    let out = work.path().join("denoised.wav");
    let msg = t!("voice-denoising", name = name.to_string());
    crate::engines::denoise::denoise_file(&p.models_root, &audio, &out, limit_db, &|f| progress(json!({ "type": "progress", "stage": "voice_clean", "msg": msg, "pct": (f * 100.0) as i64 })))?;
    lib::replace_with_cleaned(&p.voices_dir, name, &out)?;
    Ok(json!({ "name": name, "denoised": true }))
}

/// Сколько кандидатов голоса по описанию за раз.
pub const DESIGN_MAX: u32 = 6;

/// Папка пачки кандидатов дизайна голоса.
pub fn design_dir(voices: &Path, id: &str) -> PathBuf {
    voices.join(".design").join(id)
}

/// Кандидаты голоса по описанию (Qwen3-TTS VoiceDesign): образец текста озвучен `count` раз разными сидами в
/// `.design/<ид>/<k>.wav`; ответ — ид пачки и длительность каждого кандидата.
pub fn design(p: &VoicePaths, description: &str, sample: &str, lang: &str, count: u32, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let quant = crate::engines::models::qwen_quant(&p.models_root);
    let model = if quant == "Q8_0" { "qwen3-tts-design-q8_0" } else { "qwen3-tts-design" };
    // every build brings the codec with it
    let mut parts = vec![model];
    parts.extend(crate::engines::qwen_tts::build_components(&p.models_root)?);
    crate::app::ensure_components(&p.repo_root, &parts, &|ev| progress(ev))?;
    crate::engines::tts_host::make_room();
    let engine = crate::engines::tts_host::qwen_design(&p.models_root)?;
    drop_old_designs(&p.voices_dir);
    let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
    let dir = design_dir(&p.voices_dir, &id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let lang = if lang == "auto" { "" } else { lang };
    let mut params = crate::engines::tts_host::Params { seed_mode: "random".into(), ..Default::default() };
    params.temperature = 0.9;
    let mut out = Vec::new();
    for k in 0..count.clamp(1, DESIGN_MAX) {
        progress(json!({ "type": "progress", "stage": "voice_design", "msg": t!("voice-designing", n = (k + 1) as i64, total = count as i64), "pct": (k * 100 / count.max(1)) as i64 }));
        let req = crate::engines::tts_host::Request {
            text: sample,
            lang,
            ref_wav: None,
            ref_text: None,
            preset: None,
            instruct: Some(description),
            params: &params,
            fresh_seed: u64::from_str_radix(&id, 16).unwrap_or(0).wrapping_add(k as u64),
            live: None,
        };
        let s = crate::engines::tts_host::synth(&engine, &req)?;
        let file = dir.join(format!("{k}.wav"));
        crate::audio::wavio::write_mono_f32(&file, &s.samples, s.sr as u32)?;
        out.push(json!({ "k": k, "seconds": s.samples.len() as f64 / s.sr.max(1) as f64, "seed": s.seed }));
    }
    Ok(json!({ "id": id, "candidates": out }))
}

/// Кандидаты прошлых дизайнов старше суток: их уже не сохраняют, а место они занимают в папке голосов.
fn drop_old_designs(voices: &Path) {
    let Ok(rd) = std::fs::read_dir(voices.join(".design")) else { return };
    for e in rd.flatten() {
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_secs() > 24 * 3600);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Кандидат дизайна — голос библиотеки: звук с громкостью рефа, текст образца — его транскрипт, описание — источник.
pub fn design_keep(p: &VoicePaths, id: &str, k: u32, name: &str, description: &str, sample: &str, lang: &str) -> Result<String, String> {
    if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.is_empty() {
        return Err(t!("voice-design-gone"));
    }
    let file = design_dir(&p.voices_dir, id).join(format!("{k}.wav"));
    if !file.is_file() {
        return Err(t!("voice-design-gone"));
    }
    let card = lib::VoiceCard { kind: "design".into(), source: description.trim().to_string(), lang: if lang == "auto" { String::new() } else { lang.to_string() }, ..Default::default() };
    let made = lib::import_audio(&p.voices_dir, &lib::sanitize(name), &file, card)?;
    lib::write_transcript(&p.voices_dir, &made, sample.trim())?;
    Ok(made)
}

/// Можно ли чистить голос сейчас: движок и модель Roformer на месте.
pub fn clean_ready(p: &VoicePaths) -> bool {
    let sel = crate::engines::models::load_selection(&p.models_root);
    crate::engines::models::sep_cli(&p.repo_root, &p.models_root).is_ok_and(|cli| cli.is_file())
        && crate::engines::models::resolve_sep(&p.models_root, &sel).is_file()
}

/// Куски рефа по словам распознавания, не длиннее лимита рефа движка (настройка `higgs_ref_secs`, 12 с).
pub fn ref_candidates(p: &VoicePaths, name: &str) -> Result<Value, String> {
    let audio = lib::audio_path(&p.voices_dir, name).ok_or_else(|| t!("voice-missing", name = name.to_string()))?;
    let words = lib::read_words(&p.voices_dir, name).ok_or_else(|| t!("voice-no-words", name = name.to_string()))?;
    let sel = crate::engines::models::load_selection(&p.models_root);
    let max_len = sel.get("higgs_ref_secs").and_then(Value::as_str).and_then(|v| v.parse::<f64>().ok()).unwrap_or(12.0);
    let work = work_dir(&p.voices_dir, name, "pick")?;
    let wav = work.path().join("ref-16k.wav");
    crate::audio::media::to_16k_mono(&audio, &wav)?;
    let (samples, sr) = crate::audio::wavio::read_mono_f32(&wav)?;
    let found = crate::voices::ref_pick::candidates(&words, &samples, sr, (max_len * 0.4).min(5.0), max_len, 5);
    Ok(json!({ "name": name, "max_len": max_len, "duration": samples.len() as f64 / sr.max(1) as f64, "candidates": found }))
}
