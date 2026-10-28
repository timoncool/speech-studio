//! Задачи Listening. Распознавание — конвейер анализа Dub: запись в 16 кГц моно, при желании голос отдельно от музыки
//! (Roformer), диаризация (Sortformer; при заданном числе спикеров — с WeSpeaker), распознаватель (Parakeet, Whisper
//! или OpenRouter), реплика режется на слове смены спикера, галлюцинации помечаются и скрываются по голосу на
//! интервале, обрывки одной фразы склеиваются, словарь исправлений правит слова. Монтаж по тексту собирает новую
//! запись без вырезанного и новый проект с теми же строками.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use dub_asr::{hallucination_kind, HallucinationRules};
use serde_json::{json, Value};

use crate::listening::project as lp;
use crate::listening::project::{Fix, Line, ListenProject, Made, Word};
use crate::app::Localize;

#[derive(Clone)]
pub struct ListenPaths {
    pub repo_root: PathBuf,
    pub workspace: PathBuf,
    pub models_root: PathBuf,
    pub sortformer_onnx: PathBuf,
}

/// Доля интервала реплики со звуком, начиная с которой похожая на галлюцинацию реплика считается сказанной.
const VOICE_MIN_COVER: f64 = 0.3;

enum Engine {
    Local(crate::engines::models::AsrChoice, &'static str),
    Cloud(String),
    Fal(String),
}

/// The recogniser a recording in `lang` gets: the project's, else the studio's, and Whisper for a language the studio's
/// transcribe.cpp model does not know (then `true`).
fn picked_engine(p: &ListenPaths, project: &ListenProject, lang: &str) -> (String, bool) {
    let sel = crate::engines::models::load_selection(&p.models_root);
    let studio = sel.get("asr_engine").and_then(Value::as_str).unwrap_or("parakeet").to_string();
    match project.settings.engine.as_str() {
        "" => match crate::engines::tcpp_asr::chosen(&studio, sel.get("asr").and_then(Value::as_str)) {
            Some(m) if !crate::engines::tcpp_asr::knows(m, lang) => ("whisper".to_string(), true),
            _ => (studio, false),
        },
        e => (e.to_string(), false),
    }
}

/// `engine_for` after the local recogniser it picks is downloaded, if it was not.
fn ready_engine(p: &ListenPaths, project: &ListenProject, lang: &str, progress: &dyn Fn(Value)) -> Result<Engine, String> {
    let (picked, _) = picked_engine(p, project, lang);
    if !matches!(picked.as_str(), "fal" | "openrouter") {
        crate::app::recognition::ensure(&p.repo_root, &p.models_root, &picked, &|ev| progress(ev))?;
    }
    engine_for(p, project, lang)
}

fn engine_for(p: &ListenPaths, project: &ListenProject, lang: &str) -> Result<Engine, String> {
    let mut sel = crate::engines::models::load_selection(&p.models_root);
    let (picked, for_lang) = picked_engine(p, project, lang);
    if picked == "fal" {
        let model = project.settings.cloud_model.trim().to_string();
        if model.is_empty() {
            return Err(t!("cloud-no-model-chosen", provider = "fal"));
        }
        return Ok(Engine::Fal(model));
    }
    if picked == "openrouter" {
        let model = if project.settings.cloud_model.trim().is_empty() { crate::engines::models::openrouter_model(&p.models_root, "asr") } else { project.settings.cloud_model.trim().to_string() };
        if model.is_empty() {
            return Err(t!("cloud-asr-no-model"));
        }
        return Ok(Engine::Cloud(model));
    }
    if let Some(obj) = sel.as_object_mut() {
        obj.insert("asr_engine".into(), json!(picked));
    }
    let choice = crate::engines::models::resolve_asr_choice(&p.repo_root, &p.models_root, &sel);
    match &choice {
        crate::engines::models::AsrChoice::CrisperWhisper { .. } => Ok(Engine::Local(choice, "whisper")),
        _ if picked == "whisper" => Err(if for_lang { t!("listen-whisper-for-lang", lang = lang.to_string()) } else { t!("listen-whisper-missing") }),
        crate::engines::models::AsrChoice::Ggml { build, model, .. } => {
            if !crate::engines::tcpp_asr::installed(&p.models_root, *build, model) {
                return Err(t!("listen-gguf-missing", engine = model.name));
            }
            if !crate::engines::tcpp_asr::knows(model, lang) {
                return Err(t!("listen-gguf-lang", engine = model.name, lang = lang.to_string()));
            }
            Ok(Engine::Local(choice.clone(), if model.id == "nemotron" { "nemotron" } else { "parakeet" }))
        }
        crate::engines::models::AsrChoice::Server { model, .. } => {
            if !dub_asr::tcpp::model_path(&p.models_root, model).is_file() {
                return Err(t!("listen-gguf-missing", engine = model.name));
            }
            if !crate::engines::tcpp_asr::knows(model, lang) {
                return Err(t!("listen-gguf-lang", engine = model.name, lang = lang.to_string()));
            }
            Ok(Engine::Local(choice.clone(), if model.id == "nemotron" { "nemotron" } else { "parakeet" }))
        }
    }
}

fn step(progress: &dyn Fn(Value), msg: String, pct: i64) {
    progress(json!({ "type": "progress", "stage": "listen", "msg": msg, "pct": pct }));
}

fn active_spans(wav: &Path) -> Result<Vec<(f64, f64)>, String> {
    let (samples, sr) = crate::audio::wavio::read_mono_f32(wav)?;
    let cfg = dub_asr::WindowConfig::default();
    let (env, frame_sec) = dub_asr::speech_envelope(&samples, sr, cfg.frame_sec);
    Ok(dub_asr::detect_active_spans(&env, frame_sec, &cfg))
}

fn covered(spans: &[(f64, f64)], start: f64, end: f64) -> bool {
    let dur = (end - start).max(1e-3);
    spans.iter().map(|&(a, b)| (b.min(end) - a.max(start)).max(0.0)).sum::<f64>() / dur >= VOICE_MIN_COVER
}

/// Фильтр галлюцинаций Dub: текстовое правило находит кандидата, голос на интервале решает. По отделённому голосу
/// и диаризации звук — это голос, и кандидат остаётся помеченным; по сырой записи звук может быть музыкой, поэтому
/// сильный признак (титр, звук в скобках, текст без букв) скрывается по тексту. В тишине — скрывается всегда.
fn filter(lines: &mut [Line], rules: HallucinationRules, evidence: &str, spans: &[(f64, f64)]) -> usize {
    let mut hidden = 0;
    for l in lines.iter_mut() {
        let Some(kind) = hallucination_kind(&l.text, rules) else { continue };
        l.flag = Some(kind.as_str().to_string());
        l.evidence = Some(evidence.to_string());
        let hide = !covered(spans, l.start, l.end) || (evidence == "mix" && kind.is_strong());
        if hide {
            l.hidden = true;
            hidden += 1;
        }
    }
    hidden
}

/// Склейка обрывков одной фразы (как в Dub): тот же спикер, зазор меньше 0,35 с (перекрытие до 0,2 с), одна из
/// частей короче 1,6 с или из одного слова, вместе не длиннее реплики и 200 знаков. Помеченные фильтром не клеятся.
fn merge_short(lines: Vec<Line>) -> Vec<Line> {
    const GAP: f64 = 0.35;
    const OVERLAP: f64 = 0.2;
    const SHORT: f64 = 1.6;
    const MAX_CH: usize = 200;
    let mut out: Vec<Line> = Vec::with_capacity(lines.len());
    for l in lines {
        if let Some(last) = out.last_mut() {
            let one_word = |x: &Line| x.text.split_whitespace().count() <= 1;
            let gap = l.start - last.end;
            let short = last.end - last.start < SHORT || l.end - l.start < SHORT || one_word(last) || one_word(&l);
            if last.flag.is_none()
                && l.flag.is_none()
                && last.speaker == l.speaker
                && gap > -OVERLAP
                && gap < GAP
                && short
                && l.end - last.start <= dub_asr::SEG_MAX_DUR
                && last.text.chars().count() + l.text.chars().count() < MAX_CH
            {
                last.text = format!("{} {}", last.text.trim_end(), l.text.trim_start()).trim().to_string();
                last.end = last.end.max(l.end);
                last.words.extend(l.words);
                last.approx |= l.approx;
                continue;
            }
        }
        out.push(l);
    }
    out
}

fn bare(w: &str) -> String {
    w.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase().replace('ё', "е")
}

/// Исправления словаря по словам: «как услышал» (одно или несколько слов подряд) → «как правильно»; знаки
/// препинания по краям сохраняются, время нового слова — от первого до последнего заменённого.
pub fn apply_fixes(words: &mut Vec<Word>, fixes: &[Fix]) -> usize {
    let mut n = 0;
    for f in fixes {
        let heard: Vec<String> = f.heard.split_whitespace().map(bare).filter(|w| !w.is_empty()).collect();
        if heard.is_empty() || f.say.trim().is_empty() {
            continue;
        }
        let mut i = 0;
        while i + heard.len() <= words.len() {
            if (0..heard.len()).all(|k| bare(&words[i + k].text) == heard[k]) {
                let first = &words[i].text;
                let last = &words[i + heard.len() - 1].text;
                let lead: String = first.chars().take_while(|c| !c.is_alphanumeric()).collect();
                let tail: String = last.chars().rev().take_while(|c| !c.is_alphanumeric()).collect::<Vec<_>>().into_iter().rev().collect();
                let end = words[i + heard.len() - 1].end;
                let cut = words[i..i + heard.len()].iter().all(|w| w.cut);
                words[i] = Word { text: format!("{lead}{}{tail}", f.say.trim()), start: words[i].start, end, cut };
                words.drain(i + 1..i + heard.len());
                n += 1;
            }
            i += 1;
        }
    }
    n
}

fn join_words(words: &[Word]) -> String {
    dub_asr::join_text(words.iter().map(|w| w.text.as_str()))
}

/// Отделённый голос записи в 16 кГц (Roformer); ошибка — явная, без тихого перехода на сырую запись.
fn separate(p: &ListenPaths, src: &Path, work: &Path) -> Result<PathBuf, String> {
    crate::engines::tts_host::make_room();
    let sel = crate::engines::models::load_selection(&p.models_root);
    let cli = crate::engines::models::sep_cli(&p.repo_root, &p.models_root)?;
    let model = crate::engines::models::resolve_sep(&p.models_root, &sel);
    if !cli.is_file() || !model.is_file() {
        return Err(t!("listen-separate-missing"));
    }
    let hq = work.join("mix-44k.wav");
    crate::audio::media::extract_audio(src, &hq, 44_100, 2)?;
    let stems = work.join("stems");
    tracing::info!("separation: {}", cli.display());
    let res = dub_sep::separate(&hq, &stems, &cli, &model).map_err(|e| e.localize())?;
    let clean = work.join("vocals16.wav");
    crate::audio::media::to_16k_mono(&res.vocals, &clean)?;
    let _ = std::fs::remove_file(&hq);
    Ok(clean)
}

/// Голос и остальное файла отдельно (Roformer) — FLAC в `out`: vocals.flac и instrumental.flac.
pub fn separate_to(repo_root: &Path, models_root: &Path, src: &Path, out: &Path, progress: &dyn Fn(Value)) -> Result<Value, String> {
    crate::engines::tts_host::make_room();
    crate::app::ensure_job_components(repo_root, models_root, false, false, &|ev| progress(ev))?;
    crate::app::ensure_components(repo_root, &crate::engines::models::sep_components(models_root)?, &|ev| progress(ev))?;
    let sel = crate::engines::models::load_selection(models_root);
    let cli = crate::engines::models::sep_cli(repo_root, models_root)?;
    let model = crate::engines::models::resolve_sep(models_root, &sel);
    if !cli.is_file() || !model.is_file() {
        return Err(t!("listen-separate-missing"));
    }
    step(progress, t!("listen-step-separate"), 10);
    let work = tempfile::Builder::new().prefix("separate-").tempdir().map_err(|e| e.to_string())?;
    let hq = work.path().join("mix-44k.wav");
    crate::audio::media::extract_audio(src, &hq, 44_100, 2)?;
    tracing::info!("separation: {} ({})", cli.display(), crate::engines::models::server_choice(models_root, "sep_backend"));
    let res = dub_sep::separate(&hq, &work.path().join("stems"), &cli, &model).map_err(|e| e.localize())?;
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let vocals = out.join("vocals.flac");
    let rest = out.join("instrumental.flac");
    crate::audio::media::encode_audio(&res.vocals, &vocals, "flac", false)?;
    crate::audio::media::encode_audio(&res.instrumental, &rest, "flac", false)?;
    step(progress, t!("listen-step-done", lines = 0), 100);
    Ok(json!({ "done": true, "vocals": vocals, "instrumental": rest }))
}

fn diarize(p: &ListenPaths, wav: &Path, speakers: u8) -> Result<Vec<dub_asr::Turn>, String> {
    if speakers == 1 {
        return Ok(Vec::new());
    }
    let build = crate::engines::acpp::build_for(crate::engines::models::server_choice(&p.models_root, "diar_backend"))?;
    let server = crate::engines::acpp::server(&p.repo_root, &p.models_root, build, crate::engines::acpp::DIAR)?;
    tracing::info!("diarization: Nemotron 3 Diarization on audiocpp_server {}", crate::engines::models::build_name(build));
    let diarizer = dub_asr::Diarizer::Server { server, model: crate::engines::acpp::DIAR };
    let turns = (|| match speakers {
        0 => dub_asr::turns(wav, &diarizer, 0.8, 2.5).map_err(|e| e.localize()),
        n => {
            let mut embedder = dub_faces::VoiceEmbedder::load(&dub_faces::wespeaker_path(&p.models_root)).map_err(|e| e.to_string())?;
            let mut embed = |samples: &[f32]| embedder.embed_samples(samples).map_err(|e| e.to_string());
            dub_asr::turns_with_speaker_count(wav, &diarizer, n as usize, &mut embed).map_err(|e| e.localize())
        }
    })();
    crate::engines::acpp::unload(&[crate::engines::acpp::DIAR]);
    Ok(turns?.turns)
}

/// Распознать по кускам, на которые запись режут паузы (Silero VAD): тишина не идёт в распознаватель, а длинная
/// запись не грузится в него целиком.
#[allow(clippy::too_many_arguments)]
fn recognise_by_pauses(p: &ListenPaths, choice: &crate::engines::models::AsrChoice, wav: &Path, work: &Path, lang: &str, vad: &crate::engines::vad::VadConfig, progress: &dyn Fn(Value)) -> Result<Vec<dub_asr::Segment>, String> {
    let lacking = crate::engines::vad::missing(&p.models_root);
    crate::app::ensure_components(&p.repo_root, &lacking.iter().map(String::as_str).collect::<Vec<_>>(), &|ev| progress(ev))?;
    let (samples, sr) = crate::audio::wavio::read_mono_f32(wav)?;
    if sr as usize != crate::engines::vad::SR {
        return Err(format!("{}: {sr} Hz, the speech finder takes {} Hz", wav.display(), crate::engines::vad::SR));
    }
    let probs = crate::engines::vad::probabilities(&p.models_root, &samples)?;
    let spans = crate::engines::vad::speech(&probs, samples.len(), vad);
    let windows = crate::engines::vad::windows(&spans, &probs, crate::engines::vad::MAX_WINDOW_SECS);
    if windows.is_empty() {
        return Err(t!("listen-no-speech", threshold = format!("{:.2}", vad.threshold)));
    }
    let mut asr = crate::engines::asr::build_engine(choice, None)?;
    let mut all = Vec::new();
    for (k, &(a, b)) in windows.iter().enumerate() {
        step(progress, t!("listen-step-asr-piece", n = (k + 1) as i64, total = windows.len() as i64), 45 + (k * 40 / windows.len().max(1)) as i64);
        let piece = work.join("piece.wav");
        let (from, to) = ((a * sr as f64) as usize, ((b * sr as f64) as usize).min(samples.len()));
        crate::audio::wavio::write_mono_f32(&piece, &samples[from..to], sr)?;
        for mut s in asr.transcribe(&piece, lang).map_err(|e| e.localize())? {
            s.start += a;
            s.end += a;
            for w in &mut s.words {
                w.start += a;
                w.end += a;
            }
            all.push(s);
        }
    }
    Ok(all)
}

#[allow(clippy::too_many_arguments)]
fn recognise(p: &ListenPaths, engine: &Engine, wav: &Path, work: &Path, s: &lp::Settings, turns: &[dub_asr::Turn], prompt: Option<String>, progress: &dyn Fn(Value)) -> Result<Vec<Line>, String> {
    let lang = if s.lang.is_empty() { "auto" } else { s.lang.as_str() };
    let lines = match engine {
        Engine::Local(choice, _) => {
            std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(&p.models_root, "asr_backend"));
            // the server's models window a long recording themselves; transcribe.cpp's get it in pieces cut at pauses
            let segments = match choice {
                crate::engines::models::AsrChoice::CrisperWhisper { .. } | crate::engines::models::AsrChoice::Server { .. } => {
                    crate::engines::asr::build_engine(choice, prompt)?.transcribe(wav, lang).map_err(|e| e.localize())?
                }
                crate::engines::models::AsrChoice::Ggml { .. } => recognise_by_pauses(p, choice, wav, work, lang, &s.vad, progress)?,
            };
            dub_asr::split_at_speaker_turns(segments, turns)
                .into_iter()
                .map(|(s, spk)| {
                    let words: Vec<Word> = s.words.iter().map(|w| Word { text: w.word.trim().to_string(), start: w.start, end: w.end, cut: false }).filter(|w| !w.text.is_empty()).collect();
                    let text = if words.is_empty() { s.text.trim().to_string() } else { join_words(&words) };
                    let approx = words.is_empty();
                    let words = if approx { lp::spread_words(&text, s.start, s.end) } else { words };
                    Line { start: s.start, end: s.end, speaker: spk.max(0).to_string(), text, words, approx, ..Default::default() }
                })
                .collect()
        }
        Engine::Fal(model) => {
            let packed = work.join("upload.mp3");
            crate::audio::media::encode_audio(wav, &packed, "mp3", false)?;
            let handle = work.join("cloud-asr.json");
            let pid = work.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let run = crate::app::remote::Run { repo_root: &p.repo_root, models_root: &p.models_root, project: &pid, task: vs_providers::Task::Transcribe, handle_file: &handle };
            let (text, _) = crate::app::remote::transcribe(&run, model, &packed, lang)?;
            let end = crate::audio::media::duration(wav)?;
            vec![Line { start: 0.0, end, speaker: "0".into(), words: lp::spread_words(text.trim(), 0.0, end), text: text.trim().to_string(), approx: true, ..Default::default() }]
        }
        Engine::Cloud(model) => {
            let key = crate::engines::models::openrouter_key().ok_or_else(|| t!("cloud-asr-no-key"))?;
            crate::app::costs::check_limit(&p.repo_root, &p.models_root, None)?;
            let packed = work.join("upload.mp3");
            crate::audio::media::encode_audio(wav, &packed, "mp3", false)?;
            let audio = std::fs::read(&packed).map_err(|e| format!("{}: {e}", packed.display()))?;
            let client = dub_llm::openrouter::OpenRouter::new(Some(key)).map_err(|e| t!("cloud-asr-failed", error = format!("{e:#}")))?;
            let heard = client.transcribe(model, &audio, "mp3", lang).map_err(|e| t!("cloud-asr-failed", error = format!("{e:#}")))?;
            let pid = work.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            crate::app::costs::record(&p.repo_root, crate::app::costs::Spent { at: chrono::Utc::now().timestamp_millis(), day: chrono::Local::now().format("%Y-%m-%d").to_string(), project: pid, provider: "openrouter".into(), model: model.clone(), task: "transcribe".into(), cost_source: "unknown".into(), origin: crate::app::current_actor(), ..Default::default() });
            let mut segments: Vec<(f64, f64, String)> = heard.segments.into_iter().map(|s| (s.start, s.end, s.text)).collect();
            if segments.is_empty() {
                if heard.text.trim().is_empty() {
                    return Err(t!("cloud-asr-empty"));
                }
                let end = heard.duration.unwrap_or_else(|| crate::audio::media::duration(wav).unwrap_or(0.0));
                segments.push((0.0, end.max(0.1), heard.text));
            }
            segments
                .into_iter()
                .map(|(start, end, text)| {
                    let spk = if turns.is_empty() { 0 } else { dub_asr::speaker_for_overlap(start, end, turns).max(0) };
                    Line { start, end, speaker: spk.to_string(), words: lp::spread_words(text.trim(), start, end), text: text.trim().to_string(), approx: true, ..Default::default() }
                })
                .collect()
        }
    };
    Ok(lines)
}

/// Распознать запись проекта заново (прежний транскрипт заменяется; имена спикеров по ключам сохраняются).
pub fn transcribe(p: &ListenPaths, pid: &str, progress: &dyn Fn(Value)) -> Result<Value, String> {
    crate::engines::tts_host::make_room();
    let mut project = crate::listening::load(&p.workspace, pid)?;
    let src = crate::listening::source_path(&p.workspace, pid, &project)?;
    let mut s = project.settings.clone();
    let mut engine = ready_engine(p, &project, &s.lang, progress)?;
    let diar = s.speakers != 1;
    crate::app::ensure_job_components(&p.repo_root, &p.models_root, s.speakers > 1, diar, &|ev| progress(ev))?;
    let mut extra = Vec::new();
    if let Engine::Local(crate::engines::models::AsrChoice::Ggml { build, .. }, _) = &engine {
        extra.extend([build.dir(), "silero-vad"]);
    }
    if s.denoise {
        extra.push("dpdfnet");
    }
    crate::app::ensure_components(&p.repo_root, &extra, &|ev| progress(ev))?;
    if diar && !p.sortformer_onnx.is_file() {
        return Err(t!("listen-diar-missing"));
    }
    let work = crate::listening::project_dir(&p.workspace, pid).join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    step(progress, t!("listen-step-audio"), 2);
    let src = if s.denoise {
        step(progress, t!("listen-step-denoise"), 4);
        let out = work.join("denoised48.wav");
        let limit = (s.denoise_db > 0.0).then_some(s.denoise_db as f32);
        crate::engines::denoise::denoise_file(&p.models_root, &src, &out, limit, &|f| step(progress, t!("listen-step-denoise"), 4 + (f * 4.0) as i64))?;
        out
    } else {
        src
    };
    let mix = work.join("audio16.wav");
    crate::audio::media::to_16k_mono(&src, &mix)?;
    let studio_model = match &engine {
        Engine::Local(crate::engines::models::AsrChoice::Ggml { model, .. } | crate::engines::models::AsrChoice::Server { model, .. }, _)
            if matches!(s.lang.as_str(), "" | "auto") =>
        {
            Some(*model)
        }
        _ => None,
    };
    if let Some(model) = studio_model {
        crate::app::ensure_components(&p.repo_root, &[crate::engines::models::tcpp_build(&p.models_root).dir(), "langid"], &|ev| progress(ev))?;
        if let Some(lang) = crate::app::recognition::spoken_language(&p.models_root, &mix, model.languages)? {
            step(progress, t!("listen-step-langid", lang = lang.to_uppercase()), 7);
            engine = ready_engine(p, &project, &lang, progress)?;
            s.lang = lang;
        }
    }
    let clean = if s.separate {
        step(progress, t!("listen-step-separate"), 8);
        crate::app::ensure_components(&p.repo_root, &crate::engines::models::sep_components(&p.models_root)?, &|ev| progress(ev))?;
        Some(separate(p, &src, &work)?)
    } else {
        None
    };
    let wav = clean.clone().unwrap_or_else(|| mix.clone());
    let turns = if diar {
        step(progress, t!("listen-step-diarize"), 30);
        diarize(p, &wav, s.speakers)?
    } else {
        Vec::new()
    };
    step(progress, t!("listen-step-asr"), 45);
    let mut lines = recognise(p, &engine, &wav, &work, &s, &turns, lp::whisper_prompt(&project), progress)?;
    let mut hidden = 0;
    if s.filter {
        step(progress, t!("listen-step-filter"), 85);
        let rules = if matches!(engine, Engine::Local(_, "whisper")) { HallucinationRules::Whisper } else { HallucinationRules::CaseAware };
        let (evidence, spans) = if let Some(c) = &clean {
            ("vocals", active_spans(c)?)
        } else if !turns.is_empty() {
            ("diarization", turns.iter().map(|t| (t.start, t.end)).collect())
        } else {
            ("mix", active_spans(&mix)?)
        };
        hidden = filter(&mut lines, rules, evidence, &spans);
    }
    let mut lines = merge_short(lines);
    for (i, l) in lines.iter_mut().enumerate() {
        l.id = format!("l{i}");
        if !project.fixes.is_empty() && apply_fixes(&mut l.words, &project.fixes) > 0 {
            l.text = join_words(&l.words);
        }
    }
    let mut speakers = BTreeMap::new();
    for l in &lines {
        if !speakers.contains_key(&l.speaker) {
            let n = l.speaker.parse::<i64>().unwrap_or(0) + 1;
            let name = project.speakers.get(&l.speaker).cloned().unwrap_or_else(|| t!("listen-speaker-n", n = n));
            speakers.insert(l.speaker.clone(), name);
        }
    }
    let by = match &engine {
        Engine::Local(crate::engines::models::AsrChoice::CrisperWhisper { .. }, _) => "whisper:crisperwhisper-2.0-turbo".to_string(),
        Engine::Local(_, tag) => tag.to_string(),
        Engine::Cloud(model) => format!("openrouter:{model}"),
        Engine::Fal(model) => format!("fal:{model}"),
    };
    project.duration = crate::audio::media::duration(&mix).unwrap_or(project.duration);
    project.speakers = speakers;
    project.made = Some(Made { by, at: crate::platform::time::unix_seconds(), lang: s.lang.clone(), separated: clean.is_some(), hidden });
    let count = lines.len();
    project.lines = lines;
    crate::listening::save(&p.workspace, pid, &project)?;
    step(progress, t!("listen-step-done", lines = count as i64), 100);
    Ok(json!({ "pid": pid, "lines": count, "speakers": project.speakers.len(), "hidden": hidden }))
}

/// Монтаж по тексту: запись без отмеченных слов — новый проект Listening с FLAC-записью и теми же строками на
/// сдвинутых таймкодах. Исходный проект и его запись не меняются.
pub fn cut(p: &ListenPaths, pid: &str, progress: &dyn Fn(Value)) -> Result<Value, String> {
    let project = crate::listening::load(&p.workspace, pid)?;
    let src = crate::listening::source_path(&p.workspace, pid, &project)?;
    let cuts = project.cut_ranges();
    if cuts.is_empty() {
        return Err(t!("listen-nothing-marked"));
    }
    step(progress, t!("listen-step-cut", n = cuts.len() as i64), 10);
    let work = crate::listening::project_dir(&p.workspace, pid).join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let wav = work.join("audio16.wav");
    if !wav.is_file() {
        crate::audio::media::to_16k_mono(&src, &wav)?;
    }
    let (samples, sr) = crate::audio::wavio::read_mono_f32(&wav)?;
    let cuts: Vec<(f64, f64)> = cuts.iter().map(|&(a, b)| snap_cut(&samples, sr, a, b)).collect();
    let cuts = lp::merge_ranges(cuts);
    let total = crate::audio::media::duration(&src)?;
    let keep = lp::keep_ranges(&cuts, total);
    if keep.is_empty() {
        return Err(t!("listen-cut-everything"));
    }
    let new_pid = crate::platform::ids::new_id();
    let dir = crate::listening::project_dir(&p.workspace, &new_pid);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let out = dir.join("source.flac");
    if let Err(e) = crate::audio::media::keep_audio_ranges(&src, &out, &keep) {
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    let mut edited = ListenProject::new(&t!("listen-cut-name", name = project.name.clone()), "source.flac", &project.name);
    edited.duration = keep.iter().map(|(a, b)| b - a).sum();
    edited.settings = project.settings.clone();
    edited.speakers = project.speakers.clone();
    edited.fixes = project.fixes.clone();
    edited.made = project.made.clone();
    for l in &project.lines {
        let words: Vec<Word> = l.words.iter().filter(|w| !w.cut).map(|w| Word { text: w.text.clone(), start: lp::remap(w.start, &cuts), end: lp::remap(w.end, &cuts), cut: false }).collect();
        if words.is_empty() && !l.words.is_empty() {
            continue;
        }
        let (start, end) = match (words.first(), words.last()) {
            (Some(a), Some(b)) => (a.start, b.end),
            _ => (lp::remap(l.start, &cuts), lp::remap(l.end, &cuts)),
        };
        let text = if words.len() == l.words.len() { l.text.clone() } else { join_words(&words) };
        edited.lines.push(Line { id: format!("l{}", edited.lines.len()), start, end, text, words, ..l.clone() });
    }
    crate::listening::save(&p.workspace, &new_pid, &edited)?;
    step(progress, t!("listen-step-cut-done"), 100);
    Ok(json!({ "pid": new_pid, "removed": cuts.iter().map(|(a, b)| b - a).sum::<f64>() }))
}

/// Окно поиска тихой точки вокруг метки границы: метки слов у распознавателя чаще запаздывают, поэтому назад
/// окно шире.
const SNAP_BEFORE: f64 = 0.25;
const SNAP_AFTER: f64 = 0.05;
const SNAP_FRAME: f64 = 0.01;
/// Насколько тише звука на метке должно быть место, куда сдвигается граница.
const SNAP_QUIETER_DB: f64 = 6.0;

/// Середина самого тихого участка записи в [t - before, t + after] (RMS по 10 мс; участок — кадры не громче
/// минимума на 1 дБ подряд); из нескольких таких — ближайший к метке. Нет места заметно тише метки — сама метка.
fn quietest(samples: &[f32], sr: u32, t: f64, before: f64, after: f64) -> f64 {
    let frame = ((sr as f64 * SNAP_FRAME) as usize).max(1);
    let total = samples.len() as f64 / sr.max(1) as f64;
    let (lo, hi) = ((t - before).max(0.0), (t + after).min(total));
    let mut frames: Vec<(f64, f64)> = Vec::new();
    let mut at = lo;
    while at + SNAP_FRAME <= hi + 1e-9 {
        let i = ((at * sr as f64) as usize).min(samples.len());
        let chunk = &samples[i..(i + frame).min(samples.len())];
        if !chunk.is_empty() {
            let energy = chunk.iter().map(|x| x * x).sum::<f32>() as f64 / chunk.len() as f64;
            frames.push((at + SNAP_FRAME / 2.0, 10.0 * (energy + 1e-12).log10()));
        }
        at += SNAP_FRAME;
    }
    let Some(floor) = frames.iter().map(|f| f.1).reduce(f64::min) else { return t };
    let at_label = frames.iter().min_by(|x, y| (x.0 - t).abs().total_cmp(&(y.0 - t).abs())).map_or(floor, |f| f.1);
    if floor > at_label - SNAP_QUIETER_DB {
        return t;
    }
    let mut best: Option<(f64, f64)> = None;
    let mut k = 0;
    while k < frames.len() {
        if frames[k].1 > floor + 1.0 {
            k += 1;
            continue;
        }
        let first = k;
        while k + 1 < frames.len() && frames[k + 1].1 <= floor + 1.0 {
            k += 1;
        }
        let mid = (frames[first].0 + frames[k].0) / 2.0;
        if best.is_none_or(|(_, d)| (mid - t).abs() < d) {
            best = Some((mid, (mid - t).abs()));
        }
        k += 1;
    }
    best.map_or(t, |(m, _)| m)
}

/// Границы выреза — в тишину рядом с метками слов, чтобы не оставались хвосты слов и не съедались соседние.
fn snap_cut(samples: &[f32], sr: u32, a: f64, b: f64) -> (f64, f64) {
    let a2 = quietest(samples, sr, a, SNAP_BEFORE, SNAP_AFTER);
    let b2 = quietest(samples, sr, b, SNAP_BEFORE, SNAP_AFTER);
    if b2 - a2 > SNAP_FRAME * 2.0 { (a2, b2) } else { (a, b) }
}

/// Проверка озвучки по эталонному тексту: что распознано против того, что задумано, по словам.
/// Выравнивание по известному тексту: слова эталона берут время услышанных слов (`word_align` Dub), строки — по
/// предложениям эталона, спикер строки — тот, чьи строки распознавания больше всего с ней пересекаются.
pub fn align_to_text(project: &ListenProject, reference: &str) -> Vec<Line> {
    let heard: Vec<(String, f64, f64)> = project.shown().flat_map(|l| l.words.iter().filter(|w| !w.cut).map(|w| (w.text.clone(), w.start, w.end))).collect();
    let written: Vec<&str> = reference.split_whitespace().collect();
    let end = project.duration.max(heard.last().map_or(0.0, |w| w.2));
    let times = dub_captions::word_align::align_words(&written, &heard, heard.first().map_or(0.0, |w| w.1), end);
    let mut lines: Vec<Line> = Vec::new();
    let mut words: Vec<Word> = Vec::new();
    for (k, (w, (a, z))) in written.iter().zip(times).enumerate() {
        words.push(Word { text: w.to_string(), start: a, end: z, cut: false });
        let closes = w.ends_with(['.', '!', '?', '…']) || w.ends_with(".»") || w.ends_with("!»") || w.ends_with("?»") || k + 1 == written.len();
        if closes {
            let (start, stop) = (words[0].start, words.last().map_or(a, |x| x.end));
            let speaker = project
                .shown()
                .map(|l| (l.speaker.as_str(), (l.end.min(stop) - l.start.max(start)).max(0.0)))
                .fold(std::collections::BTreeMap::<&str, f64>::new(), |mut m, (s, o)| {
                    *m.entry(s).or_default() += o;
                    m
                })
                .into_iter()
                .max_by(|x, y| x.1.total_cmp(&y.1))
                .map_or_else(|| "0".to_string(), |(s, _)| s.to_string());
            lines.push(Line { id: format!("l{}", lines.len()), start, end: stop, speaker, text: join_words(&words), words: std::mem::take(&mut words), ..Default::default() });
        }
    }
    lines
}

pub fn check_reference(project: &ListenProject, reference: &str) -> Result<Value, String> {
    let heard = dub_asr::join_text(project.shown().map(|l| l.text.as_str()));
    let (reference, heard) = crate::audio::qc::said_alike(reference, &heard, &crate::text::prep::block_lang(reference, &project.settings.lang))?;
    let e = crate::audio::qc::normalize(&reference);
    let h = crate::audio::qc::normalize(&heard);
    let ops = crate::audio::qc::word_diff(&e, &h);
    let errors = ops.iter().filter(|o| o.op != "same").count();
    let wer = if e.is_empty() { if h.is_empty() { 0.0 } else { 1.0 } } else { errors as f64 / e.len() as f64 };
    Ok(json!({ "words": e.len(), "wer": wer, "ops": ops }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_text_takes_the_times_of_the_heard_words_sentence_by_sentence() {
        let line = |speaker: &str, words: &[(&str, f64, f64)]| Line {
            start: words[0].1,
            end: words.last().unwrap().2,
            speaker: speaker.into(),
            text: words.iter().map(|w| w.0).collect::<Vec<_>>().join(" "),
            words: words.iter().map(|w| Word { text: w.0.into(), start: w.1, end: w.2, cut: false }).collect(),
            ..Default::default()
        };
        let project = ListenProject {
            duration: 6.0,
            lines: vec![line("0", &[("привет", 0.5, 0.9), ("гит", 1.0, 1.2), ("хаб", 1.2, 1.5)]), line("1", &[("как", 3.0, 3.3), ("дела", 3.4, 3.9)])],
            ..Default::default()
        };
        let lines = align_to_text(&project, "Привет, GitHub! Как дела?");
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[0].text.as_str(), lines[0].speaker.as_str()), ("Привет, GitHub!", "0"));
        assert_eq!((lines[1].text.as_str(), lines[1].speaker.as_str()), ("Как дела?", "1"));
        assert!((lines[0].start - 0.5).abs() < 1e-6 && (lines[1].words[0].start - 3.0).abs() < 1e-6, "{lines:?}");
    }

    /// A line's text joins its words as the language writes: Chinese without spaces, Russian with them.
    #[test]
    fn a_line_joins_its_words_as_the_language_writes() {
        let words = |ws: &[&str]| ws.iter().map(|w| Word { text: (*w).into(), start: 0.0, end: 0.0, cut: false }).collect::<Vec<_>>();
        assert_eq!(join_words(&words(&["今天", "天", "气", "很好", ","])), "今天天气很好,");
        assert_eq!(join_words(&words(&["Победа", "пришла", "в", "1945", "году", "."])), "Победа пришла в 1945 году.");
    }

    fn word(text: &str, start: f64, end: f64) -> Word {
        Word { text: text.into(), start, end, cut: false }
    }

    #[test]
    fn fixes_replace_one_or_several_words_and_keep_punctuation() {
        let mut words = vec![word("Это", 0.0, 0.3), word("джи", 0.3, 0.5), word("пи", 0.5, 0.7), word("ти,", 0.7, 0.9), word("ага.", 1.0, 1.2)];
        let fixes = [Fix { heard: "джи пи ти".into(), say: "GPT".into() }, Fix { heard: "ага".into(), say: "да".into() }];
        assert_eq!(apply_fixes(&mut words, &fixes), 2);
        assert_eq!(join_words(&words), "Это GPT, да.");
        assert!((words[1].start - 0.3).abs() < 1e-9 && (words[1].end - 0.9).abs() < 1e-9);
    }

    #[test]
    fn fragments_of_one_phrase_merge_but_flagged_lines_stay() {
        let line = |t: &str, s: f64, e: f64, spk: &str| Line { text: t.into(), start: s, end: e, speaker: spk.into(), ..Default::default() };
        let merged = merge_short(vec![line("If", 0.0, 0.3, "0"), line("they find you", 0.32, 1.4, "0"), line("Run!", 1.5, 1.8, "1")]);
        assert_eq!(merged.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), vec!["If they find you", "Run!"]);
        let mut flagged = line("Субтитры сделал DimaTorzok", 2.0, 2.5, "0");
        flagged.flag = Some("credit".into());
        assert_eq!(merge_short(vec![line("Да", 1.5, 1.8, "0"), flagged]).len(), 2);
    }

    #[test]
    fn cut_edges_move_into_the_nearest_quiet_spot() {
        let sr = 16_000u32;
        let mut x = vec![0.5f32; sr as usize * 2];
        for v in &mut x[(0.62 * sr as f64) as usize..(0.66 * sr as f64) as usize] {
            *v = 0.0;
        }
        let (a, b) = snap_cut(&x, sr, 0.76, 1.5);
        assert!((a - 0.64).abs() < 0.011, "a word label late by 0.1 s snaps to the middle of the pause: {a}");
        assert!((b - 1.5).abs() < 0.06, "no quiet spot nearby keeps the edge near its label: {b}");
    }

    #[test]
    fn silent_hallucinations_hide_and_voiced_ones_only_get_flagged() {
        let line = |t: &str, s: f64, e: f64| Line { text: t.into(), start: s, end: e, ..Default::default() };
        let mut lines = vec![line("Продолжение следует...", 0.0, 2.0), line("Субтитры сделал DimaTorzok", 5.0, 7.0), line("Привет всем", 8.0, 9.0)];
        let hidden = filter(&mut lines, HallucinationRules::CaseAware, "vocals", &[(0.0, 2.0), (8.0, 9.0)]);
        assert_eq!(hidden, 1);
        assert!(lines[1].hidden && !lines[0].hidden && !lines[2].hidden);
        assert!(lines[1].flag.is_some());
    }
}
