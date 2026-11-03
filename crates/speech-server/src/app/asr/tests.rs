use super::*;

fn selection(root: &Path, value: serde_json::Value) -> std::path::PathBuf {
    let models = root.join("models");
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(models.join("active.json"), value.to_string()).unwrap();
    models
}

#[test]
fn asr_preparation_uses_its_stage_backend_instead_of_other_stages() {
    let root = tempfile::tempdir().unwrap();
    let models = selection(
        root.path(),
        json!({"local_backend":"gpu", "asr_backend":"cpu", "sep_backend":"gpu", "diar_backend":"gpu"}),
    );
    // Parakeet v3 runs in the server's processor build
    let cpu = crate::engines::acpp::component(crate::engines::acpp::build_for("cpu").unwrap()).to_string();
    assert_eq!(missing_parts(root.path(), &models, "parakeet"), ["parakeet".to_string(), cpu]);
    selection(
        root.path(),
        json!({"local_backend":"cpu", "asr_backend":"gpu", "sep_backend":"cpu", "diar_backend":"cpu"}),
    );
    // CrisperWhisper: its weights, the language id on transcribe.cpp, and the server build of the card with its runtime
    let mut whisper: Vec<String> =
        ["crisperwhisper-turbo", "langid", crate::engines::models::tcpp_build(&models).dir()].map(String::from).to_vec();
    if let Ok(build) = crate::engines::acpp::build_for("gpu") {
        whisper.push(crate::engines::acpp::component(build).to_string());
        whisper.extend(crate::engines::acpp::runtime_component(build).map(String::from));
    }
    assert_eq!(missing_parts(root.path(), &models, "whisper"), whisper);
}

#[test]
fn an_explicit_missing_recognizer_returns_an_error_without_installing() {
    let _language = crate::i18n::test_language("ru");
    let root = tempfile::tempdir().unwrap();
    let models = selection(root.path(), json!({"local_backend":"cpu", "asr_engine":"parakeet"}));
    let before = std::fs::read(models.join("active.json")).unwrap();
    assert_eq!(
        choice_of(root.path(), &models, "en", "voice", "whisper").unwrap_err(),
        t!("listen-whisper-missing")
    );
    assert!(!root.path().join("tools").exists());
    assert!(!models.join("whisper").exists());
    assert_eq!(std::fs::read(models.join("active.json")).unwrap(), before);
}

#[test]
fn each_request_resolves_current_selection_without_changing_previous_specification() {
    let root = tempfile::tempdir().unwrap();
    let models = selection(root.path(), json!({"local_backend":"cpu", "asr":"int8"}));
    for id in ["transcribe-engine-cpu", "parakeet", "parakeet-ultra"] {
        // installed means the markers at their pinned size: the content is not read here
        for m in crate::setup::manifest().iter().find(|c| c.id == id).unwrap().markers {
            let path = root.path().join(m.rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::File::create(path).unwrap().set_len(m.expect).unwrap();
        }
    }
    let first = choice(root.path(), &models, "en", "voice").unwrap();
    selection(root.path(), json!({"local_backend":"cpu", "asr":"ultra-int8"}));
    let second = choice(root.path(), &models, "en", "voice").unwrap();
    assert!(
        matches!(first, crate::engines::asr::AsrChoice::Server {model, choice: "cpu", ..} if model.id == "parakeet-v3")
    );
    assert!(
        matches!(second, crate::engines::asr::AsrChoice::Ggml {model, build: crate::engines::tcpp_asr::Build::Cpu, ..} if model.id == "parakeet-ultra")
    );
    assert_ne!(first.label(), second.label());
}

#[test]
fn an_explicit_engine_language_error_keeps_the_requested_engine() {
    let _language = crate::i18n::test_language("ru");
    let root = tempfile::tempdir().unwrap();
    let models = selection(root.path(), json!({"local_backend":"cpu"}));
    let err = choice_of(root.path(), &models, "zz", "voice", "nemotron").unwrap_err();
    assert_eq!(
        err,
        t!(
            "listen-gguf-lang",
            engine = crate::engines::tcpp_asr::model("nemotron").unwrap().name,
            lang = "zz".to_string()
        )
    );
}

#[test]
fn missing_parts_name_the_chosen_recogniser() {
    let root = tempfile::tempdir().unwrap();
    let models = root.path().join("models");
    let whisper = super::missing_parts(root.path(), &models, "whisper");
    assert!(whisper.iter().any(|id| id == "crisperwhisper-turbo"), "{whisper:?}");
    assert!(whisper.iter().any(|id| id == "langid"), "{whisper:?}");
    let nemotron = super::missing_parts(root.path(), &models, "nemotron");
    assert!(nemotron.iter().any(|id| id == "asr-gguf-nemotron"), "{nemotron:?}");
    assert!(!nemotron.iter().any(|id| id.starts_with("crisperwhisper")), "{nemotron:?}");
}

/// Outside the model's own languages the recording goes to CrisperWhisper, so «auto» picks the likeliest language it
/// can read: VoxLingua107 also names languages Whisper has no token for (Esperanto, Guarani, Interlingua).
#[test]
fn a_language_no_recogniser_knows_gives_way_to_the_likeliest_one_it_does() {
    let ranked = |cs: &[(&str, f32)]| cs.iter().map(|(c, p)| ((*c).to_string(), *p)).collect::<Vec<_>>();
    assert_eq!(recognisable_language(&ranked(&[("eo", 0.5), ("zh", 0.3), ("ru", 0.2)])).as_deref(), Some("zh"));
    assert_eq!(recognisable_language(&ranked(&[("ru", 0.6), ("be", 0.4)])).as_deref(), Some("ru"));
    // all candidates outside any recogniser: the top is named, to be refused by name
    assert_eq!(recognisable_language(&ranked(&[("gn", 0.7), ("ia", 0.3)])).as_deref(), Some("gn"));
    assert_eq!(recognisable_language(&[]), None);
}
