use crate::engines::{hw, models};
use std::path::Path;
use std::path::PathBuf;

/// Корень моделей: env SPEECH_STUDIO_MODELS_ROOT, иначе <repo_root>/models.
pub(crate) fn models_root(repo_root: &Path) -> PathBuf {
    std::env::var("SPEECH_STUDIO_MODELS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root.join("models"))
}

/// Linux: CUDA-библиотеки «Первого запуска» (models/higgs-engine) нужны и самому процессу — CUDA-провайдер
/// onnxruntime связан с cuBLAS и cuRAND без RUNPATH, — а загрузчик читает LD_LIBRARY_PATH только при старте.
/// Если каталога в нём нет, процесс заменяет себя собой же с этим каталогом (exec). Вызывать первым делом при
/// старте, до того как что-то загружено.
pub fn ensure_library_path(repo_root: &Path) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;

        let dir = models_root(repo_root).join("higgs-engine");
        let current = std::env::var_os("LD_LIBRARY_PATH");
        if current.as_ref().is_some_and(|v| std::env::split_paths(v).any(|p| p == dir)) {
            return;
        }
        let mut paths = vec![dir];
        if let Some(v) = &current {
            paths.extend(std::env::split_paths(v));
        }
        let restarted: Result<(), String> = std::env::join_paths(paths).map_err(|e| e.to_string()).and_then(|joined| {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            Err(std::process::Command::new(exe)
                .args(std::env::args_os().skip(1))
                .env("LD_LIBRARY_PATH", joined)
                .exec()
                .to_string())
        });
        if let Err(e) = restarted {
            eprintln!("[ERROR] the restart with LD_LIBRARY_PATH failed ({e}): onnxruntime will not find the CUDA libraries, recognition runs on the CPU");
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = repo_root;
}

/// Прописать в PATH процесса каталоги скачанных бинарей (ffmpeg, llama, движок Higgs с CUDA/VC-DLL),
/// чтобы `Command::new("ffmpeg")` и подобные находили их без перезапуска после автозакачки. Вызывается
/// один раз при старте сервера (main.rs / Tauri-shell). Идемпотентно: не дублирует уже присутствующие.
pub fn augment_path_for_tools(repo_root: &Path) {
    let mut dirs = vec![
        repo_root.join("tools").join("ffmpeg"),
        crate::setup::llama_dir(repo_root, crate::setup::llama_build()),
        repo_root.join("models").join("higgs-engine"),
    ];
    if crate::engines::hw::gpu_report().cuda_build == Some(crate::engines::hw::CudaBuild::Cuda12) {
        dirs.push(repo_root.join("models").join("cuda12-runtime"));
    }
    let sep = if cfg!(windows) { ";" } else { ":" };
    let cur = std::env::var("PATH").unwrap_or_default();
    let mut prefix = String::new();
    for d in dirs {
        let s = d.to_string_lossy().to_string();
        if !s.is_empty() && !cur.split(sep).any(|p| p == s) && !prefix.split(sep).any(|p| p == s) {
            if !prefix.is_empty() {
                prefix.push_str(sep);
            }
            prefix.push_str(&s);
        }
    }
    if !prefix.is_empty() {
        std::env::set_var("PATH", format!("{prefix}{sep}{cur}"));
    }
}

/// The graphics card chosen in the settings goes into the process environment before anything starts CUDA: the studio's
/// engines and every engine process it starts see that card alone, counted in PCI bus order as NVML and nvidia-smi
/// count. Applies from the next start.
pub fn apply_gpu_choice(repo_root: &Path) {
    let sel = crate::engines::models::load_selection(&models_root(repo_root));
    if let Some(i) = sel.get("gpu_index").and_then(serde_json::Value::as_str).and_then(|v| v.parse::<u32>().ok()) {
        std::env::set_var("CUDA_DEVICE_ORDER", "PCI_BUS_ID");
        std::env::set_var("CUDA_VISIBLE_DEVICES", i.to_string());
        tracing::info!("graphics card: {i} (chosen in the settings)");
    }
}

/// Прописать ORT_DYLIB_PATH (onnxruntime 1.31.1) в окружение процесса до старта сервера: ort грузит dylib лениво,
/// а по голому имени Windows нашла бы раньше PATH onnxruntime.dll другой версии из System32 (Windows AI).
/// GPU-сборка (cuda13) — где CUDA работает, иначе процессорная (`pick_ort`); иначе — поиск
/// `dub_core::runtime::ensure_ort_dylib` (копия рядом с exe в портативе и др.).
pub fn set_ort_dylib_env(repo_root: &Path) {
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return;
    }
    ORT_PATH_OURS.store(true, std::sync::atomic::Ordering::Relaxed);
    let candidates = dub_core::runtime::ort_candidates(&repo_root.join("models"));
    let cuda = hw::gpu_report().cuda13_ok;
    if let Some(build) = pick_ort(&candidates, cuda) {
        std::env::set_var("ORT_DYLIB_PATH", build);
        return;
    }
    dub_core::runtime::ensure_ort_dylib();
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return;
    }
    // Nothing downloaded yet: point at the build we will fetch (GPU first on CUDA) so ort finds it once the ONNX
    // Runtime component lands. macOS ships a single Apple-silicon build, so there can be only one candidate.
    let pick = if cuda { candidates.first() } else { candidates.last() };
    if let Some(pick) = pick {
        std::env::set_var("ORT_DYLIB_PATH", pick);
    }
    eprintln!(
        "[ERROR] ONNX Runtime ({}) not found in {}: recognition and diarization will not start until the ONNX Runtime component is downloaded",
        dub_core::runtime::ORT_LIBRARY,
        repo_root.join("models").join("runtime").display()
    );
}

/// The ONNX Runtime build the process loads from `candidates` (the GPU build first, as `ort_candidates` gives them)
/// that are on disk: the GPU build only where CUDA works (`cuda`), the processor build otherwise — the small models
/// (stresses, the voice gate, denoising) need no card, and a GPU build on a machine without working CUDA is where it
/// fails to load; the GPU build when it is the only one there.
fn pick_ort(candidates: &[std::path::PathBuf], cuda: bool) -> Option<std::path::PathBuf> {
    let present: Vec<&std::path::PathBuf> = candidates.iter().filter(|c| c.is_file()).collect();
    let preferred = if cuda { present.first() } else { present.last() };
    preferred.map(|c| (*c).clone())
}

/// A session builder in the process's one ONNX Runtime environment (`dub_core::runtime::ort_env`, telemetry off); a
/// runtime that is not downloaded or cannot be loaded is told with its file and what to do.
pub(crate) fn ort_session() -> Result<ort::session::builder::SessionBuilder, String> {
    let env = dub_core::runtime::ort_env().map_err(|e| ort_failure(&e))?;
    ort::session::Session::builder(&env).map_err(|e| ort_failure(&e.to_string()))
}

fn ort_failure(error: &str) -> String {
    if !error.contains("failed to load from") {
        return t!("onnx-runtime", error = error.to_string());
    }
    let file = std::env::var("ORT_DYLIB_PATH").unwrap_or_default();
    t!("onnx-runtime-load", file = file, error = error.to_string())
}

#[cfg(test)]
mod tests {
    /// Without working CUDA the processor build of ONNX Runtime is loaded, with it the GPU build as before; one build
    /// on disk is loaded whatever it is, none is none.
    #[test]
    fn the_processor_build_is_loaded_where_cuda_does_not_work() {
        let dir = tempfile::tempdir().unwrap();
        let candidates = dub_core::runtime::ort_candidates(dir.path());
        assert_eq!(super::pick_ort(&candidates, false), None);
        for c in &candidates {
            std::fs::create_dir_all(c.parent().unwrap()).unwrap();
            std::fs::write(c, b"MZ").unwrap();
        }
        assert_eq!(super::pick_ort(&candidates, true).as_ref(), candidates.first(), "CUDA works: the GPU build");
        assert_eq!(super::pick_ort(&candidates, false).as_ref(), candidates.last(), "no CUDA: the processor build");
        std::fs::remove_file(candidates.last().unwrap()).unwrap();
        // a platform with one build of ONNX Runtime (a Mac) has none left; where there are two, the GPU build is
        let left = (candidates.len() > 1).then(|| candidates.first()).flatten();
        assert_eq!(super::pick_ort(&candidates, false).as_ref(), left, "the only build there");
    }

    /// A runtime that does not load is named by its file with what to do; another error of it is told as it is.
    #[test]
    fn a_runtime_that_does_not_load_is_named_with_what_to_do() {
        let told = super::ort_failure("ONNX Runtime: failed to load from `onnxruntime.dll`: LoadLibraryExW failed (source 1114)");
        assert!(told.contains("Visual C++") && told.contains("1114"), "{told}");
        assert!(!super::ort_failure("model is broken").contains("Visual C++"));
    }
}

/// ONNX Runtime just downloaded: the path on the build `pick_ort` takes, while the process has not loaded one yet (ort
/// loads it at the first session; a build loaded before is replaced only by a restart).
pub(crate) fn point_ort(repo_root: &Path) {
    if !ORT_PATH_OURS.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if let Some(build) = pick_ort(&dub_core::runtime::ort_candidates(&repo_root.join("models")), hw::gpu_report().cuda13_ok) {
        std::env::set_var("ORT_DYLIB_PATH", build);
    }
}

/// Маршрут прокси из models/active.json для всех HTTP-клиентов приложения (dub_llm::net): вызывать на старте.
/// Сохранение формы прокси перестраивает его сразу, без рестарта.
pub fn init_proxy_route(repo_root: &Path) {
    models::apply_proxy_route(&repo_root.join("models"));
}

static ORT_PATH_OURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) const UI_LANG_FILE: &str = "ui-lang.txt";
