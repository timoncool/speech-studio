mod api;
mod export;
mod jobs;
mod live;
mod project;
mod recording;
mod source;
mod speakers;
mod storage;
mod transcript;
mod watch;

pub(crate) use jobs::ListenPaths;
pub(crate) use project::{summary, ListenProject, KIND};
#[cfg(test)]
pub(crate) use project::{Line, Word};
pub(crate) use recording::{recording, stop_recording, stop_response as stop_recording_response};
pub(crate) use source::{create_from_source, file_key, project_of_file, stem};
pub(crate) use speakers::speakers_audio;
pub(crate) use storage::{load, project_dir, save, source_path};
pub(crate) use transcript::export_transcript;

pub(crate) mod http {
    pub(crate) use super::api::{
        aligned, audio, check, cut, export, from_path, get, put, record_live, record_start, record_stop, separate_file, speaker_forget,
        speaker_profiles, speaker_remember, speakers_match, stats, subtitle_rules, to_speech, transcribe, upload, voices, watch_start,
        watch_status, watch_stop,
    };
}
pub(crate) mod watching {
    pub(crate) use super::watch::{resume, Dependencies};
}
pub(crate) mod recognition {
    pub(crate) use super::live::{engine, heard_at, mute, muted, recording_secs, state, state_with_words, switch, type_line};
    #[cfg(test)]
    pub(crate) use super::live::{conversation_for_test, end_for_test, words_for_test, TEST_SLOT};
}
pub(crate) mod subtitles {
    pub(crate) use super::export::{render_rows, Opts, Row};
}
