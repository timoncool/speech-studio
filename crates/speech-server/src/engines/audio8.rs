//! Audio8 TTS (arktts DualAR) on the audio.cpp server: a compact multilingual voice that clones zero-shot from a
//! reference and its transcript, streamed at 44.1 kHz mono. The server reads the standalone GGUF
//! (`Audio8-TTS-Realistic-Air-Q8_0.gguf`) in place, on the build of the voicing device (`tts_backend`); dropping the
//! model frees its memory there. At the Rust layer it behaves like Higgs — clone-or-own-voice, options, streaming — so
//! the dispatch in `tts_host` reuses the Higgs path; only the engine id and the model file differ.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use acpp_server::{Backend, Flow, Server, SpeechRequest, VoiceRef};
use serde_json::{Map, Value};

/// The folder of the weights inside the models root.
pub const DIR: &str = "audio8";
pub const ENGINE: &str = "audio8";
/// The single GGUF the server reads (the Realistic-Air package, q8_0).
pub const GGUF: &str = "Audio8-TTS-Realistic-Air-Q8_0.gguf";

/// The model ends every phrase with one codec frame (2048 samples at 44.1 kHz) that is not speech: after the voice
/// has died away it holds a burst, heard as a click at the end of the take.
const TERMINAL_FRAME: usize = 2048;
const CODEC_RATE: usize = 44_100;

/// How many of `len` samples at `rate` are the phrase itself, without the terminal frame.
fn spoken(len: usize, rate: u32) -> usize {
    len.saturating_sub(TERMINAL_FRAME * rate as usize / CODEC_RATE)
}
/// The longest wait for the next piece of a phrase; the first one also waits for the model to load.
const SPEECH_STALL: Duration = Duration::from_secs(120);

pub fn dir(models_root: &Path) -> PathBuf {
    models_root.join(DIR)
}

pub fn gguf(models_root: &Path) -> PathBuf {
    dir(models_root).join(GGUF)
}

pub fn installed(models_root: &Path) -> bool {
    gguf(models_root).is_file()
}

/// Audio8 TTS on the audio.cpp server; dropping it frees its memory there.
pub struct Model {
    server: Arc<Server>,
    rate: u32,
    cancel: AtomicBool,
}

impl Model {
    /// On the build of the voicing device chosen in the settings (`tts_backend`; «auto» — see [`crate::engines::acpp::build_for`]).
    pub fn load(repo_root: &Path, models_root: &Path) -> Result<Model, String> {
        let gguf = gguf(models_root);
        if !gguf.is_file() {
            return Err(t!("render-audio8-model-missing", path = gguf.display().to_string()));
        }
        let build = crate::engines::acpp::build_for(crate::engines::models::server_choice(models_root, "tts_backend"))?;
        let server = crate::engines::acpp::server(repo_root, models_root, build, crate::engines::acpp::AUDIO8)?;
        let rate = server.speech_rate(crate::engines::acpp::AUDIO8).map_err(|e| crate::engines::acpp::localize(&e))?;
        Ok(Model { server, rate, cancel: AtomicBool::new(false) })
    }

    pub fn backend(&self) -> Backend {
        self.server.config().backend
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Stops the phrase being spoken at its next piece.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// One phrase, every piece to `piece` as it comes; the whole phrase in the answer. With a reference WAV and its
    /// transcript the model clones that voice, without one it speaks in its own. The model finds the language itself.
    pub fn speak(&self, text: &str, reference: Option<&Path>, reference_text: Option<&str>, options: Map<String, Value>, piece: &mut dyn FnMut(&[f32])) -> Result<Vec<f32>, String> {
        self.cancel.store(false, Ordering::SeqCst);
        let req = SpeechRequest {
            model: crate::engines::acpp::AUDIO8.to_string(),
            input: text.to_string(),
            voice_ref: reference.map(|p| VoiceRef::Path(p.to_path_buf())),
            reference_text: reference_text.filter(|t| !t.trim().is_empty()).map(str::to_string),
            language: None,
            options,
            stall: SPEECH_STALL,
        };
        let mut all: Vec<f32> = Vec::new();
        let mut sent = 0;
        self.server
            .speech_stream(&req, &self.cancel, &mut |d| {
                all.extend_from_slice(d);
                let ready = spoken(all.len(), self.rate);
                if ready > sent {
                    piece(&all[sent..ready]);
                    sent = ready;
                }
                Flow::Continue
            })
            .map_err(|e| crate::engines::acpp::localize(&e))?;
        all.truncate(spoken(all.len(), self.rate));
        Ok(all)
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        crate::engines::acpp::unload(&[crate::engines::acpp::AUDIO8]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gguf_lives_under_the_audio8_folder() {
        let root = Path::new("models_root");
        assert_eq!(dir(root), root.join("audio8"));
        assert_eq!(gguf(root), root.join("audio8").join("Audio8-TTS-Realistic-Air-Q8_0.gguf"));
        assert!(!installed(root), "a bogus path holds no model file");
    }

    #[test]
    fn a_phrase_is_given_without_its_terminal_frame() {
        assert_eq!(spoken(2048 * 10, 44_100), 2048 * 9);
        assert_eq!(spoken(1024 * 10, 22_050), 1024 * 9, "the frame is counted at the rate the server answers in");
        assert_eq!(spoken(1000, 44_100), 0, "less than a frame holds no speech");
    }

    #[test]
    fn the_server_entry_is_the_audio8_tts_family_cloning_at_44k() {
        let m = acpp_server::ModelEntry::audio8_tts(ENGINE, Path::new("x.gguf"));
        assert_eq!(m.id, ENGINE);
        assert_eq!(m.family, "audio8_tts");
        assert_eq!(m.task, "tts", "a reference clones its voice, none speaks the model's own");
        assert_eq!(m.output_rate, Some(44_100), "the neural codec renders 44.1 kHz");
    }

    #[test]
    fn the_catalog_lists_audio8_as_a_reference_cloning_voice() {
        let e = crate::engines::get(ENGINE).expect("audio8 is in the engine catalog");
        assert_eq!(e.name, "Audio8 TTS");
        assert_eq!(e.clone, "ref_text");
        assert!(!e.streaming, "the server answers a phrase whole, its first sound comes with the last");
        assert!(!e.reads.stress, "the model places Russian stress itself");
        assert!(e.ref_text_required, "cloning needs the reference transcript");
        assert!(crate::engines::all().iter().any(|x| x.id == ENGINE), "registered in all()");
    }
}
