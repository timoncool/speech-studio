//! Задачи Speech: синтез блоков и экспорт. Синтез перечитывает проект перед каждым блоком и пишет дубль сразу —
//! правки окна и агента во время озвучки не теряются, а окно видит блоки готовыми по одному. Сырой выход модели —
//! FLAC в `takes/`. Экспорт склеивает выбранные дубли с паузами (отрицательная — перекрытие), ставит громкость
//! пресета в два прохода и кодирует в нужный формат; блоки без звука или устаревшие экспорт не пропускает.

use super::naming::{name_keys, EXPORT_NAME_KEYS};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::speech::{project as sp, text as stx};
use crate::speech::project::{Block, SpeechProject, Take};
use crate::speech::text::Piece;
use crate::engines::tts_host;
use super::preparation::{self, opts as prep_opts};

pub const SR: u32 = 24_000;

#[derive(Clone)]
pub struct SpeechPaths {
    pub repo_root: PathBuf,
    pub workspace: PathBuf,
    pub voices_dir: PathBuf,
    pub models_root: PathBuf,
}

/// Loads what voicing `engine` with `voice` in `lang` takes before its first phrase: the engine as that phrase will
/// use it (a clone or the engine's own voice) and, for Russian, the stress model.
/// A phrase said aloud goes into the timings: its engine and voice, the wait the person heard, the engine's time.
fn time_aloud(live: &crate::speech::live::Phrase, take: &Take) {
    if live.is_aloud() {
        crate::app::note("tts", json!({
            "block": live.block(), "engine": take.engine, "voice": take.voice, "chars": take.text.chars().count(),
            "heard_ms": take.heard_ms, "first_ms": take.first_ms, "synth_ms": (take.synth_secs * 1000.0).round() as u64,
            "audio_secs": (take.duration * 100.0).round() / 100.0,
        }));
    }
}

/// Loads `engine` (and the stress model for Russian) ahead of its first phrase; how long it took goes into the timings.
pub fn warm(p: &SpeechPaths, engine: &str, voice: Option<&str>, lang: &str) -> Result<(), String> {
    let started = std::time::Instant::now();
    let warmed = load(p, engine, voice, lang);
    crate::app::note("load", json!({ "engine": engine, "voice": voice, "ms": started.elapsed().as_millis() as u64, "error": warmed.as_ref().err() }));
    warmed
}

fn load(p: &SpeechPaths, engine: &str, voice: Option<&str>, lang: &str) -> Result<(), String> {
    let clones = voice.is_some_and(|v| crate::voices::audio_path(&p.voices_dir, v).is_some());
    if engine == crate::engines::models::DRAMA_TTS {
        tts_host::drama(&p.models_root)?;
    } else if engine == crate::engines::supertonic::ENGINE {
        tts_host::supertonic(&p.repo_root, &p.models_root)?;
    } else if crate::engines::qwen_tts::is_engine(engine) {
        tts_host::qwen(&p.models_root, clones, engine == crate::engines::qwen_tts::ENGINE_STRESS)?;
    } else if engine == "higgs" {
        tts_host::higgs(&p.repo_root, &p.models_root)?;
    }
    if matches!(lang, "ru" | "auto" | "") && crate::engines::stress::installed(&p.repo_root) {
        crate::engines::stress::shared(&p.models_root)?;
    }
    Ok(())
}

/// The Higgs quantization the settings pick.
pub fn higgs_quant(p: &SpeechPaths) -> String {
    crate::engines::models::resolve_tts(&p.models_root, &crate::engines::models::load_selection(&p.models_root)).1
}

pub fn ref_limit(models_root: &Path) -> f64 {
    crate::engines::models::load_selection(models_root).get("higgs_ref_secs").and_then(Value::as_str).and_then(|v| v.parse::<f64>().ok()).unwrap_or(12.0)
}

/// The voice, when it is one of the engine's own voices rather than a library voice.
fn preset_of<'a>(engine: &str, voice: Option<&'a str>) -> Option<&'a str> {
    let voice = voice?;
    crate::engines::get(engine).filter(|e| e.presets.iter().any(|p| p.id == voice)).map(|_| voice)
}

/// Ключи синтеза всех блоков проекта при нынешних голосах и настройках.
pub fn keys(p: &SpeechPaths, project: &SpeechProject) -> BTreeMap<String, String> {
    let limit = ref_limit(&p.models_root);
    let entries = dictionary(p, project);
    let compiled = crate::text::prep::compile_all(&entries);
    let mut tags: BTreeMap<(String, Option<String>), String> = BTreeMap::new();
    let mut engines: BTreeMap<String, String> = BTreeMap::new();
    project
        .blocks()
        .map(|b| {
            let voice = project.voice_of(b);
            let own = project.engine_of(b);
            let engine = engines.entry(own.to_string()).or_insert_with(|| engine_tag(p, project, own)).clone();
            let tag = tags
                .entry((own.to_string(), voice.clone()))
                .or_insert_with(|| match preset_of(own, voice.as_deref()) {
                    Some(preset) => format!("preset:{preset}"),
                    None => crate::speech::voice_tag(&p.voices_dir, voice.as_deref()),
                })
                .clone();
            let prep = preparation::fingerprint(&b.text, &prep_opts(p, project, project.lang_of(b), own, voice.as_deref(), &entries, project.settings.auto_stress), &compiled);
            (b.id.clone(), sp::synth_key(project, b, &tag, &engine, limit, &prep))
        })
        .collect()
}

/// Renames a library voice (`rename` moves its files) and the Speech projects that speak with it follow: they name
/// the new voice, and a take made with the old name stays the block's voiced take, the voice being the same sound.
pub fn rename_voice(p: &SpeechPaths, from: &str, to: &str, rename: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    let mut using: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    for entry in std::fs::read_dir(&p.workspace).into_iter().flatten().flatten() {
        let pid = entry.file_name().to_string_lossy().into_owned();
        if !crate::speech::project_path(&p.workspace, &pid).is_file() {
            continue;
        }
        let Ok(project) = crate::speech::load(&p.workspace, &pid) else { continue };
        if project.clone().rename_voice(from, to) {
            using.push((pid, keys(p, &project)));
        }
    }
    rename()?;
    for (pid, before) in using {
        crate::speech::edit(&p.workspace, &pid, |project| {
            project.rename_voice(from, to);
            let after = keys(p, project);
            for b in project.chapters.iter_mut().flat_map(|c| c.blocks.iter_mut()) {
                if let (Some(old), Some(new)) = (before.get(&b.id), after.get(&b.id)) {
                    b.takes.iter_mut().filter(|t| &t.key == old).for_each(|t| t.key = new.clone());
                }
            }
        })?;
    }
    Ok(())
}

/// Что за движок и модель озвучивают блок — часть ключа дубля.
pub(crate) fn engine_tag(p: &SpeechPaths, project: &SpeechProject, engine: &str) -> String {
    if engine == "openrouter" {
        let (m, v) = crate::app::voice_synthesis::model_and_voice(&p.models_root, &project.settings.cloud_model, &project.settings.cloud_voice).unwrap_or_default();
        format!("openrouter:{m}:{v}")
    } else if engine == "google" {
        let g = crate::app::google::setup(&p.models_root, &project.settings.cloud_model, &project.settings.cloud_voice);
        g.map(|g| format!("google:{}:{}", g.model, g.voice)).unwrap_or_else(|_| "google".into())
    } else if matches!(engine, "fal" | "runware") {
        let values = serde_json::to_string(&project.settings.cloud_values).unwrap_or_default();
        format!("{}:{}:{}", engine, project.settings.cloud_model, &blake3::hash(values.as_bytes()).to_hex()[..10])
    } else if engine == crate::engines::models::DRAMA_TTS {
        crate::engines::models::DRAMA_TTS.to_string()
    } else if engine == crate::engines::supertonic::ENGINE {
        format!("{}:int8", crate::engines::supertonic::ENGINE)
    } else if crate::engines::qwen_tts::is_engine(&engine) {
        format!("{}:{}", engine, crate::engines::models::qwen_quant(&p.models_root))
    } else {
        format!("higgs:{}", higgs_quant(p))
    }
}

/// Записи словаря для проекта сверху вниз: его собственные, затем глобальные.
pub fn dictionary(p: &SpeechPaths, project: &SpeechProject) -> Vec<crate::text::pron_dict::Entry> {
    let mut all = project.dictionary.clone();
    all.extend(crate::text::pron_dict::load_global(&p.repo_root));
    all
}

fn takes_dir(p: &SpeechPaths, pid: &str) -> PathBuf {
    p.workspace.join(pid).join("takes")
}

fn seed_for(block: &str, n: u32) -> u64 {
    let h = blake3::hash(format!("{block}|{n}|{}", sp::now()).as_bytes());
    u64::from_le_bytes(h.as_bytes()[..8].try_into().expect("8 bytes"))
}

fn resample_to(samples: Vec<f32>, sr: i32) -> Vec<f32> {
    if sr as u32 == SR || sr <= 0 {
        return samples;
    }
    let ratio = SR as f64 / sr as f64;
    let n = (samples.len() as f64 * ratio) as usize;
    (0..n)
        .map(|i| {
            let x = i as f64 / ratio;
            let a = x.floor() as usize;
            let f = (x - a as f64) as f32;
            let s0 = samples.get(a).copied().unwrap_or(0.0);
            let s1 = samples.get(a + 1).copied().unwrap_or(s0);
            s0 + (s1 - s0) * f
        })
        .collect()
}

/// 10 ms fade-in and fade-out at a piece's edges (Dub's `timeline`): no click where pieces and silences join.
fn fade_edges(samples: &mut [f32], sr: u32) {
    let n = ((sr as usize) / 100).min(samples.len() / 2);
    let len = samples.len();
    for k in 0..n {
        let f = k as f32 / n as f32;
        samples[k] *= f;
        samples[len - 1 - k] *= f;
    }
}

/// A soft breath of 0.18–0.26 s (Dub's `generate_breath_sample`), at `level` RMS of the speech it comes before.
fn breath(sr: u32, seed: usize, level: f32) -> Vec<f32> {
    let n = ((0.18 + (seed % 5) as f64 * 0.02) * sr as f64) as usize;
    let mut state: u32 = (seed as u32).wrapping_add(12345);
    let (mut lp, mut hp) = (0.0f32, 0.0f32);
    (0..n)
        .map(|i| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = ((state >> 9) as f32 / 8388608.0) - 1.0;
            lp += 0.35 * (noise - lp);
            hp += 0.12 * (lp - hp);
            let at = i as f32 / n as f32;
            let env = if at < 0.35 { (at / 0.35).powf(1.5) } else { ((1.0 - at) / 0.65).powf(1.2) };
            (lp - hp) * env * 0.75 * level
        })
        .collect()
}

/// Звук одного блока: фразы синтезом, паузы тишиной. Возвращает сэмплы (SR), что разметки проигнорировано и сводку.
/// Блок облаком: фраза за фразой, паузы и темп — как у Higgs. OpenRouter — своим клиентом, fal и Runware — через
/// каталог (`cloud`), с ручкой задачи на диске до ответа и ценой в журнале расходов.
fn synth_block_cloud(p: &SpeechPaths, pid: &str, project: &SpeechProject, b: &Block, n: u32) -> Result<(Vec<f32>, Value, Take), String> {
    let provider = project.engine_of(b);
    let google = if provider == "google" { Some(crate::app::google::setup(&p.models_root, &project.settings.cloud_model, &project.settings.cloud_voice)?) } else { None };
    let (model, voice) = if let Some(g) = &google {
        (g.model.clone(), g.voice.clone())
    } else if provider == "openrouter" {
        crate::app::voice_synthesis::model_and_voice(&p.models_root, &project.settings.cloud_model, &project.settings.cloud_voice)?
    } else {
        let model = project.settings.cloud_model.trim().to_string();
        if model.is_empty() {
            return Err(t!("cloud-no-model-chosen", provider = provider.to_string()));
        }
        let voice = project.settings.cloud_values.iter().find(|(k, _)| k.to_lowercase().contains("voice")).and_then(|(_, v)| v.as_str()).unwrap_or_default().to_string();
        (model, voice)
    };
    let entries = dictionary(p, project);
    let stress = project.settings.auto_stress && crate::engines::stress::is_russian(&b.text, project.lang_of(b));
    let prepared = preparation::prepare(&b.text, &prep_opts(p, project, project.lang_of(b), project.engine_of(b), Some(voice.as_str()), &entries, stress)).map_err(|e| t!("stress-failed", detail = e))?;
    let (mut out, mut rate, started) = (Vec::new(), 1.0f64, std::time::Instant::now());
    let (mut cost, mut estimate) = (None::<f64>, None::<f64>);
    let live = crate::speech::live::Phrase::begin(pid, &b.id);
    for (k, piece) in stx::pieces(&prepared.text).into_iter().enumerate() {
        match piece {
            Piece::Pause(ms) => {
                live.pause(ms, SR);
                out.extend(std::iter::repeat_n(0.0, (SR as u64 * ms as u64 / 1000) as usize))
            }
            Piece::Rate(r) => rate = r,
            Piece::Say(text) => {
                let say = stx::visible(&text);
                if say.trim().is_empty() {
                    continue;
                }
                let samples = if let Some(g) = &google {
                    let (samples, est) = crate::app::google::phrase(&p.repo_root, &p.models_root, &p.workspace.join(pid), pid, g, &say, b.direction.trim(), n)?;
                    estimate = match (estimate, est) { (Some(a), Some(b)) => Some(a + b), (a, b) => a.or(b) };
                    samples
                } else if provider == "openrouter" {
                    crate::app::costs::check_limit(&p.repo_root, &p.models_root, None)?;
                    let samples = crate::app::voice_synthesis::synth(&say, &model, &voice, b.direction.trim())?;
                    crate::app::costs::record(&p.repo_root, crate::app::costs::Spent { at: chrono::Utc::now().timestamp_millis(), day: chrono::Local::now().format("%Y-%m-%d").to_string(), project: pid.to_string(), provider: "openrouter".into(), model: model.clone(), task: "speech".into(), cost_source: "unknown".into(), origin: crate::app::current_actor(), ..Default::default() });
                    samples
                } else {
                    let handle = p.workspace.join(pid).join("cloud").join(format!("{}-{n}-{k}.json", b.id));
                    let run = crate::app::remote::Run { repo_root: &p.repo_root, models_root: &p.models_root, project: pid, task: vs_providers::Task::Speech, handle_file: &handle };
                    let (samples, done) = crate::app::remote::speak(&run, provider, &model, &project.settings.cloud_values, &say)?;
                    cost = match (cost, done.cost) { (Some(a), Some(b)) => Some(a + b), (a, b) => a.or(b) };
                    estimate = match (estimate, done.estimate) { (Some(a), Some(b)) => Some(a + b), (a, b) => a.or(b) };
                    samples
                };
                let samples = if (rate - 1.0).abs() > 1e-3 { at_rate(&samples, rate)? } else { samples };
                live.chunk(&samples, SR);
                out.extend(samples);
            }
        }
    }
    if out.is_empty() {
        return Err(t!("speech-block-silent"));
    }
    let fitted = fit_to_cue(project, b, out)?;
    let take = Take {
        n,
        created: sp::now(),
        source: "cloud".into(),
        synth_secs: started.elapsed().as_secs_f64(),
        duration: fitted.samples.len() as f64 / SR as f64,
        tidy: Some(crate::audio::trim::tidy(&fitted.samples, SR).samples.len() as f64 / SR as f64),
        text: b.text.clone(),
        direction: b.direction.clone(),
        voice: voice.clone(),
        engine: format!("{provider}:{model}"),
        fit_stretch: fitted.stretch,
        fit_lead: fitted.lead,
        fit_over: fitted.over,
        cost,
        estimate,
        first_ms: live.first_ms(),
        heard_ms: live.heard_ms(),
        ..Default::default()
    };
    time_aloud(&live, &take);
    live.end(true);
    Ok((fitted.samples, serde_json::to_value(&prepared.ignored).unwrap_or(Value::Null), take))
}

/// Gemini: фразы всех блоков задачи, которые будут озвучены, заранее в кэш — обычными запросами в несколько потоков
/// или пачкой, по режиму; синтез блоков берёт звук оттуда. Фразы собираются тем же разбором, что и в
/// `synth_block_cloud`, с номерами дублей, которые получат блоки (`takes` на блок при нескольких дублях).
fn google_prefetch(p: &SpeechPaths, pid: &str, ids: &[String], force: bool, takes: u32, progress: &dyn Fn(Value)) -> Result<(), String> {
    let project = crate::speech::load(&p.workspace, pid)?;
    let g = crate::app::google::setup(&p.models_root, &project.settings.cloud_model, &project.settings.cloud_voice)?;
    let entries = dictionary(p, &project);
    let mut keys = keys(p, &project);
    let mut phrases = Vec::new();
    for b in project.blocks().filter(|b| ids.contains(&b.id) && project.engine_of(b) == "google") {
        let state = sp::status(b, &keys.remove(&b.id).unwrap_or_default());
        if state == "skipped" || (!force && state == "ready") {
            continue;
        }
        let stress = project.settings.auto_stress && crate::engines::stress::is_russian(&b.text, project.lang_of(b));
        let prepared = preparation::prepare(&b.text, &prep_opts(p, &project, project.lang_of(b), project.engine_of(b), Some(g.voice.as_str()), &entries, stress)).map_err(|e| t!("stress-failed", detail = e))?;
        let first = b.takes.iter().map(|t| t.n).max().unwrap_or(0) + 1;
        for piece in stx::pieces(&prepared.text) {
            if let Piece::Say(text) = piece {
                let say = stx::visible(&text);
                if !say.trim().is_empty() {
                    for n in first..first + takes.max(1) {
                        phrases.push(crate::app::google::Want { text: say.clone(), style: b.direction.trim().to_string(), n });
                    }
                }
            }
        }
    }
    if phrases.is_empty() {
        return Ok(());
    }
    crate::app::google::prefetch(&p.repo_root, &p.models_root, &p.workspace.join(pid), pid, &g, &phrases, progress).map(|_| ())
}

fn synth_block(p: &SpeechPaths, pid: &str, project: &SpeechProject, b: &Block, n: u32) -> Result<(Vec<f32>, Value, Take), String> {
    if crate::engines::is_cloud(project.engine_of(b)) {
        return synth_block_cloud(p, pid, project, b, n);
    }
    synth_block_with(p, pid, project, b, n, &dictionary(p, project))
}

/// A block voiced locally with these dictionary rules.
fn synth_block_with(p: &SpeechPaths, pid: &str, project: &SpeechProject, b: &Block, n: u32, entries: &[crate::text::pron_dict::Entry]) -> Result<(Vec<f32>, Value, Take), String> {
    let own = project.engine_of(b);
    let drama = own == crate::engines::models::DRAMA_TTS;
    let manifest = crate::engines::get(own);
    let presets: Vec<&str> = manifest.as_ref().map(|m| m.presets.iter().map(|p| p.id).collect()).unwrap_or_default();
    let voice = project.voice_of(b);
    if drama && voice.is_none() {
        return Err(t!("drama-needs-voice"));
    }
    if own == crate::engines::qwen_tts::ENGINE_STRESS && voice.is_none() {
        return Err(t!("render-qwen-stress-needs-voice"));
    }
    // an engine with its own voices speaks one of them; a library voice there is cloned when the engine clones,
    // otherwise named, not quietly swapped
    let clones_too = manifest.as_ref().is_some_and(|m| m.clone != "none");
    let preset = match (presets.is_empty(), voice.as_deref()) {
        (true, _) => None,
        (false, None) => presets.first().map(|v| v.to_string()),
        (false, Some(v)) if presets.contains(&v) => Some(v.to_string()),
        (false, Some(_)) if clones_too => None,
        (false, Some(v)) => return Err(t!("speech-voice-not-preset", voice = v.to_string(), engine = manifest.as_ref().map_or("", |m| m.name).to_string())),
    };
    let limit = if own == crate::engines::qwen_tts::ENGINE_STRESS { ref_limit(&p.models_root).min(crate::engines::qwen_tts::STRESS_REF_SECS) } else { ref_limit(&p.models_root) };
    let clones = preset.is_none();
    let continues = manifest.as_ref().is_some_and(|m| m.continuation);
    let own_ref = b.own_ref.as_ref().filter(|r| clones && !r.file.is_empty());
    let continued = sp::continuation_source(project, b).filter(|(_, t)| own_ref.is_none() && clones && continues && t.duration > 0.5 && t.duration <= limit + 0.5);
    if let (None, None, Some(v)) = (own_ref, &continued, voice.as_deref().filter(|_| clones)) {
        let vp = crate::voices::VoicePaths { repo_root: p.repo_root.clone(), models_root: p.models_root.clone(), voices_dir: p.voices_dir.clone() };
        // a reference without its text is heard here, as «Recognise the reference text» would do
        if crate::voices::read_transcript(&p.voices_dir, v).is_none_or(|t| t.trim().is_empty()) {
            crate::voices::transcribe(&vp, v, &|_| {})?;
        }
        crate::voices::ensure_words(&vp, v, limit)?;
    }
    let reference = match (own_ref, continued, voice.as_deref().filter(|_| clones)) {
        (Some(r), _, _) => Some(own_reference(&p.workspace.join(pid), r, limit)?),
        (None, Some((src, take)), _) => Some(continuation_ref(&takes_dir(p, pid), take, &src.text)?),
        (None, None, Some(v)) => Some(crate::voices::prepare_reference(&p.voices_dir, v, limit, n)?),
        (None, None, None) => None,
    };
    if drama && reference.as_ref().is_some_and(|(_, text)| text.as_deref().is_none_or(|t| t.trim().is_empty())) {
        return Err(match voice.as_deref().filter(|_| own_ref.is_none()) {
            Some(v) if crate::voices::read_transcript(&p.voices_dir, v).is_some() => t!("drama-voice-no-piece", name = v.to_string(), secs = format!("{limit:.0}")),
            Some(v) => t!("drama-voice-no-text", name = v.to_string()),
            None => t!("render-drama-no-ref-text"),
        });
    }
    let engine = if drama {
        tts_host::drama(&p.models_root)?
    } else if own == crate::engines::supertonic::ENGINE {
        tts_host::supertonic(&p.repo_root, &p.models_root)?
    } else if crate::engines::qwen_tts::is_engine(own) {
        tts_host::qwen(&p.models_root, preset.is_none(), own == crate::engines::qwen_tts::ENGINE_STRESS)?
    } else {
        tts_host::higgs(&p.repo_root, &p.models_root)?
    };
    let lang = match project.lang_of(b) {
        "auto" => "",
        own => own,
    };
    let mut ignored = stx::Ignored::default();
    let (mut secs, mut retried) = (0.0, Vec::<String>::new());
    let fresh = seed_for(&b.id, n);
    let mut params = take_params(project.params_of(b), n);
    if let Some(seed) = REPEAT_SEED.with(|s| s.get()) {
        params.seed_mode = "fixed".into();
        params.seed = seed % tts_host::SEED_RANGE;
    }
    let stress = project.settings.auto_stress && crate::engines::stress::is_russian(&b.text, project.lang_of(b));
    let text = preparation::prepare(&b.text, &prep_opts(p, project, project.lang_of(b), own, voice.as_deref(), entries, stress)).map_err(|e| t!("stress-failed", detail = e))?.text;
    let max_chars = manifest.as_ref().map_or(usize::MAX, |m| m.max_chars.max(1));
    let speeds = manifest.as_ref().is_some_and(|m| m.params.iter().any(|s| s.key == "speed"));
    if let Some(ahead) = tempo_ahead(project, b).filter(|_| speeds) {
        params.speed = (params.speed * ahead).min(SPEED_MAX);
    }
    let live = std::sync::Arc::new(crate::speech::live::Phrase::begin(pid, &b.id));
    let mut render = |params: &tts_host::Params, salt: u64, live: Option<&std::sync::Arc<crate::speech::live::Phrase>>| -> Result<(Vec<f32>, u64, f64), String> {
        let mut out: Vec<f32> = Vec::new();
        let (mut seed, mut temperature) = (0u64, params.temperature);
        let mut rate = 1.0f64;
        for (k, piece) in stx::pieces(&text).into_iter().enumerate() {
            match piece {
                Piece::Pause(ms) => {
                    if let Some(l) = live {
                        l.pause(ms, SR);
                    }
                    out.extend(std::iter::repeat_n(0.0, (SR as u64 * ms as u64 / 1000) as usize))
                }
                Piece::Rate(r) => rate = r,
                Piece::Say(text) => {
                    let retimed = (rate - 1.0).abs() > 1e-3;
                    let (say, ig) = stx::for_engine(&text, lang, own);
                    ignored.accent |= ig.accent;
                    ignored.phonemes |= ig.phonemes;
                    ignored.stress |= ig.stress;
                    if say.is_empty() {
                        continue;
                    }
                    let cut = stx::phrases(&say, max_chars);
                    for (j, (phrase, gap)) in cut.iter().enumerate() {
                        // a cut between words is no pause: the pieces meet with a crossfade instead of a gap
                        let joined = *gap == stx::WORD_GAP_MS;
                        let joins_next = cut.get(j + 1).is_some_and(|(_, g)| *g == stx::WORD_GAP_MS);
                        if *gap > 0 && !joined {
                            if let Some(l) = live {
                                l.pause(*gap, SR);
                            }
                            out.extend(std::iter::repeat_n(0.0, (SR * gap / 1000) as usize));
                        }
                        // Drama 3 читает указание стилевой приставкой строки: `[тихо и устало] текст`
                        let styled = Some(b.direction.trim()).filter(|d| drama && !d.is_empty()).map(|d| format!("[{d}] {phrase}"));
                        let req = tts_host::Request {
                            text: styled.as_deref().unwrap_or(phrase),
                            lang,
                            ref_wav: reference.as_ref().map(|(w, _)| w.as_path()),
                            ref_text: reference.as_ref().and_then(|(_, t)| t.as_deref()),
                            preset: preset.as_deref(),
                            instruct: Some(b.direction.trim()).filter(|d| !d.is_empty() && crate::engines::qwen_tts::is_engine(own)),
                            params,
                            fresh_seed: fresh.wrapping_add(salt).wrapping_add(k as u64).wrapping_add((j as u64) << 32),
                            live: live.filter(|_| !retimed).cloned(),
                        };
                        let s = tts_host::synth(&engine, &req)?;
                        if (k == 0 && j == 0) || seed == 0 {
                            seed = s.seed;
                            temperature = s.temperature;
                        }
                        secs += s.seconds;
                        retried.extend(s.defects.iter().map(|d| d.to_string()));
                        let mut samples = resample_to(s.samples, s.sr);
                        if joined || joins_next {
                            samples = crate::audio::trim::trim_edges(&samples, SR).0;
                        } else {
                            fade_edges(&mut samples, SR);
                        }
                        let samples = if retimed { at_rate(&samples, rate)? } else { samples };
                        if retimed {
                            if let Some(l) = live {
                                l.chunk(&samples, SR);
                            }
                        }
                        if joined {
                            crate::audio::trim::crossfade_join(&mut out, &samples, SR);
                        } else {
                            out.extend(samples);
                        }
                    }
                }
            }
        }
        Ok((out, seed, temperature))
    };
    let (out, mut seed, mut temperature) = render(&params, 0, Some(&live))?;
    if out.is_empty() {
        return Err(t!("speech-block-silent"));
    }
    let mut fitted = fit_to_cue(project, b, out)?;
    if fitted.over {
        // the fitplan ladder of Dub: once more, faster while the engine's speed has room, else another seed
        let mut again = params.clone();
        if speeds && params.speed < SPEED_MAX {
            again.speed = (params.speed * fitted.needed / fitted.stretch.max(1.0)).min(SPEED_MAX);
        } else {
            again.seed_mode = "random".into();
        }
        let (second, s2, t2) = render(&again, FIT_RETRY_SALT, None)?;
        retried.push("fit".to_string());
        let second = fit_to_cue(project, b, second)?;
        if !second.samples.is_empty() && second.needed < fitted.needed {
            (fitted, params, seed, temperature) = (second, again, s2, t2);
            live.restart();
            live.chunk(&fitted.samples, SR);
        }
    }
    ignored.stress = stx::has_stress(&b.text) && !manifest.as_ref().is_some_and(|m| m.reads.stress);
    let out = fitted.samples;
    let take = Take {
        n,
        seed,
        temperature,
        created: sp::now(),
        source: "synth".into(),
        synth_secs: secs,
        retried,
        duration: out.len() as f64 / SR as f64,
        tidy: Some(crate::audio::trim::tidy(&out, SR).samples.len() as f64 / SR as f64),
        text: b.text.clone(),
        direction: b.direction.clone(),
        top_k: params.top_k,
        top_p: params.top_p,
        max_tokens: params.max_tokens,
        voice: b.own_ref.as_ref().filter(|r| clones && !r.file.is_empty()).map(|r| format!("ref:{}", r.file)).or_else(|| preset.clone()).or_else(|| voice.clone()).unwrap_or_default(),
        engine: engine_tag(p, project, own),
        fit_stretch: fitted.stretch,
        fit_lead: fitted.lead,
        fit_over: fitted.over,
        first_ms: live.first_ms(),
        heard_ms: live.heard_ms(),
        ..Default::default()
    };
    time_aloud(&live, &take);
    live.end(true);
    Ok((out, serde_json::to_value(&ignored).unwrap_or(Value::Null), take))
}

struct Fitted {
    samples: Vec<f32>,
    stretch: f64,
    lead: f64,
    over: bool,
    /// Во сколько раз клип длиннее места, что ему нашлось; 1 — влез без ускорения.
    needed: f64,
}

/// Быстрее этого движок со скоростью речи не разгоняется подгонкой.
const SPEED_MAX: f64 = 1.6;
/// Сдвиг сида повтора подгонки, чтобы повтор не совпал с первым синтезом.
const FIT_RETRY_SALT: u64 = 0x5eed_f17;

/// Место реплики блока в проекте по таймкодам: слот от её начала до следующей, сколько тишины перед ней и правила.
fn cue_slot(project: &SpeechProject, b: &Block) -> Option<(dub_core::fit::Slot, f64, dub_core::fit::FitRules)> {
    let (true, Some(start), Some(end)) = (project.settings.timeline, b.start, b.end) else { return None };
    let timed: Vec<&Block> = project.blocks().filter(|x| !x.hidden && x.start.is_some()).collect();
    let at = timed.iter().position(|x| x.id == b.id);
    let next_start = at.and_then(|i| timed.get(i + 1)).and_then(|x| x.start).unwrap_or(end.max(start) + 3600.0);
    let prev_end = at.and_then(|i| i.checked_sub(1)).and_then(|i| timed.get(i)).and_then(|x| x.end).unwrap_or(0.0);
    let rules = dub_core::fit::FitRules {
        speech_rate_on: false,
        qc_duration: true,
        max_stretch: project.settings.max_stretch,
        lead_into_silence: project.settings.lead_into_silence,
    };
    Some((dub_core::fit::slot(start, end, start, next_start, &rules), (start - prev_end).max(0.0), rules))
}

/// Темп заранее (подгонка Dub): во сколько раз быстрее говорить, чтобы текст по оценке длины влез в реплику вместе с
/// тишиной перед ней; None — влезает и так.
fn tempo_ahead(project: &SpeechProject, b: &Block) -> Option<f64> {
    let (slot, free, rules) = cue_slot(project, b)?;
    let lang = match project.lang_of(b) { "auto" => if crate::engines::stress::is_russian(&b.text, "auto") { "ru" } else { "en" }, own => own };
    let est = dub_core::fit::estimate(&crate::speech::text::visible(&b.text), dub_core::fit::table_cps(lang));
    let room = slot.target + dub_core::fit::lead(est, &slot, free, &rules);
    (room > 0.0 && est > room).then(|| (est / room).min(SPEED_MAX))
}

/// В проекте по таймкодам — уложить клип в его реплику (механизм подгонки Dub): до начала следующей реплики;
/// не влезает — начать раньше, в тишину перед репликой; ещё не влезает — ускорить до предела; дальше — пометить.
fn fit_to_cue(project: &SpeechProject, b: &Block, samples: Vec<f32>) -> Result<Fitted, String> {
    let Some((slot, free, rules)) = cue_slot(project, b) else { return Ok(Fitted { samples, stretch: 1.0, lead: 0.0, over: false, needed: 1.0 }) };
    let clip = samples.len() as f64 / SR as f64;
    let lead = dub_core::fit::lead(clip, &slot, free, &rules);
    let needed = dub_core::fit::needed(clip, slot.target + lead);
    if needed <= 1.0 + 1e-6 {
        return Ok(Fitted { samples, stretch: 1.0, lead, over: false, needed });
    }
    let stretch = needed.min(slot.cap);
    let samples = at_rate(&samples, stretch)?;
    Ok(Fitted { samples, stretch, lead, over: dub_core::fit::over(needed, slot.cap), needed })
}

/// Реф продолжения: дубль предыдущего блока (24 кГц моно) и его видимый текст.
/// Реф реплики из `refs/` проекта: целиком (продолжение клипа) или его начало до предела рефа.
fn own_reference(project_dir: &Path, r: &sp::OwnRef, limit: f64) -> Result<(PathBuf, Option<String>), String> {
    let src = project_dir.join("refs").join(&r.file);
    if !src.is_file() {
        return Err(t!("speech-own-ref-missing", file = r.file.clone()));
    }
    let text = Some(r.text.trim().to_string()).filter(|t| !t.is_empty());
    if r.whole || crate::audio::media::duration(&src)? <= limit + 0.5 {
        return Ok((src, text));
    }
    let cut = project_dir.join("refs").join(format!(".{}-{limit:.1}.wav", r.file.trim_end_matches(".wav")));
    if !cut.is_file() {
        crate::audio::media::trim_ref(&src, &cut, 0.0, limit)?;
    }
    // a cut reference no longer says the whole transcript, so the engine clones the timbre without it
    Ok((cut, None))
}

fn continuation_ref(takes: &Path, take: &sp::Take, text: &str) -> Result<(PathBuf, Option<String>), String> {
    let src = takes.join(&take.file);
    let wav = takes.join(format!(".ref-{}.wav", take.file.trim_end_matches(".flac")));
    if !wav.is_file() {
        crate::audio::media::extract_audio(&src, &wav, SR, 1)?;
    }
    let said = crate::speech::text::visible(text);
    Ok((wav, (!said.trim().is_empty()).then_some(said)))
}

/// Фраза в темпе `rate` (atempo ffmpeg, как подгонка длины в Dub): высота голоса не меняется.
fn at_rate(samples: &[f32], rate: f64) -> Result<Vec<f32>, String> {
    let dir = tempfile::Builder::new().prefix("rate-").tempdir().map_err(|e| e.to_string())?;
    let src = dir.path().join("in.wav");
    let dst = dir.path().join("out.wav");
    crate::audio::wavio::write_mono_f32(&src, samples, SR)?;
    crate::audio::media::time_stretch(&src, &dst, rate)?;
    let (out, sr) = crate::audio::wavio::read_mono_f32(&dst)?;
    Ok(resample_to(out, sr as i32))
}

/// Параметры дубля `n`: при сиде «+1» / «−1» сид сдвигается на номер дубля и дальше фиксирован.
fn take_params(base: &tts_host::Params, n: u32) -> tts_host::Params {
    let mut p = base.clone();
    let range = tts_host::SEED_RANGE;
    let shift = u64::from(n.saturating_sub(1)) % range;
    match p.seed_mode.as_str() {
        "inc" => p.seed = (p.seed + shift) % range,
        "dec" => p.seed = (p.seed + range - shift) % range,
        _ => {}
    }
    p
}

/// A dictionary rule tried by ear: the phrase as written (no dictionaries) and with this one rule, in the voice asked or
/// the project's, by the project's local engine. The audio is kept in the project's `tries/` by what it was made of.
pub fn try_rule(p: &SpeechPaths, pid: &str, phrase: &str, rule: crate::text::pron_dict::Entry, voice: Option<String>, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let project = crate::speech::load(&p.workspace, pid)?;
    let b = Block { id: "try".into(), voice: voice.filter(|v| !v.trim().is_empty()), text: phrase.trim().to_string(), ..Default::default() };
    if crate::engines::is_cloud(project.engine_of(&b)) {
        return Err(t!("dict-try-local-only"));
    }
    if b.text.is_empty() {
        return Err(t!("dict-try-empty"));
    }
    let rule = crate::text::pron_dict::Entry { enabled: true, ..rule };
    let dir = p.workspace.join(pid).join("tries");
    let voice_of = project.voice_of(&b).unwrap_or_default();
    let mut out = json!({});
    for (name, entries) in [("before", Vec::new()), ("after", vec![rule])] {
        let stress = project.settings.auto_stress && crate::engines::stress::is_russian(&b.text, project.lang_of(&b));
        let read = preparation::prepare(&b.text, &prep_opts(p, &project, project.lang_of(&b), project.engine_of(&b), Some(voice_of.as_str()).filter(|v| !v.is_empty()), &entries, stress)).map_err(|e| t!("stress-failed", detail = e))?.text;
        let made_of = format!("{read}|{voice_of}|{}|{}|{}", project.engine_of(&b), higgs_quant(p), serde_json::to_string(project.params_of(&b)).unwrap_or_default());
        let file = format!("{}.flac", &blake3::hash(made_of.as_bytes()).to_hex()[..20]);
        let path = dir.join(&file);
        if !path.is_file() {
            progress(json!({ "type": "progress", "stage": "speech_synth", "msg": if name == "before" { t!("dict-try-before") } else { t!("dict-try-after") } }));
            let (samples, _, _) = synth_block_with(p, pid, &project, &b, 1, &entries)?;
            write_flac(&samples, &path)?;
        }
        out[name] = json!({ "file": file, "text": read });
    }
    Ok(out)
}

fn write_flac(samples: &[f32], dst: &Path) -> Result<(), String> {
    let dir = dst.parent().expect("a folder");
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let wav = dir.join(format!(".{}.wav", sp::new_id()));
    crate::audio::wavio::write_mono_f32(&wav, samples, SR)?;
    let made = crate::audio::media::encode_audio(&wav, dst, "flac", false);
    let _ = std::fs::remove_file(&wav);
    made
}

/// Какие блоки озвучивать: `ids` — эти (нет — все), `from` — начиная с этого (Play until end), `force` — и готовые
/// (новый дубль), иначе только `pending`, `stale` и `failed`.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Scope {
    pub ids: Option<Vec<String>>,
    pub from: Option<String>,
    pub force: bool,
    /// Повтор с теми же параметрами: сид дубля, по которому повторяют.
    #[serde(default)]
    pub seed: Option<u64>,
}

/// Запрос озвучки, вставший в очередь, пока проект озвучивается.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Queued {
    pub n: u64,
    pub scope: Scope,
    pub added: u64,
}

/// Очереди озвучки по проектам. `None` — задача проекта уже заканчивается: новые запросы ставятся новой задачей.
pub static QUEUE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, Option<Vec<Queued>>>>> = std::sync::LazyLock::new(Default::default);

pub fn queue_of(pid: &str) -> Vec<Queued> {
    QUEUE.lock().map(|q| q.get(pid).cloned().flatten().unwrap_or_default()).unwrap_or_default()
}

/// Встать в очередь работающей задачи проекта; `None` — задачи нет или она уже заканчивается.
pub fn try_queue(pid: &str, scope: &Scope) -> Option<u64> {
    let mut q = QUEUE.lock().ok()?;
    let list = q.get_mut(pid)?.as_mut()?;
    let n = list.iter().map(|x| x.n).max().unwrap_or(0) + 1;
    list.push(Queued { n, scope: scope.clone(), added: sp::now() });
    Some(n)
}

pub fn unqueue(pid: &str, n: Option<u64>) -> usize {
    let Ok(mut q) = QUEUE.lock() else { return 0 };
    let Some(Some(list)) = q.get_mut(pid) else { return 0 };
    let before = list.len();
    list.retain(|x| n.is_some_and(|n| x.n != n));
    before - list.len()
}

thread_local! {
    /// Сид повтора «с теми же параметрами» для текущей задачи.
    static REPEAT_SEED: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// Озвучить один блок `count` дублями и записать их в проект; номера новых дублей. Ошибка — в блок.
fn voice_block(p: &SpeechPaths, pid: &str, id: &str, count: u32, source: &str, progress: &dyn Fn(Value)) -> Result<Vec<u32>, String> {
    let mut made = Vec::new();
    for _ in 0..count {
        if crate::jobs::current().is_some_and(|c| c.is_cancelled()) {
            return Err(crate::jobs::CANCELLED.to_string());
        }
        let project = crate::speech::load(&p.workspace, pid)?;
        let Some(b) = project.blocks().find(|b| b.id == id).cloned() else { return Ok(made) };
        let key = keys(p, &project).remove(id).unwrap_or_default();
        let n = b.takes.iter().map(|t| t.n).max().unwrap_or(0) + 1;
        let result = synth_block(p, pid, &project, &b, n).and_then(|(samples, ignored, mut take)| {
            let file = format!("{}-{n}.flac", b.id);
            let path = takes_dir(p, pid).join(&file);
            write_flac(&samples, &path)?;
            take.bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
            take.file = file;
            take.key = key.clone();
            take.source = source.into();
            Ok((take, ignored))
        });
        let kept = crate::speech::edit(&p.workspace, pid, |fresh| {
            let block = fresh.block_mut(id)?;
            Some(match result {
                Ok((take, ignored)) => {
                    block.takes.push(take);
                    if block.pinned.is_none() {
                        block.active = Some(n);
                    }
                    block.error = None;
                    Ok(ignored)
                }
                Err(e) => {
                    block.error = Some(sp::BlockError { key, text: e.clone() });
                    Err(e)
                }
            })
        })?;
        match kept {
            None => return Ok(made),
            Some(Ok(ignored)) => {
                made.push(n);
                progress(json!({ "type": "progress", "stage": "speech_block_done", "block": id, "ignored": ignored }));
            }
            Some(Err(e)) => return Err(e),
        }
    }
    Ok(made)
}

/// Распознать новые дубли блоков, выбрать лучший по WER; блоки, где проверку не прошёл ни один, — в ответе.
/// Голос рефа и дубля в одном пространстве WeSpeaker: эмбеддинг 16 кГц моно.
fn voice_embedding(embedder: &mut dub_faces::VoiceEmbedder, audio: &Path) -> Result<Vec<f32>, String> {
    let dir = tempfile::Builder::new().prefix("emb-").tempdir().map_err(|e| e.to_string())?;
    let wav = dir.path().join("16k.wav");
    crate::audio::media::to_16k_mono(audio, &wav)?;
    embedder.embed_wav(&wav).map_err(|e| e.to_string())
}

/// Проверить новые дубли блоков: текст — распознаванием (WER), голос — похожестью на реф; лучший по
/// «прошёл всё → WER → похожесть» становится активным; блоки, где не прошёл ни один, — в ответе.
/// Дубль, где вместо речи музыка или шум: его звук в 16 кГц через Silero VAD.
fn non_speech(p: &SpeechPaths, file: &Path) -> Result<bool, String> {
    let tmp = tempfile::Builder::new().prefix("vad-").tempdir().map_err(|e| e.to_string())?;
    let wav = tmp.path().join("take16.wav");
    crate::audio::media::to_16k_mono(file, &wav)?;
    let (samples, _) = crate::audio::wavio::read_mono_f32(&wav)?;
    let probs = crate::engines::vad::probabilities(&p.models_root, &samples)?;
    Ok(crate::audio::qc::non_speech(&samples, &probs, crate::engines::vad::FRAME))
}

/// What a voice sounds like elsewhere in the project, from the takes already checked: seconds per syllable (the
/// project's when the voice has too few takes) and loudness (the voice's only).
fn voice_norm(project: &SpeechProject, voice: Option<&str>) -> crate::audio::flags::Norm {
    let (mut rate_v, mut rate_all, mut lufs_v) = (Vec::new(), Vec::new(), Vec::new());
    for b in project.blocks() {
        let same = project.voice_of(b).as_deref() == voice;
        for q in b.takes.iter().filter_map(|t| t.qc.as_ref().map(|q| (q, q.syllables.unwrap_or_else(|| crate::audio::flags::syllables(&t.text))))) {
            let (q, syl) = q;
            if let Some(s) = q.speech_secs.filter(|_| syl >= 6) {
                rate_all.push(s / syl as f64);
                if same {
                    rate_v.push(s / syl as f64);
                }
            }
            if let (Some(l), true) = (q.lufs, same) {
                lufs_v.push(l);
            }
        }
    }
    crate::audio::flags::Norm {
        secs_per_syllable: crate::audio::flags::median(rate_v, 5).or_else(|| crate::audio::flags::median(rate_all, 5)),
        lufs: crate::audio::flags::median(lufs_v, 5),
    }
}

/// The take's sound measured and the flags of `want` it raises (Dub Studio's synthesis defect first), over the part of
/// the take the project uses; its expected length is counted on `spoken`, the take's text as the engine was given it.
fn sound_flags(take: &Take, spoken: &str, lang: &str, file: &Path, want: &[String], norm: crate::audio::flags::Norm) -> Result<(crate::audio::flags::Features, Vec<String>), String> {
    const SR: u32 = 24_000;
    let tmp = tempfile::Builder::new().prefix("flags-").tempdir().map_err(|e| e.to_string())?;
    let wav = tmp.path().join("take.wav");
    crate::audio::media::extract_audio(file, &wav, SR, 1)?;
    let (all, _) = crate::audio::wavio::read_mono_f32(&wav)?;
    let samples = match take.cut {
        Some([a, z]) => {
            let a = ((a * SR as f64) as usize).min(all.len());
            all[a..((z * SR as f64) as usize).clamp(a, all.len())].to_vec()
        }
        None => all,
    };
    let on = |k: &str| want.iter().any(|w| w == k);
    let features = crate::audio::flags::measure(&samples, SR, on("whistle"), on("audio_repeat"));
    let mut raised = crate::audio::flags::flags(&features, want, spoken, norm);
    if on("defect") {
        let chars = stx::visible(spoken).chars().filter(|c| c.is_alphanumeric()).count();
        if let Some(d) = tts_host::defect(&samples, SR as i32, chars, lang) {
            raised.insert(0, d.to_string());
        }
    }
    Ok((features, raised))
}

/// `recognise` checks the text by recognition whatever the project's settings say.
fn check_takes(p: &SpeechPaths, pid: &str, made: &BTreeMap<String, Vec<u32>>, recognise: bool, progress: &dyn Fn(Value)) -> Result<Vec<String>, String> {
    tts_host::make_room();
    let settings = crate::speech::load(&p.workspace, pid)?.settings;
    let asr_on = recognise || settings.qc.asr_check || settings.qc.takes > 1;
    let sim_on = settings.qc.similarity;
    let want = settings.qc.heuristics.clone();
    crate::app::ensure_job_components(&p.repo_root, &p.models_root, sim_on, false, &|ev| progress(ev))?;
    if asr_on {
        crate::app::recognition::ensure(&p.repo_root, &p.models_root, "", &|ev| progress(ev))?;
        let vad = crate::engines::vad::missing(&p.models_root);
        crate::app::ensure_components(&p.repo_root, &vad.iter().map(String::as_str).collect::<Vec<_>>(), &|ev| progress(ev))?;
    }
    let mut embedder = if sim_on {
        Some(dub_faces::VoiceEmbedder::load(&dub_faces::wespeaker_path(&p.models_root)).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let threshold = dub_faces::voice_cos_threshold() as f64;
    // every new take recognised up front, a language's takes in one batch, instead of one after another below
    let mut heard_all = if asr_on {
        let project = crate::speech::load(&p.workspace, pid)?;
        let mut files = Vec::new();
        for (id, takes) in made {
            let Some(b) = project.blocks().find(|b| &b.id == id) else { continue };
            for take in b.takes.iter().filter(|t| takes.contains(&t.n)) {
                files.push((takes_dir(p, pid).join(&take.file), project.lang_of(b).to_string(), b.id.clone()));
            }
        }
        crate::app::recognition::transcribe_many(&p.repo_root, &p.models_root, &files)?
    } else {
        Default::default()
    };
    let mut refs: BTreeMap<String, Vec<f32>> = BTreeMap::new();
    let mut failed = Vec::new();
    for (i, (id, takes)) in made.iter().enumerate() {
        progress(json!({ "type": "progress", "stage": "speech_qc", "msg": t!("speech-qc-block", n = (i + 1) as i64, total = made.len() as i64), "pct": (i * 100 / made.len().max(1)) as i64 }));
        let project = crate::speech::load(&p.workspace, pid)?;
        let Some(b) = project.blocks().find(|b| &b.id == id).cloned() else { continue };
        let lang = project.lang_of(&b).to_string();
        let strictness = project.settings.qc.strictness.clone();
        let voice = project.voice_of(&b).filter(|v| preset_of(project.engine_of(&b), Some(v)).is_none());
        let norm = voice_norm(&project, project.voice_of(&b).as_deref());
        let reference = match (&mut embedder, voice.as_deref()) {
            (Some(emb), Some(v)) => match refs.get(v) {
                Some(e) => Some(e.clone()),
                None => {
                    let audio = crate::voices::audio_path(&p.voices_dir, v).ok_or_else(|| t!("voice-missing", name = v.to_string()))?;
                    let e = voice_embedding(emb, &audio)?;
                    refs.insert(v.to_string(), e.clone());
                    Some(e)
                }
            },
            _ => None,
        };
        let entries = dictionary(p, &project);
        let mut checks: Vec<(u32, crate::audio::qc::Check)> = Vec::new();
        for n in takes {
            let Some(take) = b.takes.iter().find(|t| t.n == *n) else { continue };
            let file = takes_dir(p, pid).join(&take.file);
            let mut check = if asr_on {
                let heard = heard_all.remove(&file).ok_or_else(|| format!("{}: not recognised", file.display()))??;
                let expected = preparation::prepare(&b.text, &prep_opts(p, &project, project.lang_of(&b), project.engine_of(&b), voice.as_deref(), &entries, false)).map_err(|e| t!("stress-failed", detail = e))?.text;
                let (expected, heard_said) = crate::audio::qc::said_alike(&expected, &heard.text, &crate::text::prep::block_lang(&b.text, &lang))?;
                let mut c = crate::audio::qc::check(&expected, &heard_said, &strictness);
                c.heard = heard.text.clone();
                let pause = stx::pieces(&b.text).iter().filter_map(|x| if let Piece::Pause(ms) = x { Some(*ms as f64 / 1000.0) } else { None }).fold(0.0, f64::max);
                c.issues = crate::audio::qc::timing_issues(&heard.words, take.duration, pause);
                if crate::audio::qc::repeated(&crate::audio::qc::normalize(&expected), &crate::audio::qc::normalize(&heard_said)) {
                    c.issues.push("repeat".to_string());
                }
                if non_speech(p, &file)? {
                    c.issues.push("noise".to_string());
                }
                c.timed = heard.words.iter().map(|w| (w.word.clone(), (w.start * 1000.0).round() / 1000.0, (w.end * 1000.0).round() / 1000.0)).collect();
                c
            } else {
                crate::audio::qc::Check { passed: true, ..Default::default() }
            };
            if !want.is_empty() {
                let spoken = preparation::prepare(&take.text, &prep_opts(p, &project, project.lang_of(&b), project.engine_of(&b), voice.as_deref(), &entries, false)).map_err(|e| t!("stress-failed", detail = e))?.text;
                let (features, raised) = sound_flags(take, &spoken, &lang, &file, &want, norm)?;
                check.speech_secs = Some(features.trimmed_s);
                check.syllables = Some(crate::audio::flags::syllables(&spoken));
                check.lufs = features.lufs;
                let mut suspects = Vec::new();
                for flag in raised {
                    match crate::audio::flags::role(&flag) {
                        crate::audio::flags::Role::Defect => check.issues.push(flag),
                        crate::audio::flags::Role::Note => check.notes.push(flag),
                        crate::audio::flags::Role::Suspect => suspects.push(flag),
                    }
                }
                if !suspects.is_empty() {
                    // a suspect is recognised by the pair, as in Game Dub: the studio's model and the other one when it is here
                    let first = if asr_on {
                        check.heard.clone()
                    } else {
                        crate::app::recognition::ensure(&p.repo_root, &p.models_root, "", &|ev| progress(ev))?;
                        let heard = crate::app::recognition::transcribe(&p.repo_root, &p.models_root, &file, &lang, &b.id)?;
                        check.timed = heard.words.iter().map(|w| (w.word.clone(), (w.start * 1000.0).round() / 1000.0, (w.end * 1000.0).round() / 1000.0)).collect();
                        check.heard = heard.text.clone();
                        heard.text
                    };
                    let second = crate::app::recognition::second_opinion(&p.repo_root, &p.models_root, &file, &lang)?.map(|h| h.text);
                    let heard: Vec<&str> = std::iter::once(first.as_str()).chain(second.as_deref()).collect();
                    match crate::audio::flags::verdict(&stx::visible(&take.text), &heard) {
                        Some("listen") => check.notes.push("listen".to_string()),
                        Some(v) => check.issues.push(v.to_string()),
                        None => check.notes.extend(suspects),
                    }
                }
            }
            if let (Some(emb), Some(r)) = (&mut embedder, &reference) {
                let e = voice_embedding(emb, &file)?;
                let cos: f64 = e.iter().zip(r).map(|(a, b)| (*a as f64) * (*b as f64)).sum();
                check.similarity = Some(cos);
                check.similar = Some(cos >= threshold);
            }
            checks.push((*n, check));
        }
        let rank = |c: &crate::audio::qc::Check| (!c.ok(), c.wer, -c.similarity.unwrap_or(0.0));
        let bad = crate::speech::edit(&p.workspace, pid, |fresh| {
            let block = fresh.block_mut(id)?;
            for (n, c) in &checks {
                if let Some(t) = block.takes.iter_mut().find(|t| t.n == *n) {
                    t.qc = Some(c.clone());
                }
            }
            let best = checks.iter().min_by(|a, b| rank(&a.1).partial_cmp(&rank(&b.1)).unwrap_or(std::cmp::Ordering::Equal))?;
            if block.pinned.is_none() {
                block.active = Some(best.0);
            }
            Some(!best.1.ok())
        })?;
        if bad == Some(true) {
            failed.push(id.clone());
        }
    }
    Ok(failed)
}

/// Задача озвучки проекта: свой запрос, затем всё, что встало в очередь, пока она шла.
pub fn synth(p: &SpeechPaths, pid: &str, scope: &Scope, progress: &dyn Fn(Value)) -> Result<Value, String> {
    if let Ok(mut q) = QUEUE.lock() {
        q.insert(pid.to_string(), Some(Vec::new()));
    }
    let mut current = scope.clone();
    let mut total = json!({ "done": 0, "failed": [], "skipped": 0, "qc_failed": [] });
    let result = loop {
        match synth_scope(p, pid, &current, progress) {
            Ok(v) => {
                total["done"] = json!(total["done"].as_u64().unwrap_or(0) + v["done"].as_u64().unwrap_or(0));
                total["skipped"] = json!(total["skipped"].as_u64().unwrap_or(0) + v["skipped"].as_u64().unwrap_or(0));
                for k in ["failed", "qc_failed"] {
                    if let (Some(all), Some(more)) = (total[k].as_array().cloned(), v[k].as_array()) {
                        total[k] = json!(all.into_iter().chain(more.iter().cloned()).collect::<Vec<_>>());
                    }
                }
            }
            Err(e) => break Err(e),
        }
        let next = QUEUE.lock().ok().and_then(|mut q| {
            let slot = q.get_mut(pid)?;
            let list = slot.as_mut()?;
            if list.is_empty() {
                *slot = None;
                None
            } else {
                Some(list.remove(0))
            }
        });
        match next {
            Some(item) => current = item.scope,
            None => break Ok(total.clone()),
        }
    };
    if let Ok(mut q) = QUEUE.lock() {
        q.remove(pid);
    }
    result
}

fn synth_scope(p: &SpeechPaths, pid: &str, scope: &Scope, progress: &dyn Fn(Value)) -> Result<Value, String> {
    REPEAT_SEED.with(|s| s.set(scope.seed));
    let project = crate::speech::load(&p.workspace, pid)?;
    let qc = project.settings.qc.clone();
    let random_seed = project.settings.params.seed_mode != "fixed";
    let order: Vec<String> = project.blocks().map(|b| b.id.clone()).collect();
    let start = scope.from.as_ref().and_then(|f| order.iter().position(|id| id == f)).unwrap_or(0);
    let wanted: Vec<String> = order[start..].iter().filter(|id| scope.ids.as_ref().is_none_or(|ids| ids.contains(id))).cloned().collect();
    let mut used: Vec<&str> = vec![project.settings.engine.as_str()];
    for b in project.blocks().filter(|b| wanted.contains(&b.id)) {
        if !used.contains(&project.engine_of(b)) {
            used.push(project.engine_of(b));
        }
    }
    for id in &used {
        crate::engines::get(id).ok_or_else(|| t!("speech-engine-unknown", engine = id.to_string()))?;
    }
    for b in project.blocks().filter(|b| wanted.contains(&b.id)) {
        let engine = crate::engines::get(project.engine_of(b)).ok_or_else(|| t!("speech-engine-unknown", engine = project.engine_of(b).to_string()))?;
        if !crate::engines::supports(&engine, project.lang_of(b)) {
            return Err(t!("speech-lang-unsupported", lang = project.lang_of(b).to_uppercase(), engine = engine.name.to_string()));
        }
    }
    // one error naming every block that cannot be voiced, before any model loads
    let silent: Vec<String> = project
        .blocks()
        .enumerate()
        .filter(|(_, b)| wanted.contains(&b.id) && !b.hidden && project.voice_of(b).is_none())
        .filter(|(_, b)| crate::engines::get(project.engine_of(b)).is_some_and(|e| e.needs_voice))
        .map(|(n, b)| format!("{} «{}»", n + 1, sp::visible_snippet(&b.text)))
        .collect();
    if !silent.is_empty() {
        let mut blocks = silent.iter().take(10).cloned().collect::<Vec<_>>().join(", ");
        if silent.len() > 10 {
            blocks.push_str(", …");
        }
        return Err(t!("speech-no-voice-blocks", blocks = blocks));
    }
    let library = crate::voices::list(&p.voices_dir).0;
    let unknown: Vec<String> = project
        .blocks()
        .enumerate()
        .filter(|(_, b)| wanted.contains(&b.id) && !b.hidden)
        .filter_map(|(n, b)| {
            let engine = crate::engines::get(project.engine_of(b))?;
            let voice = project.voice_of(b)?;
            let missing = engine.clone != "none" && !engine.presets.iter().any(|x| x.id == voice) && !library.iter().any(|x| x.name == voice);
            missing.then(|| format!("{} «{}» ({voice})", n + 1, sp::visible_snippet(&b.text)))
        })
        .collect();
    if !unknown.is_empty() {
        let mut blocks = unknown.iter().take(10).cloned().collect::<Vec<_>>().join(", ");
        if unknown.len() > 10 {
            blocks.push_str(", …");
        }
        return Err(t!("speech-unknown-voice-blocks", blocks = blocks));
    }
    // a library voice without its reference text is heard once here, before the takes' keys are made of that text
    let mut heard: Vec<String> = Vec::new();
    for b in project.blocks().filter(|b| wanted.contains(&b.id) && !b.hidden && b.own_ref.as_ref().is_none_or(|r| r.file.is_empty())) {
        let Some(voice) = project.voice_of(b) else { continue };
        let clones = crate::engines::get(project.engine_of(b)).is_some_and(|e| e.clone != "none" && !e.presets.iter().any(|x| x.id == voice));
        let untold = crate::voices::read_transcript(&p.voices_dir, &voice).is_none_or(|t| t.trim().is_empty());
        if clones && untold && !heard.contains(&voice) && library.iter().any(|x| x.name == voice) {
            let vp = crate::voices::VoicePaths { repo_root: p.repo_root.clone(), models_root: p.models_root.clone(), voices_dir: p.voices_dir.clone() };
            crate::voices::transcribe(&vp, &voice, progress)?;
            heard.push(voice);
        }
    }
    // the build of the voicing device the local engines run on, once their models are here
    let mut builds: Vec<&str> = Vec::new();
    let higgs = used.contains(&"higgs") && crate::engines::acpp::higgs_gguf(&p.models_root).is_file();
    let supertonic = used.contains(&crate::engines::supertonic::ENGINE) && crate::engines::supertonic::installed(&p.models_root);
    if higgs || supertonic {
        let build = crate::engines::acpp::build_for(crate::engines::models::server_choice(&p.models_root, "tts_backend"))?;
        builds.push(crate::engines::acpp::component(build));
        builds.extend(crate::engines::acpp::runtime_component(build));
    }
    if used.iter().any(|e| crate::engines::qwen_tts::is_engine(e)) && crate::engines::qwen_tts::dir(&p.models_root).join(crate::engines::qwen_tts::CODEC).is_file() {
        builds.extend(crate::engines::qwen_tts::build_components(&p.models_root)?);
    }
    if !builds.is_empty() {
        crate::app::ensure_components(&p.repo_root, &builds, &|ev| progress(ev))?;
    }
    if project.settings.auto_stress && wanted.iter().any(|id| project.blocks().any(|b| &b.id == id && crate::engines::stress::is_russian(&b.text, project.lang_of(b)))) {
        crate::app::ensure_components(&p.repo_root, &[crate::engines::stress::COMPONENT, "onnxruntime"], &|ev| progress(ev))?;
    }
    if project.blocks().any(|b| wanted.contains(&b.id) && project.engine_of(b) == "google") {
        google_prefetch(p, pid, &wanted, scope.force, qc.takes, progress)?;
    }
    let checking = qc.asr_check || qc.takes > 1 || qc.similarity || !qc.heuristics.is_empty();
    let (mut done, mut failed, mut skipped) = (0usize, Vec::new(), 0usize);
    let mut made: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let total = wanted.len();
    for (i, id) in wanted.iter().enumerate() {
        let project = crate::speech::load(&p.workspace, pid)?;
        let Some(b) = project.blocks().find(|b| &b.id == id).cloned() else { continue };
        let key = keys(p, &project).remove(id).unwrap_or_default();
        let state = sp::status(&b, &key);
        let approved = b.review == "approved" && scope.ids.as_ref().is_none_or(|ids| !ids.contains(id));
        if state == "skipped" || (!scope.force && state == "ready") || (approved && state == "ready") {
            skipped += 1;
            continue;
        }
        progress(json!({ "type": "progress", "stage": "speech_synth", "msg": t!("speech-synth-block", n = (i + 1) as i64, total = total as i64, text = sp::visible_snippet(&b.text)), "pct": (i * 100 / total.max(1)) as i64 }));
        match voice_block(p, pid, id, qc.takes, if qc.takes > 1 { "multitake" } else { "synth" }, progress) {
            Ok(ns) => {
                done += 1;
                made.insert(id.clone(), ns);
            }
            Err(e) if e.starts_with(tts_host::ENGINE_STUCK) || e == crate::jobs::CANCELLED => return Err(e),
            Err(e) => failed.push(json!({ "block": id, "error": e })),
        }
    }
    if let Err(e) = report_speed(p, pid, &made) {
        tracing::warn!("hub: the voicing speed of project {pid} was not reported: {e}");
    }
    let mut qc_failed = Vec::new();
    if checking && !made.is_empty() {
        qc_failed = check_takes(p, pid, &made, false, progress)?;
        if random_seed && !qc_failed.is_empty() {
            let mut again: BTreeMap<String, Vec<u32>> = BTreeMap::new();
            for id in &qc_failed {
                if let Ok(ns) = voice_block(p, pid, id, 1, "retry", progress) {
                    again.insert(id.clone(), ns);
                }
            }
            if !again.is_empty() {
                qc_failed = check_takes(p, pid, &again, false, progress)?;
            }
        }
        mark_checked(p, pid, &qc_failed, &qc_failed)?;
    }
    if done == 0 && !failed.is_empty() {
        return Err(t!("speech-synth-all-failed", error = failed[0]["error"].as_str().unwrap_or("").to_string()));
    }
    Ok(json!({ "done": done, "failed": failed, "skipped": skipped, "qc_failed": qc_failed }))
}

/// The speed of the card the voicing ran on: the seconds of speech the local engines made in this job and the seconds
/// they took, the job queue running one job at a time. Cloud engines run on no card of this computer.
fn report_speed(p: &SpeechPaths, pid: &str, made: &BTreeMap<String, Vec<u32>>) -> Result<(), String> {
    let project = crate::speech::load(&p.workspace, pid)?;
    let (mut media, mut work) = (0.0f64, 0.0f64);
    for b in project.blocks().filter(|b| made.contains_key(&b.id) && !crate::engines::is_cloud(project.engine_of(b))) {
        for take in b.takes.iter().filter(|t| made[&b.id].contains(&t.n)) {
            media += take.duration;
            work += take.synth_secs;
        }
    }
    if media > 0.0 && work > 0.0 {
        crate::app::hub_worked(crate::jobs::JobKind::Synth.as_str(), media, std::time::Duration::from_secs_f64(work));
    }
    Ok(())
}

/// Пауза между главами при сборке одним файлом (граница раздела у tts-audiobook-tool — 2–2,5 с).
const CHAPTER_PAUSE_MS: i64 = 2_000;
/// MP3 at a constant 192 kbps: ACX's rate, and plenty for speech.
const MP3_KBPS: u32 = 192;

/// Запас под сжатие: кодеки с потерями поднимают пики между отсчётами, поэтому для них потолок ниже на 1 дБ (AES TD1004).
const LOSSY_HEADROOM_DB: f64 = 1.0;

/// Потолок пиков перед кодированием в `format`.
pub fn peak_for(format: &str, tp: f64) -> f64 {
    if matches!(format, "wav" | "flac") { tp } else { tp - LOSSY_HEADROOM_DB }
}

/// Пресет громкости: интегральная громкость и потолок пиков; ACX — средний уровень −20 (RMS-цель ACX −23…−18), пики −3.
pub fn loudness(preset: &str) -> Option<(f64, f64, f64)> {
    match preset {
        "video" => Some((-14.0, -1.0, 11.0)),
        "podcast" => Some((-16.0, -1.0, 11.0)),
        "acx" => Some((-20.0, -3.0, 11.0)),
        _ => None,
    }
}

/// Где блок встаёт в склейке: после предыдущего через паузу (паузу после предыдущего блока, иначе по смене спикера или
/// того же; не короче паузы главы; отрицательная — перебивание), а в режиме таймкодов — на время реплики. Один расчёт
/// для экспорта и шкалы окна.
pub(crate) struct Placer<'a> {
    project: &'a SpeechProject,
    chapter_of: std::collections::HashMap<&'a str, usize>,
    sr: u32,
    cursor: i64,
    prev: Option<(&'a str, Option<usize>, Option<i64>)>,
}

impl<'a> Placer<'a> {
    pub fn new(project: &'a SpeechProject, sr: u32) -> Self {
        let chapter_of = project.chapters.iter().enumerate().flat_map(|(ci, c)| c.blocks.iter().map(move |b| (b.id.as_str(), ci))).collect();
        Placer { project, chapter_of, sr, cursor: 0, prev: None }
    }

    /// Начало блока длиной `len` сэмплов; `lead` — на сколько секунд дубль начат раньше реплики при подгонке.
    pub fn place(&mut self, b: &'a Block, lead: f64, len: usize) -> usize {
        let s = &self.project.settings;
        let chapter = self.chapter_of.get(b.id.as_str()).copied();
        if let Some((speaker, prev_chapter, after)) = self.prev {
            let ms = after.unwrap_or(if speaker == b.speaker { s.pause_same_ms } else { s.pause_switch_ms });
            let ms = if prev_chapter != chapter { ms.max(CHAPTER_PAUSE_MS) } else { ms };
            self.cursor = (self.cursor + self.sr as i64 * ms / 1000).max(0);
        }
        let at = match (s.timeline, b.start) {
            (true, Some(start)) => ((start - lead).max(0.0) * self.sr as f64) as usize,
            _ => self.cursor as usize,
        };
        self.cursor = self.cursor.max((at + len) as i64);
        self.prev = Some((b.speaker.as_str(), chapter, b.pause_ms));
        at
    }
}

/// The SHA-256 of every file a build made, in the `sha256sum` format (`<hash>  <name>`), so whoever receives the
/// files checks them with `sha256sum -c`.
fn sha256_manifest(dir: &Path, files: &[String]) -> Result<String, String> {
    let mut out = String::new();
    for name in files {
        let path = dir.join(name);
        let hash = crate::setup::sha256_file(&path, &|| false).map_err(|e| format!("{}: {e}", path.display()))?.ok_or_else(|| format!("{}: hashing stopped", path.display()))?;
        out.push_str(&format!("{hash}  {name}\n"));
    }
    Ok(out)
}

/// The lines of an exported file as a table (RFC 4180, UTF-8 with a BOM so spreadsheets read any script): where each
/// line sounds in the file, its chapter, speaker and text, and the take, engine and voice it was voiced with. `from`
/// is where the file starts in the whole export, for a part.
fn lines_csv(rows: &[(&(f64, f64, String, String), &(f64, Value))], chapters: &[(f64, f64, String)], from: f64) -> Result<Vec<u8>, String> {
    let clock = |s: f64| {
        let ms = (s.max(0.0) * 1000.0).round() as u64;
        format!("{:02}:{:02}:{:02}.{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
    };
    let mut out = csv::Writer::from_writer(b"\xEF\xBB\xBF".to_vec());
    let fail = |e: csv::Error| e.to_string();
    out.write_record(["n", "start", "end", "chapter", "speaker", "text", "take", "engine", "voice", "block"]).map_err(fail)?;
    for (k, ((start, end, speaker, text), (_, made))) in rows.iter().enumerate() {
        let chapter = chapters.iter().rev().find(|c| c.0 <= *start + 1e-6).map(|c| c.2.as_str()).unwrap_or_default();
        let field = |key: &str| made[key].as_str().map(str::to_string).or_else(|| made[key].as_u64().map(|n| n.to_string())).unwrap_or_default();
        out.write_record([(k + 1).to_string(), clock(start - from), clock(end - from), chapter.to_string(), speaker.clone(), text.clone(), field("take"), field("engine"), field("voice"), field("block")]).map_err(fail)?;
    }
    out.into_inner().map_err(|e| e.to_string())
}

/// The gain the loudness step put on each sample of the mix (level and limiter alike), for the speakers' tracks: where
/// the mix was near silence (room tone may have been added there) the last gain carries on.
fn gain_envelope(before: &[f32], after: &[f32]) -> Vec<f32> {
    let mut last = 1.0f32;
    before
        .iter()
        .enumerate()
        .map(|(n, b)| {
            if b.abs() > 1e-4 {
                if let Some(a) = after.get(n) {
                    last = (a / b).clamp(0.0, 100.0);
                }
            }
            last
        })
        .collect()
}

/// Перебивание обрывает предыдущую реплику: её звук после `cut` уходит из склейки за 30 мс; перебивание, что начато
/// раньше самой реплики, её не трогает.
fn cut_tail(mix: &mut [f32], prev_at: usize, prev: &[f32], cut: usize) {
    if cut <= prev_at {
        return;
    }
    let fade = (SR as usize * 30 / 1000).max(1);
    for (k, s) in prev.iter().enumerate().skip(cut.saturating_sub(prev_at)) {
        let into = prev_at + k - cut;
        let keep = if into < fade { 1.0 - into as f32 / fade as f32 } else { 0.0 };
        if let Some(m) = mix.get_mut(prev_at + k) {
            *m -= s * (1.0 - keep);
        }
    }
}

/// Шкала проекта для окна: каждый блок, что войдёт в экспорт, с началом и концом в секундах (по дублю или, пока его
/// нет, по оценке длины текста) и перекрытием с тем из прежних, что звучит дольше всех; перебивание, что обрывает
/// предыдущую реплику, укорачивает её.
pub fn timeline(project: &SpeechProject) -> Vec<Value> {
    const RATE: u32 = 1000;
    let mut placer = Placer::new(project, RATE);
    let mut furthest: Option<(&str, f64)> = None;
    let mut rows: Vec<Value> = Vec::new();
    for b in project.blocks().filter(|b| !b.hidden && !b.text.trim().is_empty()) {
        let take = b.active.and_then(|n| b.takes.iter().find(|t| t.n == n));
        let lang = match project.lang_of(b) { "auto" => if crate::engines::stress::is_russian(&b.text, "auto") { "ru" } else { "en" }, own => own };
        let secs = take.map(|t| match t.cut {
            Some([a, z]) => z - a,
            None if project.settings.tidy_pauses => t.tidy.unwrap_or(t.duration),
            None => t.duration,
        }).unwrap_or_else(|| dub_core::fit::estimate(&crate::speech::text::visible(&b.text), dub_core::fit::table_cps(lang)));
        let start = placer.place(b, take.map(|t| t.fit_lead).unwrap_or(0.0), (secs * RATE as f64) as usize) as f64 / RATE as f64;
        let end = start + secs;
        let overlap = furthest.filter(|(_, prev_end)| *prev_end > start + 1e-3).map(|(id, prev_end)| json!({ "with": id, "secs": round3(prev_end - start) }));
        if let (true, Some(last)) = (b.cut_previous && overlap.is_some(), rows.last_mut()) {
            last["end"] = json!(round3(start));
        }
        let prev = rows.last().and_then(|r| r["block"].as_str().map(str::to_string));
        rows.push(json!({
            "block": b.id, "speaker": b.speaker, "start": round3(start), "end": round3(end), "voiced": take.is_some(), "overlap": overlap, "cut_previous": b.cut_previous,
            "prev": prev, "take": take.map(|t| t.file.clone()), "take_secs": take.map(|t| t.duration), "cut": take.and_then(|t| t.cut), "mix": b.mix,
        }));
        if b.cut_previous || furthest.is_none_or(|(_, e)| end >= e) {
            furthest = Some((b.id.as_str(), end));
        }
    }
    rows
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// Что мешает экспорту: блоки без звука, устаревшие, упавшие, со стёртым или изменённым после записи файлом дубля, не
/// влезшие в реплику даже с ускорением (`duration` — единственное, что пользователь может разрешить). Пусто — можно.
pub fn export_check(p: &SpeechPaths, pid: &str, project: &SpeechProject) -> Vec<Value> {
    let keys = keys(p, project);
    project
        .blocks()
        .filter_map(|b| {
            let key = keys.get(&b.id).cloned().unwrap_or_default();
            match sp::status(b, &key) {
                "skipped" => None,
                "ready" => {
                    let take = b.active.and_then(|n| b.takes.iter().find(|t| t.n == n));
                    let size = take.and_then(|t| std::fs::metadata(takes_dir(p, pid).join(&t.file)).ok()).filter(|m| m.is_file()).map(|m| m.len());
                    let state = match (take, size) {
                        (_, None) => "missing",
                        (Some(t), Some(len)) if t.bytes > 0 && t.bytes != len => "modified",
                        (Some(t), _) if t.fit_over => "duration",
                        _ => return None,
                    };
                    Some(json!({ "block": b.id, "state": state }))
                }
                state => Some(json!({ "block": b.id, "state": state })),
            }
        })
        .collect()
}

/// Share of checked takes that passed for each colour of the project's light: green from the first, amber from the second.
pub const LIGHT_GREEN: f64 = 0.95;
pub const LIGHT_AMBER: f64 = 0.8;

/// How the voiced blocks sound by the check of their takes: how many passed, failed and are not checked by recognition,
/// the light of the project, and the takes that failed, the worst first, with what was expected and what was heard.
pub fn quality(p: &SpeechPaths, project: &SpeechProject) -> Value {
    let keys = keys(p, project);
    let (mut total, mut ready, mut passed) = (0usize, 0usize, 0usize);
    let mut failed: Vec<(usize, f64, Value)> = Vec::new();
    let mut unchecked: Vec<String> = Vec::new();
    for ch in &project.chapters {
        for b in &ch.blocks {
            let key = keys.get(&b.id).cloned().unwrap_or_default();
            let state = sp::status(b, &key);
            if state == "skipped" {
                continue;
            }
            total += 1;
            let Some(take) = voiced(b, state, &key) else { continue };
            ready += 1;
            match &take.qc {
                Some(c) if c.text_checked && c.ok() => passed += 1,
                // a defect the sound check found fails the take whether or not its words were checked
                Some(c) if !c.ok() => {
                    let item = json!({
                        "block": b.id, "chapter": ch.title, "speaker": b.speaker, "take": take.n,
                        "text": if take.text.is_empty() { &b.text } else { &take.text }, "heard": c.heard, "off": c.off, "text_checked": c.text_checked,
                        "wer": c.wer, "passed": c.passed, "issues": c.issues, "notes": c.notes, "similarity": c.similarity, "similar": c.similar,
                    });
                    failed.push((c.issues.len(), c.wer, item));
                }
                _ => unchecked.push(b.id.clone()),
            }
        }
    }
    failed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(b.0.cmp(&a.0)));
    let checked = passed + failed.len();
    let light = match checked {
        0 => "none",
        _ if passed as f64 / checked as f64 >= LIGHT_GREEN => "green",
        _ if passed as f64 / checked as f64 >= LIGHT_AMBER => "amber",
        _ => "red",
    };
    json!({
        "total": total, "ready": ready, "checked": checked, "passed": passed, "failed": failed.len(),
        "unchecked": unchecked, "light": light, "green": LIGHT_GREEN, "amber": LIGHT_AMBER,
        "worst": failed.into_iter().map(|f| f.2).collect::<Vec<_>>(),
    })
}

/// The take a block sounds with: a ready block's, or that of one the check failed after its retries (the block then
/// stands as failed with the check's reason, its take still there).
fn voiced<'a>(b: &'a sp::Block, state: &str, key: &str) -> Option<&'a sp::Take> {
    let take = b.active.and_then(|n| b.takes.iter().find(|t| t.n == n))?;
    match state {
        "ready" => Some(take),
        "failed" if take.key == key && take.qc.is_some() => Some(take),
        _ => None,
    }
}

/// A failed take of block `id` mended (`take_fix`): the sentences gone wrong voiced again alone and put in their place,
/// a tail or a gap cut, or the block voiced again whole when nothing can be mended in place. The new take is checked by
/// recognition and becomes the active one when it does better than the old.
pub fn fix_take(p: &SpeechPaths, pid: &str, id: &str, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let project = crate::speech::load(&p.workspace, pid)?;
    let b = project.blocks().find(|b| b.id == id).cloned().ok_or_else(|| t!("speech-no-block", id = id.to_string()))?;
    let old = b.active.and_then(|n| b.takes.iter().find(|t| t.n == n)).cloned().ok_or_else(|| t!("speech-fix-no-take"))?;
    if !old.qc.as_ref().is_some_and(|c| c.text_checked) {
        check_takes(p, pid, &BTreeMap::from([(id.to_string(), vec![old.n])]), true, progress)?;
    }
    let project = crate::speech::load(&p.workspace, pid)?;
    let b = project.blocks().find(|b| b.id == id).cloned().ok_or_else(|| t!("speech-no-block", id = id.to_string()))?;
    let old = b.takes.iter().find(|t| t.n == old.n).cloned().ok_or_else(|| t!("speech-fix-no-take"))?;
    let qc = old.qc.clone().unwrap_or_default();
    if qc.ok() {
        mark_checked(p, pid, &[id.to_string()], &[])?;
        return Ok(json!({ "mode": "none", "take": old.n, "passed": true }));
    }
    let lang = crate::text::prep::block_lang(&b.text, project.lang_of(&b));
    let pause = stx::pieces(&b.text).iter().filter_map(|x| if let Piece::Pause(ms) = x { Some(*ms as f64 / 1000.0) } else { None }).fold(0.0, f64::max);
    let plan = crate::speech::take_fix::plan(&b.text, &lang, &qc, old.duration, pause);
    let n = if plan.whole || plan.edits.is_empty() {
        let made = voice_block(p, pid, id, 1, "patch", progress)?;
        *made.first().ok_or_else(|| t!("speech-fix-no-take"))?
    } else {
        let mut pieces = std::collections::BTreeMap::new();
        for e in &plan.edits {
            if let crate::speech::take_fix::Edit::Revoice { sentence, .. } = e {
                progress(json!({ "type": "progress", "stage": "speech_fix", "msg": t!("speech-fix-sentence", n = (*sentence + 1) as i64), "pct": 20 }));
                let mut one = b.clone();
                one.text = plan.sentences[*sentence].clone();
                let (samples, _, _) = synth_block(p, pid, &project, &one, old.n)?;
                pieces.insert(*sentence, crate::audio::trim::trim_edges(&samples, SR).0);
            }
        }
        let tmp = tempfile::Builder::new().prefix("fix-").tempdir().map_err(|e| e.to_string())?;
        let wav = tmp.path().join("take.wav");
        crate::audio::media::extract_audio(&takes_dir(p, pid).join(&old.file), &wav, SR, 1)?;
        let (x, sr) = crate::audio::wavio::read_mono_f32(&wav)?;
        let out = crate::speech::take_fix::splice(&x, sr, &plan.edits, &pieces);
        crate::speech::edit(&p.workspace, pid, |fresh| -> Result<u32, String> {
            let key = keys(p, fresh).remove(id).unwrap_or_default();
            let block = fresh.block_mut(id).ok_or_else(|| t!("speech-no-block", id = id.to_string()))?;
            let n = block.takes.iter().map(|t| t.n).max().unwrap_or(0) + 1;
            let file = format!("{id}-{n}.flac");
            let path = takes_dir(p, pid).join(&file);
            write_flac(&out, &path)?;
            let mut take = old.clone();
            take.n = n;
            take.file = file;
            take.duration = out.len() as f64 / sr as f64;
            take.bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
            take.key = key;
            take.source = "patch".into();
            take.created = sp::now();
            take.qc = None;
            take.cut = None;
            take.tidy = None;
            take.text = b.text.clone();
            block.takes.push(take);
            if block.pinned.is_none() {
                block.active = Some(n);
            }
            Ok(n)
        })??
    };
    let failed = check_takes(p, pid, &BTreeMap::from([(id.to_string(), vec![n])]), true, progress)?;
    let rank = |c: &crate::audio::qc::Check| (!c.ok(), c.wer, c.issues.len());
    let kept = crate::speech::edit(&p.workspace, pid, |fresh| {
        let block = fresh.block_mut(id)?;
        let new = block.takes.iter().find(|t| t.n == n).and_then(|t| t.qc.clone()).unwrap_or_default();
        if block.pinned.is_none() && rank(&new) > rank(&qc) {
            block.active = Some(old.n);
        }
        Some(block.active)
    })?;
    if let Some(kept) = kept {
        let still = if kept == Some(n) { failed.clone() } else { vec![id.to_string()] };
        mark_checked(p, pid, &[id.to_string()], &still)?;
        let mode = if plan.whole || plan.edits.is_empty() { "whole" } else { "patch" };
        let sentences: Vec<usize> = plan.edits.iter().filter_map(|e| if let crate::speech::take_fix::Edit::Revoice { sentence, .. } = e { Some(sentence + 1) } else { None }).collect();
        let cuts = plan.edits.iter().filter(|e| matches!(e, crate::speech::take_fix::Edit::Cut { .. })).count();
        return Ok(json!({ "mode": mode, "take": n, "active": kept, "passed": failed.is_empty(), "sentences": sentences, "cuts": cuts }));
    }
    Err(t!("speech-no-block", id = id.to_string()))
}

/// A take of block `id` by each side of a blind comparison: the engine and voice set on a copy of the block, the take
/// kept beside the others and the active one left as it was; the answer is the take of each side in order.
pub fn ab_takes(p: &SpeechPaths, pid: &str, id: &str, sides: &[sp::AbSide], progress: &dyn Fn(Value)) -> Result<Value, String> {
    if sides.len() != 2 {
        return Err(t!("speech-ab-two"));
    }
    let mut made = Vec::new();
    for (i, side) in sides.iter().enumerate() {
        if crate::engines::is_cloud(&side.engine) || crate::engines::get(&side.engine).is_none() {
            return Err(t!("speech-ab-engine", engine = side.engine.clone()));
        }
        if crate::jobs::current().is_some_and(|c| c.is_cancelled()) {
            return Err(crate::jobs::CANCELLED.to_string());
        }
        progress(json!({ "type": "progress", "stage": "speech_ab", "msg": t!("speech-ab-side", n = (i + 1) as i64), "pct": (i * 50) as i64 }));
        let mut project = crate::speech::load(&p.workspace, pid)?;
        let block = project.block_mut(id).ok_or_else(|| t!("speech-no-block", id = id.to_string()))?;
        block.engine = Some(side.engine.clone());
        if side.voice.is_some() {
            block.voice = side.voice.clone();
        }
        let b = block.clone();
        let key = keys(p, &project).remove(id).unwrap_or_default();
        let n = b.takes.iter().map(|t| t.n).max().unwrap_or(0) + 1;
        let (samples, _, mut take) = synth_block(p, pid, &project, &b, n)?;
        let file = format!("{}-{n}.flac", b.id);
        let path = takes_dir(p, pid).join(&file);
        write_flac(&samples, &path)?;
        take.bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
        take.file = file;
        take.key = key;
        take.source = "ab".into();
        crate::speech::edit(&p.workspace, pid, |fresh| match fresh.block_mut(id) {
            Some(block) => {
                block.takes.push(take);
                Ok(())
            }
            None => Err(t!("speech-no-block", id = id.to_string())),
        })??;
        made.push(n);
    }
    Ok(json!({ "takes": made }))
}

fn mark_checked(p: &SpeechPaths, pid: &str, checked: &[String], failed: &[String]) -> Result<(), String> {
    crate::speech::edit(&p.workspace, pid, |fresh| {
        let keys = keys(p, fresh);
        for id in checked {
            let key = keys.get(id).cloned().unwrap_or_default();
            let Some(block) = fresh.block_mut(id) else { continue };
            if !failed.contains(id) {
                if block.error.as_ref().is_some_and(|e| e.key == key) {
                    block.error = None;
                }
                continue;
            }
            let qc = block.active.and_then(|n| block.takes.iter().find(|t| t.n == n)).and_then(|t| t.qc.clone()).unwrap_or_default();
            let text = if qc.text_checked && !qc.passed {
                t!("speech-qc-failed", heard = qc.heard.clone())
            } else if qc.similar == Some(false) {
                t!("speech-qc-unlike", similarity = format!("{:.2}", qc.similarity.unwrap_or(0.0)))
            } else {
                t!("speech-qc-defects", list = crate::audio::qc::issue_names(&qc.issues))
            };
            block.error = Some(sp::BlockError { key, text });
        }
    })
}

/// The ready blocks' takes checked by recognition after the fact: those of `ids`, or every one not checked yet.
pub fn check_ready(p: &SpeechPaths, pid: &str, ids: Option<&[String]>, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let project = crate::speech::load(&p.workspace, pid)?;
    let keys = keys(p, &project);
    let made: BTreeMap<String, Vec<u32>> = project
        .blocks()
        .filter_map(|b| {
            let key = keys.get(&b.id).map(String::as_str).unwrap_or_default();
            let take = voiced(b, sp::status(b, key), key)?;
            let wanted = match ids {
                Some(ids) => ids.contains(&b.id),
                None => !take.qc.as_ref().is_some_and(|c| c.text_checked),
            };
            wanted.then(|| (b.id.clone(), vec![take.n]))
        })
        .collect();
    let failed = if made.is_empty() { Vec::new() } else { check_takes(p, pid, &made, true, progress)? };
    mark_checked(p, pid, &made.keys().cloned().collect::<Vec<_>>(), &failed)?;
    Ok(json!({ "checked": made.len(), "failed": failed }))
}

/// What would go wrong voicing the project, found before voicing it: block by block, an `error` stops the block from
/// being voiced as written, a `warn` voices it but not all of it as the text says.
pub fn lint(p: &SpeechPaths, project: &SpeechProject) -> Vec<Value> {
    let entries = dictionary(p, project);
    let library = crate::voices::list(&p.voices_dir).0;
    let mut out = Vec::new();
    let mut issue = |block: &str, level: &str, kind: &str, detail: Value| out.push(json!({ "block": block, "level": level, "kind": kind, "detail": detail }));
    for b in project.blocks().filter(|b| !b.hidden) {
        if crate::speech::text::visible(&b.text).trim().is_empty() {
            issue(&b.id, "error", "empty", Value::Null);
            continue;
        }
        let engine_id = project.engine_of(b);
        let Some(engine) = crate::engines::get(engine_id) else {
            issue(&b.id, "error", "engine", json!(engine_id));
            continue;
        };
        if let Some(why) = &engine.unavailable {
            issue(&b.id, "error", "engine_unavailable", json!({ "engine": engine.name, "why": why }));
        }
        let lang = crate::text::prep::block_lang(&b.text, project.lang_of(b));
        if !lang.is_empty() && !engine.languages.is_empty() && !engine.languages.contains(&lang.as_str()) {
            issue(&b.id, "error", "language", json!({ "engine": engine.name, "lang": lang }));
        }
        let voice = project.voice_of(b);
        let preset = voice.as_deref().is_some_and(|v| engine.presets.iter().any(|x| x.id == v));
        match voice.as_deref().filter(|_| !preset) {
            None if engine.needs_voice => issue(&b.id, "error", "no_voice", json!({ "engine": engine.name })),
            Some(v) if engine.clone != "none" => match library.iter().find(|x| x.name == v) {
                None => issue(&b.id, "error", "voice_missing", json!({ "voice": v })),
                Some(x) if engine.ref_text_required && x.transcript.as_deref().is_none_or(|t| t.trim().is_empty()) => issue(&b.id, "error", "ref_text", json!({ "engine": engine.name, "voice": v })),
                _ => {}
            },
            _ => {}
        }
        for (problem, markup) in crate::speech::text::markup_problems(&b.text) {
            issue(&b.id, "warn", "markup", json!({ "problem": problem, "text": markup }));
        }
        if !b.direction.trim().is_empty() && !engine.direction {
            issue(&b.id, "warn", "direction_ignored", json!({ "engine": engine.name }));
        }
        let prepared = match preparation::prepare(&b.text, &prep_opts(p, project, project.lang_of(b), engine_id, voice.as_deref(), &entries, false)) {
            Ok(x) => x,
            Err(e) => {
                issue(&b.id, "error", "prepare", json!(e));
                continue;
            }
        };
        if !prepared.ignored.unknown_tags.is_empty() {
            issue(&b.id, "warn", "tags_ignored", json!({ "engine": engine.name, "tags": prepared.ignored.unknown_tags }));
        }
        if prepared.ignored.stress || prepared.ignored.phonemes || prepared.ignored.accent {
            issue(&b.id, "warn", "marks_ignored", json!({ "engine": engine.name, "stress": prepared.ignored.stress, "phonemes": prepared.ignored.phonemes, "accent": prepared.ignored.accent }));
        }
        if !prepared.unknown.is_empty() {
            issue(&b.id, "warn", "unread", json!(prepared.unknown));
        }
        if !prepared.broken.is_empty() {
            issue(&b.id, "error", "dictionary", json!(prepared.broken.iter().map(|(rule, why)| json!({ "rule": rule, "why": why })).collect::<Vec<_>>()));
        }
    }
    out
}

pub struct ExportOpts {
    pub format: String,
    pub chapter: Option<String>,
    /// Пакет: каждый блок — свой файл с громкостью по пресету.
    pub per_block: bool,
    /// Длинная книга — частями не длиннее стольких минут (по границам глав).
    pub part_minutes: Option<u32>,
    /// A file per chapter, all levelled as one book: the loudness is set on the whole mix before it is cut, so the
    /// chapters keep their levels against each other.
    pub per_chapter: bool,
    /// Beside the file a ZIP with a track per speaker, as long as the file, with only that speaker's lines in their
    /// places and the file's own gain on them, so the tracks add up to the file.
    pub stems: bool,
    /// Экспортировать, хотя дубли не влезли в реплику по длительности.
    pub allow_duration: bool,
    pub quality: crate::audio::media::Quality,
}

/// Склеить проект в файл `exports/<имя>.<формат>`.
pub fn export(p: &SpeechPaths, pid: &str, opts: &ExportOpts, progress: &dyn Fn(Value)) -> Result<Value, String> {
    if !crate::audio::media::EXPORT_FORMATS.contains(&opts.format.as_str()) {
        return Err(t!("media-bad-format", format = opts.format.clone()));
    }
    let project = crate::speech::load(&p.workspace, pid)?;
    let problems: Vec<Value> = export_check(p, pid, &project).into_iter().filter(|x| !(opts.allow_duration && x["state"] == "duration")).collect();
    if !problems.is_empty() {
        return Err(t!("speech-export-not-ready", count = problems.len() as i64));
    }
    let blocks: Vec<&Block> = project
        .chapters
        .iter()
        .filter(|c| opts.chapter.as_ref().is_none_or(|id| &c.id == id))
        .flat_map(|c| c.blocks.iter())
        .filter(|b| !b.hidden && !b.text.trim().is_empty())
        .collect();
    if blocks.is_empty() {
        return Err(t!("speech-export-empty"));
    }
    let dir = p.workspace.join(pid);
    let work = tempfile::Builder::new().prefix("export-").tempdir_in(&dir).map_err(|e| e.to_string())?;
    if opts.per_block {
        return export_clips(p, pid, &project, &blocks, opts, work.path(), progress);
    }
    let mut mix: Vec<f32> = Vec::new();
    let mut placer = Placer::new(&project, SR);
    let mut marks: Vec<Value> = Vec::new();
    let mut prev_audio: Option<(usize, Vec<f32>)> = None;
    let mut cues: Vec<(f64, f64, String, String)> = Vec::new();
    // each block's place in the file and the words recognition timed in its take, for synced text
    let mut timed: Vec<(f64, f64, String, String, Vec<(String, f64, f64)>)> = Vec::new();
    let chapter_of: std::collections::HashMap<&str, usize> = project.chapters.iter().enumerate().flat_map(|(ci, c)| c.blocks.iter().map(move |b| (b.id.as_str(), ci))).collect();
    let mut chapter_starts: Vec<(usize, f64)> = Vec::new();
    let mut prev_end: Option<usize> = None;
    // how each block was voiced, for the recipe written next to the file
    let mut made: Vec<(f64, Value)> = Vec::new();
    let mut stems: std::collections::BTreeMap<String, Vec<f32>> = std::collections::BTreeMap::new();
    let mut prev_speaker = String::new();
    for (i, b) in blocks.iter().enumerate() {
        progress(json!({ "type": "progress", "stage": "speech_export", "msg": t!("speech-export-block", n = (i + 1) as i64, total = blocks.len() as i64), "pct": (i * 90 / blocks.len()) as i64 }));
        let take = b.active.and_then(|n| b.takes.iter().find(|t| t.n == n)).ok_or_else(|| t!("speech-export-not-ready", count = 1))?;
        let src = takes_dir(p, pid).join(&take.file);
        let wav = work.path().join(format!("{i}.wav"));
        crate::audio::media::extract_audio(&src, &wav, SR, 1)?;
        let (samples, _) = crate::audio::wavio::read_mono_f32(&wav)?;
        let samples = match take.cut {
            Some([a, z]) => {
                let a = ((a * SR as f64) as usize).min(samples.len());
                samples[a..((z * SR as f64) as usize).clamp(a, samples.len())].to_vec()
            }
            None => samples,
        };
        let tidied = project.settings.tidy_pauses.then(|| crate::audio::trim::tidy(&samples, SR));
        let place = |secs: f64| tidied.as_ref().map_or(secs, |t| t.at(secs));
        let samples = match &tidied { Some(t) => t.samples.clone(), None => samples };
        let mut samples = match project.fx_of(b) {
            Some(fx) => crate::voices::fx::apply(&samples, SR, fx)?,
            None => samples,
        };
        if project.settings.level_pieces {
            crate::audio::loudness::level_piece(&mut samples, SR);
        }
        b.mix.apply(&mut samples, SR);
        let at = placer.place(b, take.fit_lead, samples.len());
        if let Some((prev_at, prev)) = prev_audio.as_ref().filter(|(pa, pv)| b.cut_previous && at < pa + pv.len()) {
            cut_tail(&mut mix, *prev_at, prev, at);
            if let Some(track) = stems.get_mut(&prev_speaker) {
                cut_tail(track, *prev_at, prev, at);
            }
            let cut = at as f64 / SR as f64;
            if let Some(m) = marks.last_mut() {
                m["end"] = json!(cut);
            }
            if let Some(c) = cues.last_mut() {
                c.1 = cut;
            }
            if let Some(t) = timed.last_mut() {
                t.1 = cut;
                t.4.retain(|w| w.1 < cut);
            }
            prev_end = Some(at);
        }
        marks.push(json!({ "block": b.id, "start": at as f64 / SR as f64, "end": (at + samples.len()) as f64 / SR as f64 }));
        if let Some(ci) = chapter_of.get(b.id.as_str()) {
            if chapter_starts.last().is_none_or(|(last, _)| last != ci) {
                chapter_starts.push((*ci, at as f64 / SR as f64));
            }
        }
        cues.push((at as f64 / SR as f64, (at + samples.len()) as f64 / SR as f64, b.speaker.clone(), crate::speech::text::visible(&b.text)));
        made.push((at as f64 / SR as f64, recipe_block(b, take)));
        let [from, to] = take.cut.unwrap_or([0.0, f64::INFINITY]);
        let words = take.qc.as_ref().map(|q| q.timed.iter().filter(|(_, a, z)| *a >= from && *z <= to).map(|(w, a, z)| (w.clone(), at as f64 / SR as f64 + place(a - from), at as f64 / SR as f64 + place(z - from))).collect()).unwrap_or_default();
        timed.push((at as f64 / SR as f64, (at + samples.len()) as f64 / SR as f64, b.speaker.clone(), crate::speech::text::visible(&b.text), words));
        if mix.len() < at + samples.len() {
            mix.resize(at + samples.len(), 0.0);
        }
        if let Some(end) = prev_end.filter(|_| project.settings.breaths) {
            let gap = at.saturating_sub(end) as f64 / SR as f64;
            if (0.40..=1.80).contains(&gap) {
                let level = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
                let b = breath(SR, i, level);
                let margin = (0.04 * SR as f64) as usize;
                let b_at = at.saturating_sub(b.len() + margin).max(end + margin);
                for (k, s) in b.iter().enumerate().take(at.saturating_sub(b_at)) {
                    mix[b_at + k] += *s;
                }
                if opts.stems {
                    let track = stems.entry(blocks[i].speaker.clone()).or_default();
                    if track.len() < at {
                        track.resize(at, 0.0);
                    }
                    for (k, s) in b.iter().enumerate().take(at.saturating_sub(b_at)) {
                        track[b_at + k] += *s;
                    }
                }
            }
        }
        for (k, s) in samples.iter().enumerate() {
            mix[at + k] += *s;
        }
        if opts.stems {
            let track = stems.entry(b.speaker.clone()).or_default();
            if track.len() < at + samples.len() {
                track.resize(at + samples.len(), 0.0);
            }
            for (k, s) in samples.iter().enumerate() {
                track[at + k] += *s;
            }
        }
        prev_speaker = b.speaker.clone();
        prev_end = Some(at + samples.len());
        if let Some(own) = b.pause_ms.filter(|_| i + 1 == blocks.len()) {
            mix.resize(mix.len() + (SR as i64 * own.max(0) / 1000) as usize, 0.0);
        }
        prev_audio = Some((at, samples));
    }
    if !project.tracks.is_empty() && opts.chapter.is_none() {
        crate::speech::tracks::mix_in(&p.workspace, pid, &project, SR, &mut mix, work.path(), opts.stems.then_some(&mut stems))?;
    }
    progress(json!({ "type": "progress", "stage": "speech_export", "msg": t!("speech-export-loudness"), "pct": 92 }));
    let unlevelled = opts.stems.then(|| mix.clone());
    let levelled = match loudness(&project.settings.loudness) {
        Some((i, tp, _)) => Some(crate::audio::loudness::level(&mut mix, SR, i, peak_for(&opts.format, tp), project.settings.loudness != "acx")?),
        None => None,
    };
    let acx = project.settings.loudness == "acx";
    if acx {
        let head = 0.8;
        crate::speech::acx::room_tone(&mut mix, SR, head, 2.0);
        chapter_starts.iter_mut().for_each(|c| c.1 += head);
        cues.iter_mut().for_each(|c| {
            c.0 += head;
            c.1 += head;
        });
        made.iter_mut().for_each(|m| m.0 += head);
        timed.iter_mut().for_each(|t| {
            t.0 += head;
            t.1 += head;
            t.4.iter_mut().for_each(|w| {
                w.1 += head;
                w.2 += head;
            });
        });
    } else if project.settings.room_tone {
        crate::speech::acx::room_tone(&mut mix, SR, 0.0, 0.0);
    }
    let acx_report = if acx { Some(crate::speech::acx::report(&mix, SR)?) } else { None };
    if let Some(before) = &unlevelled {
        let after = &mix[if acx { (0.8 * SR as f64) as usize } else { 0 }..];
        let head = mix.len() - after.len();
        let gains = gain_envelope(before, after);
        for track in stems.values_mut() {
            track.resize(before.len(), 0.0);
            track.iter_mut().zip(&gains).for_each(|(s, g)| *s *= g);
            track.splice(0..0, std::iter::repeat_n(0.0, head));
            track.resize(mix.len(), 0.0);
        }
    }
    let exports = dir.join("exports");
    std::fs::create_dir_all(&exports).map_err(|e| format!("{}: {e}", exports.display()))?;
    let base = file_stem(&project.name, opts.chapter.as_deref().and_then(|id| project.chapters.iter().find(|c| c.id == id)).map(|c| c.title.as_str()));
    let total = mix.len() as f64 / SR as f64;
    let chapters: Vec<(f64, f64, String)> = chapter_starts
        .iter()
        .enumerate()
        .map(|(k, (ci, a))| {
            let z = chapter_starts.get(k + 1).map(|n| n.1).unwrap_or(total);
            let title = project.chapters[*ci].title.trim();
            (*a, z, if title.is_empty() { t!("speech-chapter-n", n = (k + 1) as i64) } else { title.to_string() })
        })
        .collect();
    let cuts = match opts.part_minutes.filter(|m| *m > 0 && total > *m as f64 * 60.0) {
        _ if opts.per_chapter => chapters.iter().skip(1).map(|c| c.0).collect(),
        Some(m) => part_cuts(total, m as f64 * 60.0, &chapters.iter().map(|c| c.0).collect::<Vec<_>>(), &cues.iter().map(|c| c.0).collect::<Vec<_>>()),
        None => Vec::new(),
    };
    let bounds: Vec<(f64, f64)> = std::iter::once(0.0).chain(cuts.iter().copied()).zip(cuts.iter().copied().chain(std::iter::once(total))).collect();
    let voice = project.settings.voice.clone().unwrap_or_default();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if bounds.len() > 1 && !project.settings.export_name.is_empty() {
        let keys = name_keys(&project.settings.export_name, &EXPORT_NAME_KEYS)?;
        if !keys.contains(&"n") && !(opts.per_chapter && keys.contains(&"chapter")) {
            return Err(t!("speech-name-template-same", template = project.settings.export_name.clone(), count = bounds.len() as i64));
        }
    }
    let mut outputs = Vec::new();
    let mut tag_problems: Vec<String> = Vec::new();
    for (k, (a, z)) in bounds.iter().enumerate() {
        let chapter_title = (opts.per_chapter && bounds.len() > 1).then(|| chapters[k].2.clone());
        let name = match &chapter_title {
            _ if !project.settings.export_name.is_empty() => {
                let chapter = chapter_title.clone().or_else(|| chapters.iter().find(|c| c.0 >= *a - 1e-6).map(|c| c.2.clone())).unwrap_or_default();
                fill_name(&project.settings.export_name, &[("project", &project.name), ("chapter", &chapter), ("n", &format!("{:02}", k + 1)), ("date", &today)])
            }
            Some(title) => format!("{:02} {}", k + 1, file_stem(&project.name, Some(title))),
            None if bounds.len() > 1 => format!("{base} ({})", t!("speech-part-n", n = (k + 1) as i64)),
            None => base.clone(),
        };
        let within = |s: f64| s >= *a - 1e-6 && s < *z - 1e-6;
        let part_chapters: Vec<(f64, f64, String)> = chapters.iter().filter(|c| within(c.0)).map(|(s, e, t)| (s - a, e.min(*z) - a, t.clone())).collect();
        let part_cues: Vec<(f64, f64, String, String)> = cues.iter().filter(|c| within(c.0)).map(|(s, e, w, t)| (s - a, e - a, w.clone(), t.clone())).collect();
        let part_timed: Vec<(f64, f64, String, String, Vec<(String, f64, f64)>)> = timed
            .iter()
            .filter(|t| within(t.0))
            .map(|(s, e, w, t, words)| (s - a, e - a, w.clone(), t.clone(), words.iter().map(|(x, ws, we)| (x.clone(), ws - a, we - a)).collect()))
            .collect();
        let from = (a * SR as f64) as usize;
        let to = ((z * SR as f64) as usize).min(mix.len());
        let joined = work.path().join(format!("part-{k}.wav"));
        crate::audio::wavio::write_mono_f32(&joined, &mix[from..to], SR)?;
        let file = format!("{name}.{}", opts.format);
        let target = exports.join(&file);
        let meta = crate::audio::media::AudioMeta {
            title: match &chapter_title {
                Some(title) => title.clone(),
                None if bounds.len() > 1 => format!("{} ({})", project.name, t!("speech-part-n", n = (k + 1) as i64)),
                None => project.name.clone(),
            },
            artist: project.book.as_ref().map(|b| b.author.clone()).filter(|a| !a.is_empty()).unwrap_or_else(|| voice.clone()),
            album: project.book.as_ref().map(|b| b.title.clone()).unwrap_or_default(),
            comment: format!("Speech Studio · {}", crate::engines::get(&project.settings.engine).map(|e| e.name).unwrap_or("")),
            genre: if project.book.is_some() { "Audiobook".into() } else { String::new() },
            chapters: if part_chapters.len() > 1 { part_chapters.clone() } else { Vec::new() },
            cover: project.book.as_ref().and_then(|b| b.cover.as_ref()).map(|c| dir.join(c)),
            lyrics: lrc(&part_timed),
        };
        progress(json!({ "type": "progress", "stage": "speech_export", "msg": t!("speech-export-encoding", n = (k + 1) as i64, total = bounds.len() as i64), "pct": 94 + (k * 5 / bounds.len()) as i64 }));
        match opts.format.as_str() {
            // FLAC and MP3 as the music studios write them: libFLAC 24-bit and LAME from audio-post, tags by lofty
            "flac" | "mp3" if !(opts.format == "flac" && opts.quality.bits == Some(16)) => {
                let part = mix[from..to].to_vec();
                let stereo = audio_post::Stereo::new(part.clone(), part, SR);
                let bytes = if opts.format == "mp3" {
                    let (rate, kbps) = if acx { (44_100, MP3_KBPS) } else { (opts.quality.rate.unwrap_or(48_000), opts.quality.kbps.unwrap_or(MP3_KBPS)) };
                    audio_post::encode::mp3(&stereo.resampled(rate).map_err(|e| e.to_string())?, kbps)
                } else {
                    match opts.quality.rate {
                        Some(rate) => audio_post::encode::flac(&stereo.resampled(rate).map_err(|e| e.to_string())?),
                        None => audio_post::encode::flac(&stereo),
                    }
                }
                .map_err(|e| e.to_string())?;
                std::fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
            }
            _ => crate::audio::media::encode_audio_meta(&joined, &target, &opts.format, acx, &meta, &opts.quality)?,
        }
        if crate::audio::tagging::taggable(&target) {
            let cover = meta.cover.as_ref().and_then(|c| std::fs::read(c).ok().map(|bytes| (if c.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) { "image/png" } else { "image/jpeg" }.to_string(), bytes)));
            let tags = crate::audio::tagging::TrackTags {
                title: meta.title.clone(),
                album: if meta.album.is_empty() { "Speech Studio".into() } else { meta.album.clone() },
                // as the studios name the engine the performer when nobody else is
                artist: if meta.artist.is_empty() { crate::engines::get(&project.settings.engine).map(|e| e.name.to_string()).unwrap_or_default() } else { meta.artist.clone() },
                genre: Some(meta.genre.clone()).filter(|g| !g.is_empty()),
                lyrics: Some(meta.lyrics.clone()).filter(|l| !l.is_empty()),
                bpm: None,
                cover,
            };
            // as in the studios a tag never loses the file: the problem is logged and named in the answer
            if let Err(e) = crate::audio::tagging::write_tags(&target, &tags).and_then(|()| crate::audio::tagging::write_chapters(&target, &part_chapters)) {
                tracing::error!("[ERROR] could not tag {}: {e:#}", target.display());
                tag_problems.push(format!("{file}: {e:#}"));
            }
        }
        let chapter_list = (part_chapters.len() > 1).then(|| format!("{name}.chapters.txt"));
        if let Some(list) = &chapter_list {
            let text: String = part_chapters.iter().map(|(s, _, t)| format!("{}\t{t}\n", hms(*s))).collect();
            std::fs::write(exports.join(list), text).map_err(|e| format!("{list}: {e}"))?;
        }
        let subtitles = format!("{name}.srt");
        std::fs::write(exports.join(&subtitles), srt(&part_cues)).map_err(|e| format!("{subtitles}: {e}"))?;
        let rows: Vec<crate::listening::subtitles::Row> = part_cues.iter().map(|(a, z, who, text)| crate::listening::subtitles::Row { start: *a, end: *z, speaker: who.clone(), text: text.clone(), words: Vec::new() }).collect();
        let mut subtitle_files = vec![subtitles.clone()];
        for format in ["vtt", "ass"] {
            let text = crate::listening::subtitles::render_rows(&rows, format, &crate::listening::subtitles::Opts { speakers: true, times: true, rules: None, digits: false }).unwrap_or_default();
            let file = format!("{name}.{format}");
            std::fs::write(exports.join(&file), text).map_err(|e| format!("{file}: {e}"))?;
            subtitle_files.push(file);
        }
        let table = format!("{name}.csv");
        let rows: Vec<(&(f64, f64, String, String), &(f64, Value))> = cues.iter().zip(made.iter()).filter(|(c, _)| within(c.0)).collect();
        std::fs::write(exports.join(&table), lines_csv(&rows, &chapters, *a)?).map_err(|e| format!("{table}: {e}"))?;
        let lyrics = format!("{name}.lrc");
        std::fs::write(exports.join(&lyrics), lrc(&part_timed)).map_err(|e| format!("{lyrics}: {e}"))?;
        subtitle_files.push(lyrics);
        let recipe = format!("{file}.recipe.json");
        let body = json!({
            "made_with": format!("Speech Studio {}", env!("CARGO_PKG_VERSION")),
            "project": project.name,
            "file": file,
            "export": { "format": opts.format, "quality": opts.quality, "chapter": opts.chapter, "part": (bounds.len() > 1).then_some(k + 1), "parts": bounds.len() },
            "settings": project.settings,
            "blocks": made.iter().filter(|m| within(m.0)).map(|(s, m)| { let mut m = m.clone(); m["start"] = json!(s - a); m }).collect::<Vec<_>>(),
        });
        std::fs::write(exports.join(&recipe), serde_json::to_vec_pretty(&body).map_err(|e| e.to_string())?).map_err(|e| format!("{recipe}: {e}"))?;
        let timings = format!("{name}.timings.json");
        let timed_blocks = json!(part_timed.iter().map(|(s, e, who, text, words)| json!({ "start": s, "end": e, "speaker": who, "text": text, "words": words.iter().map(|(w, ws, we)| json!({ "word": w, "start": ws, "end": we })).collect::<Vec<_>>() })).collect::<Vec<_>>());
        let body = json!({ "file": file, "blocks": timed_blocks });
        std::fs::write(exports.join(&timings), serde_json::to_vec_pretty(&body).map_err(|e| e.to_string())?).map_err(|e| format!("{timings}: {e}"))?;
        let player = format!("{name}.player.html");
        std::fs::write(exports.join(&player), crate::speech::read_along::player_html(&project.name, &file, &timed_blocks)).map_err(|e| format!("{player}: {e}"))?;
        outputs.push(json!({ "file": file, "table": table, "subtitles": subtitles, "subtitle_files": subtitle_files, "timings": timings, "player": player, "recipe": recipe, "chapters": chapter_list, "path": target.to_string_lossy(), "duration": z - a }));
    }
    let stems_zip = if opts.stems && !stems.is_empty() {
        progress(json!({ "type": "progress", "stage": "speech_export", "msg": t!("speech-export-stems", count = stems.len() as i64), "pct": 99 }));
        let zip_name = format!("{base} - {}.zip", t!("speech-by-speaker"));
        let path = exports.join(&zip_name);
        let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut zip = zip::ZipWriter::new(file);
        // FLAC is compressed already, and a track is mostly silence it squeezes well
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (k, (speaker, track)) in stems.iter().enumerate() {
            let bytes = audio_post::encode::flac(&audio_post::Stereo::new(track.clone(), track.clone(), SR)).map_err(|e| e.to_string())?;
            zip.start_file(format!("{:02} {}.flac", k + 1, file_stem(speaker, None)), options).map_err(|e| e.to_string())?;
            std::io::Write::write_all(&mut zip, &bytes).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
        Some(zip_name)
    } else {
        None
    };
    let mut built: Vec<String> = Vec::new();
    for out in &outputs {
        for key in ["file", "table", "timings", "player", "recipe", "chapters"] {
            built.extend(out[key].as_str().map(str::to_string));
        }
        built.extend(out["subtitle_files"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string));
    }
    built.extend(stems_zip.clone());
    let manifest = format!("{base}.sha256");
    std::fs::write(exports.join(&manifest), sha256_manifest(&exports, &built)?).map_err(|e| format!("{manifest}: {e}"))?;
    let first = outputs.first().cloned().unwrap_or(Value::Null);
    Ok(json!({ "manifest": manifest, "stems": stems_zip, "table": first["table"], "file": first["file"], "subtitles": first["subtitles"], "subtitle_files": first["subtitle_files"], "timings": first["timings"], "player": first["player"], "recipe": first["recipe"], "chapters": first["chapters"], "path": first["path"], "parts": if outputs.len() > 1 { Value::Array(outputs) } else { Value::Null }, "tag_problems": tag_problems, "acx": acx_report, "duration": total, "marks": marks, "lufs_before": levelled.map(|l| l.before), "lufs": levelled.map(|l| l.after) }))
}

/// How a block was voiced: the take in the file and what made it, to make it again.
fn recipe_block(b: &Block, take: &Take) -> Value {
    json!({
        "block": b.id, "speaker": b.speaker, "text": take.text, "take": take.n, "engine": take.engine, "voice": take.voice,
        "seed": take.seed, "temperature": take.temperature, "top_k": take.top_k, "top_p": take.top_p, "max_tokens": take.max_tokens,
        "direction": take.direction, "source": take.source, "wer": take.qc.as_ref().map(|q| q.wer),
    })
}

/// Where a long export is cut into parts of at most `limit` seconds: at the last chapter start that keeps a part under
/// it, else at the last block start; a single block longer than the limit stays whole.
fn part_cuts(total: f64, limit: f64, chapters: &[f64], blocks: &[f64]) -> Vec<f64> {
    let mut cuts = Vec::new();
    let mut start = 0.0;
    while total - start > limit {
        let inside = |marks: &[f64]| marks.iter().copied().filter(|m| *m > start + 1e-6 && *m <= start + limit).fold(None, |best: Option<f64>, m| Some(best.map_or(m, |b| b.max(m))));
        let cut = inside(chapters).or_else(|| inside(blocks)).or_else(|| blocks.iter().copied().find(|m| *m > start + 1e-6));
        match cut {
            Some(c) => {
                cuts.push(c);
                start = c;
            }
            None => break,
        }
    }
    cuts
}

/// Пакет: каждый блок — отдельный файл `exports/<проект>/<NNN> <имя>.<формат>`, громкость — по пресету каждого.
fn export_clips(p: &SpeechPaths, pid: &str, project: &SpeechProject, blocks: &[&Block], opts: &ExportOpts, work: &Path, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let base = file_stem(&project.name, None);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let folder = p.workspace.join(pid).join("exports").join(&base);
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut files = Vec::new();
    let mut made = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        progress(json!({ "type": "progress", "stage": "speech_export", "msg": t!("speech-export-block", n = (i + 1) as i64, total = blocks.len() as i64), "pct": (i * 100 / blocks.len()) as i64 }));
        let take = b.active.and_then(|n| b.takes.iter().find(|t| t.n == n)).ok_or_else(|| t!("speech-export-not-ready", count = 1))?;
        let wav = work.join(format!("{i}.wav"));
        crate::audio::media::extract_audio(&takes_dir(p, pid).join(&take.file), &wav, SR, 1)?;
        let (samples, _) = crate::audio::wavio::read_mono_f32(&wav)?;
        let samples = if project.settings.tidy_pauses { crate::audio::trim::tidy(&samples, SR).samples } else { samples };
        let mut samples = match project.fx_of(b) {
            Some(fx) => crate::voices::fx::apply(&samples, SR, fx)?,
            None => samples,
        };
        b.mix.apply(&mut samples, SR);
        if let Some((target, tp, _)) = loudness(&project.settings.loudness) {
            crate::audio::loudness::level(&mut samples, SR, target, peak_for(&opts.format, tp), project.settings.loudness != "acx")?;
        }
        crate::audio::wavio::write_mono_f32(&wav, &samples, SR)?;
        let label = if b.note.trim().is_empty() { crate::speech::text::visible(&b.text).chars().take(40).collect::<String>() } else { b.note.trim_end_matches(".txt").to_string() };
        let stem = if project.settings.clip_name.is_empty() {
            format!("{:03} {}", i + 1, file_stem(&label, None))
        } else {
            let chapter = project.chapters.iter().find(|c| c.blocks.iter().any(|x| x.id == b.id)).map(|c| c.title.as_str()).unwrap_or_default();
            fill_name(&project.settings.clip_name, &[("project", &project.name), ("chapter", chapter), ("n", &format!("{:03}", i + 1)), ("date", &today), ("speaker", &b.speaker), ("text", &label), ("block", &b.id)])
        };
        if files.contains(&format!("{base}/{stem}.{}", opts.format)) {
            return Err(t!("speech-name-template-same", template = project.settings.clip_name.clone(), count = blocks.len() as i64));
        }
        let name = format!("{stem}.{}", opts.format);
        let meta = crate::audio::media::AudioMeta { title: crate::speech::text::visible(&b.text), artist: project.voice_of(b).unwrap_or_default(), album: project.name.clone(), ..Default::default() };
        crate::audio::media::encode_audio_meta(&wav, &folder.join(&name), &opts.format, project.settings.loudness == "acx", &meta, &opts.quality)?;
        let mut how = recipe_block(b, take);
        how["file"] = json!(name);
        made.push(how);
        files.push(format!("{base}/{name}"));
    }
    let body = json!({
        "made_with": format!("Speech Studio {}", env!("CARGO_PKG_VERSION")),
        "project": project.name,
        "export": { "format": opts.format, "quality": opts.quality, "chapter": opts.chapter, "per_block": true },
        "settings": project.settings,
        "blocks": made,
    });
    let recipe = folder.join("recipe.json");
    std::fs::write(&recipe, serde_json::to_vec_pretty(&body).map_err(|e| e.to_string())?).map_err(|e| format!("{}: {e}", recipe.display()))?;
    Ok(json!({ "files": files, "recipe": format!("{base}/recipe.json"), "folder": folder.to_string_lossy() }))
}

fn hms(t: f64) -> String {
    let s = t.max(0.0).round() as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

fn srt_time(t: f64) -> String {
    let ms = (t.max(0.0) * 1000.0).round() as u64;
    format!("{:02}:{:02}:{:02},{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}

/// Synced text in LRC: a line per block at its start, word by word (enhanced LRC `<mm:ss.xx>`) where its take was timed.
fn lrc(timed: &[(f64, f64, String, String, Vec<(String, f64, f64)>)]) -> String {
    let at = |s: f64| {
        let cs = (s.max(0.0) * 100.0).round() as u64;
        format!("{:02}:{:02}.{:02}", cs / 6000, cs / 100 % 60, cs % 100)
    };
    timed
        .iter()
        .map(|(a, _, _, text, words)| {
            if words.is_empty() {
                format!("[{}]{}", at(*a), text.replace('\n', " "))
            } else {
                format!("[{}]{}", at(*a), words.iter().map(|(w, s, _)| format!("<{}>{w}", at(*s))).collect::<Vec<_>>().join(" "))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn srt(cues: &[(f64, f64, String, String)]) -> String {
    let many = cues.iter().map(|c| &c.2).collect::<std::collections::BTreeSet<_>>().len() > 1;
    cues.iter()
        .enumerate()
        .map(|(i, (a, z, who, text))| {
            let next = cues.get(i + 1).map(|c| c.0);
            let z = match next {
                Some(n) if n < *z && *z - n <= SUBTITLE_OVERLAP_TRIM => n.max(*a),
                _ => *z,
            };
            let line = if many { format!("{who}: {text}") } else { text.clone() };
            format!("{}\n{} --> {}\n{line}\n", i + 1, srt_time(*a), srt_time(z))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Перекрытие соседних субтитров не длиннее этого — следствие сдвига реплики в паузу, а не перебивания: конец
/// предыдущей подрезается до начала следующей, чтобы плеер не показывал две строки.
const SUBTITLE_OVERLAP_TRIM: f64 = 0.5;

/// A file name by a template: each `{key}` takes its value, the result made safe for file systems as `file_stem`.
pub fn fill_name(template: &str, values: &[(&str, &str)]) -> String {
    let filled = values.iter().fold(template.to_string(), |name, (key, value)| name.replace(&format!("{{{key}}}"), value));
    file_stem(&filled, None)
}

/// Имя файла экспорта по проекту и главе: без символов, которых не любят файловые системы.
pub fn file_stem(project: &str, chapter: Option<&str>) -> String {
    let raw = match chapter.filter(|c| !c.trim().is_empty()) {
        Some(c) => format!("{project} - {c}"),
        None => project.to_string(),
    };
    let clean: String = raw.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { ' ' } else { c }).collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean: String = clean.chars().take(100).collect();
    if clean.trim().is_empty() { "speech".into() } else { clean.trim().trim_end_matches('.').to_string() }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_long_book_is_cut_at_chapters_then_blocks() {
        // 3 h book, chapters at 0, 50, 100, 150 min; parts of at most 2 h cut at the 100 min chapter
        let ch = [0.0, 3000.0, 6000.0, 9000.0];
        let blocks: Vec<f64> = (0..108).map(|k| k as f64 * 100.0).collect();
        assert_eq!(part_cuts(10800.0, 7200.0, &ch, &blocks), vec![6000.0]);
        // no chapters: the last block start within the limit
        assert_eq!(part_cuts(10800.0, 7200.0, &[], &blocks), vec![7200.0]);
        // short enough: one part
        assert!(part_cuts(3000.0, 7200.0, &ch, &blocks).is_empty());
    }

    #[test]
    fn synced_text_is_lrc_with_words_where_timed() {
        let timed = vec![
            (1.5, 3.0, "A".to_string(), "Привет мир".to_string(), vec![("Привет".to_string(), 1.5, 2.0), ("мир".to_string(), 2.1, 2.6)]),
            (65.25, 67.0, "B".to_string(), "Без слов".to_string(), Vec::new()),
        ];
        assert_eq!(lrc(&timed), "[00:01.50]<00:01.50>Привет <00:02.10>мир\n[01:05.25]Без слов");
    }

    use super::*;

    #[test]
    fn a_short_overlap_is_trimmed_and_an_interruption_stays() {
        let cue = |a: f64, z: f64, who: &str| (a, z, who.to_string(), "x".to_string());
        let out = srt(&[cue(0.0, 3.0, "A"), cue(2.88, 5.0, "A"), cue(4.0, 6.0, "B")]);
        assert!(out.contains("00:00:00,000 --> 00:00:02,880"), "{out}");
        assert!(out.contains("00:00:02,880 --> 00:00:05,000"), "a second of overlap is an interruption: {out}");
        assert!((peak_for("mp3", -1.0) + 2.0).abs() < 1e-9 && (peak_for("flac", -1.0) + 1.0).abs() < 1e-9);
    }

    #[test]
    fn export_names_drop_what_file_systems_refuse() {
        assert_eq!(file_stem("Глава: 1/2?", None), "Глава 1 2");
        assert_eq!(file_stem("Книга", Some("Пролог")), "Книга - Пролог");
        assert_eq!(file_stem("  ", None), "speech");
    }

    #[test]
    fn speakers_tracks_take_the_mixs_gain_sample_by_sample_and_add_up_to_it() {
        let a: Vec<f32> = (0..2400).map(|i| (i as f32 * 0.05).sin() * 0.3).collect();
        let b: Vec<f32> = (0..2400).map(|i| if i > 1200 { (i as f32 * 0.11).sin() * 0.5 } else { 0.0 }).collect();
        let before: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x + y).collect();
        let mut after = before.clone();
        after.iter_mut().enumerate().for_each(|(n, s)| *s *= if n < 1500 { 2.0 } else { 0.5 });
        let g = gain_envelope(&before, &after);
        for n in 0..before.len() {
            let sum = a[n] * g[n] + b[n] * g[n];
            assert!((sum - after[n]).abs() < 1e-4, "{n}: {sum} {}", after[n]);
        }
    }

    #[test]
    fn the_lines_table_quotes_what_needs_it_and_times_lines_within_the_part() {
        let cues = [(61.5, 63.25, "Анна".to_string(), "Да, \"конечно\", приходи".to_string()), (64.0, 65.0, "Олег".to_string(), "Иду".to_string())];
        let made = [(61.5, json!({ "block": "b1", "take": 2, "engine": "higgs", "voice": "anna" })), (64.0, json!({ "block": "b2", "take": 1, "engine": "higgs", "voice": "" }))];
        let rows: Vec<_> = cues.iter().zip(made.iter()).collect();
        let chapters = [(0.0, 62.0, "Глава 1".to_string()), (62.0, 70.0, "Глава 2".to_string())];
        let bytes = lines_csv(&rows, &chapters, 60.0).unwrap();
        assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
        let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "n,start,end,chapter,speaker,text,take,engine,voice,block");
        assert_eq!(lines[1], r#"1,00:00:01.500,00:00:03.250,Глава 1,Анна,"Да, ""конечно"", приходи",2,higgs,anna,b1"#);
        assert_eq!(lines[2], "2,00:00:04.000,00:00:05.000,Глава 2,Олег,Иду,1,higgs,,b2");
    }

    #[test]
    fn a_name_template_takes_its_keys_and_refuses_what_it_cannot_name() {
        assert_eq!(name_keys("{n} {project} - {chapter}", &EXPORT_NAME_KEYS).unwrap(), vec!["n", "project", "chapter"]);
        assert!(name_keys("{speaker}", &EXPORT_NAME_KEYS).unwrap_err().contains("{speaker}"));
        assert!(name_keys("{n", &EXPORT_NAME_KEYS).is_err());
        assert!(name_keys("{speaker}_{n}", &crate::speech::naming::CLIP_NAME_KEYS).is_ok());
        assert_eq!(fill_name("{n}. {project}: {chapter}", &[("n", "02"), ("project", "Книга"), ("chapter", "Глава/2?")]), "02. Книга Глава 2");
    }

    #[test]
    fn resampling_keeps_the_length_in_time() {
        let x = vec![0.5f32; 48_000];
        assert_eq!(resample_to(x.clone(), 48_000).len(), 24_000);
        assert_eq!(resample_to(x, 24_000).len(), 48_000);
    }

    #[test]
    fn export_check_names_a_changed_take_file_and_a_take_too_long_for_its_cue() {
        let root = tempfile::tempdir().unwrap();
        let p = SpeechPaths { repo_root: root.path().into(), workspace: root.path().join("ws"), voices_dir: root.path().join("voices"), models_root: root.path().join("models") };
        let block = |id: &str| Block { id: id.into(), speaker: "A".into(), text: "Текст.".into(), ..Default::default() };
        let mut project = SpeechProject::default();
        project.chapters = vec![sp::Chapter { id: "c".into(), title: String::new(), blocks: vec![block("a"), block("b"), block("c"), block("d")] }];
        let keys = keys(&p, &project);
        let dir = takes_dir(&p, "pid");
        std::fs::create_dir_all(&dir).unwrap();
        for (i, b) in project.chapters[0].blocks.iter_mut().enumerate() {
            let file = format!("{}-1.flac", b.id);
            std::fs::write(dir.join(&file), b"flac!").unwrap();
            let bytes = match i { 1 => 9, 3 => 0, _ => 5 };
            b.takes.push(Take { n: 1, file, key: keys[&b.id].clone(), bytes, fit_over: i == 2, ..Default::default() });
            b.active = Some(1);
        }
        let states: Vec<(String, String)> = export_check(&p, "pid", &project).iter().map(|x| (x["block"].as_str().unwrap().to_string(), x["state"].as_str().unwrap().to_string())).collect();
        assert_eq!(states, vec![("b".to_string(), "modified".to_string()), ("c".to_string(), "duration".to_string())], "a take of an old project has no size and is not compared");
        std::fs::remove_file(dir.join("a-1.flac")).unwrap();
        assert_eq!(export_check(&p, "pid", &project)[0]["state"], "missing");
    }

    /// A renamed library voice stays the project's voice and its takes stay voiced; before, the project kept the old
    /// name and voicing failed with «There is no voice named …».
    #[test]
    fn a_renamed_voice_is_followed_by_its_projects_and_their_takes() {
        let root = tempfile::tempdir().unwrap();
        let p = SpeechPaths { repo_root: root.path().into(), workspace: root.path().join("ws"), voices_dir: root.path().join("voices"), models_root: root.path().join("models") };
        std::fs::create_dir_all(&p.voices_dir).unwrap();
        std::fs::write(p.voices_dir.join("Designed voice.wav"), b"RIFF0000WAVE").unwrap();
        let mut project = SpeechProject::default();
        project.settings.voice = Some("Designed voice".into());
        project.cast.insert("A".into(), "Designed voice".into());
        project.chapters = vec![sp::Chapter { id: "c".into(), title: String::new(), blocks: vec![Block { id: "a".into(), speaker: "A".into(), text: "Текст.".into(), ..Default::default() }] }];
        let before = keys(&p, &project);
        project.chapters[0].blocks[0].takes.push(Take { n: 1, key: before["a"].clone(), voice: "Designed voice".into(), ..Default::default() });
        project.chapters[0].blocks[0].active = Some(1);
        let pid = "0123456789";
        crate::speech::save(&p.workspace, pid, &project).unwrap();

        rename_voice(&p, "Designed voice", "Narrator", || crate::voices::rename(&p.voices_dir, "Designed voice", "Narrator")).unwrap();

        let after = crate::speech::load(&p.workspace, pid).unwrap();
        assert_eq!(after.settings.voice.as_deref(), Some("Narrator"));
        assert_eq!(after.cast["A"], "Narrator");
        let b = &after.chapters[0].blocks[0];
        assert_eq!(b.takes[0].voice, "Narrator");
        assert_eq!(sp::status(b, &keys(&p, &after)["a"]), "ready", "the take made with the old name is still the block's voice");
    }

    #[test]
    fn the_timeline_places_blocks_as_the_export_does_and_names_an_interruption() {
        let block = |id: &str, speaker: &str, pause: Option<i64>, secs: f64| Block {
            id: id.into(),
            speaker: speaker.into(),
            text: "Текст.".into(),
            pause_ms: pause,
            takes: vec![Take { n: 1, duration: secs, ..Default::default() }],
            active: Some(1),
            ..Default::default()
        };
        let mut project = SpeechProject::default();
        project.chapters = vec![sp::Chapter { id: "c".into(), title: String::new(), blocks: vec![block("a", "A", Some(-500), 2.0), block("b", "B", Some(300), 2.0), block("c", "B", None, 1.0)] }];
        let rows = timeline(&project);
        let at = |i: usize, k: &str| rows[i][k].as_f64().unwrap();
        assert_eq!((at(0, "start"), at(1, "start"), at(2, "start")), (0.0, 1.5, 3.8));
        assert_eq!(rows[1]["overlap"], json!({ "with": "a", "secs": 0.5 }));
        assert!(rows[2]["overlap"].is_null());
        project.chapters[0].blocks[1].cut_previous = true;
        let rows = timeline(&project);
        assert_eq!((rows[0]["end"].as_f64(), rows[1]["overlap"]["secs"].as_f64()), (Some(1.5), Some(0.5)), "the cut reply ends where the interruption starts");

        let mut mix = vec![1.0f32; 2000];
        cut_tail(&mut mix, 0, &[1.0; 2000], 1000);
        let fade = SR as usize * 30 / 1000;
        assert_eq!((mix[999], mix[1000]), (1.0, 1.0));
        assert!(mix[1000 + fade / 2] > 0.4 && mix[1000 + fade / 2] < 0.6);
        assert!(mix[1000 + fade..].iter().all(|v| v.abs() < 1e-6));
    }

    #[test]
    fn a_text_too_long_for_its_cue_is_spoken_faster_ahead() {
        let block = |id: &str, text: &str, start: f64, end: f64| Block { id: id.into(), speaker: "A".into(), text: text.into(), start: Some(start), end: Some(end), ..Default::default() };
        let mut project = SpeechProject::default();
        project.settings.timeline = true;
        project.settings.lang = "ru".into();
        project.chapters = vec![sp::Chapter {
            id: "c".into(),
            title: String::new(),
            blocks: vec![block("a", "Да.", 0.0, 2.0), block("b", "Очень длинная реплика, которая никак не помещается в одну секунду эфира.", 2.0, 3.0), block("c", "Нет.", 3.0, 4.0)],
        }];
        assert_eq!(tempo_ahead(&project, &project.chapters[0].blocks[0]), None);
        let faster = tempo_ahead(&project, &project.chapters[0].blocks[1]).expect("too long for one second");
        assert!(faster > 1.0 && faster <= SPEED_MAX, "{faster}");
        project.settings.timeline = false;
        assert_eq!(tempo_ahead(&project, &project.chapters[0].blocks[1]), None, "without cues there is nothing to fit");
    }

    #[test]
    fn loudness_presets_are_the_documented_ones() {
        assert_eq!(loudness("video"), Some((-14.0, -1.0, 11.0)));
        assert_eq!(loudness("acx").map(|l| l.1), Some(-3.0));
        assert_eq!(loudness("off"), None);
    }
}
