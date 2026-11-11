use super::Fetches;
use super::{models_root, runtime::UI_LANG_FILE};
use crate::{app, i18n, setup, JobQueue};
use crate::app::{hub, spa, Downloads};
use crate::jobs::store as job_store;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use dub_core::EngineOpts;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub repo_root: PathBuf,
    pub workspace: PathBuf,
    pub web_root: Option<PathBuf>,
    pub opts: Arc<EngineOpts>,
    pub jobs: JobQueue,
    /// Путь к модели диаризации (Nemotron 3 Diarization .onnx). Env SPEECH_STUDIO_SORTFORMER, иначе
    /// <models_root>/nemotron-diar/nemotron3_diar_v3.onnx.
    pub sortformer_onnx: PathBuf,
    /// llama-server(.exe) — сайдкар перевода/vision. Env SPEECH_STUDIO_LLAMA_BIN, иначе <repo>/tools/llama/llama-server(.exe).
    pub llama_bin: PathBuf,
    /// BSRoformer CLI (сепарация). Env SPEECH_STUDIO_BSROFORMER_DIR/<cli>, иначе <repo>/tools/bsroformer/bs_roformer-cli.exe.
    pub bsroformer_cli: PathBuf,
    /// GGUF модель сепарации. Env SPEECH_STUDIO_BSROFORMER_MODEL, иначе <repo>/models/bsroformer/voc_fv6-Q8_0.gguf.
    pub bsroformer_model: PathBuf,
    /// Каталог bundled-шрифтов субтитров. Env SPEECH_STUDIO_FONTS_DIR, иначе <repo>/fonts.
    pub fonts_dir: PathBuf,
    /// Корень моделей (для OCR: <root>/ocr/…). Env SPEECH_STUDIO_MODELS_ROOT, иначе <repo>/models.
    pub models_root: PathBuf,
    /// Каталог голосов-паков + записей с микрофона. Env SPEECH_STUDIO_VOICES, иначе <repo>/voices.
    pub voices_dir: PathBuf,
    /// Фоновая закачка компонентов (мимо GPU-очереди джоб; состояние — в /setup/status).
    pub downloads: Downloads,
    /// Загрузки видео по ссылке (yt-dlp), тоже мимо GPU-очереди; состояние — в /url/fetches.
    pub fetches: Fetches,
}
impl AppState {
    pub fn new(repo_root: impl AsRef<Path>) -> Self {
        let repo_root = repo_root.as_ref().to_path_buf();
        let workspace = repo_root.join("workspace");
        let _ = std::fs::create_dir_all(&workspace);
        // the language the window last showed, so what the studio says before a window opens is in it too
        match std::fs::read_to_string(repo_root.join(UI_LANG_FILE)) {
            Ok(code) => {
                if let Err(e) = i18n::set_language(code.trim()) {
                    tracing::error!("[ERROR] {UI_LANG_FILE}: {e}");
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => tracing::error!("[ERROR] {UI_LANG_FILE}: {e}"),
        }
        let interrupted = job_store::recover(&workspace);
        if interrupted > 0 {
            eprintln!("[jobs] interrupted project jobs: {interrupted} (they can be resumed)");
        }
        let web_root = spa::find_web_root(&repo_root);
        let mroot = models_root(&repo_root);
        setup::remove_retired(&repo_root, &mroot);
        setup::remove_retired_whisper(&repo_root);
        match app::credential_migration::migrate_legacy_selection(&mroot) {
            Ok(moved) if moved.openrouter_key || moved.proxy_password => {
                tracing::info!("secrets moved from active.json to the secret store: {moved:?}")
            }
            Ok(_) => {}
            Err(e) => tracing::error!("secrets from active.json were not moved to the store: {e:#}"),
        }
        let sortformer_onnx = crate::engines::acpp::diar_onnx(&mroot);
        // llama-server: env-override, иначе <repo>/tools/llama/llama-server(.exe) (негитуемый каталог,
        // кладёт установщик/раунд 3). Существование проверяет сама стадия перевода (fail-safe).
        let llama_bin = dub_llm::resolve_llama_bin(&crate::setup::llama_dir(&repo_root, crate::setup::llama_build()));
        let bsroformer_cli = std::env::var("SPEECH_STUDIO_BSROFORMER_DIR")
            .map(|d| PathBuf::from(d).join(dub_sep::ENGINE_CLI_FILE))
            .unwrap_or_else(|_| dub_sep::engine_dir(&repo_root).join(dub_sep::ENGINE_CLI_FILE));
        let bsroformer_model = dub_sep::model_path(&repo_root);
        let fonts_dir = std::env::var("SPEECH_STUDIO_FONTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| repo_root.join("fonts"));
        let voices_dir = std::env::var("SPEECH_STUDIO_VOICES")
            .map(PathBuf::from)
            .unwrap_or_else(|_| repo_root.join("voices"));
        let downloads = Downloads::open(&repo_root);
        let fetches = Fetches::open(&repo_root, &workspace);
        AppState {
            repo_root,
            workspace,
            web_root,
            opts: Arc::new(EngineOpts::default()),
            jobs: JobQueue::with_completion_observer(hub::job_ended),
            sortformer_onnx,
            llama_bin,
            bsroformer_cli,
            bsroformer_model,
            fonts_dir,
            models_root: mroot,
            voices_dir,
            downloads,
            fetches,
        }
    }

    pub(super) fn proj_dir(&self, pid: &str) -> Result<PathBuf, Response> {
        // pid — hex uuid; отсекаем любые сепараторы/`..` до касания ФС (защита от traversal в pid).
        if pid.is_empty() || !pid.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err((StatusCode::NOT_FOUND, "project not found").into_response());
        }
        let d = self.workspace.join(pid);
        if !d.exists() {
            return Err((StatusCode::NOT_FOUND, "project not found").into_response());
        }
        Ok(d)
    }

}
