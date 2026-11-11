//! Точка входа speech-server: поднимает axum на 127.0.0.1:8797 (порт env SPEECH_STUDIO_PORT).
//! Корень репо резолвится из env SPEECH_STUDIO_ROOT, иначе — рабочий каталог процесса.

use speech_server::service::{self, Claim};
use speech_server::{augment_path_for_tools, init_proxy_route, serve, AppState};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Рецепт «taskkill //IM speech-server.exe //F» не должен оставлять сирот (llama-server, ffmpeg, roformer).
    let bound = speech_server::process_group::bind_children_to_this_process();

    speech_server::diagnostics::init_logging();
    if !bound {
        tracing::error!("the process did not join its job object: sidecars are stopped only by their own destructors");
    }

    let repo_root = std::env::var("SPEECH_STUDIO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("cwd"));
    speech_server::diagnostics::log_to_file(&repo_root);
    speech_server::ensure_library_path(&repo_root);

    let port = service::listen_port().map_err(anyhow::Error::msg)?;
    let listener = match tokio::task::spawn_blocking(move || service::claim_port(port, Duration::from_secs(20))).await? {
        Ok(Claim::Bound(l)) => l,
        Ok(Claim::AlreadyRunning(r)) => anyhow::bail!(
            "Speech Studio {} is already running on 127.0.0.1:{port} ({}, repo_root={}); not starting a second service",
            r.version,
            r.service_executable,
            r.repo_root
        ),
        Err(busy) => return Err(busy.into()),
    };

    // Прописать в PATH каталоги скачанных бинарей (ffmpeg/llama/higgs-engine) до старта — чтобы после
    // автозакачки они находились без рестарта процесса.
    speech_server::apply_gpu_choice(&repo_root);
    augment_path_for_tools(&repo_root);
    init_proxy_route(&repo_root);
    speech_server::set_ort_dylib_env(&repo_root);

    let state = AppState::new(&repo_root);
    tracing::info!(
        "speech-server {}: repo_root={}, workspace={}, web_root={:?}",
        service::app_version(),
        state.repo_root.display(),
        state.workspace.display(),
        state.web_root
    );

    tracing::info!("listening on http://{}", listener.local_addr()?);
    serve(state, listener).await
}
