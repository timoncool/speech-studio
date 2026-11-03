//! Speech Studio service and desktop embedding API.

#[macro_use]
pub mod i18n;
mod app;
mod audio;
mod engines;
mod integrations;
mod jobs;
mod listening;
mod mcp;
mod platform;
mod setup;
mod speech;
mod text;
mod voice_gate;
mod voices;
mod wake_word;

pub use app::flush_hub_on_exit;
pub use app::{
    apply_gpu_choice, augment_path_for_tools, build_router, diagnostics, ensure_library_path, init_proxy_route, serve, serve_blocking, service,
    set_ort_dylib_env, AppState,
};
pub use dub_llm::net;
pub use engines::models::apply_faces_backend;
pub use jobs::JobQueue;
pub use platform::process_group;
#[cfg(windows)]
pub use setup::windows_downloads;
pub use setup::{Download, Extract, FileSpec, Requirement};
