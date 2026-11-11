//! Silero VAD of the studio: the shared module of `dub_asr::vad` (Dub Studio takes the same) on the transcribe.cpp
//! build of the recognition device chosen in the settings.

use std::path::Path;

pub use dub_asr::vad::{speech, windows, VadConfig, FRAME, MAX_WINDOW_SECS, SR};

/// Скачаны ли модель разметки речи и движок, что её запускает.
pub fn installed(models_root: &Path) -> bool {
    missing(models_root).is_empty()
}

/// Компоненты разметки речи, которых нет на диске: модель Silero VAD и сборка transcribe.cpp устройства распознавания.
pub fn missing(models_root: &Path) -> Vec<String> {
    dub_asr::vad::missing(models_root, crate::engines::models::tcpp_build(models_root))
}

/// Вероятность речи на каждый кадр в 32 мс (16 кГц моно).
pub fn probabilities(models_root: &Path, samples: &[f32]) -> Result<Vec<f32>, String> {
    dub_asr::vad::probabilities(models_root, crate::engines::models::tcpp_build(models_root), samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_downloaded_is_not_installed() {
        let root = tempfile::tempdir().unwrap();
        let build = crate::engines::models::tcpp_build(root.path());
        assert_eq!(missing(root.path()), vec!["silero-vad".to_string(), build.dir().to_string()]);
        assert!(!installed(root.path()));
    }

    #[test]
    #[ignore = "needs a models folder with transcribe.cpp and Silero VAD and a 16 kHz mono WAV: SPEECH_VAD_MODELS, SPEECH_VAD_IN"]
    fn probabilities_of_a_file_for_comparison_with_the_python_wrapper() {
        let var = |k: &str| std::path::PathBuf::from(std::env::var(k).unwrap());
        let (x, _) = crate::audio::wavio::read_mono_f32(&var("SPEECH_VAD_IN")).unwrap();
        let p = probabilities(&var("SPEECH_VAD_MODELS"), &x).unwrap();
        println!("PROBS {}", p.iter().map(|v| format!("{v:.5}")).collect::<Vec<_>>().join(","));
        println!("SPANS {:?}", speech(&p, x.len(), &VadConfig::default()));
    }
}
