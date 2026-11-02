//! ASR launch specifications and construction. Settings and preparation belong to the caller.

use std::path::{Path, PathBuf};

/// Выбор ASR-движка для одной джобы: модель transcribe.cpp (Parakeet, Nemotron) либо Whisper (бинарь + модель + квант + девайс).
#[derive(Debug, Clone)]
pub enum AsrChoice {
    /// A GGUF model run by transcribe.cpp on the GPU.
    Ggml { models_root: PathBuf, build: crate::engines::tcpp_asr::Build, model: &'static crate::engines::tcpp_asr::Model },
    /// CrisperWhisper 2.0 turbo in the studio's audiocpp_server on the build of the recognition device (`choice`, see
    /// `acpp::build_for`); the language of «auto» is heard by VoxLingua107 on the transcribe.cpp build of that device.
    CrisperWhisper { repo_root: PathBuf, models_root: PathBuf, choice: &'static str, build: crate::engines::tcpp_asr::Build },
    /// A model of transcribe.cpp's catalogue (Parakeet TDT v3, Nemotron 3.5) run by the studio's audiocpp_server on the build of the
    /// recognition device, reading the same GGUF; Live streams it through transcribe.cpp's `build`.
    Server { repo_root: PathBuf, models_root: PathBuf, choice: &'static str, build: crate::engines::tcpp_asr::Build, model: &'static crate::engines::tcpp_asr::Model },
}

/// What the window names as the device of a server build.
fn server_device(choice: &str) -> &'static str {
    match crate::engines::acpp::build_for(choice) {
        Ok(acpp_server::asset::Build::Cuda13 | acpp_server::asset::Build::Cuda12) => "CUDA",
        Ok(acpp_server::asset::Build::Vulkan) => "Vulkan",
        Ok(acpp_server::asset::Build::Cpu) => "CPU",
        Ok(acpp_server::asset::Build::Metal) => "Metal",
        Err(_) => "CUDA",
    }
}

impl AsrChoice {
    /// Строка для лога `[models]` — видно, каким движком реально пойдёт транскрипция.
    /// ВАЖНО: участвует в param_hash ASR-стадии (analyze.rs) — только СТАБИЛЬНЫЕ токены (вариант
    /// каталога, не абсолютный путь), иначе кэш/чекпоинты инвалидируются от переноса репо между
    /// машинами/папками (ревью-находка).
    /// What the window names as the recogniser: the model, its quantization and the transcribe.cpp build, or Whisper's
    /// model and compute.
    pub fn label(&self) -> String {
        match self {
            AsrChoice::Ggml { model, build, .. } => {
                let quant = model.file.trim_end_matches(".gguf").rsplit('-').next().unwrap_or(model.file);
                let on = match build {
                    crate::engines::tcpp_asr::Build::Cuda => "CUDA",
                    crate::engines::tcpp_asr::Build::Cuda12 => "CUDA 12.9",
                    crate::engines::tcpp_asr::Build::Vulkan => "Vulkan",
                    crate::engines::tcpp_asr::Build::Cpu => "CPU",
                };
                format!("{} {quant} ({on})", model.name)
            }
            AsrChoice::CrisperWhisper { choice, .. } => format!("CrisperWhisper 2.0 turbo ({})", server_device(choice)),
            AsrChoice::Server { model, choice, .. } => {
                let quant = model.file.trim_end_matches(".gguf").rsplit('-').next().unwrap_or(model.file);
                format!("{} {quant} (audio.cpp, {})", model.name, server_device(choice))
            }
        }
    }

    pub fn describe(&self) -> String {
        match self {
            AsrChoice::Ggml { model, .. } => format!("transcribe.cpp ({})", model.file),
            AsrChoice::CrisperWhisper { choice, .. } => format!("CrisperWhisper 2.0 turbo (intended, {choice})"),
            AsrChoice::Server { model, .. } => format!("audiocpp_server ({})", model.file),
        }
    }

    /// Whether it transcribes `lang` (`""` or `auto` is any).
    pub fn knows(&self, lang: &str) -> bool {
        lang.is_empty()
            || lang == "auto"
            || match self {
                AsrChoice::Ggml { model, .. } | AsrChoice::Server { model, .. } => crate::engines::tcpp_asr::knows(model, lang),
                AsrChoice::CrisperWhisper { .. } => {
                    let primary = lang.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
                    acpp_server::language::whisper_code(&primary).is_ok()
                }
            }
    }
}

/// A transcribe.cpp recogniser that unloads the process's model when its holder lets it go: a Listening job or a take
/// check at its end, Live when it is switched off (it holds one recogniser for the whole conversation).
struct Unloading(dub_asr::tcpp::TcppEngine);

impl Drop for Unloading {
    fn drop(&mut self) {
        dub_asr::tcpp::unload();
    }
}

impl dub_asr::AsrEngine for Unloading {
    fn transcribe(&mut self, wav: &Path, lang: &str) -> Result<Vec<dub_asr::Segment>, dub_asr::AsrError> {
        self.0.transcribe(wav, lang)
    }
    fn transcribe_turns(&mut self, wav: &Path, turns: &[dub_asr::Turn], lang: &str) -> Result<Vec<dub_asr::SpeakerSegment>, dub_asr::AsrError> {
        self.0.transcribe_turns(wav, turns, lang)
    }
    fn transcribe_many(&mut self, files: &[PathBuf], lang: &str) -> Vec<Result<String, dub_asr::AsrError>> {
        self.0.transcribe_many(files, lang)
    }
    fn transcribe_many_words(&mut self, files: &[PathBuf], lang: &str) -> Vec<Result<Vec<dub_asr::Word>, dub_asr::AsrError>> {
        self.0.transcribe_many_words(files, lang)
    }
}

/// A recogniser in the server of a job: its weights leave the card when the job lets it go, the server stays.
struct ServerUnloading(dub_asr::acpp::AcppAsr, &'static str);

impl Drop for ServerUnloading {
    fn drop(&mut self) {
        crate::engines::acpp::unload(&[self.1]);
    }
}

impl dub_asr::AsrEngine for ServerUnloading {
    fn transcribe(&mut self, wav: &Path, lang: &str) -> Result<Vec<dub_asr::Segment>, dub_asr::AsrError> {
        self.0.transcribe(wav, lang)
    }
    fn transcribe_turns(&mut self, wav: &Path, turns: &[dub_asr::Turn], lang: &str) -> Result<Vec<dub_asr::SpeakerSegment>, dub_asr::AsrError> {
        self.0.transcribe_turns(wav, turns, lang)
    }
    fn transcribe_many(&mut self, files: &[PathBuf], lang: &str) -> Vec<Result<String, dub_asr::AsrError>> {
        self.0.transcribe_many(files, lang)
    }
    fn transcribe_many_words(&mut self, files: &[PathBuf], lang: &str) -> Vec<Result<Vec<dub_asr::Word>, dub_asr::AsrError>> {
        self.0.transcribe_many_words(files, lang)
    }
}

pub fn build_engine(choice: &AsrChoice, prompt: Option<String>) -> Result<Box<dyn dub_asr::AsrEngine>, String> {
    tracing::info!("recognition: {} [{}]", choice.label(), choice.describe());
    let engine: Box<dyn dub_asr::AsrEngine> = match choice {
        AsrChoice::Ggml { models_root, build, model } => Box::new(Unloading(dub_asr::tcpp::TcppEngine::new(models_root, *build, model))),
        AsrChoice::CrisperWhisper { repo_root, models_root, choice, build } => {
            let id = crate::engines::acpp::CRISPERWHISPER;
            let server = crate::engines::acpp::server(repo_root, models_root, crate::engines::acpp::build_for(choice)?, id)?;
            let language = dub_asr::acpp::Language::Whisper {
                engine_dir: dub_asr::tcpp::engine_dir(models_root, *build),
                langid: dub_asr::tcpp::helper_path(models_root, &dub_asr::tcpp::LANGID),
            };
            return Ok(Box::new(LoadRetry(Box::new(ServerUnloading(dub_asr::acpp::AcppAsr::new(server, id, language).with_prompt(prompt), id)))));
        }
        AsrChoice::Server { repo_root, models_root, choice, model, .. } => {
            let id = model.id;
            // Nemotron is told the language; Parakeet finds it itself
            let language = if id == crate::engines::acpp::NEMOTRON { dub_asr::acpp::Language::Told } else { dub_asr::acpp::Language::Own };
            let server = crate::engines::acpp::server(repo_root, models_root, crate::engines::acpp::build_for(choice)?, id)?;
            Box::new(ServerUnloading(dub_asr::acpp::AcppAsr::new(server, id, language), id))
        }
    };
    // The heard-language helper and the Parakeet/Nemotron engine both load transcribe.dll the first time recognition
    // needs them; LoadRetry re-attempts a recognition whose only failure was that transient load (#21).
    match choice {
        AsrChoice::Ggml { models_root, build, model } | AsrChoice::Server { models_root, build, model, .. } => {
            Ok(Box::new(LoadRetry(dub_asr::tcpp::with_heard_language(engine, models_root, *build, model))))
        }
        _ => Ok(Box::new(LoadRetry(engine))),
    }
}


/// Retries a recognition whose only failure was a transient library load.
///
/// transcribe.dll (the VoxLingua107 heard-language helper and the Parakeet/Nemotron engine) and its CUDA modules are
/// loaded the instant recognition first needs them. On a busy GPU or a slow or antivirus-scanned disk `LoadLibraryExW`
/// can return nothing with no error of its own, failing a recognition that the same call makes a moment later (#21:
/// the third re-voicing failed on `transcribe.dll: LoadLibraryExW failed`). The library caches on the first success,
/// so a retry only re-runs the load, never the whole session.
struct LoadRetry(Box<dyn dub_asr::AsrEngine>);

const LOAD_RETRIES: usize = 2;

/// A load that failed with no OS error of its own — the transient loader failure, not a missing dependency.
fn transient_library_load(e: &dub_asr::AsrError) -> bool {
    e.to_string().contains("LoadLibraryExW failed")
}

fn retry_load<T>(mut run: impl FnMut() -> Result<T, dub_asr::AsrError>) -> Result<T, dub_asr::AsrError> {
    let mut out = run();
    let mut wait = std::time::Duration::from_millis(250);
    for _ in 0..LOAD_RETRIES {
        match &out {
            Err(e) if transient_library_load(e) => {
                tracing::warn!("recognition: library load failed, retrying in {} ms: {e}", wait.as_millis());
                std::thread::sleep(wait);
                wait *= 2;
                out = run();
            }
            _ => break,
        }
    }
    out
}

/// A batch where the load failed fails every file with the same error (`TcppEngine::many`): retry the whole batch.
fn retry_load_batch<T>(mut run: impl FnMut() -> Vec<Result<T, dub_asr::AsrError>>) -> Vec<Result<T, dub_asr::AsrError>> {
    let mut out = run();
    let mut wait = std::time::Duration::from_millis(250);
    for _ in 0..LOAD_RETRIES {
        if out.iter().any(|r| matches!(r, Err(e) if transient_library_load(e))) {
            tracing::warn!("recognition: library load failed, retrying the batch in {} ms", wait.as_millis());
            std::thread::sleep(wait);
            wait *= 2;
            out = run();
        } else {
            break;
        }
    }
    out
}

impl dub_asr::AsrEngine for LoadRetry {
    fn transcribe(&mut self, wav: &Path, lang: &str) -> Result<Vec<dub_asr::Segment>, dub_asr::AsrError> {
        retry_load(|| self.0.transcribe(wav, lang))
    }

    fn transcribe_turns(&mut self, wav: &Path, turns: &[dub_asr::Turn], lang: &str) -> Result<Vec<dub_asr::SpeakerSegment>, dub_asr::AsrError> {
        retry_load(|| self.0.transcribe_turns(wav, turns, lang))
    }

    fn transcribe_many(&mut self, files: &[PathBuf], lang: &str) -> Vec<Result<String, dub_asr::AsrError>> {
        retry_load_batch(|| self.0.transcribe_many(files, lang))
    }

    fn transcribe_many_words(&mut self, files: &[PathBuf], lang: &str) -> Vec<Result<Vec<dub_asr::Word>, dub_asr::AsrError>> {
        retry_load_batch(|| self.0.transcribe_many_words(files, lang))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_labels_survive_relocation_and_distinguish_the_selected_build() {
        let model = crate::engines::tcpp_asr::model("parakeet-v3").unwrap();
        let spec = |root: &str, build| AsrChoice::Ggml {
            models_root: root.into(),
            model,
            build,
        };
        assert_eq!(
            spec("old/models", crate::engines::tcpp_asr::Build::Cpu).label(),
            spec("new/models", crate::engines::tcpp_asr::Build::Cpu).label()
        );
        assert_eq!(spec("models", crate::engines::tcpp_asr::Build::Cpu).label(), "Parakeet TDT v3 Q8_0 (CPU)");
        assert_ne!(
            spec("models", crate::engines::tcpp_asr::Build::Cpu).label(),
            spec("models", crate::engines::tcpp_asr::Build::Cuda).label()
        );
    }

    /// SPEECH_STUDIO_ROOT: a studio root whose recognition is Nemotron 3.5; ASR_WAV: a recording; ASR_EXPECT: text the
    /// recognised recording must hold. Under Auto Nemotron is told the language VoxLingua107 hears.
    #[test]
    #[ignore]
    fn nemotron_under_auto_is_told_the_language_it_hears() {
        struct StopServers;
        impl Drop for StopServers {
            fn drop(&mut self) {
                acpp_server::SERVERS.stop_all();
            }
        }
        let _servers = StopServers;
        let root = PathBuf::from(std::env::var("SPEECH_STUDIO_ROOT").expect("SPEECH_STUDIO_ROOT"));
        let models = root.join("models");
        crate::set_ort_dylib_env(&root, &root).unwrap();
        std::env::set_var("DUB_ASR_BACKEND", crate::engines::models::stage_backend(&models, "asr_backend"));
        let choice = crate::app::recognition::choice(&root, &models, "auto", "test").unwrap();
        let mut asr = build_engine(&choice, None).unwrap();
        let heard = asr.transcribe(Path::new(&std::env::var("ASR_WAV").expect("ASR_WAV")), "auto").unwrap();
        let text = dub_asr::join_text(heard.iter().map(|s| s.text.as_str()));
        eprintln!("{} {text}", choice.label());
        assert!(text.contains(&std::env::var("ASR_EXPECT").expect("ASR_EXPECT")), "{text}");
    }
}
