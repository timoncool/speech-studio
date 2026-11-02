//! Qwen3-TTS CustomVoice through qwentts.cpp (ServeurpersoCom, ggml on CUDA) loaded at run time: its public C ABI
//! `qwen.h` (struct ABI 7) — nine named speakers, a style instruction, streaming chunks, cooperative cancel and a LoRA
//! adapter on the talker applied beside its quantised weights.

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use acpp_server::asset::Build;
use libloading::Library;

pub const ENGINE: &str = "qwen3-tts";
/// The Base talker with the Russian stress adapter: a clone that reads U+0301 after the stressed vowel.
pub const ENGINE_STRESS: &str = "qwen3-tts-ru";
/// siriusfreak/qwen3-tts-12hz-1.7b-ru-stress-cf `lora/adapter.safetensors`: r 16, alpha 32 on the Base talker.
pub const STRESS_LORA: &str = "qwen-talker-1.7b-base-ru-stress-lora.safetensors";
const STRESS_LORA_SCALE: f32 = 32.0 / 16.0;
/// The longest reference the stress adapter clones from: trained on short clips, it ends phrases early after a longer one.
pub const STRESS_REF_SECS: f64 = 10.0;
/// The folder of the weights and the library inside the models root.
pub const DIR: &str = "qwen3-tts";
pub const CODEC: &str = "qwen-tokenizer-12hz-Q8_0.gguf";
/// Talker quantisations, the first the default (Q4_K_M, as GGUF models go here): the CustomVoice talker speaks the
/// named speakers, the Base talker clones a reference.
pub const QUANTS: &[(&str, &str, &str)] = &[
    ("Q4_K_M", "qwen-talker-1.7b-customvoice-Q4_K_M.gguf", "qwen-talker-1.7b-base-Q4_K_M.gguf"),
    ("Q8_0", "qwen-talker-1.7b-customvoice-Q8_0.gguf", "qwen-talker-1.7b-base-Q8_0.gguf"),
];
#[cfg(windows)]
pub const LIBRARY: &str = "qwen.dll";
#[cfg(target_os = "linux")]
pub const LIBRARY: &str = "libqwen.so";
/// macOS: the CPU backend and Metal are linked into the library, so no device has a module beside it.
#[cfg(target_os = "macos")]
pub const LIBRARY: &str = "libqwen.dylib";
/// The speakers of the CustomVoice checkpoints with the native language the model card gives them.
pub const SPEAKERS: &[(&str, &str, &str, &str, &str)] = &[
    ("vivian", "Vivian", "female", "zh", "Bright, slightly edgy young female voice."),
    ("serena", "Serena", "female", "zh", "Warm, gentle young female voice."),
    ("uncle_fu", "Uncle Fu", "male", "zh", "Seasoned male voice with a low, mellow timbre."),
    ("dylan", "Dylan", "male", "zh", "Youthful Beijing male voice with a clear, natural timbre."),
    ("eric", "Eric", "male", "zh", "Lively Chengdu male voice with a slightly husky brightness."),
    ("ryan", "Ryan", "male", "en", "Dynamic male voice with strong rhythmic drive."),
    ("aiden", "Aiden", "male", "en", "Sunny American male voice with a clear midrange."),
    ("ono_anna", "Ono Anna", "female", "ja", "Playful Japanese female voice with a light, nimble timbre."),
    ("sohee", "Sohee", "female", "ko", "Warm Korean female voice with rich emotion."),
];
pub const LANGS: &[&str] = &["ru", "en", "zh", "ja", "ko", "de", "fr", "es", "pt", "it"];
pub const SAMPLE_RATE: u32 = 24_000;
const ABI_VERSION: i32 = 7;
const STATUS_CANCELLED: i32 = -5;

#[repr(C)]
struct InitParams {
    abi_version: i32,
    talker_path: *const c_char,
    codec_path: *const c_char,
    use_fa: bool,
    clamp_fp16: bool,
    max_batch: i32,
    codec_chunk_sec: f32,
    device: *const c_char,
    lora_path: *const c_char,
    lora_scale: f32,
}

type ChunkFn = unsafe extern "C" fn(*const f32, i32, *mut c_void) -> bool;
type CancelFn = unsafe extern "C" fn(*mut c_void) -> bool;

#[repr(C)]
struct TtsParams {
    abi_version: i32,
    text: *const c_char,
    lang: *const c_char,
    instruct: *const c_char,
    speaker: *const c_char,
    ref_audio_24k: *const f32,
    ref_n_samples: i32,
    ref_text: *const c_char,
    seed: i64,
    max_new_tokens: i32,
    temperature: f32,
    top_k: i32,
    top_p: f32,
    repetition_penalty: f32,
    subtalker_temperature: f32,
    subtalker_top_k: i32,
    subtalker_top_p: f32,
    dump_dir: *const c_char,
    cancel: Option<CancelFn>,
    cancel_user_data: *mut c_void,
    on_chunk: Option<ChunkFn>,
    on_chunk_user_data: *mut c_void,
    ref_spk_emb: *const f32,
    ref_spk_dim: i32,
    ref_codes: *const i32,
    ref_t: i32,
}

#[repr(C)]
struct Audio {
    samples: *mut f32,
    n_samples: i32,
    sample_rate: i32,
    channels: i32,
}

/// Voice-clone conditioning the library extracts from a reference once: speaker embedding and codec codes.
#[repr(C)]
struct VoiceRef {
    ref_spk_emb: *mut f32,
    ref_spk_dim: i32,
    ref_codes: *mut i32,
    ref_t: i32,
    num_codebooks: i32,
}

#[repr(C)]
struct RawContext {
    _private: [u8; 0],
}

#[derive(Clone, Copy)]
struct Api {
    version: unsafe extern "C" fn() -> *const c_char,
    last_error: unsafe extern "C" fn() -> *const c_char,
    init_default: unsafe extern "C" fn(*mut InitParams),
    init: unsafe extern "C" fn(*const InitParams) -> *mut RawContext,
    free: unsafe extern "C" fn(*mut RawContext),
    tts_default: unsafe extern "C" fn(*mut TtsParams),
    synthesize: unsafe extern "C" fn(*mut RawContext, *const TtsParams, *mut Audio) -> i32,
    audio_free: unsafe extern "C" fn(*mut Audio),
    model_type: unsafe extern "C" fn(*const RawContext) -> *const c_char,
    extract_voice_ref: unsafe extern "C" fn(*mut RawContext, *const f32, i32, *mut VoiceRef) -> i32,
    voice_ref_free: unsafe extern "C" fn(*mut VoiceRef),
}

/// The library, loaded once and kept for the life of the process: ggml's static initialiser refuses a second load
/// at the address it had (its terminate handler would already be the one it installs), so freeing the library and
/// loading it again aborts the process. Its device backends are modules it loads beside itself as a device is first
/// used, so one library serves every device and a device change only reloads the model.
static LOADED: std::sync::Mutex<Option<(Library, Api)>> = std::sync::Mutex::new(None);

fn library(dir: &Path) -> Result<Api, String> {
    let mut loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, api)) = loaded.as_ref() {
        return Ok(*api);
    }
    let path = dir.join(LIBRARY);
    // Loading the library the instant the disk or device is busy can fail transiently (LoadLibraryExW returns nothing
    // with no error of its own); a couple of backed-off retries recover it (#21). A missing dependency keeps failing
    // and is reported after the last try.
    let mut attempt = unsafe { Library::new(&path) };
    let mut wait = std::time::Duration::from_millis(250);
    for _ in 0..2 {
        match &attempt {
            Err(e) if format!("{e}").contains("LoadLibraryExW failed") => {
                tracing::warn!("qwen3-tts: library load failed, retrying in {} ms: {e}", wait.as_millis());
                std::thread::sleep(wait);
                wait *= 2;
                attempt = unsafe { Library::new(&path) };
            }
            _ => break,
        }
    }
    let lib = attempt.map_err(|e| format!("{}: {e}", path.display()))?;
    let api = unsafe {
        macro_rules! sym {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes()).map_err(|e| format!("qwen3-tts {}: {e}", $name))?
            };
        }
        Api {
            version: sym!("qt_version"),
            last_error: sym!("qt_last_error"),
            init_default: sym!("qt_init_default_params"),
            init: sym!("qt_init"),
            free: sym!("qt_free"),
            tts_default: sym!("qt_tts_default_params"),
            synthesize: sym!("qt_synthesize"),
            audio_free: sym!("qt_audio_free"),
            model_type: sym!("qt_model_type"),
            extract_voice_ref: sym!("qt_extract_voice_ref"),
            voice_ref_free: sym!("qt_voice_ref_free"),
        }
    };
    *loaded = Some((lib, api));
    Ok(api)
}

/// A loaded talker and codec on the GPU; freed on drop. One synthesis at a time is the caller's to keep.
pub struct Engine {
    api: Api,
    raw: *mut RawContext,
    stop: AtomicBool,
    /// The last reference's conditioning by the hash of its samples: a block's phrases and retries share it.
    voice: std::sync::Mutex<Option<([u8; 32], VoiceRef)>>,
}

// qt_synthesize is thread safe and the handle is only freed on drop.
unsafe impl Send for Engine {}
unsafe impl Sync for Engine {}

/// What one phrase is said with: a named speaker (CustomVoice talker) or a reference to clone (Base talker) — mono
/// 24 kHz audio, with its transcript for the in-context mode or without it for the timbre alone.
pub struct Say<'a> {
    pub text: &'a str,
    pub lang: &'a str,
    pub speaker: Option<&'a str>,
    pub reference: Option<(&'a [f32], Option<&'a str>)>,
    pub instruct: Option<&'a str>,
    pub seed: u64,
    pub temperature: f32,
    pub top_k: i32,
    pub top_p: f32,
    pub max_new_tokens: i32,
}

pub fn dir(models_root: &Path) -> PathBuf {
    models_root.join(DIR)
}

/// The ggml backend module of a device the library loads beside itself; the CPU one ships with the library.
#[cfg(windows)]
fn module(build: Build) -> Option<&'static str> {
    match build {
        Build::Cuda13 | Build::Cuda12 => Some("ggml-cuda.dll"),
        Build::Vulkan => Some("ggml-vulkan.dll"),
        Build::Cpu | Build::Metal => None,
    }
}
#[cfg(not(windows))]
fn module(build: Build) -> Option<&'static str> {
    match build {
        Build::Cuda13 | Build::Cuda12 => Some("libggml-cuda.so"),
        Build::Vulkan => Some("libggml-vulkan.so"),
        Build::Cpu | Build::Metal => None,
    }
}

/// The ggml device the library runs a build on; the studio exposes one card, so the CUDA device is the first.
pub fn device(build: Build) -> &'static str {
    match build {
        Build::Cuda13 | Build::Cuda12 => "CUDA0",
        Build::Vulkan => "Vulkan0",
        Build::Cpu => "CPU",
        Build::Metal => "MTL0",
    }
}

/// The component of a device's backend module: the CUDA one is the build of this computer's card.
pub fn module_component(build: Build) -> Option<&'static str> {
    match build {
        Build::Cuda13 | Build::Cuda12 => Some("qwen3-tts-cuda"),
        Build::Vulkan => Some("qwen3-tts-vulkan"),
        Build::Cpu | Build::Metal => None,
    }
}

/// The components the voicing device needs: the engine, the device's backend module and its CUDA runtime.
pub fn build_components(models_root: &Path) -> Result<Vec<&'static str>, String> {
    let build = build(models_root)?;
    Ok(std::iter::once("qwen3-tts-engine")
        .chain(module_component(build))
        .chain(crate::engines::acpp::runtime_component(build))
        .collect())
}

/// The build the voicing device of the settings runs Qwen on (`tts_backend`, see `acpp::build_for`).
pub fn build(models_root: &Path) -> Result<Build, String> {
    crate::engines::acpp::build_for(crate::engines::models::server_choice(models_root, "tts_backend"))
}

/// The library and the backend module of the voicing device are on disk.
fn library_installed(models_root: &Path) -> bool {
    let d = dir(models_root);
    d.join(LIBRARY).is_file() && build(models_root).is_ok_and(|b| module(b).is_none_or(|m| d.join(m).is_file()))
}

pub fn is_engine(id: &str) -> bool {
    id == ENGINE || id == ENGINE_STRESS
}

pub fn talker(models_root: &Path, quant: &str, clone: bool) -> Option<PathBuf> {
    QUANTS.iter().find(|(q, _, _)| *q == quant).map(|(_, custom, base)| dir(models_root).join(if clone { base } else { custom }))
}

/// The library, the codec and the talker of `quant` for named speakers (`clone` false) or for cloning, with the
/// stress adapter when `stress`.
/// The VoiceDesign talker of a quantisation.
pub fn design_talker(models_root: &Path, quant: &str) -> PathBuf {
    dir(models_root).join(format!("qwen-talker-1.7b-voicedesign-{quant}.gguf"))
}

/// The engine, its codec and the VoiceDesign talker of the quantisation are on disk.
pub fn design_installed(models_root: &Path, quant: &str) -> bool {
    library_installed(models_root) && dir(models_root).join(CODEC).is_file() && design_talker(models_root, quant).is_file()
}

pub fn installed(models_root: &Path, quant: &str, clone: bool, stress: bool) -> bool {
    let d = dir(models_root);
    library_installed(models_root)
        && d.join(CODEC).is_file()
        && talker(models_root, quant, clone).is_some_and(|t| t.is_file())
        && (!stress || d.join(STRESS_LORA).is_file())
}

fn cstring(s: &str, what: &str) -> Result<CString, String> {
    CString::new(s).map_err(|_| format!("qwen3-tts: {what} contains NUL"))
}

/// The upstream language name the ABI reads; `None` lets the model infer it.
fn lang_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "ru" => "russian",
        "en" => "english",
        "zh" => "chinese",
        "ja" => "japanese",
        "ko" => "korean",
        "de" => "german",
        "fr" => "french",
        "es" => "spanish",
        "pt" => "portuguese",
        "it" => "italian",
        _ => return None,
    })
}

unsafe extern "C" fn chunk_trampoline(samples: *const f32, n: i32, user: *mut c_void) -> bool {
    if !samples.is_null() && n > 0 && !user.is_null() {
        let sink = &*(user as *const &(dyn Fn(&[f32]) + Sync));
        sink(std::slice::from_raw_parts(samples, n as usize));
    }
    true
}

unsafe extern "C" fn cancel_trampoline(user: *mut c_void) -> bool {
    !user.is_null() && (*(user as *const AtomicBool)).load(Ordering::Relaxed)
}

impl Engine {
    /// Loads the library (its CUDA libraries are found on PATH) and the talker of `quant`: Base when `clone`, else
    /// CustomVoice; `stress` adds the Russian stress adapter to the Base talker.
    pub fn load(models_root: &Path, build: Build, quant: &str, clone: bool, stress: bool) -> Result<Engine, String> {
        if stress && !clone {
            return Err("qwen3-tts: the stress adapter is trained on the Base talker".into());
        }
        let talker = talker(models_root, quant, clone).ok_or_else(|| format!("qwen3-tts: no quantisation {quant}"))?;
        Engine::open(models_root, build, &talker, stress, if clone { "base" } else { "custom_voice" })
    }

    /// The VoiceDesign talker: a voice made from the description in `Say::instruct`, no speaker, no reference.
    pub fn load_design(models_root: &Path, build: Build, quant: &str) -> Result<Engine, String> {
        Engine::open(models_root, build, &design_talker(models_root, quant), false, "voice_design")
    }

    fn open(models_root: &Path, build: Build, talker: &Path, stress: bool, expected: &str) -> Result<Engine, String> {
        let d = dir(models_root);
        let lib = d.clone();
        // its ggml libraries sit beside it under names other engines' libraries could share: this folder comes first
        let path = std::env::var("PATH").unwrap_or_default();
        if !std::env::split_paths(&path).any(|p| p == lib) {
            let joined = std::env::join_paths(std::iter::once(lib.clone()).chain(std::env::split_paths(&path))).map_err(|e| e.to_string())?;
            std::env::set_var("PATH", joined);
        }
        let api = library(&lib)?;
        let (t, c) = (cstring(&talker.to_string_lossy(), "talker path")?, cstring(&d.join(CODEC).to_string_lossy(), "codec path")?);
        let lora = if stress { Some(cstring(&d.join(STRESS_LORA).to_string_lossy(), "adapter path")?) } else { None };
        let device = cstring(device(build), "device")?;
        let mut p: InitParams = unsafe { std::mem::zeroed() };
        unsafe { (api.init_default)(&mut p) };
        p.abi_version = ABI_VERSION;
        p.talker_path = t.as_ptr();
        p.codec_path = c.as_ptr();
        p.device = device.as_ptr();
        if let Some(lora) = &lora {
            p.lora_path = lora.as_ptr();
            p.lora_scale = STRESS_LORA_SCALE;
        }
        let raw = unsafe { (api.init)(&p) };
        if raw.is_null() {
            return Err(format!("qwen3-tts: {}", unsafe { last_error(&api) }));
        }
        let engine = Engine { api, raw, stop: AtomicBool::new(false), voice: std::sync::Mutex::new(None) };
        let kind = unsafe { CStr::from_ptr((engine.api.model_type)(engine.raw)) }.to_string_lossy().into_owned();
        if kind != expected {
            return Err(format!("qwen3-tts: {} is a {kind} checkpoint, not {expected}", talker.display()));
        }
        Ok(engine)
    }

    pub fn version(&self) -> String {
        unsafe { CStr::from_ptr((self.api.version)()) }.to_string_lossy().into_owned()
    }

    /// Stops the running synthesis within a frame (~83 ms).
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Speaks one phrase; `on_chunk` (when given) gets the audio as it is decoded and the answer holds all of it.
    pub fn speak(&self, say: &Say, on_chunk: Option<&(dyn Fn(&[f32]) + Sync)>) -> Result<Vec<f32>, String> {
        if let Some(s) = say.speaker.filter(|s| !SPEAKERS.iter().any(|k| k.0 == *s)) {
            return Err(format!("qwen3-tts: no speaker {s}"));
        }
        let text = cstring(say.text, "text")?;
        let speaker = say.speaker.map(|s| cstring(s, "speaker")).transpose()?;
        let ref_text = say.reference.and_then(|(_, t)| t).filter(|t| !t.trim().is_empty()).map(|t| cstring(t, "reference text")).transpose()?;
        let lang = lang_name(say.lang).map(|l| cstring(l, "lang")).transpose()?;
        let instruct = say.instruct.filter(|i| !i.trim().is_empty()).map(|i| cstring(i, "instruct")).transpose()?;
        let mut p: TtsParams = unsafe { std::mem::zeroed() };
        unsafe { (self.api.tts_default)(&mut p) };
        p.abi_version = ABI_VERSION;
        p.text = text.as_ptr();
        p.speaker = speaker.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
        let mut voice = self.voice.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((audio, _)) = say.reference {
            let key: [u8; 32] = *blake3::hash(unsafe { std::slice::from_raw_parts(audio.as_ptr() as *const u8, std::mem::size_of_val(audio)) }).as_bytes();
            if voice.as_ref().is_none_or(|(k, _)| *k != key) {
                if let Some((_, mut old)) = voice.take() {
                    unsafe { (self.api.voice_ref_free)(&mut old) };
                }
                let mut r: VoiceRef = unsafe { std::mem::zeroed() };
                if unsafe { (self.api.extract_voice_ref)(self.raw, audio.as_ptr(), audio.len() as i32, &mut r) } != 0 {
                    return Err(format!("qwen3-tts: {}", unsafe { last_error(&self.api) }));
                }
                *voice = Some((key, r));
            }
            let r = &voice.as_ref().expect("set above").1;
            p.ref_spk_emb = r.ref_spk_emb;
            p.ref_spk_dim = r.ref_spk_dim;
            if let Some(t) = &ref_text {
                p.ref_codes = r.ref_codes;
                p.ref_t = r.ref_t;
                p.ref_text = t.as_ptr();
            }
        }
        p.lang = lang.as_ref().map_or(std::ptr::null(), |l| l.as_ptr());
        p.instruct = instruct.as_ref().map_or(std::ptr::null(), |i| i.as_ptr());
        p.seed = (say.seed & i64::MAX as u64) as i64;
        p.temperature = say.temperature;
        p.top_k = say.top_k;
        p.top_p = say.top_p;
        if say.max_new_tokens > 0 {
            p.max_new_tokens = say.max_new_tokens;
        }
        self.stop.store(false, Ordering::Relaxed);
        p.cancel = Some(cancel_trampoline);
        p.cancel_user_data = &self.stop as *const AtomicBool as *mut c_void;
        // the library streams to its callback and leaves `out` empty, so the whole take is gathered here
        let gathered = std::sync::Mutex::new(Vec::<f32>::new());
        let sink = |a: &[f32]| {
            gathered.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(a);
            if let Some(f) = on_chunk {
                f(a);
            }
        };
        let sink_ref: &(dyn Fn(&[f32]) + Sync) = &sink;
        p.on_chunk = Some(chunk_trampoline);
        p.on_chunk_user_data = &sink_ref as *const &(dyn Fn(&[f32]) + Sync) as *mut c_void;
        let mut out = Audio { samples: std::ptr::null_mut(), n_samples: 0, sample_rate: 0, channels: 0 };
        let status = unsafe { (self.api.synthesize)(self.raw, &p, &mut out) };
        unsafe { (self.api.audio_free)(&mut out) };
        self.stop.store(false, Ordering::Relaxed);
        match status {
            0 => Ok(gathered.into_inner().unwrap_or_else(|e| e.into_inner())),
            STATUS_CANCELLED => Err(crate::jobs::CANCELLED.to_string()),
            _ => Err(format!("qwen3-tts: {}", unsafe { last_error(&self.api) })),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some((_, mut r)) = self.voice.get_mut().unwrap_or_else(|e| e.into_inner()).take() {
            unsafe { (self.api.voice_ref_free)(&mut r) };
        }
        unsafe { (self.api.free)(self.raw) };
    }
}

unsafe fn last_error(api: &Api) -> String {
    let p = (api.last_error)();
    if p.is_null() { "unknown error".into() } else { CStr::from_ptr(p).to_string_lossy().into_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_map_to_the_names_the_library_reads() {
        assert_eq!(lang_name("ru"), Some("russian"));
        assert_eq!(lang_name("auto"), None);
        assert!(LANGS.iter().all(|l| lang_name(l).is_some()));
    }

    /// The real library and weights: `QWEN_ROOT` is a models root holding `qwen3-tts/` (qwen.dll, the codec and the
    /// talker); the CUDA libraries are on PATH.
    #[test]
    #[ignore]
    fn speaks_russian_on_the_gpu_and_streams() {
        let root = PathBuf::from(std::env::var("QWEN_ROOT").expect("QWEN_ROOT"));
        let quant = std::env::var("QWEN_QUANT").unwrap_or_else(|_| "Q4_K_M".into());
        let e = Engine::load(&root, Build::Cuda13, &quant, false, false).unwrap();
        eprintln!("qwentts {}", e.version());
        let say = Say { text: "Сегодня в восемь вечера мы встретимся у старого моста.", lang: "ru", speaker: Some("ryan"), reference: None, instruct: None, seed: 42, temperature: 0.7, top_k: 50, top_p: 0.9, max_new_tokens: 0 };
        let chunks = std::sync::atomic::AtomicUsize::new(0);
        let first = std::sync::Mutex::new(None);
        let started = std::time::Instant::now();
        let wav = e
            .speak(&say, Some(&|_: &[f32]| {
                chunks.fetch_add(1, Ordering::Relaxed);
                first.lock().unwrap().get_or_insert(started.elapsed());
            }))
            .unwrap();
        let secs = wav.len() as f64 / SAMPLE_RATE as f64;
        eprintln!("[OK] {secs:.2} s of audio in {:.3} s, first sound {:?}, {} chunks", started.elapsed().as_secs_f64(), first.lock().unwrap(), chunks.load(Ordering::Relaxed));
        assert!(secs > 1.5 && secs < 10.0, "{secs}");
        if let Ok(out) = std::env::var("QWEN_OUT") {
            crate::audio::wavio::write_mono_f32(Path::new(&out), &wav, SAMPLE_RATE).unwrap();
        }
    }

    /// Our sampling against the model card's on one long phrase, six seeds each: length and pauses over 0.6 s inside.
    /// `QWEN_ROOT` as above.
    #[test]
    #[ignore]
    fn pauses_by_sampling() {
        let root = PathBuf::from(std::env::var("QWEN_ROOT").expect("QWEN_ROOT"));
        let e = Engine::load(&root, Build::Cuda13, "Q4_K_M", false, false).unwrap();
        let text = "Сегодня утром мы обсудили выпуск, договорились перенести его на пятницу, отдать субтитры Анне, а голоса оставить такими, какие они есть сейчас, потому что менять их перед самым выпуском слишком рискованно.";
        let pauses = |wav: &[f32]| -> Vec<f64> {
            let hop = SAMPLE_RATE as usize / 50;
            let rms: Vec<f64> = wav.chunks(hop).map(|c| (c.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / c.len() as f64).sqrt()).collect();
            let thr = (rms.iter().copied().fold(0.0, f64::max) * 0.01).max(10f64.powf(-70.0 / 20.0));
            let loud: Vec<bool> = rms.iter().map(|&r| r > thr).collect();
            let (Some(a), Some(z)) = (loud.iter().position(|&l| l), loud.iter().rposition(|&l| l)) else { return Vec::new() };
            let mut out = Vec::new();
            let mut run = 0usize;
            for &l in &loud[a..=z] {
                if l {
                    if run > 30 {
                        out.push(run as f64 * 0.02);
                    }
                    run = 0;
                } else {
                    run += 1;
                }
            }
            out
        };
        for (name, temperature, top_k, top_p) in [("ours", 0.7, 50, 0.9), ("card", 0.9, 50, 1.0)] {
            let mut lens = Vec::new();
            let mut counts = Vec::new();
            for seed in 1..=6u64 {
                let say = Say { text, lang: "ru", speaker: Some("ryan"), reference: None, instruct: None, seed, temperature, top_k, top_p, max_new_tokens: 0 };
                let wav = e.speak(&say, None).unwrap();
                let secs = wav.len() as f64 / SAMPLE_RATE as f64;
                let p = pauses(&wav);
                eprintln!("{name} seed {seed}: {secs:.1} s, pauses over 0.6 s: {p:.2?}");
                lens.push(secs);
                counts.push(p.len());
            }
            eprintln!("[OK] {name}: mean {:.1} s, pauses over 0.6 s per take {:.1}", lens.iter().sum::<f64>() / 6.0, counts.iter().sum::<usize>() as f64 / 6.0);
        }
    }

    /// Cloning on the Base talker, after the CustomVoice one was loaded and freed in the same process: `QWEN_REF` is a
    /// mono 24 kHz WAV, `QWEN_REF_TEXT` its transcript.
    #[test]
    #[ignore]
    fn clones_a_reference_after_the_named_voices_were_freed() {
        let root = PathBuf::from(std::env::var("QWEN_ROOT").expect("QWEN_ROOT"));
        drop(Engine::load(&root, Build::Cuda13, "Q4_K_M", false, false).unwrap());
        let e = Engine::load(&root, Build::Cuda13, "Q4_K_M", true, false).unwrap();
        let (reference, sr) = crate::audio::wavio::read_mono_f32(Path::new(&std::env::var("QWEN_REF").expect("QWEN_REF"))).unwrap();
        assert_eq!(sr, SAMPLE_RATE);
        let text = std::env::var("QWEN_REF_TEXT").ok();
        let say = Say { text: "Только после завтрака.", lang: "ru", speaker: None, reference: Some((&reference, text.as_deref())), instruct: None, seed: 7, temperature: 0.7, top_k: 50, top_p: 0.9, max_new_tokens: 0 };
        let wav = e.speak(&say, None).unwrap();
        eprintln!("[OK] clone {:.2} s", wav.len() as f64 / SAMPLE_RATE as f64);
        if let Ok(out) = std::env::var("QWEN_OUT") {
            crate::audio::wavio::write_mono_f32(Path::new(&out), &wav, SAMPLE_RATE).unwrap();
        }
    }

    /// The same marked phrases from the Base talker without and with the stress adapter, same reference and seed:
    /// `QWEN_REF`, `QWEN_REF_TEXT` as above, `QWEN_OUT_DIR` gets `<n>-base.wav` and `<n>-stress.wav`.
    #[test]
    #[ignore]
    fn the_stress_adapter_loads_beside_the_quantised_base_and_speaks() {
        let root = PathBuf::from(std::env::var("QWEN_ROOT").expect("QWEN_ROOT"));
        let out = PathBuf::from(std::env::var("QWEN_OUT_DIR").expect("QWEN_OUT_DIR"));
        let (reference, sr) = crate::audio::wavio::read_mono_f32(Path::new(&std::env::var("QWEN_REF").expect("QWEN_REF"))).unwrap();
        assert_eq!(sr, SAMPLE_RATE);
        let ref_text = std::env::var("QWEN_REF_TEXT").ok();
        let phrases = [
            "Он запер ста\u{301}рый замо\u{301}к и пошёл к за\u{301}мку на горе\u{301}.",
            "Мука\u{301} была\u{301} в мешке\u{301}, а му\u{301}ка — в душе\u{301}.",
            "В а\u{301}тласе на столе\u{301} лежа\u{301}л кусо\u{301}к атла\u{301}са.",
        ];
        assert!(Engine::load(&root, Build::Cuda13, "Q4_K_M", false, true).is_err(), "the adapter is for the Base talker");
        for stress in [false, true] {
            let e = Engine::load(&root, Build::Cuda13, "Q4_K_M", true, stress).unwrap();
            for (n, text) in phrases.iter().enumerate() {
                let say = Say { text, lang: "ru", speaker: None, reference: Some((&reference, ref_text.as_deref())), instruct: None, seed: 11, temperature: 0.7, top_k: 50, top_p: 0.9, max_new_tokens: 0 };
                let started = std::time::Instant::now();
                let wav = e.speak(&say, None).unwrap();
                let secs = wav.len() as f64 / SAMPLE_RATE as f64;
                eprintln!("[OK] {} {n}: {secs:.2} s in {:.2} s", if stress { "stress" } else { "base" }, started.elapsed().as_secs_f64());
                assert!(secs > 1.0 && secs < 15.0, "{secs}");
                crate::audio::wavio::write_mono_f32(&out.join(format!("{n}-{}.wav", if stress { "stress" } else { "base" })), &wav, SAMPLE_RATE).unwrap();
            }
        }
    }
}
