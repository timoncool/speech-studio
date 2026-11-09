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

/// Прописать ORT_DYLIB_PATH (onnxruntime 1.28) в окружение процесса до старта сервера: ort грузит dylib лениво,
/// а по голому имени Windows нашла бы раньше PATH onnxruntime.dll другой версии из System32 (Windows AI).
/// GPU-сборка (cuda13) приоритетнее — суперсет CPU+CUDA; `beside_exe` — копия рядом с exe (портатив).
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

pub fn set_ort_dylib_env(repo_root: &Path, beside_exe: &Path) -> Result<(), String> {
    ort_telemetry_off()?;
    if std::env::var_os("ORT_DYLIB_PATH").is_some() {
        return Ok(());
    }
    ORT_PATH_OURS.store(true, std::sync::atomic::Ordering::Relaxed);
    let rt = repo_root.join("models").join("runtime");
    let candidates = dub_core::runtime::ort_candidates(&repo_root.join("models"));
    let found = candidates
        .iter()
        .cloned()
        .chain([
            beside_exe.join(dub_core::runtime::ORT_LIBRARY),
            rt.join("onnxruntime-1.28.dll"),
            rt.join("onnxruntime.dll"),
        ])
        .find(|cand| cand.is_file());
    match found {
        Some(cand) => std::env::set_var("ORT_DYLIB_PATH", cand),
        None => {
            // Nothing downloaded yet: point at the build we will fetch (GPU first on CUDA) so ort
            // finds it once the ONNX Runtime component lands. macOS ships a single Apple-silicon
            // build, so there can be only one candidate -- never assume a GPU+CPU pair here.
            let pick = if hw::gpu_report().cuda13_ok { candidates.first() } else { candidates.last() };
            if let Some(pick) = pick {
                std::env::set_var("ORT_DYLIB_PATH", pick);
            }
            eprintln!("[ERROR] onnxruntime 1.28 not found in {}: recognition and diarization will not start until the ONNX Runtime component is downloaded", rt.display());
        }
    }
    Ok(())
}

/// Microsoft's ONNX Runtime builds send usage telemetry unless the environment turns it off. The options of the one
/// environment of the process are set before any session makes it; an environment set up earlier is an error.
fn ort_telemetry_off() -> Result<(), String> {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *OFF.get_or_init(|| ort::init().with_telemetry(false).commit()) {
        Ok(())
    } else {
        Err("ONNX Runtime was set up before its telemetry could be turned off".to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_environment_turns_telemetry_off_once_and_keeps_it() {
        assert_eq!(super::ort_telemetry_off(), Ok(()));
        assert_eq!(super::ort_telemetry_off(), Ok(()), "a second start of the setup keeps the options it set");
        assert!(!ort::init().with_telemetry(true).commit(), "options set later do not replace telemetry off");
    }
}

/// ONNX Runtime just downloaded: the path on the best installed build (CUDA first), while the process has not loaded
/// one yet (ort loads it at the first session; a build loaded before is replaced only by a restart).
pub(crate) fn point_ort(repo_root: &Path) {
    if !ORT_PATH_OURS.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if let Some(gpu) = dub_core::runtime::ort_candidates(&repo_root.join("models"))
        .into_iter()
        .find(|c| c.is_file())
    {
        std::env::set_var("ORT_DYLIB_PATH", gpu);
    }
}

/// Маршрут прокси из models/active.json для всех HTTP-клиентов приложения (dub_llm::net): вызывать на старте.
/// Сохранение формы прокси перестраивает его сразу, без рестарта.
pub fn init_proxy_route(repo_root: &Path) {
    models::apply_proxy_route(&repo_root.join("models"));
}

static ORT_PATH_OURS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) const UI_LANG_FILE: &str = "ui-lang.txt";
