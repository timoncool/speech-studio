//! Модель озвучки на весь процесс — Higgs Audio или Drama 3, одна на видеокарте: загрузка при первом синтезе,
//! выгрузка по команде (перед другой тяжёлой моделью). Синтез — в отдельном потоке под сторожем: отмена задачи и
//! таймаут доходят до движка, зависший вызов DLL называется прямо. У Higgs нет своего потолка длины, поэтому
//! `max_tokens` считается по ожидаемой длине фразы; дефектный дубль (разнос, обрыв, тишина, гул) повторяется
//! с другой температурой, потому что сид движок может не учитывать.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};


const GUARD_GRACE_SECS: u64 = 20;
pub const ENGINE_STUCK: &str = "ENGINE_STUCK";
/// Higgs v3 in audio.cpp counts `max_tokens` in codec frames of 40 ms (model_specs/higgs_audio_tts.json).
const FRAMES_PER_SEC: f64 = 25.0;
/// Frames the delay of the codebooks adds after the last sound.
const CODEBOOK_DELAY: u32 = 8;
/// Движок принимает сид как небольшое беззнаковое целое.
pub const SEED_RANGE: u64 = 1 << 31;

/// Модель на видеокарте: одна на процесс — Higgs или Drama 3 (Drama 3 занимает почти всю карту 24 ГБ).
#[derive(Clone)]
pub enum Tts {
    Higgs(Arc<crate::engines::acpp::Higgs>),
    Drama(Arc<drama::DramaEngine>),
    Supertonic(Arc<crate::engines::supertonic::Model>),
    Qwen(Arc<crate::engines::qwen_tts::Engine>),
}

impl Tts {
    /// A Supertonic phrase is a few graph runs of well under a second and has no stop of its own: the guard waits it out.
    fn cancel(&self) {
        match self {
            Tts::Higgs(e) => e.cancel(),
            Tts::Drama(e) => e.cancel(),
            Tts::Supertonic(_) => {}
            Tts::Qwen(e) => e.cancel(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Tts::Higgs(_) => "Higgs Audio v3",
            Tts::Drama(_) => "Drama 3",
            Tts::Supertonic(_) => "Supertonic 3",
            Tts::Qwen(_) => "Qwen3-TTS",
        }
    }

    fn in_use(&self) -> bool {
        match self {
            Tts::Higgs(e) => Arc::strong_count(e) > 1,
            Tts::Drama(e) => Arc::strong_count(e) > 1,
            Tts::Supertonic(e) => Arc::strong_count(e) > 1,
            Tts::Qwen(e) => Arc::strong_count(e) > 1,
        }
    }
}

static HOST: Mutex<Option<(String, Tts)>> = Mutex::new(None);
/// When the voicing model was last taken for a phrase: the idle watcher unloads it after the set minutes.
static LAST_USE: Mutex<Option<Instant>> = Mutex::new(None);
/// A card this small does not hold the voicing model next to recognition or separation.
const SHARED_VRAM: u64 = 12 * 1024 * 1024 * 1024;

fn host() -> std::sync::MutexGuard<'static, Option<(String, Tts)>> {
    HOST.lock().unwrap_or_else(|p| p.into_inner())
}

fn touch() {
    *LAST_USE.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
}

/// Room for another heavy model (recognition, separation, similarity): on a card under 12 GB the voicing model goes.
pub fn make_room() {
    if crate::engines::hw::total_vram() < SHARED_VRAM {
        unload();
    }
}

/// Unloads the voicing model once it has not been used for `idle`; one voicing a phrase now (its engine held by the
/// synthesis) stays. True when it went.
pub fn unload_if_idle(idle: Duration) -> bool {
    let last = *LAST_USE.lock().unwrap_or_else(|p| p.into_inner());
    if !last.is_some_and(|at| at.elapsed() >= idle) {
        return false;
    }
    let mut slot = host();
    let busy = match slot.as_ref() {
        None => return false,
        Some((_, e)) => e.in_use(),
    };
    if busy {
        return false;
    }
    *slot = None;
    true
}

/// Watches the voicing model and unloads it after the idle minutes of the setting `tts_idle_unload_min` (0 never).
pub fn watch_idle(models_root: PathBuf) {
    std::thread::Builder::new()
        .name("tts-idle".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(30));
            let minutes = crate::engines::models::load_selection(&models_root).get("tts_idle_unload_min").and_then(|v| v.as_str()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(15);
            if minutes > 0 && unload_if_idle(Duration::from_secs(minutes * 60)) {
                tracing::info!("the voicing model was unloaded after {minutes} min idle");
            }
        })
        .expect("tts idle watcher thread");
}

/// Higgs на сервере audio.cpp в выбранном кванте; другой квант или другая модель озвучки выгружаются.
pub fn higgs(repo_root: &Path, models_root: &Path) -> Result<Tts, String> {
    let key = format!("{}:{}", crate::engines::acpp::HIGGS, crate::engines::acpp::higgs_gguf(models_root).display());
    let mut slot = host();
    if let Some((k, Tts::Higgs(e))) = slot.as_ref() {
        if *k == key {
            touch();
            return Ok(Tts::Higgs(e.clone()));
        }
    }
    *slot = None;
    let gpu = crate::engines::hw::gpu_report();
    if gpu.nvidia && gpu.cuda_build.is_none() {
        let card = gpu.summary().unwrap_or_else(|| "-".to_string());
        return Err(match gpu.reason {
            Some("gpu_old") => t!("render-higgs-gpu-old", card = card, min = gpu.min_compute.clone()),
            Some("driver_old") => t!("render-higgs-driver-old", card = card, min = gpu.min_driver),
            _ => t!("render-higgs-no-gpu", card = card),
        });
    }
    let e = Arc::new(crate::engines::acpp::Higgs::load(repo_root, models_root)?);
    tracing::info!("voicing: Higgs Audio v3 {} on audiocpp_server {:?}", crate::engines::acpp::higgs_gguf(models_root).display(), e.backend());
    *slot = Some((key, Tts::Higgs(e.clone())));
    touch();
    Ok(Tts::Higgs(e))
}

/// Строк промпта Drama 3 на одну фразу: реф (~22 кадра на секунду) + текст + сгенерированное.
const DRAMA_MAX_LEN: u32 = 2048;

/// Загруженная Drama 3: только на CUDA; не та машина (см. `drama_unavailable`) или нет файлов — ошибка с причиной,
/// Higgs её не подменяет. Видеопамяти берёт не больше лимита из настроек: не входит целиком — эксперты в оперативной
/// памяти; другой лимит — перезагрузка. cuBLAS она находит в models/higgs-engine, который при старте кладётся в PATH.
pub fn drama(models_root: &Path) -> Result<Tts, String> {
    let limit = crate::engines::models::drama_vram_limit_mb(models_root);
    let want = format!("{}:{limit}", crate::engines::models::DRAMA_TTS);
    let mut slot = host();
    if let Some((key, Tts::Drama(e))) = slot.as_ref() {
        if *key == want {
            touch();
            return Ok(Tts::Drama(e.clone()));
        }
    }
    *slot = None;
    if let Some(why) = crate::engines::models::drama_unavailable(models_root) {
        return Err(why);
    }
    if !crate::engines::models::drama_installed(models_root) {
        return Err(t!("render-drama-missing"));
    }
    let e = drama::DramaEngine::load_limited(&crate::engines::models::drama_dll(models_root), &[], &models_root.join(crate::engines::models::DRAMA_TTS), DRAMA_MAX_LEN, limit)
        .map_err(|e| drama_load_error(&e.to_string(), limit))?;
    let e = Arc::new(e);
    tracing::info!("voicing: Drama 3 on CUDA, VRAM limit {limit} MB (0: auto)");
    *slot = Some((want, Tts::Drama(e.clone())));
    touch();
    Ok(Tts::Drama(e))
}

/// The expert cache gets what the limit leaves after Drama's own weights: a hand limit at or under them, or a card other
/// programs filled while Drama loaded, leaves it nothing, and the engine's numbers alone do not say which setting to move.
fn drama_load_error(error: &str, limit_mb: u32) -> String {
    if !error.contains("not enough VRAM for the expert cache") {
        t!("render-drama-load-failed", error = error.to_string())
    } else if limit_mb > 0 {
        t!("render-drama-vram-limit-low", gb = (limit_mb / 1024).to_string(), error = error.to_string())
    } else {
        t!("render-drama-vram-busy", error = error.to_string())
    }
}


/// Supertonic 3 на сервере audio.cpp, на сборке устройства озвучки (`tts_backend`); другое устройство — перезагрузка.
pub fn supertonic(repo_root: &Path, models_root: &Path) -> Result<Tts, String> {
    let build = crate::engines::acpp::build_for(crate::engines::models::server_choice(models_root, "tts_backend"))?;
    let key = format!("{}:{}", crate::engines::supertonic::ENGINE, crate::engines::models::build_name(build));
    let mut slot = host();
    if let Some((k, Tts::Supertonic(e))) = slot.as_ref() {
        if *k == key {
            touch();
            return Ok(Tts::Supertonic(e.clone()));
        }
    }
    *slot = None;
    if !crate::engines::supertonic::installed(models_root) {
        return Err(t!("render-supertonic-missing"));
    }
    let e = Arc::new(crate::engines::supertonic::Model::load(repo_root, models_root).map_err(|e| t!("render-supertonic-load-failed", error = e))?);
    tracing::info!("voicing: Supertonic 3 on audiocpp_server {:?}", e.backend());
    *slot = Some((key, Tts::Supertonic(e.clone())));
    touch();
    Ok(Tts::Supertonic(e))
}


/// Загруженный Qwen3-TTS выбранного кванта: CustomVoice для голосов модели, Base для клона (`clone`), Base с
/// адаптером русских ударений (`stress`); другой квант, другой чекпоинт или другой движок выгружаются. На сборке
/// устройства озвучки; другое устройство — после перезапуска, с причиной.
pub fn qwen(models_root: &Path, clone: bool, stress: bool) -> Result<Tts, String> {
    let quant = crate::engines::models::qwen_quant(models_root);
    let talker = match (clone, stress) {
        (_, true) => "base+ru-stress",
        (true, false) => "base",
        (false, false) => "custom",
    };
    let device = crate::engines::qwen_tts::build(models_root)?;
    let key = format!("{}:{talker}:{quant}:{device:?}", crate::engines::qwen_tts::ENGINE);
    let mut slot = host();
    if let Some((k, Tts::Qwen(e))) = slot.as_ref() {
        if *k == key {
            touch();
            return Ok(Tts::Qwen(e.clone()));
        }
    }
    *slot = None;
    if !crate::engines::qwen_tts::installed(models_root, &quant, clone || stress, stress) {
        return Err(match (clone, stress) {
            (_, true) => t!("render-qwen-stress-missing"),
            (true, false) => t!("render-qwen-clone-missing"),
            (false, false) => t!("render-qwen-missing"),
        });
    }
    let build = qwen_build(models_root)?;
    let e = Arc::new(crate::engines::qwen_tts::Engine::load(models_root, build, &quant, clone || stress, stress).map_err(|e| t!("render-qwen-load-failed", error = e))?);
    tracing::info!("voicing: Qwen3-TTS {talker} {quant} on {} ({})", crate::engines::qwen_tts::device(build), e.version());
    *slot = Some((key, Tts::Qwen(e.clone())));
    touch();
    Ok(Tts::Qwen(e))
}

/// The build of Qwen for the voicing device, once the computer can run it: the processor's floor, and the CUDA 13 card
/// and driver for that build.
fn qwen_build(models_root: &Path) -> Result<acpp_server::asset::Build, String> {
    let build = crate::engines::qwen_tts::build(models_root)?;
    if build == acpp_server::asset::Build::Cuda13 {
        cuda13_ready()?;
    }
    Ok(build)
}

fn cuda13_ready() -> Result<(), String> {
    let gpu = crate::engines::hw::gpu_report();
    if gpu.cuda13_ok {
        return Ok(());
    }
    let card = gpu.summary().unwrap_or_else(|| "-".to_string());
    Err(match gpu.reason {
        Some("gpu_old") => t!("render-higgs-gpu-old", card = card, min = gpu.min_compute.clone()),
        Some("driver_old") => t!("render-higgs-driver-old", card = card, min = gpu.min_driver),
        _ => t!("render-higgs-no-gpu", card = card),
    })
}

/// Qwen3-TTS VoiceDesign: голос по описанию, одна модель на процесс вместо прочих.
pub fn qwen_design(models_root: &Path) -> Result<Tts, String> {
    let quant = crate::engines::models::qwen_quant(models_root);
    let device = crate::engines::qwen_tts::build(models_root)?;
    let key = format!("{}:design:{quant}:{device:?}", crate::engines::qwen_tts::ENGINE);
    let mut slot = host();
    if let Some((k, Tts::Qwen(e))) = slot.as_ref() {
        if *k == key {
            touch();
            return Ok(Tts::Qwen(e.clone()));
        }
    }
    *slot = None;
    if !crate::engines::qwen_tts::design_installed(models_root, &quant) {
        return Err(t!("voice-design-missing"));
    }
    let build = qwen_build(models_root)?;
    let e = Arc::new(crate::engines::qwen_tts::Engine::load_design(models_root, build, &quant).map_err(|e| t!("render-qwen-load-failed", error = e))?);
    tracing::info!("voicing: Qwen3-TTS voice design {quant} on {}", crate::engines::qwen_tts::device(build));
    *slot = Some((key, Tts::Qwen(e.clone())));
    touch();
    Ok(Tts::Qwen(e))
}

/// Выгрузить модель озвучки и освободить видеопамять.
pub fn unload() {
    if let Some((_, e)) = host().take() {
        e.cancel();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Params {
    pub temperature: f64,
    pub top_k: u32,
    pub top_p: f64,
    /// `fixed` — всегда `seed`; `inc` / `dec` — `seed` плюс / минус номер дубля; `random` — новый на каждый синтез.
    pub seed_mode: String,
    pub seed: u64,
    /// Потолок токенов ответа; 0 — по длине текста.
    pub max_tokens: u32,
    /// Шаги потока и скорость речи Supertonic.
    pub steps: u32,
    pub speed: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            temperature: 1.0,
            top_k: 50,
            top_p: 0.95,
            seed_mode: "random".into(),
            seed: 42,
            max_tokens: 0,
            steps: crate::engines::supertonic::DEFAULT_STEPS,
            speed: crate::engines::supertonic::DEFAULT_SPEED,
        }
    }
}

impl Params {
    pub fn checked(&self) -> Result<(), String> {
        if !(0.05..=2.0).contains(&self.temperature) {
            return Err(format!("temperature {} is outside 0.05..2.0", self.temperature));
        }
        if !(1..=500).contains(&self.top_k) {
            return Err(format!("top_k {} is outside 1..500", self.top_k));
        }
        if !(0.05..=1.0).contains(&self.top_p) {
            return Err(format!("top_p {} is outside 0.05..1.0", self.top_p));
        }
        if self.seed >= SEED_RANGE {
            return Err(format!("seed {} is outside 0..{SEED_RANGE}", self.seed));
        }
        if !matches!(self.seed_mode.as_str(), "fixed" | "random" | "inc" | "dec") {
            return Err(format!("seed_mode must be fixed, inc, dec or random, not {}", self.seed_mode));
        }
        if self.max_tokens > crate::engines::MAX_TOKENS_LIMIT {
            return Err(format!("max_tokens {} is above {}", self.max_tokens, crate::engines::MAX_TOKENS_LIMIT));
        }
        if !(1..=32).contains(&self.steps) {
            return Err(format!("steps {} is outside 1..32", self.steps));
        }
        if !(0.5..=2.0).contains(&self.speed) {
            return Err(format!("speed {} is outside 0.5..2.0", self.speed));
        }
        Ok(())
    }

    /// The sampling defaults of an engine's manifest, as the window sets them when the engine is chosen.
    pub fn engine_defaults(&mut self, engine: &crate::engines::Engine) {
        for spec in &engine.params {
            match spec.key {
                "temperature" => self.temperature = spec.default,
                "top_k" => self.top_k = spec.default as u32,
                "top_p" => self.top_p = spec.default,
                "max_tokens" => self.max_tokens = spec.default as u32,
                "steps" => self.steps = spec.default as u32,
                "speed" => self.speed = spec.default,
                _ => {}
            }
        }
    }
}

/// Сколько секунд обычно звучит текст на этом языке (для потолка токенов и сторожа).
pub fn expected_secs(text: &str, lang: &str) -> f64 {
    let chars = text.chars().filter(|c| c.is_alphanumeric()).count() as f64;
    (chars / dub_core::fit::table_cps(pace_lang(text, lang))).max(0.6)
}

/// The language a phrase is paced by: the one it is set to, else what its letters say — ideographs Chinese, kana
/// Japanese, Hangul Korean (a sign of theirs is a syllable, two to three times a Russian letter), any other Russian.
pub fn pace_lang<'a>(text: &str, lang: &'a str) -> &'a str {
    if !lang.is_empty() && lang != "auto" {
        return lang;
    }
    let (mut han, mut kana, mut hangul, mut other) = (0usize, 0usize, 0usize, 0usize);
    for c in text.chars().filter(|c| c.is_alphanumeric()) {
        match c as u32 {
            0x3040..=0x30ff => kana += 1,
            0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff => han += 1,
            0xac00..=0xd7af => hangul += 1,
            _ => other += 1,
        }
    }
    if kana > 0 && kana + han > other {
        "ja"
    } else if han > other {
        "zh"
    } else if hangul > other {
        "ko"
    } else {
        "ru"
    }
}

/// Seconds of sound past which a phrase of `chars` letters and digits is a runaway.
fn runaway_secs(chars: usize, lang: &str) -> f64 {
    let lang = if lang.is_empty() || lang == "auto" { "ru" } else { lang };
    chars as f64 * RUNAWAY_PER_CHAR * (RUNAWAY_CPS / dub_core::fit::table_cps(lang)).max(1.0) + RUNAWAY_SLACK
}

/// Higgs stops a second past the runaway length: what it says by then is judged a runaway and voiced again, and a
/// phrase read at any pace short of that is not cut.
fn token_cap(chars: usize, lang: &str) -> u32 {
    ((((runaway_secs(chars, lang) + 1.0) * FRAMES_PER_SEC).ceil() as u32) + CODEBOOK_DELAY).clamp(64, 2048)
}

/// Секунд звука на знак, выше которых фраза считается разнесённой (HiggsAudio-Studio: 0,13 с на знак + 3 с).
const RUNAWAY_PER_CHAR: f64 = 0.13;
const RUNAWAY_SLACK: f64 = 3.0;
/// Знаков в секунду у языка, на котором подобран предел разноса (русский): у языков, где знак звучит дольше
/// (иероглиф — слог), предел на знак растёт во столько же раз.
const RUNAWAY_CPS: f64 = 13.0;
/// Доля отсчётов у полной шкалы, с которой звук считается клиппированным (open-tts-eval).
const CLIP_LEVEL: f32 = 0.999;
const CLIP_SHARE: f64 = 0.001;
/// Щелчок в конце: наибольший скачок последних 30 мс к уровню предыдущих 200 мс (open-tts-eval, порог 4).
const CLICK_RATIO: f64 = 4.0;

/// Скачок в последних 30 мс относительно RMS 200 мс перед ними.
fn tail_click(samples: &[f32], sr: usize) -> f64 {
    let tail = sr * 30 / 1000;
    let body = sr * 200 / 1000;
    if samples.len() < tail + body + 1 {
        return 0.0;
    }
    let end = samples.len();
    let jump = samples[end - tail - 1..end].windows(2).map(|w| (w[1] - w[0]).abs() as f64).fold(0.0, f64::max);
    let before = &samples[end - tail - body..end - tail];
    let rms = (before.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / body as f64).sqrt();
    if rms < 1e-4 { if jump > 0.05 { f64::INFINITY } else { 0.0 } } else { jump / rms }
}

/// Дефект синтеза по сэмплам: разнос, обрыв, тишина, гул (пороги Dub, подобраны на 149 клипах QC-прогона),
/// клиппинг и щелчок в конце (open-tts-eval).
pub fn defect(samples: &[f32], sr: i32, chars: usize, lang: &str) -> Option<&'static str> {
    if sr <= 0 || samples.is_empty() {
        return Some("silence");
    }
    let srn = sr as usize;
    let dur = samples.len() as f64 / srn as f64;
    if chars >= 1 && dur > runaway_secs(chars, lang) {
        return Some("runaway");
    }
    if chars >= 4 && dur < chars as f64 * 0.045 {
        return Some("cutoff");
    }
    let clipped = samples.iter().filter(|v| v.abs() >= CLIP_LEVEL).count() as f64 / samples.len() as f64;
    if clipped >= CLIP_SHARE {
        return Some("clipping");
    }
    let w = srn / 40;
    if w == 0 || samples.len() < w * 4 {
        return None;
    }
    let rms: Vec<f64> = samples.chunks_exact(w).map(|c| (c.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / w as f64).sqrt()).collect();
    let peak = rms.iter().cloned().fold(0.0f64, f64::max);
    if peak < 0.02 {
        return Some("silence");
    }
    let silent = rms.iter().filter(|&&r| r < peak * 0.0316).count() as f64 / rms.len() as f64;
    let trough = rms.iter().cloned().filter(|&r| r > 1e-9).fold(peak, f64::min);
    let range = 20.0 * (peak / trough.max(1e-9)).log10();
    if range < 16.0 && silent < 0.03 {
        return Some("hum");
    }
    if tail_click(samples, srn) > CLICK_RATIO {
        return Some("click");
    }
    None
}

pub fn defect_name(kind: &str) -> String {
    match kind {
        "runaway" => t!("render-defect-runaway"),
        "cutoff" => t!("render-defect-cutoff"),
        "silence" => t!("render-defect-silence"),
        "hum" => t!("render-defect-hum"),
        "clipping" => t!("render-defect-clipping"),
        "click" => t!("render-defect-click"),
        other => other.to_string(),
    }
}

pub struct Synth {
    pub samples: Vec<f32>,
    pub sr: i32,
    pub seed: u64,
    pub temperature: f64,
    pub defects: Vec<&'static str>,
    pub seconds: f64,
}

pub struct Request<'a> {
    pub text: &'a str,
    pub lang: &'a str,
    pub ref_wav: Option<&'a Path>,
    pub ref_text: Option<&'a str>,
    /// Голос движка без клона (Supertonic, Qwen3-TTS).
    pub preset: Option<&'a str>,
    /// Как говорить — указание движку, который его читает (Qwen3-TTS).
    pub instruct: Option<&'a str>,
    pub params: &'a Params,
    /// Сид случайного режима: вызывающий даёт разный на каждый дубль.
    pub fresh_seed: u64,
    /// Куда отдавать звук по мере синтеза, если движок умеет потоком.
    pub live: Option<Arc<crate::speech::stream::Phrase>>,
}

/// One attempt at a phrase, owned by the guarded synthesis thread.
struct Call {
    text: String,
    lang: String,
    ref_wav: Option<PathBuf>,
    ref_text: Option<String>,
    preset: Option<String>,
    instruct: Option<String>,
    opts: String,
    drama: drama::Params,
    max_tokens: u32,
    steps: u32,
    speed: f64,
    seed: u64,
}

fn run_guarded(engine: &Tts, call: Call, timeout: Duration, live: Option<Arc<crate::speech::stream::Phrase>>) -> Result<(Vec<f32>, i32), String> {
    let (tx, rx) = mpsc::channel();
    let eng = engine.clone();
    std::thread::spawn(move || {
        let Call { text: t, lang, ref_wav: rw, ref_text: rt, preset, instruct, opts: op, drama: dp, max_tokens, steps, speed, seed } = call;
        // an engine that does not stream gives the window the whole take once it is done
        let mut streamed = false;
        let r = match (&eng, rw) {
            (Tts::Higgs(e), rw) => {
                streamed = live.is_some();
                let sr = e.sample_rate();
                serde_json::from_str::<serde_json::Map<String, Value>>(&op).map_err(|e| e.to_string()).and_then(|opts| {
                    e.speak(&t, rw.as_deref(), rt.as_deref(), opts, &mut |piece| {
                        if let Some(phrase) = &live {
                            phrase.chunk(piece, sr);
                        }
                    })
                    .map(|samples| (samples, sr as i32))
                })
            }
            (Tts::Drama(e), Some(rw)) => match rt.as_deref().filter(|t| !t.trim().is_empty()) {
                Some(rt) => {
                    let sr = e.sample_rate();
                    let spoken = match &live {
                        Some(phrase) if e.supports_streaming() => {
                            streamed = true;
                            e.speak_stream(&t, &rw, rt, &dp, &|a: &[f32]| phrase.chunk(a, sr))
                        }
                        _ => e.speak(&t, &rw, rt, &dp),
                    };
                    spoken.map(|a| (a, sr as i32)).map_err(|err| match err {
                        drama::DramaError::Cancelled => crate::jobs::CANCELLED.to_string(),
                        other => other.to_string(),
                    })
                }
                None => Err(t!("render-drama-no-ref-text")),
            },
            (Tts::Drama(_), None) => Err(t!("drama-needs-voice")),
            (Tts::Supertonic(e), _) => match preset.as_deref() {
                Some(voice) => e.speak(&t, &lang, voice, steps, speed, seed).map(|a| (a, e.sample_rate() as i32)),
                None => Err(t!("speech-voice-preset-needed")),
            },
            (Tts::Qwen(e), rw) => {
                // the VoiceDesign talker makes the voice from the description alone
                let designed = preset.is_none() && rw.is_none() && instruct.as_deref().is_some_and(|d| !d.trim().is_empty());
                match if designed { Ok(None) } else { qwen_reference(preset.as_deref(), rw.as_deref()) } {
                    Ok(reference) => {
                        let say = crate::engines::qwen_tts::Say {
                            text: &t,
                            lang: &lang,
                            speaker: preset.as_deref(),
                            reference: reference.as_ref().map(|a| (a.as_slice(), rt.as_deref())),
                            instruct: instruct.as_deref().filter(|_| preset.is_some() || designed),
                            seed,
                            temperature: dp.temperature,
                            top_k: dp.top_k,
                            top_p: dp.top_p,
                            max_new_tokens: max_tokens as i32,
                        };
                        streamed = live.is_some();
                        let sink = live.clone().map(|phrase| move |a: &[f32]| phrase.chunk(a, crate::engines::qwen_tts::SAMPLE_RATE));
                        e.speak(&say, sink.as_ref().map(|f| f as &(dyn Fn(&[f32]) + Sync))).map(|a| (a, crate::engines::qwen_tts::SAMPLE_RATE as i32))
                    }
                    Err(err) => Err(err),
                }
            }
        };
        if let (Some(phrase), Ok((samples, sr)), false) = (&live, &r, streamed) {
            phrase.chunk(samples, *sr as u32);
        }
        let _ = tx.send(r);
    });
    let ctl = crate::jobs::current();
    let deadline = Instant::now() + timeout;
    let cancelled = loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left.min(Duration::from_millis(200))) {
            Ok(r) => return r,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err(t!("render-synth-thread-ended")),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let cancelled = ctl.as_ref().is_some_and(|c| c.is_cancelled());
                if cancelled || left.is_zero() {
                    break cancelled;
                }
            }
        }
    };
    engine.cancel();
    match rx.recv_timeout(Duration::from_secs(GUARD_GRACE_SECS)) {
        Ok(_) if cancelled => Err(crate::jobs::CANCELLED.to_string()),
        Ok(_) => Err(t!("render-synth-timeout", engine = engine.name(), seconds = timeout.as_secs())),
        Err(_) => {
            unload();
            Err(format!("{ENGINE_STUCK}: {}", t!("render-engine-stuck", engine = engine.name(), seconds = timeout.as_secs() + GUARD_GRACE_SECS)))
        }
    }
}


/// What Qwen3-TTS speaks with: a voice of the model needs nothing more; a clone needs its reference as mono 24 kHz, kept
/// for the phrases and retries of the same file.
fn qwen_reference(preset: Option<&str>, reference: Option<&Path>) -> Result<Option<Arc<Vec<f32>>>, String> {
    type Kept = (PathBuf, u64, Option<std::time::SystemTime>, Arc<Vec<f32>>);
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    match (preset, reference) {
        (Some(_), _) => Ok(None),
        (None, Some(wav)) => {
            let meta = std::fs::metadata(wav).map_err(|e| format!("{}: {e}", wav.display()))?;
            let (len, modified) = (meta.len(), meta.modified().ok());
            let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((p, l, m, audio)) = kept.as_ref() {
                if p == wav && *l == len && *m == modified {
                    return Ok(Some(audio.clone()));
                }
            }
            let tmp = tempfile::Builder::new().suffix(".wav").tempfile().map_err(|e| e.to_string())?;
            crate::audio::media::extract_audio(wav, tmp.path(), crate::engines::qwen_tts::SAMPLE_RATE, 1)?;
            let audio = Arc::new(crate::audio::wavio::read_mono_f32(tmp.path())?.0);
            *kept = Some((wav.to_path_buf(), len, modified, audio.clone()));
            Ok(Some(audio))
        }
        (None, None) => Err(t!("speech-voice-preset-needed")),
    }
}

/// Кадров Drama 3 на фразу: ожидаемая длина с запасом вдвое (у Drama 21,5 кадра на секунду речи).
fn drama_frames(expected: f64, max_tokens: u32) -> u32 {
    if max_tokens > 0 {
        return max_tokens.clamp(64, DRAMA_MAX_LEN);
    }
    ((expected * drama::FRAMES_PER_SECOND * 2.0).ceil() as u32 + 32).clamp(64, DRAMA_MAX_LEN)
}

/// Синтез фразы с повтором дефектного дубля: фиксированный сид — одна попытка, случайный — до трёх, каждая
/// с новым сидом и температурой выше на 0,15. Фраза, которая звучит вслух по мере синтеза (агент говорит), — одна
/// попытка, принятая и с дефектом: её уже услышали, повтор прозвучал бы второй раз.
pub fn synth(engine: &Tts, req: &Request) -> Result<Synth, String> {
    req.params.checked()?;
    let expected = expected_secs(req.text, req.lang);
    let chars = req.text.chars().filter(|c| c.is_alphanumeric()).count();
    let cap = if req.params.max_tokens > 0 { req.params.max_tokens as usize } else { token_cap(chars, pace_lang(req.text, req.lang)) as usize };
    let timeout = Duration::from_secs(((expected * 8.0).ceil() as u64).max(45));
    let fixed = req.params.seed_mode != "random";
    let aloud = req.live.as_ref().is_some_and(|l| l.is_aloud());
    let tries = if fixed || aloud { 1 } else { 3 };
    let mut defects = Vec::new();
    for attempt in 0..tries {
        let seed = if fixed { req.params.seed } else { req.fresh_seed.wrapping_add(attempt as u64 * 7919) } % SEED_RANGE;
        let temperature = (req.params.temperature + 0.15 * attempt as f64).min(2.0);
        let opts = json!({
            "temperature": temperature,
            "top_k": req.params.top_k,
            "top_p": req.params.top_p,
            "max_tokens": cap,
            "allow_incomplete_generation": true,
            "seed": seed,
        })
        .to_string();
        let call = Call {
            text: req.text.to_string(),
            lang: req.lang.to_string(),
            ref_wav: req.ref_wav.map(Path::to_path_buf),
            ref_text: req.ref_text.map(str::to_string),
            preset: req.preset.map(str::to_string),
            instruct: req.instruct.map(str::to_string),
            opts,
            drama: drama::Params { temperature: temperature as f32, top_k: req.params.top_k as i32, top_p: req.params.top_p as f32, seed, max_new: drama_frames(expected, req.params.max_tokens) },
            max_tokens: req.params.max_tokens,
            steps: req.params.steps,
            speed: req.params.speed,
            seed,
        };
        if attempt > 0 {
            if let Some(live) = &req.live {
                live.restart();
            }
        }
        let started = Instant::now();
        let (samples, sr) = run_guarded(engine, call, timeout, req.live.clone())?;
        let seconds = started.elapsed().as_secs_f64();
        if aloud {
            return Ok(Synth { samples, sr, seed, temperature, defects, seconds });
        }
        match defect(&samples, sr, chars, pace_lang(req.text, req.lang)) {
            None => return Ok(Synth { samples, sr, seed, temperature, defects, seconds }),
            Some(kind) => defects.push(kind),
        }
    }
    Err(t!("tts-defective", defects = defects.iter().map(|d| defect_name(d)).collect::<Vec<_>>().join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speech(secs: f64, sr: i32) -> Vec<f32> {
        let n = (secs * sr as f64) as usize;
        (0..n).map(|i| if (i / (sr as usize / 5)) % 2 == 0 { 0.3 * ((i as f32) * 0.05).sin() } else { 0.0 }).collect()
    }

    #[test]
    fn a_runaway_a_cut_off_and_silence_are_defects_and_speech_is_not() {
        let sr = 24_000;
        assert_eq!(defect(&speech(30.0, sr), sr, 10, "ru"), Some("runaway"));
        assert_eq!(defect(&speech(4.0, sr), sr, 10, "ru"), None, "0.13 s a char + 3 s is still speech");
        assert_eq!(defect(&speech(4.5, sr), sr, 10, "ru"), Some("runaway"));
        assert_eq!(defect(&speech(0.2, sr), sr, 40, "ru"), Some("cutoff"));
        assert_eq!(defect(&vec![0.001; sr as usize * 2], sr, 20, "ru"), Some("silence"));
        assert_eq!(defect(&speech(2.0, sr), sr, 25, "ru"), None);
    }

    /// A Chinese phrase in a project of language «auto» is paced as Chinese: 40 ideographs read in 9 s were a «runaway»
    /// by the Russian rate on every attempt.
    #[test]
    fn a_phrase_of_auto_language_is_paced_by_its_script() {
        let sr = 24_000;
        let zh = "今天晚上八点我们在老桥旁边见面然后一起去看电影吃饭聊天散步回家休息明天再见朋友们";
        let chars = zh.chars().filter(|c| c.is_alphanumeric()).count();
        assert_eq!(pace_lang(zh, "auto"), "zh");
        assert_eq!(pace_lang(zh, ""), "zh");
        assert_eq!(pace_lang("今日はいい天気ですね", "auto"), "ja");
        assert_eq!(pace_lang("오늘 날씨가 좋네요", "auto"), "ko");
        assert_eq!(pace_lang("Сегодня в восемь вечера", "auto"), "ru");
        assert_eq!(pace_lang(zh, "en"), "en", "a language set by hand stands");
        assert_eq!(defect(&speech(9.0, sr), sr, chars, pace_lang(zh, "auto")), None);
        assert_eq!(defect(&speech(9.0, sr), sr, chars, "ru"), Some("runaway"));
        assert!(expected_secs(zh, "auto") > 6.0);
    }

    #[test]
    fn a_chinese_line_is_given_the_time_its_syllables_take() {
        let sr = 24_000;
        let line = "床前明月光，疑是地上霜。举头望明月，低头思故乡。";
        let chars = line.chars().filter(|c| c.is_alphanumeric()).count();
        assert_eq!(defect(&speech(8.0, sr), sr, chars, "zh"), None, "20 syllables in 8 s is a calm reading");
        assert_eq!(defect(&speech(8.0, sr), sr, chars, "ru"), Some("runaway"), "20 Russian letters in 8 s are not");
        assert_eq!(defect(&speech(30.0, sr), sr, chars, "zh"), Some("runaway"));
    }

    #[test]
    fn numbers_are_measured_by_how_they_are_said() {
        let sr = 24_000;
        let raw = "Встреча 12.03 в 18:30, билет стоит 150 руб., а на улице −5 °C.";
        let opts = crate::text::prep::Opts { lang: "ru", voice: None, engine: "higgs", entries: &[], normalize: true, normalize_off: &[] };
        let spoken = crate::text::prep::prepare(raw, &opts).prepared.text;
        let count = |t: &str| t.chars().filter(|c| c.is_alphanumeric()).count();
        let take = speech(count(raw) as f64 * RUNAWAY_PER_CHAR + RUNAWAY_SLACK + 1.0, sr);
        assert_eq!(defect(&take, sr, count(raw), "ru"), Some("runaway"), "the written digits are too short a yardstick");
        assert_eq!(defect(&take, sr, count(&spoken), "ru"), None, "{spoken}");
    }

    #[test]
    fn clipping_and_a_click_at_the_end_are_defects() {
        let sr = 24_000;
        let mut loud = speech(2.0, sr);
        for v in loud.iter_mut().step_by(200) {
            *v = 1.0;
        }
        assert_eq!(defect(&loud, sr, 25, "ru"), Some("clipping"));
        let mut clicked = speech(2.0, sr);
        let n = clicked.len();
        for v in &mut clicked[n - 6_000..] {
            *v = 0.01 * *v;
        }
        clicked[n - 100] = 0.9;
        assert_eq!(defect(&clicked, sr, 25, "ru"), Some("click"));
    }

    #[test]
    fn a_steady_hum_is_a_defect() {
        let sr = 24_000;
        let hum: Vec<f32> = (0..sr as usize * 3).map(|i| 0.2 * ((i as f32) * 0.03).sin()).collect();
        assert_eq!(defect(&hum, sr, 30, "ru"), Some("hum"));
    }

    #[test]
    fn the_token_cap_follows_the_expected_length() {
        assert_eq!(token_cap(10, "ru"), ((10.0 * 0.13 + 3.0 + 1.0) * 25.0f64).ceil() as u32 + 8, "frames of 40 ms to a second past the runaway");
        assert_eq!(token_cap(2_000, "ru"), 2048);
        assert!(token_cap(40, "zh") > token_cap(40, "ru"), "an ideograph is a syllable");
        let short = expected_secs("Да.", "ru");
        let long = expected_secs(&"слово ".repeat(40), "ru");
        assert!(short < 1.0 && long > 15.0);
    }

    #[test]
    fn out_of_range_parameters_are_refused() {
        assert!(Params::default().checked().is_ok());
        assert!(Params { temperature: 5.0, ..Default::default() }.checked().is_err());
        assert!(Params { seed_mode: "sometimes".into(), ..Default::default() }.checked().is_err());
    }


    #[test]
    fn a_chosen_engine_brings_its_sampling_defaults() {
        let mut p = Params::default();
        p.engine_defaults(&crate::engines::drama());
        assert_eq!((p.temperature, p.top_k, p.top_p), (1.0, 30, 0.9));
        p.engine_defaults(&crate::engines::higgs());
        assert_eq!((p.top_k, p.top_p), (50, 0.95));
    }
}
