//! The studio's audiocpp_server (audio.cpp, through Dub Studio's crate `acpp-server`: `SERVERS`, one process per
//! build): CrisperWhisper, Parakeet TDT v3 and Nemotron 3.5 recognition, Nemotron 3 Diarization, and Higgs Audio v3
//! and Supertonic 3 voicing, with the models on disk now. The same server and
//! layout as Dub Studio's, so a model of one program serves the other.

use crate::engines::hw::CudaBuild;
use acpp_server::asset::Build;
use acpp_server::{AcppError, Backend, Flow, ModelEntry, Server, ServerConfig, SpeechRequest, VoiceRef};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const CRISPERWHISPER: &str = "crisperwhisper";
pub const DIAR: &str = "nemotron-diar";
pub const HIGGS: &str = "higgs";
pub const NEMOTRON: &str = "nemotron";
pub const PARAKEET: &str = "parakeet-v3";
pub const SUPERTONIC: &str = "supertonic";
pub const AUDIO8: &str = "audio8";

/// The models of transcribe.cpp's catalogue the server runs, reading their GGUF in place: Parakeet TDT v3 and Nemotron
/// 3.5. Parakeet Ultra stays on transcribe.cpp: audio.cpp does not read its built-in VAD head.
pub const SERVER_ASR: [&str; 2] = [PARAKEET, NEMOTRON];

/// The CrisperWhisper folder under the models root (the Hugging Face repository as it is; audio.cpp reads the folder).
pub const CRISPERWHISPER_DIR: &str = "crisperwhisper2-turbo";

/// How long a start may take before the server answers `/health` (it loads no model at start).
const READY_WITHIN: Duration = Duration::from_secs(60);
/// The longest wait for the next piece of a phrase; the first one also waits for Higgs to load.
const SPEECH_STALL: Duration = Duration::from_secs(120);

/// The build of the server a backend choice runs: `cpu`; `vulkan`; `gpu` (the card's CUDA build: 13, else 12.9); or
/// «auto»: the card's CUDA build, else Vulkan on a card of another make, else the processor. An NVIDIA card runs Vulkan
/// only when it is chosen. An error when the card is chosen and it runs neither CUDA build.
pub fn build_for(choice: &str) -> Result<Build, String> {
    // a Mac with Apple silicon has one program, Metal and the processor in it, for every choice
    if let Some(mac) = Build::for_this_mac() {
        return Ok(mac);
    }
    let report = crate::engines::hw::gpu_report();
    let cuda = report.cuda_build.map(|b| match b {
        CudaBuild::Cuda13 => Build::Cuda13,
        CudaBuild::Cuda12 => Build::Cuda12,
    });
    match choice {
        "cpu" => Ok(Build::Cpu),
        "vulkan" => Ok(Build::Vulkan),
        "gpu" | "cuda" => cuda.ok_or_else(|| t!("acpp-no-cuda-build")),
        _ => Ok(cuda.unwrap_or_else(|| {
            let other_make = crate::engines::hw::display_adapter().is_some_and(|name| !name.to_ascii_lowercase().contains("nvidia"));
            if other_make { Build::Vulkan } else { Build::Cpu }
        })),
    }
}

/// The build «auto» runs on this computer.
pub fn build() -> Build {
    build_for("auto").expect("auto always names a build")
}

/// The server program of a build: env SPEECH_STUDIO_AUDIOCPP_SERVER, else `tools/<build folder>` of the program.
pub fn exe(repo_root: &Path, build: Build) -> PathBuf {
    if let Some(p) = std::env::var_os("SPEECH_STUDIO_AUDIOCPP_SERVER") {
        return PathBuf::from(p);
    }
    let name = if cfg!(windows) { "audiocpp_server.exe" } else { "audiocpp_server" };
    repo_root.join("tools").join(build.folder()).join(name)
}

/// The component that downloads a build: `higgs-engine` is the build «auto» runs here, the others are fetched when a
/// stage is set to them.
pub fn component(build: Build) -> &'static str {
    if build == crate::engines::acpp::build() {
        return "higgs-engine";
    }
    match build {
        Build::Cuda13 => "audiocpp-cuda13",
        Build::Cuda12 => "audiocpp-cuda12",
        Build::Vulkan => "audiocpp-vulkan",
        Build::Cpu => "audiocpp-cpu",
        Build::Metal => "audiocpp-metal",
    }
}

/// The CUDA runtime component a build needs, if any.
pub fn runtime_component(build: Build) -> Option<&'static str> {
    match build {
        Build::Cuda13 => Some("cuda-runtime"),
        Build::Cuda12 => Some("cuda12-runtime"),
        Build::Vulkan | Build::Cpu | Build::Metal => None,
    }
}

/// The CUDA libraries a CUDA build imports that are in none of the places they come to: beside the program, the CUDA
/// 13 runtime component (`models/higgs-engine`), the CUDA 12.9 one (`models/cuda12-runtime`).
pub fn missing_libraries(repo_root: &Path, build: Build) -> Vec<&'static str> {
    let beside = repo_root.join("tools").join(build.folder());
    let cuda13 = repo_root.join("models").join("higgs-engine");
    let cuda12 = repo_root.join("models").join("cuda12-runtime");
    acpp_server::asset::missing_libraries(build, &[&beside, &cuda13, &cuda12])
}

/// The CrisperWhisper folder: env SPEECH_STUDIO_CRISPERWHISPER (a copy kept elsewhere, read only), else
/// `<models>/crisperwhisper2-turbo`.
pub fn crisperwhisper_dir(models_root: &Path) -> PathBuf {
    std::env::var_os("SPEECH_STUDIO_CRISPERWHISPER").map(PathBuf::from).unwrap_or_else(|| models_root.join(CRISPERWHISPER_DIR))
}

/// Nemotron 3 Diarization, which the server reads as it is: env SPEECH_STUDIO_SORTFORMER, else
/// `<models>/nemotron-diar/nemotron3_diar_v3.onnx`.
pub fn diar_onnx(models_root: &Path) -> PathBuf {
    std::env::var_os("SPEECH_STUDIO_SORTFORMER")
        .map(PathBuf::from)
        .unwrap_or_else(|| models_root.join(dub_asr::DIAR_MODEL_DIR).join(dub_asr::DIAR_MODEL_FILE))
}

/// Whether CrisperWhisper is downloaded (the Hugging Face repository folder as it is).
pub fn crisperwhisper_installed(models_root: &Path) -> bool {
    crisperwhisper_dir(models_root).join("model.safetensors").is_file()
}

/// The Higgs GGUF of the quantization the settings pick.
pub fn higgs_gguf(models_root: &Path) -> PathBuf {
    let (dir, quant) = crate::engines::models::resolve_tts(models_root, &crate::engines::models::load_selection(models_root));
    dir.join(format!("{quant}.gguf"))
}

/// Every model the server can carry now. A model whose files are not on disk stays out: the server reads each
/// configured model's contract when it starts, and one missing file would stop all of them.
fn models(models_root: &Path) -> Vec<ModelEntry> {
    let mut out = Vec::new();
    if crisperwhisper_installed(models_root) {
        out.push(ModelEntry::crisperwhisper(CRISPERWHISPER, &crisperwhisper_dir(models_root)));
    }
    let higgs = higgs_gguf(models_root);
    if higgs.is_file() {
        out.push(ModelEntry::higgs_tts(HIGGS, &higgs));
    }
    let diar = diar_onnx(models_root);
    if diar.is_file() {
        out.push(ModelEntry::nemotron_3_diar(DIAR, &diar));
    }
    if crate::engines::supertonic::installed(models_root) {
        out.push(ModelEntry::supertonic(SUPERTONIC, &crate::engines::supertonic::dir(models_root)));
    }
    if crate::engines::audio8::installed(models_root) {
        out.push(ModelEntry::audio8_tts(AUDIO8, &crate::engines::audio8::gguf(models_root)));
    }
    // the transcribe.cpp GGUFs, which audio.cpp reads in place
    for id in SERVER_ASR {
        let Some(model) = crate::engines::tcpp_asr::model(id) else { continue };
        let gguf = dub_asr::tcpp::model_path(models_root, model);
        if gguf.is_file() {
            out.push(if id == NEMOTRON { ModelEntry::nemotron_asr(id, &gguf) } else { ModelEntry::parakeet_tdt(id, &gguf) });
        }
    }
    out
}

fn config(repo_root: &Path, models_root: &Path, build: Build) -> ServerConfig {
    ServerConfig {
        exe: exe(repo_root, build),
        backend: build.backend(),
        device: 0,
        threads: std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(8).min(16),
        models: models(models_root),
        data_dir: repo_root.join("logs").join(build.folder()),
        log: false,
    }
}

/// The running server of this build carrying `model`: the one there is, or a new one in its place when the models
/// on disk changed or the process ended. An error, before the program starts, when the processor lacks the build's
/// floor, the program or its CUDA libraries are missing, or `model` is not on disk.
pub fn server(repo_root: &Path, models_root: &Path, build: Build, model: &str) -> Result<Arc<Server>, String> {
    let config = config(repo_root, models_root, build);
    if !config.exe.is_file() {
        return Err(t!("acpp-no-server", path = config.exe.display().to_string()));
    }
    if std::env::var_os("SPEECH_STUDIO_AUDIOCPP_SERVER").is_none() {
        let missing = missing_libraries(repo_root, build);
        if !missing.is_empty() {
            return Err(t!("acpp-no-libraries", files = missing.join(", ")));
        }
    }
    if !config.models.iter().any(|m| m.id == model) {
        return Err(t!("acpp-no-model", model = model.to_string()));
    }
    let server = acpp_server::SERVERS.get(&config, READY_WITHIN, &|_| {}).map_err(|e| localize(&e))?;
    tracing::info!("audiocpp_server {} ({build:?}) carries {:?}", server.pid(), config.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>());
    Ok(server)
}

/// Frees the memory of these models on every running server; their next request loads them again.
pub fn unload(ids: &[&str]) {
    for (pid, e) in acpp_server::SERVERS.unload(ids) {
        tracing::error!("[ERROR] audiocpp_server {pid} did not unload {ids:?}: {e}");
    }
}

/// The server's error in the window's language; a cancel is the job's own cancel.
pub fn localize(e: &AcppError) -> String {
    match e {
        AcppError::Cancelled => crate::jobs::CANCELLED.to_string(),
        other => {
            let error = other.to_string();
            if out_of_memory(&error) {
                t!("acpp-out-of-memory", error = error)
            } else if error.contains(" backend requested but ") {
                t!("acpp-no-device", error = error)
            } else {
                t!("acpp-error", error = error)
            }
        }
    }
}

/// ggml names a graph or buffer it could not place in video memory this way.
fn out_of_memory(error: &str) -> bool {
    let e = error.to_ascii_lowercase();
    e.contains("failed to allocate") || e.contains("out of memory") || e.contains("cudamalloc")
}

/// Higgs Audio v3 on the server; dropping it frees its memory there.
pub struct Higgs {
    server: std::sync::Mutex<Arc<Server>>,
    rate: u32,
    cancel: AtomicBool,
    /// Where its server is started again from when it ends mid-phrase.
    repo_root: PathBuf,
    models_root: PathBuf,
    build: Build,
}

/// `src` as Higgs's codec takes a reference (`media::ref_for_higgs`), kept in the studio's own cache under `cache`, by
/// the source's path, size and time: never beside the source (a library folder, a project, a neighbour's file).
fn higgs_reference(cache: &Path, src: &Path) -> Result<PathBuf, String> {
    let meta = std::fs::metadata(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let stamp = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos());
    let key = blake3::hash(format!("{}|{}|{stamp}", src.display(), meta.len()).as_bytes()).to_hex()[..16].to_string();
    std::fs::create_dir_all(cache).map_err(|e| format!("{}: {e}", cache.display()))?;
    let dst = cache.join(format!("{key}.wav"));
    if !dst.is_file() {
        crate::audio::media::ref_for_higgs(src, &dst)?;
    }
    Ok(dst)
}

/// Where the references prepared for Higgs are kept.
fn higgs_cache(repo_root: &Path) -> PathBuf {
    repo_root.join("temp").join("higgs-refs")
}

impl Higgs {
    /// On the build of the voicing device chosen in the settings (`tts_backend`, else `local_backend`; «auto» — see
    /// [`build_for`]).
    pub fn load(repo_root: &Path, models_root: &Path) -> Result<Higgs, String> {
        let gguf = higgs_gguf(models_root);
        if !gguf.is_file() {
            return Err(t!("render-higgs-model-missing", path = gguf.display().to_string()));
        }
        let build = build_for(crate::engines::models::server_choice(models_root, "tts_backend"))?;
        let server = server(repo_root, models_root, build, HIGGS)?;
        let rate = server.speech_rate(HIGGS).map_err(|e| localize(&e))?;
        Ok(Higgs { server: std::sync::Mutex::new(server), rate, cancel: AtomicBool::new(false), repo_root: repo_root.to_path_buf(), models_root: models_root.to_path_buf(), build })
    }

    fn server(&self) -> Arc<Server> {
        self.server.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn backend(&self) -> Backend {
        self.server().config().backend
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Stops the phrase being spoken at its next piece.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// One phrase, every piece to `piece` as it comes; the whole phrase in the answer. Without a reference the
    /// model speaks in its own voice. A server that ended before the phrase's first piece is started again and asked
    /// once more; one that ended later is told with what its log says.
    pub fn speak(&self, text: &str, reference: Option<&Path>, reference_text: Option<&str>, options: Map<String, Value>, piece: &mut dyn FnMut(&[f32])) -> Result<Vec<f32>, String> {
        self.cancel.store(false, Ordering::SeqCst);
        let reference = reference.map(|r| higgs_reference(&higgs_cache(&self.repo_root), r)).transpose()?;
        let req = SpeechRequest {
            model: HIGGS.to_string(),
            input: text.to_string(),
            voice_ref: reference.map(VoiceRef::Path),
            reference_text: reference_text.filter(|t| !t.trim().is_empty()).map(str::to_string),
            language: None,
            options,
            stall: SPEECH_STALL,
        };
        let mut all = Vec::new();
        let mut ask = |server: &Server, all: &mut Vec<f32>| {
            server.speech_stream(&req, &self.cancel, &mut |d| {
                piece(d);
                all.extend_from_slice(d);
                Flow::Continue
            })
        };
        let first = ask(&self.server(), &mut all);
        let Err(e) = first else { return Ok(all) };
        let Err(died) = self.server().check_alive() else { return Err(localize(&e)) };
        if !all.is_empty() {
            return Err(localize(&died));
        }
        tracing::warn!("audiocpp_server ended before the phrase's first piece ({died}); started again for one more try");
        let fresh = server(&self.repo_root, &self.models_root, self.build, HIGGS)?;
        *self.server.lock().unwrap_or_else(|p| p.into_inner()) = fresh.clone();
        ask(&fresh, &mut all).map_err(|again| format!("{} ({})", localize(&again), localize(&died)))?;
        Ok(all)
    }
}

impl Drop for Higgs {
    fn drop(&mut self) {
        unload(&[HIGGS]);
    }
}

#[cfg(test)]
mod tests {
    use super::out_of_memory;

    #[test]
    fn a_graph_ggml_cannot_place_reads_as_out_of_memory() {
        assert!(out_of_memory("audiocpp_server answered 500 (internal): failed to allocate Higgs TTS AR prefill graph"));
        assert!(out_of_memory("cudaMalloc(1048576): CUDA error 2 (out of memory)"));
        assert!(!out_of_memory("audiocpp_server sent nothing for 120 s"));
    }

    /// The frame counts of Higgs's codec encoders (audio.cpp `higgs_audio_tts/codec.cpp`) for a reference of `n` samples
    /// at 24 kHz, as the server resamples it to 16 kHz for the semantic one: a length `ref_for_higgs` gives agrees; a
    /// reference under a second, as it is cut, mostly does not.
    #[test]
    fn a_reference_of_whole_hops_gives_both_encoders_the_same_frames() {
        let conv = |len: i64, k: i64, s: i64, p: i64| (len + 2 * p - k) / s + 1;
        let semantic = |n16: i64| {
            let mut f = n16 + 2 * 160;
            for (k, s) in [(10, 5), (3, 2), (3, 2), (3, 2), (3, 2), (2, 2), (2, 2)] {
                f = conv(f, k, s, 0);
            }
            (f + 1) / 2
        };
        let acoustic = |n24: i64| {
            let mut f = conv(n24, 7, 1, 3);
            for r in [8, 5, 4, 2, 3] {
                f = conv(f, 2 * r, r, (r + 1) / 2);
            }
            conv(f, 3, 1, 1)
        };
        // the server's check: the acoustic count, or that of the input padded by half a hop each side, is the semantic one
        let agrees = |n24: i64, n16: i64| {
            let n24 = n24.max(24_000);
            acoustic(n24) == semantic(n16) || acoustic(n24 + 960) == semantic(n16)
        };
        for secs in [0.3, 1.0, 2.37, 5.5, 9.99, 12.0, 29.7] {
            let n = crate::audio::media::higgs_ref_samples(secs) as i64;
            assert!(n % 960 == 0 && n >= 24_000);
            assert!(agrees(n, n * 2 / 3), "{secs} s: {n} samples, acoustic {} semantic {}", acoustic(n), semantic(n * 2 / 3));
        }
        // a reference under a second, as it is cut: the codec pads only its acoustic side to a second, and refuses it
        let at = |n: i64, to: i64| (n * to + 22_050) / 44_100;
        let refused = (8_820..44_100).filter(|n| !agrees(at(*n, 24_000), at(*n, 16_000))).count();
        assert!(refused > 30_000, "most references under a second are refused as they are: {refused}");
    }

    /// Higgs on a real server, before and after: `HIGGS_ROOT` — a studio root with the server under `tools` and Higgs
    /// under `models`, `HIGGS_REF` — a voice recording. A reference of 0.6 s at 44.1 kHz given as it is cut, and given as
    /// `speak` prepares it; a server killed before a phrase, then asked through `speak`; a phrase cancelled as it goes.
    #[test]
    #[ignore = "needs the audio.cpp server, Higgs and a card"]
    fn higgs_on_a_real_server_before_and_after() {
        use std::sync::atomic::Ordering;
        // a test process has no job object of the studio: the servers it starts outlive it unless stopped here
        struct StopServers;
        impl Drop for StopServers {
            fn drop(&mut self) {
                acpp_server::SERVERS.stop_all();
            }
        }
        let _servers = StopServers;
        let root = std::path::PathBuf::from(std::env::var("HIGGS_ROOT").expect("HIGGS_ROOT"));
        let voice = std::path::PathBuf::from(std::env::var("HIGGS_REF").expect("HIGGS_REF"));
        let work = tempfile::tempdir_in(&root).unwrap();
        let short = work.path().join("short.wav");
        let ok = std::process::Command::new("ffmpeg").args(["-v", "error", "-y", "-ss", "1", "-t", "0.6", "-i"]).arg(&voice).args(["-ac", "1", "-ar", "44100", "-c:a", "pcm_s16le"]).arg(&short).status().unwrap();
        assert!(ok.success());
        let higgs = super::Higgs::load(&root, &root.join("models")).unwrap();
        let text = "Проверка голоса на коротком образце.";
        let opts = || serde_json::json!({ "seed": 7, "temperature": 0.7 }).as_object().unwrap().clone();
        let req = super::SpeechRequest {
            model: super::HIGGS.to_string(),
            input: text.to_string(),
            voice_ref: Some(super::VoiceRef::Path(short.clone())),
            reference_text: None,
            language: None,
            options: opts(),
            stall: super::SPEECH_STALL,
        };
        let raw = higgs.server().speech_stream(&req, &std::sync::atomic::AtomicBool::new(false), &mut |_| super::Flow::Continue);
        eprintln!("before: the 0.6 s reference as it is cut: {:?}", raw.as_ref().map(|d| d.samples).map_err(|e| e.to_string()));
        let started = std::time::Instant::now();
        let prepared = higgs.speak(text, Some(&short), None, opts(), &mut |_| {});
        eprintln!("after: the same reference prepared: {:?} in {:.1} s", prepared.as_ref().map(Vec::len), started.elapsed().as_secs_f64());
        let pid = higgs.server().pid();
        higgs.server().kill();
        let started = std::time::Instant::now();
        let again = higgs.speak(text, None, None, opts(), &mut |_| {});
        eprintln!("server {pid} killed before a phrase: {:?} in {:.1} s, server now {}", again.as_ref().map(Vec::len), started.elapsed().as_secs_f64(), higgs.server().pid());
        let started = std::time::Instant::now();
        let mut pieces = 0;
        let cancelled = higgs.speak(&"Длинная фраза для отмены посреди синтеза. ".repeat(6), None, None, opts(), &mut |_| {
            pieces += 1;
            if pieces == 3 {
                higgs.cancel.store(true, Ordering::SeqCst);
            }
        });
        eprintln!("cancelled at the third piece: {:?} after {:.1} s", cancelled.as_ref().map(Vec::len), started.elapsed().as_secs_f64());
        assert!(prepared.is_ok() && again.is_ok());
    }

    /// A reference prepared for Higgs is kept in the studio's cache, by the source's path, size and time: nothing new
    /// appears beside the source (a voice library stays the same set of voices), and a changed source is prepared anew.
    #[test]
    fn a_reference_for_higgs_is_kept_in_the_cache_not_beside_its_source() {
        let root = tempfile::tempdir().unwrap();
        let voices = root.path().join("voices");
        std::fs::create_dir_all(&voices).unwrap();
        let tone: Vec<f32> = (0..44_100 * 2).map(|i| 0.2 * (i as f32 * 0.03).sin()).collect();
        crate::audio::wavio::write_mono_f32(&voices.join("Анна.wav"), &tone, 44_100).unwrap();
        let before: Vec<String> = crate::voices::list(&voices).0.into_iter().map(|v| v.name).collect();
        let cache = super::higgs_cache(root.path());
        let first = super::higgs_reference(&cache, &voices.join("Анна.wav")).unwrap();
        assert!(first.starts_with(&cache));
        assert_eq!(super::higgs_reference(&cache, &voices.join("Анна.wav")).unwrap(), first, "kept, not made again");
        assert_eq!(std::fs::read_dir(&voices).unwrap().count(), 1, "nothing beside the voice");
        assert_eq!(crate::voices::list(&voices).0.into_iter().map(|v| v.name).collect::<Vec<_>>(), before);
        let (x, sr) = crate::audio::wavio::read_mono_f32(&first).unwrap();
        assert_eq!(sr, 24_000);
        assert_eq!(x.len() as u64 % crate::audio::media::HIGGS_HOP, 0);
    }
}
