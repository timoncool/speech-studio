use super::{capabilities_api::*, files::*, jobs_api::*, record_api::*, setup_api::*, ui_api::*};
use crate::app::{
    chat_api,
    cloud_api,
    diagnostics,
    endpoints,
    guard,
    hub,
    journal,
    openai_api,
    secrets_api,
    service,
    studio_settings,
    text_api as dict_api,
    timings,
    tools,
    url_api,
};
use crate::engines::tts_host;
use crate::listening::http as listen_api;
use crate::{mcp, AppState};
use crate::speech::{http as speech_api, stream as live, talk as say_api};
use crate::speech::http::director as director_api;
use crate::voices::http as voice_api;
use axum::routing::{delete, get, post};
use axum::Router;

#[cfg(test)]
mod tests;

pub fn build_router(state: AppState) -> Router {
    journal::open(&state.repo_root);
    tts_host::watch_idle(state.models_root.clone());
    let api = Router::new()
        .route("/health", get(service::health))
        .route("/engine/capabilities", get(capabilities))
        .route(
            "/engine/server/key",
            get(secrets_api::server_key_settings)
                .put(secrets_api::update_server_key)
                .delete(secrets_api::delete_server_key),
        )
        .route(
            "/engine/openrouter/settings",
            get(secrets_api::openrouter_settings)
                .put(secrets_api::update_openrouter_settings)
                .delete(secrets_api::delete_openrouter_settings),
        )
        .route(
            "/engine/google/settings",
            get(secrets_api::google_settings)
                .put(secrets_api::update_google_settings)
                .delete(secrets_api::delete_google_settings),
        )
        .route("/engine/google/models", get(secrets_api::google_models))
        .route(
            "/engine/proxy/settings",
            get(secrets_api::proxy_settings).put(secrets_api::update_proxy_settings),
        )
        .route("/engine/proxy/test", post(endpoints::proxy_test))
        .route("/engine/select", post(endpoints::select_model))
        .route("/engine/presets", get(endpoints::presets_list))
        .route("/engine/preset", post(endpoints::preset_apply))
        .route("/engine/openrouter/models", get(endpoints::openrouter_models))
        .route("/engine/openrouter/voices", get(endpoints::openrouter_voices))
        .route("/engine/openrouter/verify", post(endpoints::openrouter_verify))
        .route("/engine/openrouter/catalog", get(endpoints::openrouter_catalog))
        .route("/engine/openrouter/catalog/refresh", post(endpoints::openrouter_catalog_refresh))
        .route("/engine/server/models", get(endpoints::server_models))
        .route("/setup/status", get(setup_status))
        .route("/setup/download", post(setup_download))
        .route("/setup/cancel", post(setup_cancel))
        .route("/setup/remove", post(setup_remove))
        .route("/setup/open-models", post(setup_open_models))
        .route("/setup/browse", post(setup_browse))
        .route("/pick-folder", post(pick_folder))
        .route("/setup/import", post(setup_import))
        .route("/hw/snapshot", get(hw_snapshot))
        .route("/record/devices", get(record_devices))
        .route("/record/level", get(record_level))
        .route("/record/start", post(record_start))
        .route("/record/stop", post(record_stop))
        .route("/record/test", post(record_test))
        .route("/record/test/stop", post(record_test_stop))
        .route("/record/test/audio", get(record_test_audio))
        .route("/voices", get(voice_api::list))
        .route("/voices/download-pack", post(voices_download_pack))
        .route("/voices/catalog", get(voices_catalog))
        .route("/voice-gate", get(voice_gate_get).post(voice_gate_set))
        .route("/voice-gate/enroll", post(voice_gate_enroll))
        .route("/voice-gate/test", post(voice_gate_test))
        .route("/voice-gate/score", get(voice_gate_score))
        .route("/voice-gate/wake-score", get(voice_gate_wake_score))
        .route("/voice-gate/owner", delete(voice_gate_forget))
        .route("/voices/sample", get(voice_api::sample))
        .route("/voices/photo", get(voice_api::photo).post(voice_api::set_photo))
        .route("/voices/card", post(voice_api::set_card))
        .route("/voices/transcript", post(voice_api::set_transcript))
        .route("/voices/transcribe", post(voice_api::transcribe))
        .route("/voices/clean", post(voice_api::clean))
        .route("/voices/denoise", post(voice_api::denoise))
        .route("/voices/design", post(voice_api::design))
        .route("/voices/design/sample", get(voice_api::design_sample))
        .route("/voices/design/keep", post(voice_api::design_keep))
        .route("/voices/restore", post(voice_api::restore))
        .route("/voices/import", post(voice_api::import))
        .route("/voices/import-zip", post(voice_api::import_zip_route))
        .route("/voices/export", post(voice_api::export))
        .route("/voices/ref-candidates", post(voice_api::ref_candidates))
        .route("/voices/ref-apply", post(voice_api::ref_apply))
        .route("/voices/get", post(voices_get))
        .route("/voices/rename", post(voice_api::rename))
        .route("/voices/delete", post(voice_api::delete))
        .route("/settings/ui-lang", axum::routing::put(set_ui_lang))
        .route("/app/paths", get(studio_settings::app_paths))
        .route("/url/probe", get(url_api::probe_get).post(url_api::probe_post))
        .route("/url/fetches", get(url_api::fetches_list))
        .route("/url/fetches/{id}", get(url_api::fetch_get).delete(url_api::fetch_forget))
        .route("/url/fetches/{id}/cancel", post(url_api::fetch_cancel))
        .route("/url/fetches/{id}/resume", post(url_api::fetch_resume))
        .route("/url/tool", get(url_api::tool_status))
        .route("/url/tool/update", post(url_api::tool_update))
        .route("/engine/tags", get(speech_api::tags))
        .route("/engines", get(speech_api::engines))
        .route("/text/stress", post(speech_api::stress))
        .route("/text/direct-speech", post(speech_api::direct_speech))
        .merge(dict_api::router())
        .route("/cloud/catalog", get(cloud_api::catalog))
        .route("/cloud/catalog/refresh", post(cloud_api::refresh))
        .route("/cloud/form", get(cloud_api::form))
        .route("/cloud/estimate", post(cloud_api::estimate))
        .route("/cloud/keys", get(cloud_api::keys))
        .route(
            "/cloud/keys/{provider}",
            axum::routing::put(cloud_api::set_key).delete(cloud_api::delete_key),
        )
        .route("/cloud/spend", get(cloud_api::spend))
        .route("/v1/models", get(openai_api::models))
        .route("/openai/settings", get(openai_api::settings))
        .route("/openai/key", post(openai_api::new_key).delete(openai_api::delete_key))
        .route("/openai/log", get(openai_api::requests).delete(openai_api::clear_requests))
        .route("/v1/audio/speech", post(openai_api::speech))
        .route("/v1/audio/transcriptions", post(openai_api::transcriptions))
        .route("/listen/upload", post(listen_api::upload))
        .route("/listen/from-path", post(listen_api::from_path))
        .route("/separate", post(listen_api::separate_file))
        .route("/listen/record/start", post(listen_api::record_start))
        .route("/listen/record/stop", post(listen_api::record_stop))
        .route("/listen/record/live", get(listen_api::record_live))
        .route("/listen/watch", get(listen_api::watch_status).post(listen_api::watch_start))
        .route("/listen/watch/stop", post(listen_api::watch_stop))
        .route("/listen/{pid}", get(listen_api::get).put(listen_api::put))
        .route("/listen/{pid}/audio", get(listen_api::audio))
        .route("/listen/{pid}/transcribe", post(listen_api::transcribe))
        .route("/listen/{pid}/cut", post(listen_api::cut))
        .route("/listen/{pid}/export", post(listen_api::export))
        .route("/listen/subtitle-rules", get(listen_api::subtitle_rules))
        .route("/listen/{pid}/stats", get(listen_api::stats))
        .route("/listen/{pid}/check", post(listen_api::check))
        .route("/listen/{pid}/aligned", post(listen_api::aligned))
        .route("/listen/{pid}/to-speech", post(listen_api::to_speech))
        .route("/listen/{pid}/voices", post(listen_api::voices))
        .route("/listen/{pid}/speakers/match", post(listen_api::speakers_match))
        .route("/listen/{pid}/speakers/remember", post(listen_api::speaker_remember))
        .route("/speaker-profiles", get(listen_api::speaker_profiles))
        .route("/speaker-profiles/{name}", axum::routing::delete(listen_api::speaker_forget))
        .route("/projects/from_url", post(url_api::create_from_url))
        .route("/projects/podcast", post(director_api::podcast))
        .route("/projects/{pid}/cast", post(director_api::cast))
        .route("/projects", get(speech_api::list).post(speech_api::create))
        .route(
            "/projects/{pid}",
            get(speech_api::get).put(speech_api::put).delete(speech_api::delete),
        )
        .route("/projects/{pid}/restore", post(speech_api::restore))
        .route("/projects/{pid}/synth", post(speech_api::synth))
        .route("/projects/{pid}/queue", get(speech_api::queue).delete(speech_api::queue_clear))
        .route("/projects/{pid}/google", get(speech_api::google_report))
        .route("/projects/{pid}/queue/{n}", axum::routing::delete(speech_api::queue_remove))
        .route("/history", get(speech_api::history))
        .route("/speech/live", get(live::events))
        .route("/speech/say", post(say_api::start).delete(say_api::stop_all))
        .route(
            "/speech/say/{session}",
            get(say_api::state).post(say_api::text).delete(say_api::stop),
        )
        .route("/speech/listen", post(say_api::listen_start).delete(say_api::listen_stop))
        .route("/speech/listen/next", get(say_api::listen_next))
        .route("/speech/listen/metrics", get(say_api::listen_metrics))
        .route("/speech/listen/lines", get(say_api::listen_lines))
        .route("/speech/listen/floor", get(say_api::voice_floor))
        .route("/speech/listen/mute", post(say_api::listen_mute))
        .route("/mic/state", get(say_api::mic_state_route))
        .route("/timings", get(timings::route))
        .route("/speech/say/voice", get(say_api::voice_get).post(say_api::voice_set))
        .route("/speech/say/{session}/continue", post(say_api::say_continue))
        .route("/speech/listen/text", post(say_api::listen_text))
        .route("/projects/{pid}/export", post(speech_api::export))
        .route("/projects/{pid}/blocks/{bid}/ref", post(speech_api::own_ref))
        .route("/projects/{pid}/blocks/{bid}/ab", post(speech_api::ab))
        .route("/projects/{pid}/blocks/{bid}/fix", post(speech_api::fix))
        .route("/projects/{pid}/enrich", post(director_api::enrich))
        .route("/projects/{pid}/package", post(speech_api::package))
        .route("/speechproj/import", post(speech_api::import_package))
        .route("/projects/from-subtitles", post(speech_api::from_subtitles))
        .route("/books/preview", post(speech_api::book_preview))
        .route("/books/{id}/chapters/{index}", get(speech_api::book_chapter))
        .route("/projects/from-book", post(speech_api::from_book))
        .route("/projects/{pid}/export-check", get(speech_api::export_check))
        .route("/projects/{pid}/quality", get(speech_api::quality))
        .route("/projects/{pid}/check", post(speech_api::check))
        .route("/projects/{pid}/lint", get(speech_api::lint))
        .route("/projects/{pid}/auto-cast", get(speech_api::auto_cast))
        .route("/projects/{pid}/timeline", get(speech_api::timeline))
        .route("/projects/{pid}/save-export", post(speech_api::save_export))
        .route("/projects/{pid}/takes/{file}", get(speech_api::take_file))
        .route("/projects/{pid}/takes/{file}/peaks", get(speech_api::take_peaks))
        .route("/projects/{pid}/media/{file}", get(speech_api::media_file))
        .route("/projects/{pid}/media/{file}/peaks", get(speech_api::media_peaks))
        .route("/projects/{pid}/tracks/import", post(speech_api::tracks_import))
        .route("/projects/{pid}/tracks/upload", post(speech_api::tracks_upload))
        .route("/projects/{pid}/tracks/music", post(speech_api::tracks_music))
        .route("/music/studios", get(speech_api::music_studios))
        .route("/projects/{pid}/dictionary/try", post(speech_api::try_rule))
        .route("/projects/{pid}/tries/{file}", get(speech_api::try_file))
        .route("/projects/{pid}/exports/{file}", get(speech_api::export_file))
        .route("/projects/{pid}/exports", get(speech_api::exports_list))
        .route("/renders", get(speech_api::renders))
        .route("/projects/{pid}/exports-to-folder", post(speech_api::exports_to_folder))
        .route("/projects/{pid}/exports/{file}/reveal", post(speech_api::export_reveal))
        .route("/projects/{pid}/exports/{file}/save-as", post(speech_api::export_save_as))
        .route("/chat", post(chat_api::chat))
        .route("/chat/models", get(chat_api::models))
        .route("/diagnostics", get(diagnostics::get))
        .route("/diagnostics/report", post(diagnostics::make_report))
        .route("/diagnostics/report/reveal", post(diagnostics::reveal_report))
        .route("/diagnostics/logs/reveal", post(diagnostics::reveal_logs))
        .route("/journal", get(journal::read).post(journal::write).delete(journal::clear))
        .route("/journal/{id}", axum::routing::delete(journal::remove))
        .route("/jobs", get(jobs_list))
        .route("/jobs/{job_id}", get(job_get))
        .route("/jobs/{job_id}/cancel", post(job_cancel))
        .route("/jobs/{job_id}/events", get(job_events))
        .fallback(spa_fallback)
        .layer(axum::extract::DefaultBodyLimit::disable())
        .layer(axum::middleware::from_fn(tools::track))
        .merge(hub::router())
        .with_state(state);
    // counted before the MCP tools take the router, so an agent's calls count as the features they use
    let api = hub::track(api);
    tools::install(api.clone());
    api.route("/mcp", post(mcp::handle))
        .route("/mcp/status", get(tools::status))
        .route("/mcp/window", get(tools::window_events))
        .route(
            "/mcp/window/result",
            post(tools::window_result).layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route("/mcp/window/focus", post(tools::window_focus))
        .route("/mcp/agent-llm/v1/chat/completions", post(tools::agent_llm))
        .layer(guard::cors())
        .layer(axum::middleware::from_fn(guard::origin_guard))
}
