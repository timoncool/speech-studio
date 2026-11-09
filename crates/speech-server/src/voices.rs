mod api;
pub(crate) mod fx;
mod jobs;
mod library;
mod profiles;
pub(crate) mod record;
pub(crate) mod ref_pick;
mod reference;

pub(crate) use library::{audio_path, rename};
pub(crate) use reference::prepare_reference;

pub(crate) use jobs::{ensure_words, transcribe_in, VoicePaths};

pub(crate) use library::{
    default_voice, free_name, guess_card, import_audio, level_voice, list, read_transcript, sanitize, write_card, write_transcript, write_words, Voice, VoiceCard,
    AGES, GENDERS,
};

pub(crate) mod http {
    pub(crate) use super::api::{
        clean, delete, denoise, design, design_keep, design_sample, enqueue, export, export_zip, import, import_zip_mapped,
        import_zip_route, level, list, listing, photo, ref_apply, ref_candidates, rename, restore, sample, set_card, set_photo, set_transcript,
        transcribe,
    };
}

pub(crate) mod matching {
    pub(crate) use super::profiles::{forget, listing, matches, remember, Paths, Speaker, MAX_SECS, MIN_SECS};
}

#[cfg(test)]
pub(crate) use library::{get, read_words};
