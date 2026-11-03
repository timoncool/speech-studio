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
    server: Arc<Server>,
    rate: u32,
    cancel: AtomicBool,
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
        Ok(Higgs { server, rate, cancel: AtomicBool::new(false) })
    }

    pub fn backend(&self) -> Backend {
        self.server.config().backend
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Stops the phrase being spoken at its next piece.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// One phrase, every piece to `piece` as it comes; the whole phrase in the answer. Without a reference the
    /// model speaks in its own voice.
    pub fn speak(&self, text: &str, reference: Option<&Path>, reference_text: Option<&str>, options: Map<String, Value>, piece: &mut dyn FnMut(&[f32])) -> Result<Vec<f32>, String> {
        self.cancel.store(false, Ordering::SeqCst);
        let req = SpeechRequest {
            model: HIGGS.to_string(),
            input: text.to_string(),
            voice_ref: reference.map(|p| VoiceRef::Path(p.to_path_buf())),
            reference_text: reference_text.filter(|t| !t.trim().is_empty()).map(str::to_string),
            language: None,
            options,
            stall: SPEECH_STALL,
        };
        let mut all = Vec::new();
        self.server
            .speech_stream(&req, &self.cancel, &mut |d| {
                piece(d);
                all.extend_from_slice(d);
                Flow::Continue
            })
            .map_err(|e| localize(&e))?;
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
}
