//! The rest of the studio as tools: the service, the voices' sound, the cloud models of fal and Runware, the
//! OpenAI-compatible API, recording and the queue, books and subtitles, the export files, the Assistant, the
//! diagnostics, the journal, the streams the window follows and the agent connection itself.

use std::path::PathBuf;

use axum::http::Method;
use serde_json::{json, Value};

use super::{body_without, composite, file_name, get, given, id_only, nothing, object, pid, post, project_only, project_path, query, segment, send, text, Call, Payload, Tool};

/// A file on this computer an agent names, checked before it is sent.
fn local_file(args: &Value, name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(text(args, name)?);
    if !path.is_file() {
        return Err(format!("{} is not a file on this computer.", path.display()));
    }
    Ok(path)
}

/// A file sent to a route as the page uploads it, with the text fields given.
fn upload(route: &str, path: PathBuf, field: &str, fields: Vec<(String, String)>) -> Result<Call, String> {
    let name = file_name(&path);
    Ok(Call { method: Method::POST, path: route.into(), payload: Payload::Form { fields, files: vec![(field.to_string(), path, name)] } })
}

/// A stream the window follows, read for `seconds` (at most 55) or until it ends.
fn stream(method: Method, path: String, body: Option<Value>, args: &Value) -> Result<Call, String> {
    let seconds = args.get("seconds").and_then(Value::as_u64).unwrap_or(10).clamp(1, 55);
    Ok(Call { method, path, payload: Payload::Stream { body, seconds } })
}

fn seconds() -> Value {
    json!({ "type": "integer", "minimum": 1, "maximum": 55, "description": "how long to listen, 10 s by default" })
}

fn provider() -> Value {
    json!({ "type": "string", "enum": ["fal", "runware"] })
}

pub(super) fn tools() -> Vec<Tool> {
    vec![
        // ---------------------------------------------------------------- the service
        Tool {
            name: "studio_health",
            description: "The service's identity: status, app id, version, the executable that runs, its folder (repo_root) and port - what the desktop shell checks before starting a second copy.",
            schema: nothing,
            call: |_| get("/health".into()),
        },
        Tool {
            name: "folder_pick",
            description: "Ask the user to choose a folder in a dialog on their screen; answers its path (dir), null when they cancelled. Use it when the user, not you, should decide where something goes.",
            schema: nothing,
            call: |_| post("/pick-folder".into(), json!({})),
        },
        Tool {
            name: "models_import_pick",
            description: "Ask the user to choose a folder with model files in a dialog on their screen, then take every component found there as models_import does (id limits it to one component). Answers what was imported, or nothing when they cancelled.",
            schema: || object(json!({ "id": { "type": "string", "description": "one component id from models_status" } }), &[]),
            call: |args| post("/setup/browse".into(), match given(args, "id") { Some(id) => json!({ "id": id }), None => json!({}) }),
        },
        Tool {
            name: "studio_language_set",
            description: "The language the studio speaks in its messages and errors (the window sets it to the one it is shown in): en, ru, es, fr, pt or zh.",
            schema: || object(json!({ "lang": { "type": "string", "enum": ["en", "ru", "es", "fr", "pt", "zh"] } }), &["lang"]),
            call: |args| send(Method::PUT, "/settings/ui-lang".into(), json!({ "lang": text(args, "lang")? })),
        },
        Tool {
            name: "engine_tags",
            description: "The expression tags an engine reads (engine id from engines_list; higgs by default): their categories with each tag, the format a tag is written in, and whether the engine takes a block's direction. Tags differ per engine: write only the ones listed here.",
            schema: || object(json!({ "engine": { "type": "string" } }), &[]),
            call: |args| get(format!("/engine/tags{}", query(&[("engine", given(args, "engine"))]))),
        },
        // ---------------------------------------------------------------- the voices' sound
        Tool {
            name: "voice_sample",
            description: "A voice's reference sound as a file on this computer (path, type, size): listen to it or send it on. original true gives it as it was before voice_clean or voice_denoise.",
            schema: || object(json!({ "name": { "type": "string", "description": "voice name from voices_list" }, "original": { "type": "boolean" } }), &["name"]),
            call: |args| {
                let original = args.get("original").and_then(Value::as_bool).unwrap_or(false).then(|| "1".to_string());
                get(format!("/voices/sample{}", query(&[("name", Some(text(args, "name")?)), ("original", original)])))
            },
        },
        Tool {
            name: "voice_photo_get",
            description: "A voice's photo, as a picture you see.",
            schema: || id_only("name", "voice name from voices_list"),
            call: |args| get(format!("/voices/photo{}", query(&[("name", Some(text(args, "name")?))]))),
        },
        Tool {
            name: "voice_design_sample",
            description: "One candidate of voice_design as a sound file on this computer: id is the batch id its job answered, k the candidate's number.",
            schema: || object(json!({ "id": { "type": "string" }, "k": { "type": "integer" } }), &["id", "k"]),
            call: |args| get(format!("/voices/design/sample{}", query(&[("id", Some(text(args, "id")?)), ("k", given(args, "k"))]))),
        },
        // ---------------------------------------------------------------- videos and sound by link
        Tool {
            name: "url_probe_cookies_text",
            description: "url_probe with the cookies.txt content given as text (what the window sends when the user drops the file) instead of its path.",
            schema: || object(json!({ "url": { "type": "string" }, "cookies_text": { "type": "string", "description": "the content of a Netscape cookies.txt" } }), &["url", "cookies_text"]),
            call: |args| post("/url/probe".into(), json!({ "url": text(args, "url")?, "cookies_text": text(args, "cookies_text")? })),
        },
        Tool {
            name: "project_create_from_url",
            description: "Download a video or sound by link (YouTube and the other sites yt-dlp knows) into a new Listening recording: quality from url_probe (best by default), subs_lang to take the site's subtitles in that language, cookies the path of a cookies.txt for videos that need a signed-in browser. It runs in the background beside the jobs: its fetch.id goes to studio_wait (or url_fetch_get), and the finished download names the recording (pid).",
            schema: || object(json!({ "url": { "type": "string" }, "quality": { "type": "string" }, "subs_lang": { "type": "string" }, "cookies": { "type": "string", "description": "path of a Netscape cookies.txt" } }), &["url"]),
            call: |args| {
                let mut body = json!({ "url": text(args, "url")? });
                for field in ["quality", "subs_lang", "cookies"] {
                    if let Some(value) = given(args, field) {
                        body[field] = value.into();
                    }
                }
                post("/projects/from_url".into(), body)
            },
        },
        // ---------------------------------------------------------------- the pronunciation dictionary of a project
        Tool {
            name: "project_dictionary_counts",
            description: "How many times each pronunciation rule (of the studio's dictionary and the project's) fires in a project's text: a rule that never fires is misspelt or not needed.",
            schema: project_only,
            call: |args| get(project_path(args, "/dictionary-counts")?),
        },
        Tool {
            name: "project_unknown_words",
            description: "The words of a project's text that no dictionary knows (where the stress or the reading may come out wrong), with how often each occurs: the words to check and give a rule with dictionary_set.",
            schema: project_only,
            call: |args| get(project_path(args, "/unknown-words")?),
        },
        Tool {
            name: "dictionary_try",
            description: "Hear a pronunciation rule before saving it: the phrase is voiced without and with the rule (an entry as dictionary_set takes it: find, say, kind word, regex or phonemes, whole_word, case_sensitive) in the project's engine and voice (or voice). A job: follow it with studio_wait; its result names the file, which dictionary_try_file gives.",
            schema: || object(json!({ "pid": pid(), "phrase": { "type": "string" }, "rule": { "type": "object", "properties": { "find": { "type": "string" }, "say": { "type": "string" }, "kind": { "type": "string", "enum": ["word", "regex", "phonemes"] }, "whole_word": { "type": "boolean" }, "case_sensitive": { "type": "boolean" } }, "required": ["find", "say"], "description": "one dictionary entry" }, "voice": { "type": "string" } }), &["pid", "phrase", "rule"]),
            call: |args| post(project_path(args, "/dictionary/try")?, body_without(args, &["pid"])),
        },
        Tool {
            name: "dictionary_try_file",
            description: "The sound of a dictionary_try as a file on this computer: file is the name its job answered.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string", "description": "file name from the dictionary_try job" } }), &["pid", "file"]),
            call: |args| get(project_path(args, &format!("/tries/{}", segment(&text(args, "file")?)))?),
        },
        // ---------------------------------------------------------------- cloud models: fal and Runware
        Tool {
            name: "cloud_catalog",
            description: "The cloud speech models of fal and Runware (models) and the state of each provider's catalog and key (providers); task narrows the models to speech or transcribe.",
            schema: || object(json!({ "task": { "type": "string", "enum": ["speech", "transcribe"] } }), &[]),
            call: |args| get(format!("/cloud/catalog{}", query(&[("task", given(args, "task"))]))),
        },
        Tool {
            name: "cloud_catalog_refresh",
            description: "Read the catalogs of fal and Runware again now, in the background; cloud_catalog shows them when they arrive.",
            schema: nothing,
            call: |_| post("/cloud/catalog/refresh".into(), json!({})),
        },
        Tool {
            name: "cloud_model_form",
            description: "The settings a cloud model takes, read from its own schema: each field with its type, choices, default and limits - the values cloud_estimate and a block's cloud settings use.",
            schema: || object(json!({ "provider": provider(), "id": { "type": "string", "description": "model id from cloud_catalog" } }), &["provider", "id"]),
            call: |args| get(format!("/cloud/form{}", query(&[("provider", Some(text(args, "provider")?)), ("id", Some(text(args, "id")?))]))),
        },
        Tool {
            name: "cloud_estimate",
            description: "What a cloud model will cost before running it: runs times for chars characters of text with the settings in values. Nothing is paid.",
            schema: || object(json!({ "provider": provider(), "id": { "type": "string" }, "values": { "type": "object" }, "runs": { "type": "integer", "minimum": 1 }, "chars": { "type": "integer", "minimum": 0 } }), &["provider", "id"]),
            call: |args| post("/cloud/estimate".into(), json!({
                "provider": text(args, "provider")?,
                "id": text(args, "id")?,
                "values": args.get("values").cloned().unwrap_or_else(|| json!({})),
                "runs": args.get("runs").and_then(Value::as_u64).unwrap_or(1),
                "chars": args.get("chars").and_then(Value::as_u64).unwrap_or(0),
            })),
        },
        Tool {
            name: "cloud_keys",
            description: "Whether a key of fal and of Runware is stored and where it comes from (the studio's store or an environment variable). The keys themselves are never shown.",
            schema: nothing,
            call: |_| get("/cloud/keys".into()),
        },
        Tool {
            name: "cloud_set_key",
            description: "Store the key of fal or Runware: the provider checks it first and it is kept only if it works. It is never shown back.",
            schema: || object(json!({ "provider": provider(), "api_key": { "type": "string" } }), &["provider", "api_key"]),
            call: |args| send(Method::PUT, format!("/cloud/keys/{}", segment(&text(args, "provider")?)), json!({ "api_key": text(args, "api_key")? })),
        },
        Tool {
            name: "cloud_delete_key",
            description: "Remove the stored key of fal or Runware; its models stop until a key is set again.",
            schema: || object(json!({ "provider": provider() }), &["provider"]),
            call: |args| send(Method::DELETE, format!("/cloud/keys/{}", segment(&text(args, "provider")?)), json!({})),
        },
        Tool {
            name: "cloud_spend",
            description: "What the cloud models of fal and Runware cost so far (the price after each run and the estimate before it), with the daily limit for agents and programs (daily_limit_usd); pid narrows it to one project.",
            schema: || object(json!({ "pid": pid() }), &[]),
            call: |args| get(format!("/cloud/spend{}", query(&[("project", given(args, "pid"))]))),
        },
        // ---------------------------------------------------------------- the OpenAI-compatible API for other programs
        Tool {
            name: "openai_api_status",
            description: "The studio's OpenAI-compatible API for other programs on this computer (/v1/audio/speech, /v1/audio/transcriptions, /v1/models): whether a key guards it and where it comes from, and the model names it takes for speech (each with its engine) and for transcription. The key itself is never shown.",
            schema: nothing,
            call: |_| get("/openai/settings".into()),
        },
        Tool {
            name: "openai_api_key_create",
            description: "Make a new key for the OpenAI-compatible API, replacing the old one: from now on a program needs Authorization: Bearer <key>. The key is answered this once, for the user to give their program.",
            schema: nothing,
            call: |_| post("/openai/key".into(), json!({})),
        },
        Tool {
            name: "openai_api_key_delete",
            description: "Remove the key of the OpenAI-compatible API: programs on this computer reach it without one again.",
            schema: nothing,
            call: |_| send(Method::DELETE, "/openai/key".into(), json!({})),
        },
        Tool {
            name: "openai_api_log",
            description: "The latest requests other programs made to the OpenAI-compatible API, newest first: when, which endpoint, model, voice, the text's size, the status, how long it took and the error.",
            schema: nothing,
            call: |_| get("/openai/log".into()),
        },
        Tool {
            name: "openai_api_log_clear",
            description: "Empty the log of the OpenAI-compatible API's requests.",
            schema: nothing,
            call: |_| send(Method::DELETE, "/openai/log".into(), json!({})),
        },
        Tool {
            name: "openai_api_models",
            description: "GET /v1/models of the OpenAI-compatible API, as a program sees it: the model names it takes.",
            schema: nothing,
            call: |_| get("/v1/models".into()),
        },
        Tool {
            name: "openai_api_speech",
            description: "Call /v1/audio/speech as a program would (to check what it gets): model (an engine alias from openai_api_status), input text, voice (a library voice name), response_format (mp3 by default, wav, flac, aac, pcm), speed (0.25 to 4), instructions (the direction), and the studio's own engine settings over the engine's defaults: seed, temperature, top_k, top_p, max_tokens, steps (Supertonic), loudness (off, video, podcast, acx). Answers the sound as a file on this computer.",
            schema: || object(json!({ "model": { "type": "string" }, "input": { "type": "string" }, "voice": { "type": "string" }, "response_format": { "type": "string", "enum": ["mp3", "wav", "flac", "aac", "pcm"] }, "speed": { "type": "number" }, "instructions": { "type": "string" }, "seed": { "type": "integer", "minimum": 0 }, "temperature": { "type": "number" }, "top_k": { "type": "integer", "minimum": 0 }, "top_p": { "type": "number" }, "max_tokens": { "type": "integer", "minimum": 0 }, "steps": { "type": "integer", "minimum": 1 }, "loudness": { "type": "string", "enum": ["off", "video", "podcast", "acx"] } }), &["model", "input", "voice"]),
            call: |args| post("/v1/audio/speech".into(), args.clone()),
        },
        Tool {
            name: "openai_api_transcribe",
            description: "Call /v1/audio/transcriptions as a program would: a sound or video file on this computer (path), model (from openai_api_status), response_format (json, text, verbose_json, srt, vtt).",
            schema: || object(json!({ "path": { "type": "string" }, "model": { "type": "string" }, "response_format": { "type": "string", "enum": ["json", "text", "verbose_json", "srt", "vtt"] }, "language": { "type": "string" } }), &["path"]),
            call: |args| {
                let mut fields = Vec::new();
                for field in ["model", "response_format", "language"] {
                    if let Some(value) = given(args, field) {
                        fields.push((field.to_string(), value));
                    }
                }
                upload("/v1/audio/transcriptions", local_file(args, "path")?, "file", fields)
            },
        },
        // ---------------------------------------------------------------- Listening: sound in
        Tool {
            name: "block_ref_set",
            description: "Give one block of a Speech project its own reference instead of a library voice: a sound file on this computer and what is said in it (text; empty - the studio recognises it). whole true puts the whole clip into the engine's context, so the block continues that clip's speech (Higgs); otherwise the clip is cut to the reference limit like a library voice. The block goes stale. Clear it with project_update, own_ref null.",
            schema: || object(json!({ "pid": pid(), "block": { "type": "string", "description": "block id from project_get" }, "path": { "type": "string", "description": "a sound or video file on this computer" }, "text": { "type": "string" }, "whole": { "type": "boolean" } }), &["pid", "block", "path"]),
            call: |args| {
                let mut fields = Vec::new();
                if let Some(t) = args.get("text").and_then(Value::as_str) {
                    fields.push(("text".to_string(), t.to_string()));
                }
                if args.get("whole").and_then(Value::as_bool) == Some(true) {
                    fields.push(("whole".to_string(), "1".to_string()));
                }
                upload(&project_path(args, &format!("/blocks/{}/ref", segment(&text(args, "block")?)))?, local_file(args, "path")?, "file", fields)
            },
        },
        Tool {
            name: "listen_upload",
            description: "Make a Listening recording of a sound or video file by uploading a copy of it, as the window does with a dropped file (listen_create_from_file reads it where it lies). Answers the recording's id.",
            schema: || id_only("path", "a sound or video file on this computer"),
            call: |args| upload("/listen/upload", local_file(args, "path")?, "file", Vec::new()),
        },
        Tool {
            name: "listen_record_live",
            description: "What live recognition of the microphone recording under way has heard: its phrases with their start and end in seconds, whether it still runs, and why it stopped when it failed.",
            schema: || object(json!({}), &[]),
            call: |_| get("/listen/record/live".into()),
        },
        Tool {
            name: "listen_record_stop",
            description: "Stop the microphone recording the person started in Listening and keep it as a Listening recording: answers its id; listen_transcribe recognises it.",
            schema: nothing,
            call: |_| post("/listen/record/stop".into(), json!({})),
        },
        Tool {
            name: "listen_audio",
            description: "A Listening recording's sound as a file on this computer.",
            schema: || id_only("pid", "recording id"),
            call: |args| get(format!("/listen/{}/audio", segment(&text(args, "pid")?))),
        },
        // ---------------------------------------------------------------- Speech: the queue, history, the live sound
        Tool {
            name: "project_queue",
            description: "The requests of a project waiting behind its running voicing, in order, each with its number (n) and when it was added.",
            schema: project_only,
            call: |args| get(project_path(args, "/queue")?),
        },
        Tool {
            name: "project_google_report",
            description: "Google Gemini voicing of a project: the batch waiting at Google now (its state and how many phrases are ready), the last job's result (mode standard or batch, phrases asked, ready, failed with Google's reason, cost estimate, seconds) and every finished or expired batch. The mode is the setting google_tts_mode: standard (direct requests, or_concurrency at a time), batch (half the tariff, Google takes up to a day) or auto (direct up to 10 phrases, batch from 11).",
            schema: project_only,
            call: |args| get(project_path(args, "/google")?),
        },
        Tool {
            name: "project_queue_clear",
            description: "Take every waiting request of a project off its queue; the voicing that runs goes on.",
            schema: project_only,
            call: |args| send(Method::DELETE, project_path(args, "/queue")?, json!({})),
        },
        Tool {
            name: "project_queue_remove",
            description: "Take one waiting request off a project's queue by its number (n) in project_queue.",
            schema: || object(json!({ "pid": pid(), "n": { "type": "integer", "minimum": 0 } }), &["pid", "n"]),
            call: |args| send(Method::DELETE, project_path(args, &format!("/queue/{}", args.get("n").and_then(Value::as_u64).ok_or("'n' is required")?))?, json!({})),
        },
        Tool {
            name: "history_list",
            description: "The takes voiced, across projects or in one (pid): the project, the block, its speaker and text, the take's number and file, when it was made, its length, how long it took (rtf), the first sound (first_ms), the seed and sampling, the voice and engine, its recognition error (wer) and whether it is the block's active take.",
            schema: || object(json!({ "pid": pid() }), &[]),
            call: |args| get(format!("/history{}", query(&[("pid", given(args, "pid"))]))),
        },
        Tool {
            name: "speech_live_watch",
            description: "Listen to what is being voiced right now, for seconds: each block that starts, restarts (a defective take said again) and ends, with how soon its first sound came (first_ms); the sound itself is kept as a WAV file per block on this computer (files).",
            schema: || object(json!({ "seconds": seconds() }), &[]),
            call: |args| stream(Method::GET, "/speech/live".into(), None, args),
        },
        Tool {
            name: "say_start",
            description: "Start speaking aloud in the studio as you write: say_text sends your text piece by piece while you write it, each finished phrase is voiced at once and heard from the studio's window, without waiting for the end. voice is a voice of voices_list or an engine's own voice, engine an id of engines_list (default qwen3-tts: the first sound in about 0.1 s), lang a code or auto, direction how it should sound. Refused when the studio's window is not open, there being nothing to play it on. Answers the session id.",
            schema: || object(json!({ "voice": { "type": "string" }, "engine": { "type": "string" }, "lang": { "type": "string" }, "direction": { "type": "string" } }), &[]),
            call: |args| post("/speech/say".into(), args.clone()),
        },
        Tool {
            name: "say_text",
            description: "More text for a say_start session, as soon as you have it - a few words are enough; a phrase is said once it is finished (a full stop, a question or exclamation mark, a line break, or a long phrase at its comma). end true says what is left and closes the session. Answers at once.",
            schema: || object(json!({ "session": { "type": "string" }, "text": { "type": "string" }, "end": { "type": "boolean" } }), &["session", "text"]),
            // the text is passed as written: a piece's spaces join it to the next one
            call: |args| post(format!("/speech/say/{}", segment(&text(args, "session")?)), json!({ "text": args.get("text").and_then(Value::as_str).ok_or("'text' is required")?, "end": args.get("end").and_then(Value::as_bool).unwrap_or(false) })),
        },
        Tool {
            name: "say_stop",
            description: "Stop speaking in this session now: what waits is dropped, the phrase being voiced is cut off and the window falls silent - for when the person starts talking over the studio.",
            schema: || object(json!({ "session": { "type": "string" } }), &["session"]),
            call: |args| send(Method::DELETE, format!("/speech/say/{}", segment(&text(args, "session")?)), json!({})),
        },
        Tool {
            name: "say_silence",
            description: "Stop everything said aloud now, in every session, as Live's stop button does.",
            schema: nothing,
            call: |_| send(Method::DELETE, "/speech/say".into(), json!({})),
        },
        Tool {
            name: "voice_listen_start",
            description: "Join the spoken conversation the person started with Live's microphone button: each phrase they end with a pause is recognised for voice_listen, and their speaking over the studio stops its speech at once. Only the person turns the microphone on, with their own button in the window; no tool and no ui_* click does. When it is off this answers mic_off: ask the person to press Live, then call this again. engine changes the recogniser of the running conversation at once, the microphone recording on (the setting asr_engine is only the default for new recordings); the answer names the recogniser that hears now (asr), as mic_state does. Pair it with say_start: listen, answer with say_text as you write, listen again.",
            schema: || object(json!({ "lang": { "type": "string" }, "device": { "type": "string", "description": "a microphone of voice_record_devices; none: the default one" }, "engine": { "type": "string", "enum": ["parakeet", "nemotron", "whisper"], "description": "the recogniser; none: the one in the settings" } }), &[]),
            call: |args| post("/speech/listen".into(), args.clone()),
        },
        Tool {
            name: "voice_listen",
            description: "Wait for what the person says next: the phrases heard after the first `after` (the `heard` of the last answer, 0 at first), each ended by a pause, waiting up to seconds (at most 55; an answer with no lines and listening true means wait again). partial is what they are saying right now.",
            schema: || object(json!({ "after": { "type": "integer", "minimum": 0 }, "seconds": seconds() }), &[]),
            call: |args| get(format!("/speech/listen/next{}", query(&[("after", args.get("after").and_then(Value::as_u64).map(|v| v.to_string())), ("seconds", args.get("seconds").and_then(Value::as_u64).map(|v| v.to_string()))]))),
        },
        Tool {
            name: "say_voice",
            description: "The voice the person chose in Live for speaking aloud: engine, voice and sampling; say_start takes each one it is not given.",
            schema: nothing,
            call: |_| get("/speech/say/voice".into()),
        },
        Tool {
            name: "say_voice_set",
            description: "Choose the voice for speaking aloud, as Live's voice tab does: engine (engines_list), voice (voices_list; none: the engine's default) and sampling params (the engine's; none: its defaults).",
            schema: || object(json!({ "engine": { "type": "string" }, "voice": { "type": "string" }, "params": { "type": "object" } }), &["engine"]),
            call: |args| post("/speech/say/voice".into(), json!({ "engine": text(args, "engine")?, "voice": args.get("voice"), "params": args.get("params") })),
        },
        Tool {
            name: "mic_test_stop",
            description: "End the microphone test the person started: how long it heard, how loud at most, and the file of what it heard (mic_test_audio).",
            schema: nothing,
            call: |_| post("/record/test/stop".into(), json!({})),
        },
        Tool {
            name: "mic_test_audio",
            description: "The last microphone test's recording as a file on this computer.",
            schema: nothing,
            call: |_| get("/record/test/audio".into()),
        },
        Tool {
            name: "voice_listen_mute",
            description: "Mute the conversation's microphone, as the person's mute button in Live does. Only the person turns it back on: muted false answers mic_only_person. While muted it hears silence, and the conversation, what was heard and voice_listen go on.",
            schema: || object(json!({ "muted": { "type": "boolean" }, "toggle": { "type": "boolean" } }), &[]),
            call: |args| post("/speech/listen/mute".into(), match args.get("muted").and_then(Value::as_bool) { Some(m) => json!({ "muted": m }), None => json!({ "toggle": args.get("toggle").and_then(Value::as_bool).unwrap_or(false) }) }),
        },
        Tool {
            name: "mic_state",
            description: "What the microphone does now, as the window's indicator shows it: on or not, for what (live, listening, voice, test), for how many seconds, whether the conversation is muted, and in Live the recogniser that hears now (asr) and its language. The microphone turns off by itself when no window of the studio has been open for a few seconds.",
            schema: nothing,
            call: |_| get("/mic/state".into()),
        },
        Tool {
            name: "studio_timings",
            description: "How long each step took, latest last, to compare engines with the person: asr - each phrase heard live (the recogniser, the speech's length, recognise_ms, after_speech_ms from the end of the person's speech to its text); tts - each phrase said aloud (engine and voice, heard_ms from its text arriving to its first sound, first_ms the engine's own time to it, synth_ms, audio_secs); load - an engine loaded for speaking (ms). kind picks one of them; last how many (100).",
            schema: || object(json!({ "kind": { "type": "string", "enum": ["asr", "tts", "load"] }, "last": { "type": "integer", "minimum": 1 } }), &[]),
            call: |args| get(format!("/timings{}", query(&[("kind", given(args, "kind")), ("last", args.get("last").and_then(Value::as_u64).map(|v| v.to_string()))]))),
        },
        Tool {
            name: "voice_listen_type",
            description: "Put a phrase into the conversation as if the person said it, as typing in Live's text field does: voice_listen gives it next.",
            schema: || object(json!({ "text": { "type": "string" } }), &["text"]),
            call: |args| post("/speech/listen/text".into(), json!({ "text": text(args, "text")? })),
        },
        Tool {
            name: "voice_listen_stop",
            description: "Turn the conversation's microphone off, as Live's end button does: what was heard stays a Listening recording (its id and how many phrases). Only the person turns it on again. To end only your part of the conversation, close your say_start session with say_stop.",
            schema: nothing,
            call: |_| send(Method::DELETE, "/speech/listen".into(), json!({})),
        },
        Tool {
            name: "say_state",
            description: "A say_start session: each phrase with whether it was said, is waiting, was interrupted or failed, the wait the person heard from its text arriving to its first sound (heard_ms: loading the engine and the queue included), the engine's own time to the first sound (first_ms) and its length; the text not yet a phrase; whether the session is open. last gives only the last N phrases.",
            schema: || object(json!({ "session": { "type": "string" }, "last": { "type": "integer", "minimum": 1 } }), &["session"]),
            call: |args| get(format!("/speech/say/{}{}", segment(&text(args, "session")?), query(&[("last", args.get("last").and_then(Value::as_u64).map(|v| v.to_string()))]))),
        },
        Tool {
            name: "job_events",
            description: "A job's progress as the window follows it, for seconds or until it ends: every stage, message and percent in order (studio_wait answers only where it got).",
            schema: || object(json!({ "job_id": { "type": "string" }, "seconds": seconds() }), &["job_id"]),
            call: |args| stream(Method::GET, format!("/jobs/{}/events", segment(&text(args, "job_id")?)), None, args),
        },
        // ---------------------------------------------------------------- Speech: projects from files, packages
        Tool {
            name: "project_package",
            description: "Pack a project into one .speechproj file in its export folder (the project and the voices it uses; with its voiced sound unless audio is false) to move it to another computer. Answers the file's name and size; project_export_file or project_export_save takes it from there.",
            schema: || object(json!({ "pid": pid(), "audio": { "type": "boolean" } }), &["pid"]),
            call: |args| post(project_path(args, "/package")?, json!({ "audio": args.get("audio").and_then(Value::as_bool).unwrap_or(true) })),
        },
        Tool {
            name: "project_import_package",
            description: "Make a project of a .speechproj file (project_package): answers its id and the voices it added to the library.",
            schema: || id_only("path", "the .speechproj file"),
            call: |args| upload("/speechproj/import", local_file(args, "path")?, "file", Vec::new()),
        },
        Tool {
            name: "project_create_from_subtitles",
            description: "Make a Speech project of a subtitles file (SRT, VTT, ASS, SSA): its lines become blocks. name names it (the file's name otherwise).",
            schema: || object(json!({ "path": { "type": "string" }, "name": { "type": "string" } }), &["path"]),
            call: |args| upload("/projects/from-subtitles", local_file(args, "path")?, "file", given(args, "name").map(|name| vec![("name".to_string(), name)]).unwrap_or_default()),
        },
        Tool {
            name: "book_preview",
            description: "Read a book (EPUB, FB2, PDF, DOCX, HTML, Markdown, TXT) to choose its chapters: answers its id, title, author, language, whether it has a cover, and the chapters (index, title, characters, estimated seconds of speech, its opening, skip naming one to leave out: empty, toc, front or cover), removed (footnote marks and site addresses taken out of the text) and names (capitalised words inside sentences met three times or more, the most frequent first). book_chapter gives a whole one; project_create_from_book makes the audiobook project.",
            schema: || id_only("path", "the book file"),
            call: |args| upload("/books/preview", local_file(args, "path")?, "file", Vec::new()),
        },
        Tool {
            name: "book_chapter",
            description: "One chapter of a book read by book_preview: its title and text.",
            schema: || object(json!({ "id": { "type": "string", "description": "book id from book_preview" }, "index": { "type": "integer", "minimum": 0 } }), &["id", "index"]),
            call: |args| get(format!("/books/{}/chapters/{}", segment(&text(args, "id")?), args.get("index").and_then(Value::as_u64).ok_or("'index' is required")?)),
        },
        Tool {
            name: "project_create_from_book",
            description: "Make an audiobook project of a book read by book_preview: chapters lists the ones to take (index, and a title or text of your own to replace theirs), voice the narrator, name the project's name, names the names book_preview found that go into the project's dictionary as rules to check (to set their stress), dialogue true puts the direct speech (a dialogue dash, quotations) apart from the author's words under its own speaker, as text_direct_speech cuts it. With into (a project id) the book is edited and that project is rebuilt from it instead: each paragraph that reads as the project's blocks (whatever markup, stress marks or speakers were added) keeps them with their takes, other paragraphs become new blocks, blocks whose text is gone are dropped; dry_run true answers only the counts (kept, added, dropped, dropped_voiced), the real run keeps a copy of the old project (backup).",
            schema: || object(json!({ "id": { "type": "string" }, "name": { "type": "string" }, "voice": { "type": "string" }, "dialogue": { "type": "boolean" }, "into": { "type": "string" }, "dry_run": { "type": "boolean" }, "names": { "type": "array", "items": { "type": "string" } }, "chapters": { "type": "array", "items": { "type": "object", "properties": { "index": { "type": "integer" }, "title": { "type": "string" }, "text": { "type": "string" } }, "required": ["index"] } } }), &["id", "chapters"]),
            call: |args| post("/projects/from-book".into(), args.clone()),
        },
        // ---------------------------------------------------------------- Speech: takes and export files
        Tool {
            name: "project_take_file",
            description: "A block's take as a sound file on this computer: file is the take's name from project_get.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" } }), &["pid", "file"]),
            call: |args| get(project_path(args, &format!("/takes/{}", segment(&text(args, "file")?)))?),
        },
        Tool {
            name: "project_track_import",
            description: "Put sounds of this computer on a track of the book or play - music, sounds, ambience - beside the voices: paths are sound files and folders (a folder gives its sound files by name), copied into the project's media and laid one after another from at seconds (by default after the track's last clip), on the track whose id track names or a new one called name. The export mixes the tracks under the voices; each clip's start, cut and mix and each track's gain_db and muted are edited with project_update. Answers the project.",
            schema: || object(json!({ "pid": pid(), "paths": { "type": "array", "items": { "type": "string" } }, "track": { "type": "string" }, "name": { "type": "string" }, "at": { "type": "number", "minimum": 0 } }), &["pid", "paths"]),
            call: |args| post(project_path(args, "/tracks/import")?, body_without(args, &["pid"])),
        },
        Tool {
            name: "music_studios",
            description: "The music studios of this computer (YuE2 Studio, MiniMax Music3 Studio, ACE-Step Studio): whether each runs now, the fields of its song form (the input of its song_create) for project_track_music, and page, its site: what it does and how it is installed. A studio that does not run is installed or started by the person.",
            schema: || object(json!({}), &[]),
            call: |_| get("/music/studios".into()),
        },
        Tool {
            name: "project_track_music",
            description: "Make the music of a book or play in a music studio of this computer and put it on a track: studio is an id from music_studios, values its song form's fields (style, lyrics or none for an instrumental, duration...). A job, follow it with studio_wait: the song is kept in the studio's library and its audio comes into the project's media as a clip on the track whose id track names or a new one called name, at at seconds (by default after the track's last clip). Answers the job id.",
            schema: || object(json!({ "pid": pid(), "studio": { "type": "string", "enum": ["yue2-studio", "minimax-music3-studio", "ace-step-studio"] }, "values": { "type": "object" }, "track": { "type": "string" }, "name": { "type": "string" }, "at": { "type": "number", "minimum": 0 } }), &["pid", "studio", "values"]),
            call: |args| post(project_path(args, "/tracks/music")?, body_without(args, &["pid"])),
        },
        Tool {
            name: "project_track_upload",
            description: "Put one sound file on a track by uploading a copy of it, as the window does with a file picked there (project_track_import reads files and folders where they lie): path is the file, track the id of a track or none for a new one called name, at its start in seconds. Answers the project.",
            schema: || object(json!({ "pid": pid(), "path": { "type": "string" }, "track": { "type": "string" }, "name": { "type": "string" }, "at": { "type": "number", "minimum": 0 } }), &["pid", "path"]),
            call: |args| {
                let fields = ["track", "name", "at"].iter().filter_map(|k| args.get(*k).map(|v| (k.to_string(), v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())))).collect();
                upload(&project_path(args, "/tracks/upload")?, local_file(args, "path")?, "file", fields)
            },
        },
        Tool {
            name: "project_media_file",
            description: "A track clip's sound as a file on this computer: file is the clip's file from project_get (its tracks).",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" } }), &["pid", "file"]),
            call: |args| get(project_path(args, &format!("/media/{}", segment(&text(args, "file")?)))?),
        },
        Tool {
            name: "project_media_peaks",
            description: "A track clip's waveform as the track editor draws it: peaks of each 1/25 s (0..1, not scaled) and its length in seconds; file is the clip's file from project_get.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" } }), &["pid", "file"]),
            call: |args| get(project_path(args, &format!("/media/{}/peaks", segment(&text(args, "file")?)))?),
        },
        Tool {
            name: "project_take_peaks",
            description: "A take's waveform as the track editor draws it: peaks of each 1/25 s (the loudest sample, 0..1, not scaled) and its length in seconds; file is the take's name from project_get.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" } }), &["pid", "file"]),
            call: |args| get(project_path(args, &format!("/takes/{}/peaks", segment(&text(args, "file")?)))?),
        },
        Tool {
            name: "project_export_file",
            description: "An export of a project (file from project_exports) as a file on this computer.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" } }), &["pid", "file"]),
            call: |args| get(project_path(args, &format!("/exports/{}", segment(&text(args, "file")?)))?),
        },
        Tool {
            name: "project_export_save",
            description: "Copy an export of a project (file from project_exports) to path: a file, or a folder to put it in under its name.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" }, "path": { "type": "string" } }), &["pid", "file", "path"]),
            call: |args| post(project_path(args, "/save-export")?, json!({ "file": text(args, "file")?, "path": text(args, "path")? })),
        },
        Tool {
            name: "project_exports_to_folder",
            description: "Copy exports of a project (files from project_exports) into folder; without folder the user chooses it in a dialog on their screen.",
            schema: || object(json!({ "pid": pid(), "files": { "type": "array", "items": { "type": "string" } }, "folder": { "type": "string" } }), &["pid", "files"]),
            call: |args| post(project_path(args, "/exports-to-folder")?, body_without(args, &["pid"])),
        },
        Tool {
            name: "project_export_reveal",
            description: "Show an export of a project in Explorer, selected, for the user.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" } }), &["pid", "file"]),
            call: |args| post(project_path(args, &format!("/exports/{}/reveal", segment(&text(args, "file")?)))?, json!({})),
        },
        Tool {
            name: "project_export_save_as",
            description: "Ask the user where to save an export of a project, in a save dialog on their screen, and copy it there: answers the path, null when they cancelled.",
            schema: || object(json!({ "pid": pid(), "file": { "type": "string" } }), &["pid", "file"]),
            call: |args| post(project_path(args, &format!("/exports/{}/save-as", segment(&text(args, "file")?)))?, json!({})),
        },
        // ---------------------------------------------------------------- the Assistant panel
        Tool {
            name: "assistant_models",
            description: "The OpenRouter models the studio's Assistant panel can work with (those that call tools), the one chosen, and who answers there: a model, or the connected agent.",
            schema: nothing,
            call: |_| get("/chat/models".into()),
        },
        Tool {
            name: "assistant_chat",
            description: "Talk to the studio's Assistant panel as the user does: messages (role user or assistant, content) go to model (assistant_models) with project_id and language for context; answers what it said, the tools it ran and what it cost, read for seconds or until it finished. It pays from the user's OpenRouter key.",
            schema: || object(json!({ "model": { "type": "string" }, "messages": { "type": "array", "items": { "type": "object", "properties": { "role": { "type": "string" }, "content": { "type": "string" } }, "required": ["role", "content"] } }, "project_id": { "type": "string" }, "language": { "type": "string" }, "seconds": seconds() }), &["model", "messages"]),
            call: |args| stream(Method::POST, "/chat".into(), Some(body_without(args, &["seconds"])), args),
        },
        // ---------------------------------------------------------------- diagnostics and the journal
        Tool {
            name: "diagnostics_get",
            description: "The studio's diagnostics: version, system, graphics card against CUDA, ffmpeg, every component and whether it is on disk, what the engines' DLLs miss, and the latest log lines - as text to read and as data.",
            schema: nothing,
            call: |_| get("/diagnostics".into()),
        },
        Tool {
            name: "diagnostics_report",
            description: "Pack a bug report (the diagnostics, the activity journal and the log, keys masked) into a ZIP file: answers its path.",
            schema: nothing,
            call: |_| post("/diagnostics/report".into(), json!({})),
        },
        Tool {
            name: "diagnostics_report_reveal",
            description: "Show a bug report (path from diagnostics_report) in Explorer, selected, for the user.",
            schema: || id_only("path", "the report's path"),
            call: |args| post("/diagnostics/report/reveal".into(), json!({ "path": text(args, "path")? })),
        },
        Tool {
            name: "diagnostics_logs_reveal",
            description: "Open the folder of the studio's logs in Explorer, for the user.",
            schema: nothing,
            call: |_| post("/diagnostics/logs/reveal".into(), json!({})),
        },
        Tool {
            name: "journal_get",
            description: "The studio's activity journal, oldest first: what agents and the Assistant changed (verb, kind, target) and what the window told the user, each with its id and time.",
            schema: nothing,
            call: |_| get("/journal".into()),
        },
        Tool {
            name: "journal_note",
            description: "Write a line into the activity journal the user reads: text, and tone agent (default), work, done or error.",
            schema: || object(json!({ "text": { "type": "string" }, "tone": { "type": "string", "enum": ["agent", "work", "done", "error"] } }), &["text"]),
            call: |args| post("/journal".into(), json!({ "source": "studio", "tone": given(args, "tone").unwrap_or_else(|| "agent".into()), "text": text(args, "text")? })),
        },
        Tool {
            name: "journal_clear",
            description: "Empty the activity journal.",
            schema: nothing,
            call: |_| send(Method::DELETE, "/journal".into(), json!({})),
        },
        Tool {
            name: "journal_delete",
            description: "Remove one line of the activity journal by its id (journal_get).",
            schema: || object(json!({ "id": { "type": "integer", "minimum": 0 } }), &["id"]),
            call: |args| send(Method::DELETE, format!("/journal/{}", args.get("id").and_then(Value::as_u64).ok_or("'id' is required")?), json!({})),
        },
        // ---------------------------------------------------------------- this connection and the windows
        Tool {
            name: "agent_status",
            description: "This connection as the studio's settings show it: whether an agent is connected, its last call and how long ago, how many calls it made, and whether a window of the studio is open.",
            schema: nothing,
            call: |_| composite("agent"),
        },
        Tool {
            name: "windows_watch",
            description: "The studio's open windows (the last is the one the ui_* tools work in) and, for seconds, what the studio tells them: what changed behind them (a project, the journal, the voices) and the commands the ui_* tools send.",
            schema: || object(json!({ "seconds": seconds() }), &[]),
            call: |_| composite("windows"),
        },
        Tool {
            name: "window_focus",
            description: "Make an open window (its number from windows_watch) the one the ui_* tools work in, as when the user turns to it.",
            schema: || object(json!({ "window": { "type": "integer", "minimum": 0 } }), &["window"]),
            call: |_| composite("window_focus"),
        },
        Tool {
            name: "window_command_drop",
            description: "Give up a ui_* command a window has not answered (its id from windows_watch): the call waiting for it ends now with reason as its error.",
            schema: || object(json!({ "id": { "type": "string" }, "reason": { "type": "string" } }), &["id"]),
            call: |_| composite("window_drop"),
        },
        Tool {
            name: "studio_batch",
            description: "Run several tools in one call, in order, through this MCP server: calls is a list of {name, arguments}. It stops at the first that fails unless keep_going is true; answers each one's result or error. One call's time limit still applies, so put long work in jobs and studio_wait for them.",
            schema: || object(json!({ "calls": { "type": "array", "items": { "type": "object", "properties": { "name": { "type": "string" }, "arguments": { "type": "object" } }, "required": ["name"] } }, "keep_going": { "type": "boolean" } }), &["calls"]),
            call: |_| composite("batch"),
        },
    ]
}

/// What the live stream said, the sound of each block written to a WAV file in place of its pieces.
pub(super) async fn live_files(read: Value) -> Result<Value, String> {
    use base64::Engine;
    let mut events = Vec::new();
    let mut sound: Vec<((String, String), u32, Vec<f32>)> = Vec::new();
    for event in read["events"].as_array().cloned().unwrap_or_default() {
        if event["kind"] != "chunk" {
            events.push(event);
            continue;
        }
        let key = (event["project"].as_str().unwrap_or_default().to_string(), event["block"].as_str().unwrap_or_default().to_string());
        let rate = event["sr"].as_u64().and_then(|rate| u32::try_from(rate).ok()).filter(|rate| *rate > 0).ok_or("a live piece came without its sample rate")?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(event["data"].as_str().unwrap_or_default()).map_err(|e| format!("a live piece: {e}"))?;
        let samples = bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        match sound.iter_mut().find(|(known, known_rate, _)| *known == key && *known_rate == rate) {
            Some((_, _, all)) => all.extend(samples),
            None => sound.push((key, rate, samples.collect())),
        }
    }
    let folder = super::agent_files().await?;
    let mut files = Vec::new();
    for ((project, block), rate, samples) in sound {
        let file = folder.join(format!("speech_live_watch-{}-{}-{}.wav", segment(&project), segment(&block), super::chrono_stamp()));
        let spec = hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
        let mut writer = hound::WavWriter::create(&file, spec).map_err(|e| format!("{}: {e}", file.display()))?;
        for sample in &samples {
            writer.write_sample(*sample).map_err(|e| format!("{}: {e}", file.display()))?;
        }
        writer.finalize().map_err(|e| format!("{}: {e}", file.display()))?;
        files.push(json!({ "project": project, "block": block, "file": file.to_string_lossy(), "seconds": samples.len() as f64 / f64::from(rate) }));
    }
    Ok(json!({ "events": events, "files": files, "ended": read["ended"] }))
}
