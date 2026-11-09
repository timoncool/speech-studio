mod acx;
mod api;
mod auto_cast;
mod book_import;
mod book_rebuild;
mod corpus;
mod creation;
mod director;
mod director_api;
mod floor;
mod jobs;
mod live;
mod metrics;
mod package;
mod preparation;
mod project;
mod read_along;
mod say;
mod take_fix;
mod text;
mod tracks;

pub(crate) use creation::from_transcript;
pub(crate) use jobs::{rename_voice, SpeechPaths};
pub(crate) use preparation::{dictionary_counts, prepare_text, unknown_words, PrepareText};

mod naming;
mod storage;
mod validation;
pub(crate) use crate::platform::workspace::project_file as project_path;
pub(crate) use preparation::voice_tag;
pub(crate) use say::is_echo;
pub(crate) use storage::{edit, load, save};
pub(crate) use validation::validate;

#[cfg(test)]
pub(crate) use project::{from_text, SpeechProject};

pub(crate) use director::{casting_rules, podcast_rules};

pub(crate) mod http {
    pub(crate) use super::api::{
        ab, auto_cast, book_chapter, book_preview, check, create, delete, direct_speech, engines, export, export_check, export_file,
        export_reveal, export_save_as, exports_list, exports_to_folder, fix, from_book, from_subtitles, get, google_report, history,
        import_package, lint, list, media_file, media_peaks, music_studios, own_ref, package, put, quality, queue, queue_clear,
        queue_remove, renders, restore, save_export, stress, synth, tags, take_file, take_peaks, timeline, tracks_import, tracks_music,
        tracks_upload, try_file, try_rule,
    };
    pub(crate) mod director {
        pub(crate) use super::super::director_api::{cast, enrich, podcast};
    }
}

pub(crate) mod talk {
    pub(crate) use super::say::{
        guard_mic, listen_mute, listen_next, listen_start, listen_stop, listen_text, mic_state_route, start, state, stop, stop_all, text,
        voice_get, voice_set,
    };
    pub(crate) use super::floor::{say_continue, voice_floor};
    pub(crate) use super::metrics::{lines as listen_lines, metrics as listen_metrics};
}

pub(crate) mod floor_settings {
    pub(crate) use super::floor::{is_value, with_defaults};
}

pub(crate) mod stream {
    pub(crate) use super::live::{events, Phrase};
}

mod synthesis;
