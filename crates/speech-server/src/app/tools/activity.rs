//! Agent presence and activity journal policy shared by all tool entry points.

use super::*;

/// What a tool that changes something did, for the journal: the verb and the kind of thing (the window words them).
pub(super) const JOURNAL: &[(&str, &str, &str)] = &[
    ("project_create", "create", "project"),
    ("project_update", "update", "project"),
    ("project_delete", "delete", "project"),
    ("project_synth", "synth", "project"),
    ("project_export", "export", "project"),
    ("project_cast_characters", "cast", "project"),
    ("podcast_write", "write", "podcast"),
    ("speak_text", "synth", "file"),
    ("transcribe_file", "transcribe", "file"),
    ("verify_speech", "verify", "file"),
    ("separate_file", "separate", "file"),
    ("listen_create_from_file", "create", "listening"),
    ("listen_update", "update", "listening"),
    ("listen_transcribe", "transcribe", "listening"),
    ("listen_cut", "cut", "listening"),
    ("listen_export", "export", "listening"),
    ("listen_check", "verify", "listening"),
    ("listen_to_speech", "create", "project"),
    ("dictionary_set", "update", "dictionary"),
    ("dictionary_import", "import", "dictionary"),
    ("voice_download", "download", "voice"),
    ("voices_download_pack", "download", "voices"),
    ("voice_record_stop", "record", "voice"),
    ("voice_rename", "rename", "voice"),
    ("voice_delete", "delete", "voice"),
    ("voice_card_set", "update", "voice"),
    ("voice_transcript_set", "update", "voice"),
    ("voice_transcribe", "transcribe", "voice"),
    ("voice_clean", "clean", "voice"),
    ("voice_denoise", "clean", "voice"),
    ("voice_level", "update", "voice"),
    ("voice_design", "create", "voice"),
    ("voice_design_keep", "create", "voice"),
    ("voice_restore", "restore", "voice"),
    ("voice_ref_apply", "update", "voice"),
    ("listen_speaker_remember", "update", "listening"),
    ("speaker_profile_delete", "delete", "listening"),
    ("voice_import", "import", "voice"),
    ("voice_photo_set", "update", "voice"),
    ("voices_export", "export", "voices"),
    ("voices_import_zip", "import", "voices"),
    ("models_download", "download", "model"),
    ("models_cancel_download", "cancel", "download"),
    ("models_remove", "delete", "model"),
    ("models_import", "import", "model"),
    ("models_select", "select", "model"),
    ("settings_set", "update", "setting"),
    ("say_voice_set", "select", "voice"),
    ("engine_preset_apply", "apply", "preset"),
    ("proxy_settings_set", "update", "proxy"),
    ("google_set_key", "set", "key"),
    ("openrouter_set_key", "set", "key"),
    ("local_server_key_set", "set", "key"),
    ("google_delete_key", "delete", "key"),
    ("openrouter_delete_key", "delete", "key"),
    ("local_server_key_delete", "delete", "key"),
    ("openrouter_catalog_refresh", "refresh", "catalog"),
    ("url_fetch_cancel", "cancel", "download"),
    ("url_fetch_resume", "resume", "download"),
    ("url_fetch_delete", "delete", "download"),
    ("url_tool_update", "update", "tool"),
    ("job_cancel", "cancel", "job"),
    ("project_create_from_url", "create", "listening"),
    ("project_create_from_subtitles", "create", "project"),
    ("project_create_from_book", "create", "project"),
    ("project_import_package", "import", "project"),
    ("project_package", "export", "project"),
    ("project_export_save", "export", "file"),
    ("project_exports_to_folder", "export", "file"),
    ("project_export_save_as", "export", "file"),
    ("project_queue_clear", "cancel", "job"),
    ("project_queue_remove", "cancel", "job"),
    ("dictionary_try", "synth", "file"),
    ("listen_upload", "create", "listening"),
    ("listen_record_stop", "record", "listening"),
    ("cloud_set_key", "set", "key"),
    ("cloud_delete_key", "delete", "key"),
    ("cloud_catalog_refresh", "refresh", "catalog"),
    ("openai_api_key_create", "set", "key"),
    ("openai_api_key_delete", "delete", "key"),
    ("studio_language_set", "update", "setting"),
    ("models_import_pick", "import", "model"),
    ("diagnostics_report", "create", "file"),
];

/// The name a journal line gives what a tool changed: a name the agent passed, else the project's name, else the id it
/// named. Taken before the call, while a project about to be deleted still has its name.
pub(super) fn journal_target(args: &Value) -> String {
    const NAMED: &[&str] = &["name", "new_name", "title", "voice", "model"];
    const IDS: &[&str] = &["id", "key", "provider", "path", "pid", "project_id", "job_id", "fetch_id"];
    let text = |key: &&str| {
        args.get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    let project = || {
        ["pid", "project_id"]
            .iter()
            .find_map(text)
            .and_then(|pid| crate::app::journal::project_name(&pid))
    };
    NAMED
        .iter()
        .find_map(text)
        .or_else(project)
        .or_else(|| IDS.iter().find_map(text))
        .unwrap_or_default()
        .chars()
        .take(80)
        .collect()
}

/// Journals a tool's change once it succeeded; work an agent still waits for (`done: false`) is journaled when it is
/// asked for again and answers whole, and a call that found its finished work (`reused`) changed nothing.
pub(super) fn journal_call(name: &str, target: &str, answer: Option<&Value>) {
    let Some((_, verb, kind)) = JOURNAL.iter().find(|(tool, _, _)| *tool == name) else {
        return;
    };
    if answer.is_some_and(|a| a.get("done") == Some(&Value::Bool(false)) || a.get("reused") == Some(&Value::Bool(true))) {
        return;
    }
    crate::app::journal::agent_change(verb, kind, target);
}

// ---------------------------------------------------------------- origin and agent

pub(super) struct Agent {
    /// When an agent last called the server, and what it called.
    last_call: Mutex<Option<(std::time::Instant, String)>>,
    calls: AtomicU64,
    /// The name the agent gave when it connected (its clientInfo), shown beside what it says.
    name: Mutex<Option<String>>,
}

pub(super) fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| Agent {
        last_call: Mutex::new(None),
        calls: AtomicU64::new(0),
        name: Mutex::new(None),
    })
}

/// An agent that called within this long still counts as connected.
pub(super) const AGENT_PRESENT: Duration = Duration::from_secs(600);

pub(crate) fn seen(what: &str) {
    *agent().last_call.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((std::time::Instant::now(), what.to_string()));
    agent().calls.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn agent_present() -> bool {
    agent()
        .last_call
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .is_some_and(|(at, _)| at.elapsed() < AGENT_PRESENT)
}

/// Whether an agent is connected, for the settings page.
pub async fn status() -> Json<Value> {
    let last = agent().last_call.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
    Json(json!({
        "agent_connected": agent_present(),
        "agent_last_call": last.as_ref().map(|(_, what)| what.clone()),
        "agent_seconds_ago": last.as_ref().map(|(at, _)| at.elapsed().as_secs()),
        "agent_calls": agent().calls.load(Ordering::Relaxed),
        "agent_name": agent().name.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone(),
        "window_open": window::windows_open() > 0,
    }))
}

pub(crate) fn name_agent(name: &str) {
    *agent().name.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(name.chars().take(60).collect());
}
