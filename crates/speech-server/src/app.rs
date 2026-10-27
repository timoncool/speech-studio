mod chat;
mod chat_api;
mod cloud;
mod cloud_api;
mod cloud_tts;
mod context;
mod credential_migration;
pub mod diagnostics;
mod endpoints;
mod events;
mod google_speech;
mod guard;
mod hub;
mod journal;
mod listening;
mod listening_to_speech;
mod listening_voices;
mod llm_provider;
mod localize;
mod openai_api;
mod revisions;
mod secrets_api;
pub mod service;
mod spa;
mod speaker_profiles;
mod spending;
mod studio_settings;
mod text_api;
mod timings;
mod tools;
mod url_api;
mod url_import;
mod voice_rename;

mod asr;
mod downloads;
mod engine_setup;
#[cfg(test)]
mod project_primitives_tests;
#[cfg(test)]
mod project_storage_tests;

mod bootstrap;
mod capabilities_api;
mod files;
mod jobs_api;
mod record_api;
mod router;
mod runtime;
mod setup_api;
mod state;
mod ui_api;
pub use bootstrap::{serve, serve_blocking};
pub(crate) use files::{reveal_in_explorer, serve_file_range};
pub(crate) use jobs_api::enqueue_error;
pub use router::build_router;
pub use runtime::{apply_gpu_choice, augment_path_for_tools, ensure_library_path, init_proxy_route, set_ort_dylib_env};
pub(crate) use runtime::{models_root, point_ort};
pub use state::AppState;

pub(crate) use downloads::Downloads;
pub use hub::flush_on_exit as flush_hub_on_exit;
pub(crate) use hub::worked as hub_worked;
pub(crate) use url_import::Fetches;

pub(crate) use guard::{foreign_origin, local_origin};

mod http;
pub(crate) use http::{joined, refuse};

#[cfg(test)]
pub(crate) use context::RequestContext;
pub(crate) use context::{carry, carry_job, current_actor};

pub(crate) use events::publish;

#[cfg(test)]
pub(crate) use revisions::revision;
pub(crate) use revisions::{save_with_revision, PROJECT_CHANGED};

pub(crate) use localize::Localize;

pub(crate) use timings::{arrived, note, since_arrival};

#[cfg(test)]
pub(crate) use engine_setup::{download_components, remove_components, setup_status};
pub(crate) use engine_setup::{ensure_components, ensure_job_components, ensure_llm_components};

pub(crate) use listening::{paths as listening_paths, watch_dependencies};

pub(crate) use listening_to_speech::convert as convert_listening_to_speech;

pub(crate) use listening_voices::extract as extract_listening_voices;
pub(crate) use voice_rename::rename_voice;

pub(crate) mod recognition {
    #[cfg(test)]
    pub(crate) use super::asr::choice;
    pub(crate) use super::asr::{choice_of, ensure, missing_parts, second_opinion, spoken_language, transcribe, transcribe_many, Heard};
}

pub(crate) mod profiles {
    pub(crate) use super::speaker_profiles::{matches, remember, Paths};
}

pub(crate) mod remote {
    pub(crate) use super::cloud::{speak, transcribe, Run};
}

pub(crate) mod costs {
    pub(crate) use super::spending::{check_limit, record, Spent};
}

pub(crate) mod google {
    pub(crate) use super::google_speech::{phrase, prefetch, report, setup, Want};
}

pub(crate) mod llm {
    pub(crate) use super::llm_provider::{open, LlmMode, LlmOpen, LlmProvider};
}

pub(crate) mod voice_synthesis {
    pub(crate) use super::cloud_tts::{model_and_voice, synth};
}

pub(crate) mod agent {
    pub(crate) use super::tools::{
        annotations, app_version, by_agent, cut, guides, name_agent, resource, run_for_agent, seen, tools, Answer, LANGUAGES_URI, LIMIT,
        SKILL, SKILL_URI,
    };
    #[cfg(test)]
    pub(crate) use super::tools::{chat_call, chat_tools};
}
