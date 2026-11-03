use super::*;

pub(super) fn tools() -> &'static [Tool] {
    static TOOLS: OnceLock<Vec<Tool>> = OnceLock::new();
    TOOLS.get_or_init(|| {
        let mut all = vec![
            // ---------------------------------------------------------------- the studio
            Tool {
                name: "studio_status",
                description: "What the studio is doing now, in one short summary: the jobs running or waiting (voicing, export, recognition, cut by text, the director model, voice cleaning, downloads) with their project, stage and percent, the last finished ones with their result or error, and whether the required models are there. Call it first, and use studio_wait to wait.",
                schema: nothing,
                call: |_| composite("status"),
            },
            Tool {
                name: "studio_wait",
                description: "Wait for work to finish instead of polling: a job (job_id, as project_synth, project_export, project_cast_characters, project_enrich, podcast_write, listen_transcribe, listen_cut, voice_transcribe, voice_clean, voices_download_pack or a one-call tool answering done false returned it, or the fetch.id of a download by link), or until one kind of work is over - synth, export, analyze (recognition), remix (cut by text), write (podcast script), cast (characters), enrich (the director's tags), voice_transcript, voice_clean, separate, download, voices_pack - or everything (until: idle, the default). Returns when it is done or after seconds (30 by default, at most 55, under the minute clients allow a call) with how far it got; call it again to keep waiting.",
                schema: || object(json!({ "job_id": { "type": "string" }, "until": { "type": "string", "enum": ["idle", "synth", "export", "analyze", "remix", "write", "cast", "enrich", "music", "voice_transcript", "voice_clean", "voice_design", "separate", "download", "voices_pack"] }, "seconds": { "type": "integer" } }), &[]),
                call: |_| composite("wait"),
            },
            Tool {
                name: "studio_system",
                description: "The graphics card, its used and free video memory, load, temperature and power, and the computer's and the studio's RAM: check them before a long job.",
                schema: nothing,
                call: |_| get("/hw/snapshot".into()),
            },
            Tool {
                name: "studio_capabilities",
                description: "What the studio can do here: the languages it dubs, the voice modes, whether ffmpeg is found, the speech recognisers with their models and precisions, the memory limits to choose from, and the current settings (the OpenRouter key only as or_key_set).",
                schema: nothing,
                call: |_| get("/engine/capabilities".into()),
            },
            // ---------------------------------------------------------------- jobs
            Tool {
                name: "jobs_list",
                description: "The studio's jobs, running, waiting and lately finished (preview frames are not jobs here): id, kind, project, status, stage, message, percent, place in the queue, and the result or the error. pid narrows them to one project and adds its last job as stored with it (project_job), an interrupted one included: project_resume continues it.",
                schema: || object(json!({ "pid": pid() }), &[]),
                call: |args| get(format!("/jobs{}", query(&[("pid", given(args, "pid"))]))),
            },
            Tool {
                name: "job_get",
                description: "One job: its status (queued, running, done, error, cancelled), kind, project, stage, message, percent, and its result (the output's path, a new project_id) or its error.",
                schema: || id_only("job_id", "job id a tool returned"),
                call: |args| get(format!("/jobs/{}", segment(&text(args, "job_id")?))),
            },
            Tool {
                name: "job_cancel",
                description: "Cancel a job: a waiting one never starts, a running one stops at its next step and ends as cancelled. project_resume starts it again later.",
                schema: || id_only("job_id", "job id a tool returned"),
                call: |args| post(format!("/jobs/{}/cancel", segment(&text(args, "job_id")?)), json!({})),
            },
            // ---------------------------------------------------------------- models and settings
            Tool {
                name: "models_status",
                description: "Every model and engine the studio uses: whether it is installed, its size, what is on disk, the video memory it takes, whether it is required, and whether the required set is ready (ready, driver_ok). Download what is missing with models_download.",
                schema: nothing,
                call: |_| get("/setup/status".into()),
            },
            Tool {
                name: "models_download",
                description: "Download components by id (from models_status): models, their quantisations, engines and runtimes. It runs in the background beside the jobs, not as a job: studio_wait until download waits for it, models_status shows it. Resumes what is partly there; every file is checked against its pinned SHA-256.",
                schema: || object(json!({ "ids": ids("component ids from models_status") }), &["ids"]),
                call: |args| post("/setup/download".into(), json!({ "ids": args.get("ids").cloned().unwrap_or_default() })),
            },
            Tool {
                name: "models_cancel_download",
                description: "Pause the running models download: what came is kept, and models_download with the same ids continues from there.",
                schema: nothing,
                call: |_| post("/setup/cancel".into(), json!({})),
            },
            Tool {
                name: "models_remove",
                description: "Delete downloaded components by id (from models_status) with what is partly downloaded, to free disk space; a stage whose chosen variant is removed falls back to an installed one. Answers what was removed, the bytes freed, the errors and the models' status.",
                schema: || object(json!({ "ids": ids("component ids from models_status") }), &["ids"]),
                call: |args| post("/setup/remove".into(), json!({ "ids": args.get("ids").cloned().unwrap_or_default() })),
            },
            Tool {
                name: "models_folder_open",
                description: "Open the models folder in Explorer, for the user (to put model files there by hand or see what takes the disk); answers its path.",
                schema: nothing,
                call: |_| post("/setup/open-models".into(), json!({})),
            },
            Tool {
                name: "models_import",
                description: "Take model files already on this computer instead of downloading them: every component found in the folder is copied into the studio's models; id limits it to one component. Answers what was imported and the models' status.",
                schema: || object(json!({ "path": { "type": "string", "description": "folder with the model files" }, "id": { "type": "string", "description": "one component id from models_status" } }), &["path"]),
                call: |args| {
                    let mut body = json!({ "path": text(args, "path")? });
                    if let Some(id) = args.get("id").and_then(Value::as_str) {
                        body["id"] = id.into();
                    }
                    post("/setup/import".into(), body)
                },
            },
            Tool {
                name: "models_select",
                description: "Use a downloaded variant for its stage (id from models_status): a quantisation of Higgs (higgs, higgs-q6_k, higgs-q4_k_m), of Gemma (gemma, gemma-q5_0, gemma-q6_k, gemma-q8_0) or of the separator (roformer, roformer-q5, roformer-q4), or a speech recogniser (parakeet, parakeet-ultra, asr-gguf-nemotron, crisperwhisper-turbo). Applies from the next job, no restart.",
                schema: || id_only("id", "component id from models_status"),
                call: |args| post("/engine/select".into(), json!({ "id": text(args, "id")? })),
            },
            Tool {
                name: "settings_get",
                description: "The studio's settings and the values each takes: the model variant of each stage (tts, mt, sep, asr, asr_engine: parakeet, nemotron or whisper = CrisperWhisper 2.0 turbo), where each local stage runs (diar_backend, the diarization device (the audio.cpp engine): auto, gpu, vulkan, cpu; sep_backend, the separation device: auto, gpu, vulkan, cpu; tts_backend, the device of voicing with Higgs (the audio.cpp engine) and Qwen3-TTS, whose build changes after a restart, local_backend, the common one, and asr_backend: auto, gpu, vulkan, cpu), the voicing switches (qc_asr, qc_duration, multitake, breath_on, speech_rate_on, emo_ref_on, duck_on, auto_shorten: rewrite the translation of lines that do not fit their slot and voice them again during a render, lead_into_silence: a line that does not fit starts up to a second earlier, into the silence before it, instead of being sped up; off by default), the memory limits (llama_ubatch, higgs_ref_secs), who translates and who reads frames (llm_provider, vision_provider: local = the studio's Gemma, server = a local OpenAI-compatible server at srv_url with the models srv_llm and srv_vision, openrouter = the cloud with or_llm and or_vision), the other cloud stages through OpenRouter (or_tts_on, or_tts_model, or_tts_voice, or_tts_autocast, or_asr_on, or_asr, or_concurrency), whether the proxy is on (proxy_on; proxy_settings_set sets its address) and the stage benchmark (bench). The OpenRouter key is never shown: or_key_set says whether there is one.",
                schema: nothing,
                call: |_| get("/engine/capabilities".into()),
            },
            Tool {
                name: "studio_paths",
                description: "Where the studio keeps its data on this computer: data_dir, projects_dir (a folder per project), models_dir.",
                schema: nothing,
                call: |_| get("/app/paths".into()),
            },
            Tool {
                name: "settings_set",
                description: "Change one setting to a text value. TTS: tts_provider=local|openrouter|google, google_tts_model=<id without models/>, google_tts_mode=standard|batch|auto (auto: direct up to 10 phrases, batch from 11). Voice/autocast use or_tts_voice/or_tts_autocast, parallel requests use or_concurrency (1..16). Applies from the next job. Secrets: openrouter_set_key, google_set_key; proxy: proxy_settings_set.",
                schema: || object(json!({ "key": { "type": "string" }, "value": { "type": "string" } }), &["key", "value"]),
                call: |args| {
                    let key = text(args, "key")?;
                    if key == "google_key" {
                        return Err("The Google key is set with google_set_key.".into());
                    }
                    if key == "or_key" {
                        return Err("The OpenRouter key is set with openrouter_set_key.".into());
                    }
                    if key == "proxy_url" {
                        return Err("The proxy's address is set with proxy_settings_set.".into());
                    }
                    let value = match args.get("value") {
                        Some(Value::String(value)) => value.clone(),
                        Some(Value::Bool(on)) => (if *on { "1" } else { "0" }).to_string(),
                        Some(Value::Number(number)) => number.to_string(),
                        _ => return Err("'value' is required".into()),
                    };
                    post("/engine/select".into(), json!({ "key": key, "value": value }))
                },
            },
            Tool {
                name: "engine_presets_get",
                description: "The hardware presets (by graphics card, a weak card with the cloud, the cloud alone), this computer's card, video memory and RAM, the preset recommended for it, and the preset the settings are on now (custom when none gives them).",
                schema: nothing,
                call: |_| get("/engine/presets".into()),
            },
            Tool {
                name: "engine_preset_apply",
                description: "Apply a hardware preset (id from engine_presets_get): it sets the quantisations and where each stage runs, and the cloud stages for the cloud presets. Answers the settings it changed.",
                schema: || id_only("id", "preset id from engine_presets_get"),
                call: |args| post("/engine/preset".into(), json!({ "id": text(args, "id")? })),
            },
            Tool {
                name: "proxy_test",
                description: "Check, before saving, whether Hugging Face (model downloads) and OpenRouter (the cloud stages) are reachable: mode custom (default) through the proxy at url - any seller's notation, host:port:user:password included, kind giving the scheme of an address without one - mode system through the Windows proxy, mode off directly. password goes with a url that names the user without one; the stored password is used only for the stored address.",
                schema: || {
                    object(
                        json!({
                            "mode": { "type": "string", "enum": ["custom", "system", "off"] },
                            "kind": { "type": "string", "enum": ["http", "https", "socks5", "socks4"] },
                            "url": { "type": "string" },
                            "password": { "type": "string" },
                        }),
                        &[],
                    )
                },
                call: |args| {
                    let mut body = json!({ "url": args.get("url").and_then(Value::as_str).unwrap_or_default() });
                    for field in ["mode", "kind", "password"] {
                        if let Some(value) = args.get(field).and_then(Value::as_str) {
                            body[field] = value.into();
                        }
                    }
                    post("/engine/proxy/test".into(), body)
                },
            },
            Tool {
                name: "proxy_settings_get",
                description: "The proxy all the studio's traffic goes through (model downloads, OpenRouter): mode (system - as in Windows, custom - its own address, off - direct), kind (the scheme an address without one gets), its address without the password, whether a password is stored, and problem - why a stored address of its own cannot be read.",
                schema: nothing,
                call: |_| get("/engine/proxy/settings".into()),
            },
            Tool {
                name: "proxy_settings_set",
                description: "Set the proxy: mode system (as in Windows), custom (the address in url) or off (direct); kind (http, https, socks5, socks4) for an address written without a scheme; url in any seller's notation (user@host:port, host:port:user:password; a password written into it is stored apart and never shown; empty removes the address); password to change only the password (null removes it). Fields left out stay; mode custom needs an address. proxy_test checks first.",
                schema: || {
                    object(
                        json!({
                            "mode": { "type": "string", "enum": ["system", "custom", "off"] },
                            "kind": { "type": "string", "enum": ["http", "https", "socks5", "socks4"] },
                            "url": { "type": "string" },
                            "password": { "type": ["string", "null"] },
                        }),
                        &[],
                    )
                },
                call: |args| send(Method::PUT, "/engine/proxy/settings".into(), body_without(args, &[])),
            },
            // ---------------------------------------------------------------- OpenRouter
            Tool {
                name: "google_status",
                description: "Whether a direct Google Gemini API key is configured and its source. Never returns the key.",
                schema: nothing,
                call: |_| get("/engine/google/settings".into()),
            },
            Tool {
                name: "google_set_key",
                description: "Verify a Gemini API key against Google's model catalog and store it separately from settings. No paid generation is made.",
                schema: || id_only("key", "Google Gemini API key"),
                call: |args| send(Method::PUT,"/engine/google/settings".into(),json!({"api_key":text(args,"key")?})),
            },
            Tool {
                name: "google_delete_key",
                description: "Remove the stored Google Gemini API key. An environment key cannot be removed by the app.",
                schema: nothing,
                call: |_| send(Method::DELETE,"/engine/google/settings".into(),json!({})),
            },
            Tool {
                name: "google_models",
                description: "List TTS models available through the direct Google API, including supported generation methods. Needs the Google key.",
                schema: nothing,
                call: |_| get("/engine/google/models".into()),
            },
            Tool {
                name: "openrouter_status",
                description: "Whether an OpenRouter key is set for the cloud stages and where it comes from (the studio's store or an environment variable). The key itself is never shown.",
                schema: nothing,
                call: |_| get("/engine/openrouter/settings".into()),
            },
            Tool {
                name: "openrouter_set_key",
                description: "Set the OpenRouter key the cloud stages use: OpenRouter checks it first and it is stored only if it works. It is never shown back.",
                schema: || id_only("key", "the OpenRouter API key, sk-or-..."),
                call: |args| send(Method::PUT, "/engine/openrouter/settings".into(), json!({ "api_key": text(args, "key")? })),
            },
            Tool {
                name: "openrouter_delete_key",
                description: "Remove the stored OpenRouter key; the cloud stages stop until a key is set again.",
                schema: nothing,
                call: |_| send(Method::DELETE, "/engine/openrouter/settings".into(), json!({})),
            },
            Tool {
                name: "openrouter_verify",
                description: "Check an OpenRouter key without storing it: whether it works, its label, limit and usage.",
                schema: || id_only("key", "the OpenRouter API key, sk-or-..."),
                call: |args| post("/engine/openrouter/verify".into(), json!({ "key": text(args, "key")? })),
            },
            Tool {
                name: "openrouter_models",
                description: "The OpenRouter models for a cloud stage: llm (translation), vision (reading frames), tts (voices), asr (speech recognition). Needs the key.",
                schema: || object(json!({ "kind": { "type": "string", "enum": ["llm", "vision", "tts", "asr"] } }), &["kind"]),
                call: |args| get(format!("/engine/openrouter/models{}", query(&[("kind", Some(text(args, "kind")?))]))),
            },
            Tool {
                name: "openrouter_catalog",
                description: "The state of the studio's OpenRouter model catalog (kept on disk): when it was fetched and how many models each cloud stage can choose from (llm, vision, tts, asr); openrouter_models lists them with prices and context.",
                schema: nothing,
                call: |_| get("/engine/openrouter/catalog".into()),
            },
            Tool {
                name: "openrouter_catalog_refresh",
                description: "Fetch the OpenRouter model catalog again now instead of the copy on disk.",
                schema: nothing,
                call: |_| post("/engine/openrouter/catalog/refresh".into(), json!({})),
            },
            Tool {
                name: "local_server_models",
                description: "The models of the local OpenAI-compatible server (Ollama, LM Studio, vLLM, llama-server) for translation and vision: url is its address (default: the one in settings); a stored key goes only to the address it was saved for.",
                schema: || object(json!({ "url": { "type": "string" } }), &[]),
                call: |args| get(format!("/engine/server/models{}", query(&[("url", given(args, "url"))]))),
            },
            Tool {
                name: "local_server_key_status",
                description: "Whether an API key is stored for the local server's address (url, default: the one in settings); the key itself is never shown.",
                schema: || object(json!({ "url": { "type": "string" } }), &[]),
                call: |args| get(format!("/engine/server/key{}", query(&[("url", given(args, "url"))]))),
            },
            Tool {
                name: "local_server_key_set",
                description: "Store an API key for the local server (vLLM and LM Studio can ask for one); it is sent only to that address (url, default: the one in settings).",
                schema: || object(json!({ "api_key": { "type": "string" }, "url": { "type": "string" } }), &["api_key"]),
                call: |args| {
                    let mut body = json!({ "api_key": text(args, "api_key")? });
                    if let Some(url) = given(args, "url") {
                        body["url"] = url.into();
                    }
                    send(Method::PUT, "/engine/server/key".into(), body)
                },
            },
            Tool {
                name: "local_server_key_delete",
                description: "Remove the API key stored for the local server's address (url, default: the one in settings).",
                schema: || object(json!({ "url": { "type": "string" } }), &[]),
                call: |args| send(Method::DELETE, format!("/engine/server/key{}", query(&[("url", given(args, "url"))])), json!({})),
            },
            Tool {
                name: "openrouter_voices",
                description: "The voices of an OpenRouter speech model with their gender and age, and whether it speaks Russian: for or_tts_voice and the characters' voices.",
                schema: || id_only("model", "OpenRouter TTS model id"),
                call: |args| get(format!("/engine/openrouter/voices{}", query(&[("model", Some(text(args, "model")?))]))),
            },
            // ---------------------------------------------------------------- projects
            // ---------------------------------------------------------------- videos by link
            Tool {
                name: "url_probe",
                description: "Look at a video link before downloading it (YouTube and the other sites yt-dlp knows): title, length, uploader, the highest height, the qualities worth offering, the site's subtitles made by people (subtitles, each with lang and name) apart from the languages of the automatic ones (auto_subtitles), the expected size and the preview's link (thumbnail). cookies is the path of a cookies.txt for videos that need a signed-in browser (age, members, bot checks). A refusal names its code and what to do (hint). Needs the component ytdlp (models_download).",
                schema: || object(json!({ "url": { "type": "string", "description": "link of one video" }, "cookies": { "type": "string", "description": "path of a Netscape cookies.txt" } }), &["url"]),
                call: |args| get(format!("/url/probe{}", query(&[("url", Some(text(args, "url")?)), ("cookies", given(args, "cookies"))]))),
            },
            Tool {
                name: "url_fetches_list",
                description: "The downloads by link, newest first: id, link, quality, status (downloading, completed, failed, cancelled, interrupted), phase, bytes, speed, the project it made (pid), a warning about the subtitles, and the error with its code and hint.",
                schema: nothing,
                call: |_| get("/url/fetches".into()),
            },
            Tool {
                name: "url_fetch_get",
                description: "One download by link, as url_fetches_list shows it.",
                schema: || id_only("id", "download id from project_create_from_url or url_fetches_list"),
                call: |args| get(format!("/url/fetches/{}", segment(&text(args, "id")?))),
            },
            Tool {
                name: "url_fetch_cancel",
                description: "Stop a download by link: yt-dlp and what it started are stopped and the part downloaded is deleted.",
                schema: || id_only("id", "download id"),
                call: |args| post(format!("/url/fetches/{}/cancel", segment(&text(args, "id")?)), json!({})),
            },
            Tool {
                name: "url_fetch_resume",
                description: "Continue an interrupted or failed download by link from where it stopped, with the same settings (after fixing what its hint said: a proxy, cookies, an update of yt-dlp).",
                schema: || id_only("id", "download id"),
                call: |args| post(format!("/url/fetches/{}/resume", segment(&text(args, "id")?)), json!({})),
            },
            Tool {
                name: "url_fetch_delete",
                description: "Remove a finished, failed or interrupted download from the list, with what it downloaded partly; the project it made stays.",
                schema: || id_only("id", "download id"),
                call: |args| send(Method::DELETE, format!("/url/fetches/{}", segment(&text(args, "id")?)), json!({})),
            },
            Tool {
                name: "url_tool_status",
                description: "The yt-dlp that downloads by link: whether the component is there, the version in use, the pinned one, the latest release and when it was checked, whether an update runs, and why the last one failed.",
                schema: nothing,
                call: |_| get("/url/tool".into()),
            },
            Tool {
                name: "url_tool_update",
                description: "Check for a newer yt-dlp now (it is checked once a day anyway) and install it beside the pinned one: it is used only after its SHA-256 matches its release and it runs; otherwise the one in use stays. Runs in the background: url_tool_status shows when it is over.",
                schema: nothing,
                call: |_| post("/url/tool/update".into(), json!({})),
            },
            // ---------------------------------------------------------------- the lines
            // ---------------------------------------------------------------- what the project makes
            // ---------------------------------------------------------------- subtitles, titles, blur
            // ---------------------------------------------------------------- casting
            // ---------------------------------------------------------------- glossary
            // ---------------------------------------------------------------- voices
            Tool {
                name: "voices_list",
                description: "The voices of the studio's library: each with its card (gender, age, lang, tags, source, consent, favorite, kind, cleaned, transcript_auto), the transcript of its reference and whether it has a photo and a kept original. broken lists a voice whose card file is damaged, with the reason. brief true answers only {\"voices\":[{\"name\",\"card\":{\"kind\",\"lang\",\"gender\"}}],\"broken\"}: the transcripts make the full answer large.",
                schema: || object(json!({ "brief": { "type": "boolean" } }), &[]),
                call: |args| get(if args.get("brief").and_then(Value::as_bool).unwrap_or(false) { "/voices?brief=true" } else { "/voices" }.into()),
            },
            Tool {
                name: "voices_catalog",
                description: "The extra voices that can be downloaded into the library (the dataset nerualdreming/russia_voices): each with its name, display name, gender, age (adult or child), language, description, duration in seconds, birth year and whether a sample transcript comes with it. Optional filters, the same the window offers, narrow the list: gender, age, language (ISO 639-1), q (search in the name and description), min_duration and max_duration in seconds. The answer also gives the total before filtering.",
                schema: || object(json!({
                    "gender": { "type": "string", "enum": ["female", "male"] },
                    "age": { "type": "string", "enum": ["adult", "child"] },
                    "language": { "type": "string", "description": "ISO 639-1, e.g. ru, en, zh" },
                    "q": { "type": "string", "description": "search in the name and description" },
                    "min_duration": { "type": "number", "description": "seconds" },
                    "max_duration": { "type": "number", "description": "seconds" }
                }), &[]),
                call: |args| {
                    let mut qs: Vec<String> = Vec::new();
                    for key in ["gender", "age", "language", "q"] {
                        if let Some(v) = args.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
                            qs.push(format!("{key}={}", segment(v)));
                        }
                    }
                    for key in ["min_duration", "max_duration"] {
                        if let Some(n) = args.get(key).and_then(Value::as_f64) {
                            qs.push(format!("{key}={n}"));
                        }
                    }
                    get(if qs.is_empty() { "/voices/catalog".to_string() } else { format!("/voices/catalog?{}", qs.join("&")) })
                },
            },
            Tool {
                name: "voice_download",
                description: "Download one voice of voices_catalog into the library.",
                schema: || id_only("name", "voice name from voices_catalog"),
                call: |args| post("/voices/get".into(), json!({ "name": text(args, "name")? })),
            },
            Tool {
                name: "voices_download_pack",
                description: "Download the voice pack into the library. A job: studio_wait with its job_id.",
                schema: nothing,
                call: |_| post("/voices/download-pack".into(), json!({})),
            },
            Tool {
                name: "voice_record_devices",
                description: "The microphones the studio can record a voice from.",
                schema: nothing,
                call: |_| get("/record/devices".into()),
            },
            Tool {
                name: "voice_record_level",
                description: "How loud the microphone hears the user now, 0 to 1, while a recording the person started in the window records (a voice, a Listening recording, a microphone test, Live).",
                schema: nothing,
                call: |_| get("/record/level".into()),
            },
            Tool {
                name: "voice_record_stop",
                description: "Stop the voice recording the person started in the voice library and keep it as a voice of the user's own: answers its name, the job that cleans it (clean; by default as the setting clean new voices, on unless turned off) and recognises its reference text (transcribe, default true) - follow it with studio_wait - and the library.",
                schema: || object(json!({ "clean": { "type": "boolean" }, "transcribe": { "type": "boolean" } }), &[]),
                call: |args| {
                    let mut body = json!({ "transcribe": args.get("transcribe").and_then(Value::as_bool).unwrap_or(true) });
                    if let Some(clean) = args.get("clean").and_then(Value::as_bool) {
                        body["clean"] = json!(clean);
                    }
                    post("/record/stop".into(), body)
                },
            },
            Tool {
                name: "voice_rename",
                description: "Rename a voice of the library.",
                schema: || object(json!({ "from": { "type": "string" }, "to": { "type": "string" } }), &["from", "to"]),
                call: |args| post("/voices/rename".into(), json!({ "from": text(args, "from")?, "to": text(args, "to")? })),
            },
            Tool {
                name: "voice_delete",
                description: "Delete a voice from the library.",
                schema: || id_only("name", "voice name from voices_list"),
                call: |args| post("/voices/delete".into(), json!({ "name": text(args, "name")? })),
            },
            Tool {
                name: "voice_card_set",
                description: "Change a voice's card: gender (female, male or empty), age (child, teen, adult, senior or empty), lang (a code such as ru), tags (the whole list), source (where the voice comes from), consent (own, permitted, public or empty), favorite, kind (clone, preset, design), variant_of with variant (this voice is a variant of another voice, the same character angry or whispering: the other voice's name and the variant's name; empty variant_of makes it a voice of its own), license (an SPDX id such as CC-BY-4.0, or the terms in words; it travels with the voice in voices_export), rotate (a reference longer than the engines take gives each new take its next good piece instead of always the best one). Only the fields given change; an unknown field or value is refused.",
                schema: || object(json!({ "name": { "type": "string" }, "card": { "type": "object" } }), &["name", "card"]),
                call: |args| post("/voices/card".into(), json!({ "name": text(args, "name")?, "card": args.get("card").cloned().unwrap_or(Value::Null) })),
            },
            Tool {
                name: "voice_transcript_set",
                description: "Write the text spoken in a voice's reference by hand (the engine clones better with the right text); an empty text removes it.",
                schema: || object(json!({ "name": { "type": "string" }, "text": { "type": "string" } }), &["name", "text"]),
                call: |args| post("/voices/transcript".into(), json!({ "name": text(args, "name")?, "text": args.get("text").and_then(Value::as_str).unwrap_or("") })),
            },
            Tool {
                name: "voice_transcribe",
                description: "Recognise the text spoken in a voice's reference with the recognition engine chosen in the settings: a job, follow it with studio_wait.",
                schema: || id_only("name", "voice name from voices_list"),
                call: |args| post("/voices/transcribe".into(), json!({ "name": text(args, "name")? })),
            },
            Tool {
                name: "voice_clean",
                description: "Clean a voice of music and noise (Mel-Band Roformer) before cloning: a job, follow it with studio_wait. The original is kept and voice_restore brings it back. Refused with clean_missing when the separation engine is not downloaded (models_download roformer and bsroformer-engine).",
                schema: || id_only("name", "voice name from voices_list"),
                call: |args| post("/voices/clean".into(), json!({ "name": text(args, "name")? })),
            },
            Tool {
                name: "voice_denoise",
                description: "Remove noise from a voice's reference (DPDFNet) before cloning: a job, follow it with studio_wait. db limits how far the noise is pushed down (18 keeps speech natural; 0 or none removes as much as it can). The original is kept and voice_restore brings it back.",
                schema: || object(json!({ "name": { "type": "string", "description": "voice name from voices_list" }, "db": { "type": "number", "minimum": 0 } }), &["name"]),
                call: |args| post("/voices/denoise".into(), json!({ "name": text(args, "name")?, "db": args.get("db").cloned().unwrap_or(Value::Null) })),
            },
            Tool {
                name: "voice_design",
                description: "Make a voice from a description (Qwen3-TTS VoiceDesign): gender, age, timbre and manner in a sentence or two, best in English or Chinese, and a sample text in the language the voice will speak. A job, follow it with studio_wait: its result has the batch id and candidates (k, seconds); each sounds at /voices/design/sample?id=&k=. Keep one with voice_design_keep. Downloads the model (about 1.2 GB) the first time.",
                schema: || object(json!({ "description": { "type": "string" }, "sample": { "type": "string" }, "lang": { "type": "string" }, "count": { "type": "integer", "minimum": 1, "maximum": 6 } }), &["description", "sample"]),
                call: |args| post("/voices/design".into(), args.clone()),
            },
            Tool {
                name: "voice_design_keep",
                description: "Keep a candidate of voice_design as a library voice: the sample text becomes its reference transcript and the description its source.",
                schema: || object(json!({ "id": { "type": "string" }, "k": { "type": "integer" }, "name": { "type": "string" }, "description": { "type": "string" }, "sample": { "type": "string" }, "lang": { "type": "string" } }), &["id", "k", "name", "description", "sample"]),
                call: |args| post("/voices/design/keep".into(), args.clone()),
            },
            Tool {
                name: "voice_restore",
                description: "Bring back a voice's sound as it was before voice_clean or voice_denoise.",
                schema: || id_only("name", "voice name from voices_list"),
                call: |args| post("/voices/restore".into(), json!({ "name": text(args, "name")? })),
            },
            Tool {
                name: "voice_ref_candidates",
                description: "The best pieces of a voice's reference for cloning, best first: windows cut at pauses between the recognised words, no longer than the engine's reference limit, each with its duration, longest pause inside, signal-to-noise (snr_db), clipping share and issues (long_pause, noisy, clipped, short). Needs the reference text recognised first (voice_transcribe).",
                schema: || id_only("name", "voice name from voices_list"),
                call: |args| post("/voices/ref-candidates".into(), json!({ "name": text(args, "name")? })),
            },
            Tool {
                name: "voice_ref_apply",
                description: "Keep only the piece start..end (seconds) of a voice's reference: the sound is cut and its text and word timings follow; the original is kept and voice_restore brings it back.",
                schema: || object(json!({ "name": { "type": "string" }, "start": { "type": "number" }, "end": { "type": "number" } }), &["name", "start", "end"]),
                call: |args| post("/voices/ref-apply".into(), json!({ "name": text(args, "name")?, "start": args.get("start").and_then(Value::as_f64), "end": args.get("end").and_then(Value::as_f64) })),
            },
            Tool {
                name: "voice_import",
                description: "Make voices of sound files on this computer (WAV, MP3, FLAC, M4A, OGG), each brought to the reference loudness. name names a single file's voice (a taken name gets a number). clean (by default the setting clean new voices, on unless turned off) and transcribe (default true) start a job on every new voice; follow them with studio_wait.",
                schema: || object(json!({ "paths": { "type": "array", "items": { "type": "string" } }, "name": { "type": "string" }, "clean": { "type": "boolean" }, "transcribe": { "type": "boolean" } }), &["paths"]),
                call: |args| {
                    let paths = args.get("paths").and_then(Value::as_array).ok_or("paths must be a list of files")?;
                    let mut files = Vec::new();
                    for p in paths {
                        let path = PathBuf::from(p.as_str().ok_or("every path must be a string")?);
                        if !path.is_file() {
                            return Err(format!("{} is not a file on this computer.", path.display()));
                        }
                        let name = file_name(&path);
                        files.push(("file".to_string(), path, name));
                    }
                    let flag = |key: &str, default: bool| if args.get(key).and_then(Value::as_bool).unwrap_or(default) { "1" } else { "0" }.to_string();
                    let mut fields = vec![("transcribe".to_string(), flag("transcribe", true))];
                    if args.get("clean").and_then(Value::as_bool).is_some() {
                        fields.push(("clean".to_string(), flag("clean", false)));
                    }
                    if let Some(n) = args.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty()) {
                        fields.push(("name".to_string(), n.trim().to_string()));
                    }
                    Ok(Call { method: Method::POST, path: "/voices/import".into(), payload: Payload::Form { fields, files } })
                },
            },
            Tool {
                name: "voice_photo_set",
                description: "Give a voice a photo from a JPG, PNG or WebP file on this computer.",
                schema: || object(json!({ "name": { "type": "string" }, "path": { "type": "string" } }), &["name", "path"]),
                call: |args| {
                    let path = PathBuf::from(text(args, "path")?);
                    if !path.is_file() {
                        return Err(format!("{} is not a file on this computer.", path.display()));
                    }
                    let file = file_name(&path);
                    Ok(Call { method: Method::POST, path: "/voices/photo".into(), payload: Payload::Form { fields: vec![("name".to_string(), text(args, "name")?)], files: vec![("file".to_string(), path, file)] } })
                },
            },
            Tool {
                name: "voices_export",
                description: "Save chosen voices with everything of theirs (sound, reference text, card, photo) as a ZIP file at path.",
                schema: || object(json!({ "names": { "type": "array", "items": { "type": "string" } }, "path": { "type": "string", "description": "the .zip file to write" } }), &["names", "path"]),
                call: |args| post("/voices/export".into(), json!({ "names": args.get("names").cloned().unwrap_or(Value::Null), "path": text(args, "path")? })),
            },
            Tool {
                name: "voices_import_zip",
                description: "Take voices from a ZIP file made by voices_export: a voice already in the library (the same sound) is recognised and skipped, a different one under a taken name gets a number.",
                schema: || id_only("path", "the .zip file"),
                call: |args| {
                    let path = PathBuf::from(text(args, "path")?);
                    if !path.is_file() {
                        return Err(format!("{} is not a file on this computer.", path.display()));
                    }
                    let file = file_name(&path);
                    Ok(Call { method: Method::POST, path: "/voices/import-zip".into(), payload: Payload::Form { fields: Vec::new(), files: vec![("file".to_string(), path, file)] } })
                },
            },
        ];
        all.extend(speech_tools::tools());
        all.extend(atomic::tools());
        all.extend(window::tools());
        all.extend(studio_tools::tools());
        all.extend(guides::tools());
        all.extend(assistant::tools());
        all
    })
}

// ---------------------------------------------------------------- annotations

/// MCP tool annotations, from what each tool does.
pub(crate) fn annotations(name: &str) -> Value {
    const READS: &[&str] = &[
        "_get",
        "_status",
        "_list",
        "_catalog",
        "_files",
        "_capabilities",
        "_system",
        "_frame",
        "_waveform",
        "_avatar",
        "_models",
        "_voices",
    ];
    // a verb that changes something outweighs a noun that reads
    const CHANGES: &[&str] = &[
        "import",
        "delete",
        "create",
        "update",
        "cancel",
        "select",
        "download",
        "apply",
        "add",
        "set",
        "assign",
        "analyze",
        "render",
        "remix",
        "retranslate",
        "align",
        "patch",
        "put",
        "rename",
        "save",
        "hide",
        "keep",
        "reorder",
        "regen",
        "enable",
        "resume",
        "export",
        "open",
        "reveal",
        "shorten",
        "pin",
        "refresh",
    ];
    // reads whose names the rules above miss
    const READ_NAMES: &[&str] = &[
        "studio_wait",
        "proxy_test",
        "openrouter_verify",
        "ui_screenshot",
        "ui_read_page",
        "ui_console",
        "url_probe",
        "url_probe_cookies_text",
        "studio_health",
        "engine_tags",
        "voice_sample",
        "voice_design_sample",
        "dictionary_try_file",
        "project_dictionary_counts",
        "project_unknown_words",
        "cloud_model_form",
        "cloud_estimate",
        "cloud_keys",
        "cloud_spend",
        "openai_api_log",
        "openai_api_models",
        "listen_audio",
        "project_queue",
        "history_list",
        "speech_live_watch",
        "job_events",
        "book_preview",
        "book_chapter",
        "project_take_file",
        "project_take_peaks",
        "project_media_file",
        "project_media_peaks",
        "music_studios",
        "listen_subtitle_rules",
        "listen_stats",
        "listen_record_live",
        "listen_watch_status",
        "project_quality",
        "project_export_file",
        "agent_status",
        "windows_watch",
        "player_state",
    ];
    // writes over what was stored, so the earlier content is gone: a client asks first
    const OVERWRITES: &[&str] = &[
        "openrouter_set_key",
        "voice_transcript_set",
        "cloud_set_key",
        "openai_api_key_create",
        "openai_api_log_clear",
        "journal_clear",
        "project_queue_clear",
        "project_queue_remove",
        "window_command_drop",
    ];
    let changes = CHANGES.iter().any(|verb| name.split('_').any(|word| word == *verb));
    let read_only =
        READ_NAMES.contains(&name) || !changes && READS.iter().any(|part| name.ends_with(part) || name.contains(&format!("{part}_")));
    let destructive = name.ends_with("_delete")
        || name.ends_with("_cancel")
        || name.contains("_cancel_")
        || name.ends_with("_delete_key")
        || OVERWRITES.contains(&name);
    // what reaches the internet: OpenRouter, Hugging Face, the sites of links and every download
    let open_world = name.starts_with("google_") && matches!(name, "google_set_key" | "google_models")
        || name.starts_with("openrouter_") && !matches!(name, "openrouter_status" | "openrouter_delete_key")
        || matches!(
            name,
            "models_download"
                | "voice_download"
                | "voices_download_pack"
                | "voices_catalog"
                | "proxy_test"
                | "url_probe"
                | "url_probe_cookies_text"
                | "project_create_from_url"
                | "url_fetch_resume"
                | "url_tool_update"
                | "cloud_catalog_refresh"
                | "cloud_model_form"
                | "cloud_set_key"
                | "assistant_chat"
                | "assistant_models"
        );
    let title = name.replace('_', " ");
    // a one-call tool on a file finds its project again and answers from the finished work
    let idempotent = read_only || name.ends_with("_set") || name.contains("_select") || name.ends_with("_file");
    json!({ "title": title, "readOnlyHint": read_only, "destructiveHint": destructive, "idempotentHint": idempotent, "openWorldHint": open_world })
}

// ---------------------------------------------------------------- arguments

/// A required text argument, or a message saying which is missing.
pub(super) fn text(args: &Value, name: &str) -> Result<String, String> {
    args.get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("'{name}' is required"))
}

/// A path segment, escaped.
pub(crate) fn segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The arguments without the ones that went into the path.
pub(super) fn body_without(args: &Value, taken: &[&str]) -> Value {
    let mut body = args.as_object().cloned().unwrap_or_default();
    for name in taken {
        body.remove(*name);
    }
    Value::Object(body)
}

pub(super) fn object(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

pub(super) fn id_only(name: &str, what: &str) -> Value {
    object(json!({ name: { "type": "string", "description": what } }), &[name])
}

pub(super) fn nothing() -> Value {
    json!({ "type": "object", "additionalProperties": false })
}

pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "video".into())
}

/// The project a tool acts on.
pub(super) fn pid() -> Value {
    json!({ "type": "string", "description": "project id (projects_list, project_create)" })
}

pub(super) fn project_only() -> Value {
    object(json!({ "pid": pid() }), &["pid"])
}

/// The route of a project, and what follows it.
pub(super) fn project_path(args: &Value, rest: &str) -> Result<String, String> {
    Ok(format!("/projects/{}{rest}", segment(&text(args, "pid")?)))
}

/// A query string of the arguments given, in this order.
pub(super) fn query(pairs: &[(&str, Option<String>)]) -> String {
    let given: Vec<String> = pairs
        .iter()
        .filter_map(|(name, value)| value.as_ref().map(|value| format!("{name}={}", segment(value))))
        .collect();
    if given.is_empty() {
        String::new()
    } else {
        format!("?{}", given.join("&"))
    }
}

/// An argument as a query value: text as it is, a number, or a switch as 1 or 0.
pub(super) fn given(args: &Value, name: &str) -> Option<String> {
    match args.get(name)? {
        Value::String(text) => Some(text.clone()),
        Value::Bool(on) => Some((if *on { "1" } else { "0" }).into()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

pub(super) fn ids(what: &str) -> Value {
    json!({ "type": "array", "items": { "type": "string" }, "description": what })
}

/// The languages a project is dubbed from and into.
pub(super) fn languages() -> Value {
    Value::Array(
        dub_translate::WHISPER_LANGS
            .iter()
            .map(|(code, name)| json!({ "code": code, "name": name }))
            .collect(),
    )
}

/// A resource by its uri: its type and its text.
pub(crate) fn resource(uri: &str) -> Option<(&'static str, String)> {
    match uri {
        SKILL_URI => Some(("text/markdown", SKILL.to_string())),
        LANGUAGES_URI => Some(("application/json", serde_json::to_string_pretty(&languages()).unwrap_or_default())),
        _ => uri
            .strip_prefix(guides::GUIDE_URI)
            .and_then(guides::guide)
            .map(|text| ("text/plain", text)),
    }
}

/// A moment an agent names, as unix seconds in this computer's time zone:
/// today, yesterday, a date (its start, or its end for an upper bound) or a date-time.
pub(super) fn moment(text: &str, end: bool) -> Result<i64, String> {
    use chrono::{Duration as Days, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
    let text = text.trim();
    let day = match text.to_lowercase().as_str() {
        "today" => Some(Local::now().date_naive()),
        "yesterday" => Some(Local::now().date_naive() - Days::days(1)),
        _ => NaiveDate::parse_from_str(text, "%Y-%m-%d").ok(),
    };
    let at = match day {
        Some(day) => day.and_time(if end {
            NaiveTime::from_hms_opt(23, 59, 59).unwrap_or_default()
        } else {
            NaiveTime::MIN
        }),
        None => NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M"))
            .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M"))
            .map_err(|_| format!("'{text}' is not a moment: today, yesterday, 2026-09-26 or 2026-09-26T18:00."))?,
    };
    Local
        .from_local_datetime(&at)
        .earliest()
        .map(|at| at.timestamp())
        .ok_or_else(|| format!("'{text}' does not exist in this time zone."))
}
