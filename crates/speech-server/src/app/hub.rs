//! Studio Hub in Speech Studio: anonymous statistics (only after the first-run screen showed its checkbox, off under
//! DO_NOT_TRACK or STUDIO_TELEMETRY=0) and the news feed shown on top of the bundled news.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::jobs::{JobCompletion, JobKind, JobStatus};

static HUB: OnceLock<Option<studio_hub_client::Hub>> = OnceLock::new();
/// The models folder, whose active.json says what a job ran on.
static MODELS: OnceLock<PathBuf> = OnceLock::new();
/// The projects folder, whose project says which engines voiced it.
static WORKSPACE: OnceLock<PathBuf> = OnceLock::new();

/// Starts the hub once, inside the tokio runtime of the server; a state file that cannot be read leaves the studio
/// without it, said once.
pub fn start(repo_root: &Path, workspace: &Path) {
    let _ = MODELS.set(crate::app::models_root(repo_root));
    let _ = WORKSPACE.set(workspace.to_path_buf());
    HUB.get_or_init(|| {
        let gpu = crate::engines::hw::gpu_report();
        let name = gpu.name.clone().unwrap_or_default().to_ascii_lowercase();
        let vendor = if gpu.nvidia {
            "nvidia"
        } else if name.contains("amd") || name.contains("radeon") {
            "amd"
        } else if name.contains("intel") {
            "intel"
        } else if name.is_empty() {
            "none"
        } else {
            "other"
        };
        let http = match dub_llm::net::async_builder().build() {
            Ok(http) => http,
            Err(e) => {
                eprintln!("[ERROR] Studio Hub is off for this run, its HTTP client was not built: {e}");
                return None;
            }
        };
        let os_label = format!(
            "{} {}",
            sysinfo::System::name().unwrap_or_else(|_| std::env::consts::OS.to_string()),
            sysinfo::System::os_version().unwrap_or_default()
        )
        .to_ascii_lowercase();
        let config = studio_hub_client::HubConfig {
            app: "speech".into(),
            version: crate::app::service::app_version().to_string(),
            data_dir: repo_root.to_path_buf(),
            http,
            os_label,
            gpu: studio_hub_client::Gpu {
                vendor: vendor.into(),
                vram_gb: studio_hub_client::Gpu::vram_bucket(crate::engines::hw::snapshot().total_vram),
                backend: if gpu.cuda13_ok { "cuda" } else { "cpu" }.into(),
            },
            ui_lang: "en".into(),
            urls: Vec::new(),
        };
        match studio_hub_client::Hub::new(config) {
            Ok(hub) => {
                hub.spawn();
                Some(hub)
            }
            Err(e) => {
                eprintln!("[ERROR] Studio Hub is off for this run, its state file cannot be read: {e}");
                None
            }
        }
    });
}

/// The day's statistics, sent as the studio closes instead of at its next start; at most a few seconds.
pub fn flush_on_exit() {
    let Some(hub) = HUB.get().and_then(Option::as_ref).cloned() else { return };
    match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime.block_on(hub.flush(std::time::Duration::from_secs(8))),
        Err(e) => eprintln!("[ERROR] the statistics were not sent at exit, no runtime: {e}"),
    }
}

/// Counts every call of the program's own API by its route template, an agent's calls included; unchanged when the
/// hub did not start.
pub fn track(router: axum::Router) -> axum::Router {
    match HUB.get().and_then(Option::as_ref) {
        Some(hub) => hub.track(router),
        None => router,
    }
}

/// The hub's routes for the window (`/v1/hub/*`); none when the hub did not start.
pub fn router<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    HUB.get().and_then(Option::as_ref).map(studio_hub_client::Hub::router).unwrap_or_default()
}

/// The other programs of timoncool's on this computer and their data folders, to take models from.
pub fn neighbors() -> Vec<(String, std::path::PathBuf)> {
    HUB.get().and_then(Option::as_ref).map(|hub| hub.neighbors().into_iter().map(|n| (n.app, n.data_dir)).collect()).unwrap_or_default()
}

/// A model download finished: the step between installing the studio and the first voicing.
pub fn setup_ready() {
    if let Some(hub) = HUB.get().and_then(Option::as_ref) {
        hub.setup_ready();
    }
}

/// A model download failed: its error code (network, disk_space, hash_mismatch...), the components and the host it
/// failed on. The detail itself is in the window's language and would split one cause into rows per language.
pub fn setup_failed(code: &str, ids: &[String], detail: &str) {
    if let Some(hub) = HUB.get().and_then(Option::as_ref) {
        hub.setup_failed(&setup_failure(code, ids, detail));
    }
}

/// Work that had the engine to itself (the job queue runs one job at a time): the seconds of sound it made and the
/// seconds the engine took, for the speed of each class of card.
pub(crate) fn worked(kind: &str, media_secs: f64, work: std::time::Duration) {
    if let Some(hub) = HUB.get().and_then(Option::as_ref) {
        hub.worked(kind, media_secs, work);
    }
}

fn setup_failure(code: &str, ids: &[String], detail: &str) -> String {
    let host = detail.split("://").nth(1).map(|rest| rest.split(['/', ':', ' ', ')']).next().unwrap_or("")).unwrap_or("");
    [code, &ids.join(","), host].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ")
}

/// A finished job, counted as `<kind>_done`, `<kind>_failed` or `<kind>_cancelled` (frame previews are not counted);
/// a failure keeps its reason, which the hub client scrubs, and finished work names the models it ran on.
pub(crate) fn job_ended(completion: &JobCompletion) {
    record_job(HUB.get().and_then(Option::as_ref), completion);
}

fn completion_counter(kind: JobKind, status: JobStatus) -> Option<String> {
    if kind == JobKind::Frame {
        return None;
    }
    let outcome = match status {
        JobStatus::Done => "done",
        JobStatus::Cancelled => "cancelled",
        _ => "failed",
    };
    Some(format!("{}_{outcome}", kind.as_str()))
}

fn record_job(hub: Option<&studio_hub_client::Hub>, completion: &JobCompletion) {
    let Some(hub) = hub else { return };
    let (kind, status) = (completion.kind, completion.status);
    let Some(counter) = completion_counter(kind, status) else { return };
    let (error, pid) = (completion.error.as_deref(), completion.pid.as_deref());
    hub.count(&counter, 1);
    match status {
        JobStatus::Done => {
            let stages: &[&str] = match kind {
                JobKind::Analyze => &["asr", "llm", "vision", "sep"],
                JobKind::Retranslate => &["llm"],
                JobKind::DubAudio => &["tts"],
                JobKind::Render => &["tts", "sep"],
                JobKind::VoiceTranscript => &["asr"],
                JobKind::VoiceClean => &["sep"],
                JobKind::VoiceDesign => &["tts"],
                JobKind::Synth => &["tts"],
                _ => &[],
            };
            match (kind, pid.and_then(voiced_by)) {
                (JobKind::Synth, Some(engines)) => hub.used_models(None, &engines, 1),
                _ => match MODELS.get() {
                    Some(root) if !stages.is_empty() => hub.used_models(None, &models_in_use(root, stages), 1),
                    _ => {}
                },
            }
        }
        JobStatus::Cancelled => {}
        _ => hub.failed(kind.as_str(), error.unwrap_or("")),
    }
}

/// The engines a project's blocks are voiced by, each as `tts:<engine>`: a project, a character and a block each pick
/// their own, so the studio's own choice of Higgs says nothing of them.
fn voiced_by(pid: &str) -> Option<Vec<String>> {
    let project = crate::speech::load(WORKSPACE.get()?, pid).ok()?;
    let engines: std::collections::BTreeSet<String> = project.blocks().map(|b| format!("tts:{}", project.engine_of(b))).collect();
    (!engines.is_empty()).then(|| engines.into_iter().collect())
}

/// What these stages run on now, each as `stage:model`: Dub has no named sets, its choice per stage is the set.
fn models_in_use(mroot: &Path, stages: &[&str]) -> Vec<String> {
    use crate::engines::models;
    use crate::engines::models::LlmBackend;
    let sel = models::load_selection(mroot);
    let slot = |key: &str| sel.get(key).and_then(serde_json::Value::as_str).map(str::trim).filter(|value| !value.is_empty()).unwrap_or("default").to_string();
    stages
        .iter()
        .map(|&stage| match stage {
            "tts" => match models::tts_provider(mroot) {
                "local" => format!("tts:higgs-{}", slot("tts")),
                provider => format!("tts:{provider}:{}", models::tts_model(mroot)),
            },
            "asr" if models::openrouter_asr_on(mroot) => format!("asr:openrouter:{}", slot("or_asr")),
            "asr" if slot("asr_engine") == "whisper" => "asr:crisperwhisper-turbo".to_string(),
            "asr" if slot("asr_engine") == "nemotron" => "asr:nemotron-3.5-int8".to_string(),
            "asr" if slot("asr_engine").starts_with("gguf-") => format!("asr:{}", slot("asr_engine")),
            "asr" => format!("asr:parakeet-{}", slot("asr")),
            "sep" => format!("sep:roformer-{}", slot("sep")),
            stage => match models::llm_backend(mroot, stage) {
                LlmBackend::Local => format!("{stage}:gemma-{}", slot("mt")),
                LlmBackend::Server => format!("{stage}:server"),
                LlmBackend::OpenRouter => format!("{stage}:openrouter:{}", models::openrouter_model(mroot, stage)),
                LlmBackend::Agent => format!("{stage}:agent"),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_download_failure_names_its_components_and_host() {
        let ids = vec!["asr-gguf-nemotron".to_string(), "transcribe-engine".to_string()];
        let detail = "https://huggingface.co/handy-computer/x/resolve/abc/m.gguf: io: connection reset";
        assert_eq!(setup_failure("network", &ids, detail), "network asr-gguf-nemotron,transcribe-engine huggingface.co");
        assert_eq!(setup_failure("disk_space", &[], "нет места на диске"), "disk_space");
    }

    #[test]
    fn completion_counters_keep_outcomes_and_exclude_frames() {
        for status in [JobStatus::Done, JobStatus::Error, JobStatus::Cancelled] {
            assert_eq!(completion_counter(JobKind::Frame, status), None);
        }
        assert_eq!(completion_counter(JobKind::Synth, JobStatus::Done).as_deref(), Some("synth_done"));
        assert_eq!(completion_counter(JobKind::Export, JobStatus::Error).as_deref(), Some("export_failed"));
        assert_eq!(completion_counter(JobKind::VoiceClean, JobStatus::Cancelled).as_deref(), Some("voice_clean_cancelled"));
    }

    #[tokio::test]
    async fn missing_hub_completion_observer_keeps_job_results() {
        let queue = crate::jobs::JobQueue::with_completion_observer(|completion| record_job(None, completion));
        for expected in [Ok(serde_json::json!({"done":true})), Err("job error".to_string())] {
            let answer = expected.clone();
            let (_, result) = queue
                .enqueue_awaitable(crate::jobs::JobMeta::new(JobKind::Write, None), Box::new(move |_| answer))
                .await.unwrap();
            let actual = tokio::time::timeout(std::time::Duration::from_secs(5), result).await.unwrap().unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn a_job_names_only_the_stages_it_ran() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("active.json"), r#"{"tts_provider":"local","tts":"q8_0","llm_provider":"local","mt":"e4b","sep":"fv6"}"#).unwrap();
        let retranslate = models_in_use(dir.path(), &["llm"]);
        assert_eq!(retranslate, ["llm:gemma-e4b"]);
        let render = models_in_use(dir.path(), &["tts", "sep"]);
        assert_eq!(render, ["tts:higgs-q8_0", "sep:roformer-fv6"]);
    }
}
