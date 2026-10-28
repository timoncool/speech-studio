---
name: speech-studio
description: Drive Speech Studio on this computer through its MCP server - voice texts, scripts, podcasts and books (Higgs Audio, Drama 3, Supertonic, Qwen3-TTS, OpenRouter, Gemini, fal, Runware) with stress marks, normalisation and a pronunciation dictionary, recognise recordings with speakers and word times (Parakeet, Nemotron 3.5, Whisper, OpenRouter), edit them by text, check voicing against a text, manage the voice library and models, and work the studio's window while the user watches. Use whenever the user asks for anything the studio does.
---

# Speech Studio through MCP

Speech Studio serves MCP at `http://127.0.0.1:8797/mcp` while it is open (Streamable HTTP, stateless JSON-RPC). Every
tool runs the same code as a control of the studio, through the routes its window calls.

## Connect

```bash
claude mcp add --transport http speech-studio http://127.0.0.1:8797/mcp
```

Other clients: `{ "mcpServers": { "speech-studio": { "type": "streamable-http", "url": "http://127.0.0.1:8797/mcp" } } }`.
Only agents on this computer and the studio's own window may connect. Settings, **Agent (MCP)** shows whether an agent is
connected and the address to paste.

## Ground rules

- **Start with `studio_status`**: what runs now, what finished last, whether the required models are there.
  `studio_system` (the graphics card, its memory and load, RAM), `studio_capabilities`, `studio_paths` (where the data,
  projects and models are), `studio_health` (the service's version and folder), `agent_status` (this connection as
  the settings show it).
- **Long work is a job**: `studio_wait` with its `job_id` (or `until: download | idle`) instead of polling; it returns
  within a minute with how far the work got - call it again. `jobs_list` (`pid` narrows it), `job_get`, `job_cancel`,
  `job_events` (every stage and percent of a job in order, for some seconds).
- **Several calls at once**: `studio_batch` runs a list of tools in order in one call and answers each one's result.
- **Guides before writing**: `studio_guide` with `tags`, `podcast` or `audiobook` (the same texts are the resources
  `studio://guide/<topic>`); prompts `write_podcast` and `make_audiobook` start those tasks with their guides.
- **Files come back as paths**: a tool that answers sound (`voice_sample`, `project_take_file`, `project_export_file`,
  `listen_audio`, `openai_api_speech`...) writes it into the studio's `agent-files` folder and answers its path, type
  and size; the folder keeps a file for a day, so copy what you keep longer. A picture (`voice_photo_get`) comes as an
  image.
- **Models**: `models_status` lists every component with its size and state, `models_download` fetches by id in the
  background (resumable, every file checked against its pinned SHA-256), `models_cancel_download` pauses,
  `models_import` takes files already on disk, `models_select` picks a quantisation, `models_remove` frees disk space,
  `engine_presets_get` / `engine_preset_apply` set everything for this computer's card or for the cloud at once.
  `models_folder_open` shows the models folder to the user, `models_import_pick` lets them choose a folder to import.
- **Settings**: `settings_get`, `settings_set` (one key to a value), `studio_language_set` (the language of the
  studio's messages). Proxy: `proxy_settings_get`, `proxy_settings_set`, `proxy_test`.
- **Keys never come back**: `openrouter_status`, `google_status`, `local_server_key_status`, `cloud_keys` and
  `openai_api_status` say only whether a key is set and where it comes from. OpenRouter: `openrouter_set_key`,
  `openrouter_delete_key`, `openrouter_verify`, `openrouter_models` (llm, vision, tts, asr), `openrouter_voices`,
  `openrouter_catalog`, `openrouter_catalog_refresh`. Google Gemini: `google_set_key`, `google_delete_key`,
  `google_models`. A local OpenAI-compatible server: `local_server_models`, `local_server_key_set`,
  `local_server_key_delete`. fal and Runware: `cloud_catalog`, `cloud_catalog_refresh`, `cloud_model_form` (the settings
  a model takes), `cloud_estimate` (its price before a run, nothing paid), `cloud_set_key`, `cloud_delete_key`,
  `cloud_spend` (what was paid, the daily limit).
- **Voices**: `voices_list` (`brief` for names, kind, language and gender only), `voices_catalog` (the cloud voice
  library), `voice_download`, `voices_download_pack` (the starter pack), `voice_import`, `voices_import_zip`,
  `voices_export`, `voice_rename`, `voice_delete`, `voice_card_set`, `voice_photo_set`, `voice_transcribe`,
  `voice_transcript_set`; a voice from the user's microphone is recorded by the person in the voice library (no tool
  turns the microphone on): `voice_record_devices`, `voice_record_level` while it records, `voice_record_stop` keeps
  it. Cleaning a reference: `voice_clean` (voice from music, Roformer),
  `voice_denoise` (noise, DPDFNet, `db` limits how far), `voice_restore`; its best piece: `voice_ref_candidates`,
  `voice_ref_apply`. A voice from a description: `voice_design` (description best in English or Chinese, a sample text,
  1-6 candidates; a job), `voice_design_sample` (a candidate as a file), then `voice_design_keep` (one candidate into the
  library). `voice_sample` gives a voice's reference as a file (`original` - before cleaning), `voice_photo_get` its photo.
  A character's variants (angry, whispering): another library voice whose card has `variant_of` (the main voice) and
  `variant` (its name), set with `voice_card_set`; a block speaks a variant when its `voice` is that variant's name.
  Drama 3 clones only a reference with its text; the others take the text as a help. `engines_list` says it per
  engine (`ref_text_required`, `ref_max_secs`).
- **By link**: `url_probe` (title, length, qualities, the site's subtitles; `url_probe_cookies_text` with a cookies.txt
  given as text), `project_create_from_url` (downloads into a new Listening recording in the background),
  `url_fetches_list`, `url_fetch_get`, `url_fetch_cancel`, `url_fetch_resume`, `url_fetch_delete`, `url_tool_status`,
  `url_tool_update`.
- **Dialogs for the user**: `folder_pick` (a folder they choose), `project_export_save_as` (where to save an export),
  `models_import_pick`; `project_export_reveal`, `diagnostics_report_reveal`, `diagnostics_logs_reveal` and
  `models_folder_open` show a file or folder in Explorer.
- **The window**: `ui_screenshot`, `ui_read_page`, `ui_click`, `ui_type`, `ui_select`, `ui_press_key`, `ui_scroll`,
  `ui_navigate`, `ui_open_settings`, `ui_open_help`, `ui_notify`, `ui_console` work in the studio's window in front of
  the user; a key or password field reads only as filled or empty. `editor_open` opens a project in the window,
  `editor_state` says what is open (its blocks or lines, what is selected and playing), `editor_select` shows the user a
  block or a line. With several windows open: `windows_watch` (the windows and what the studio tells them),
  `window_focus` (which one the ui_* tools work in), `window_command_drop` (give up a command no window answered).
- **The player**, for the user to hear in the window: `player_play` (a block's take, or with `on` everything from that
  block on, voicing what is missing; a Listening line or a moment; `voice` a library voice's sample; with none of
  them it goes on after a pause; `take` plays another take of the block and, while the block plays, switches to it from
  the same moment, an A/B of two takes), `player_pause`, `player_seek`, `player_set` (volume, speed and, in a Listening recording, `loop`: the phrase under the playhead round and round), `player_state`.
  `ui_press_key` Ctrl+K opens the command palette: every action of the window by name.
- **The journal**: every change you make to projects, recordings, voices, models, keys and settings is written into the
  window's activity log with the name of what it changed, and stays there after a restart. `journal_get` reads it,
  `journal_note` writes a line for the user, `journal_delete` removes one, `journal_clear` empties it.
- **Diagnostics**: `diagnostics_get` (version, system, card, components, the engines' DLLs, the latest log lines),
  `diagnostics_report` (a ZIP for a bug report, keys masked).
- **The Assistant panel**: `assistant_models` (the models it can work with), `assistant_chat` (talk to it as the user
  does; it pays from the user's OpenRouter key).
- **Being the assistant**: when the user chose «Agent (MCP)» as the assistant (Settings, Agent), what they write in the
  Assistant panel waits for you: `assistant_requests_wait`, do what the request asks with the tools (the panel shows each
  call), then `assistant_request_answer` with your answer in words, in the language the request names.
- **Being the studio's model**: when the user chose «Agent (MCP)» as the script assistant's model (Settings, Models,
  instead of Gemma, a local server or OpenRouter), the studio's own model work - the podcast script, the characters of a
  book, punctuation, enrichment - comes to you through the same `assistant_requests_wait` as kind `studio_model`, with
  the `instructions` and the `request` its model would get and the `answer_schema` its answer must match. Write it
  yourself as that model would and answer with `assistant_request_answer`: the JSON object of the schema (or plain text
  without one); the job goes on with it. The job waits for you up to 9.5 minutes; keep calling while one runs.


## Speech: voicing

- **One call**: `speak_text` (text, voice, engine, lang, format, direction) gives an audio file; the same arguments
  again answer with the same file at once. Longer work answers `done: false` with its job: `studio_wait`, then call
  again.
- **Speak aloud as you write** (a voice for an agent, heard from the studio): `say_start` (voice, engine - default
  qwen3-tts, first sound in about 0.1 s - lang, direction) opens a session; send your answer with `say_text` while you
  write it, a few words at a time, without waiting for the end: each finished phrase is voiced at once and plays from
  the studio's window, the next one ready before the last stops; `end: true` with the last piece says the rest.
  `say_stop` falls silent at once (the person speaks over you), `say_state` shows each phrase and how soon it was
  heard. It needs the studio's window open; a session is a Speech project, kept with its takes.
- **Talk with a person by voice** (the studio as an agent's walkie-talkie): only the person turns the microphone on,
  with Live's button in the window - no tool and no `ui_*` click or key does, and the window shows while it is on
  (`mic_state`, with the recogniser that hears now). With no window of the studio open for a few seconds it turns off by itself. `voice_listen_start` joins
  the conversation the person started (mic_off: ask them to press Live; its `engine` changes the recogniser of the
  running conversation at once); `voice_listen` (after the last `heard`) waits
  for their next phrase; answer with `say_start` / `say_text` as you write; listen again. When the person speaks over
  the studio, its speech stops by itself. You may turn the microphone down, never up: `voice_listen_mute` mutes it,
  `voice_listen_stop` ends the conversation. A phrase the person types in Live (`voice_listen_type` puts one in) comes
  from `voice_listen` like a spoken one. `say_silence` stops everything said aloud, in every session. The person's
  microphone test in Live: `voice_record_level` while it runs, `mic_test_stop`, `mic_test_audio` for what it heard.
  To compare engines with the person, `studio_timings` tells how long each step took: each phrase recognised (the
  recogniser, recognise_ms, after_speech_ms), each phrase said aloud (heard_ms from its text to its first sound,
  first_ms, synth_ms) and each engine loaded; `say_state` with `last` gives a session's latest phrases.
  The voice the person chose in Live (`say_voice`, set with `say_voice_set`) is what `say_start` speaks with when you
  give no engine or voice: leave them out unless the person asks for another.
- **Projects**: `projects_list`, `project_create` (paragraphs become blocks; "Anna: ..." lines become a dialogue; `corpus` ru_hard instead of a text makes the engine check - hard Russian phrases with the recognition check on, voiced and read through `project_quality`),
  `project_restore` (take back a project deleted to the recycle bin, as the window's Undo), `project_get`, `project_update` (send the whole project back with your edits; changed blocks go stale; a block's `lang` marks a passage in another language, and `settings.lang_voices` gives each such language a voice; `settings.normalize_off` leaves groups of normalisation as written; `settings.one_voice` voices every block with the project's engine, voice and sampling, the cast kept; `characters` gives each character a gender and age; a take's `cut` [start, end] in seconds is the part of it the project plays and exports, the file staying whole; a block's `fx` and a character's `cast_fx` set a voice effect carried over from Game Dub - radio, phone or loudspeaker (`none` on a block takes its character's off) - applied at export and when the window plays the take, the take itself staying raw; a block's `mix` is its clip's montage from the track editor - `gain_db` (-40..24), `fade_in` and `fade_out` in seconds over the part of the take its `cut` keeps - applied after the effect and the even level at export and when the window plays the take; `settings.export_name` and `settings.clip_name` name the export's files and the per-block files by a template - {project}, {chapter}, {n}, {date}, and for blocks also {speaker}, {text}, {block}; empty keeps the studio's names, and several files need {n} or {chapter}; `settings.level_pieces` (on by default) brings every take to one loudness before the mix, as Dub levels its phrases, so quiet and loud pieces and voices sound alike, the export preset then setting the whole file's loudness; `settings.qc.heuristics` lists the checks of each new take's sound carried over from Game Dub - defect, clipping, length, edge, long_pause, little_speech, loudness, pause_noise (on by default), audio_repeat, whistle (costly, off): a defect redoes the take, a suspect is recognised by Parakeet and Whisper and redone on a repeat or mush, the rest are notes on the take's qc), `project_auto_cast` (library voices for the characters without one, by gender, age and language),
  `project_synth` (new and stale blocks, or `ids`, `force`, `from`), `project_export_check` (what keeps the export back:
  missing, stale, failed, a take file changed after voicing, a take longer than its cue), `project_quality` (how the voicing sounds by the check of the active takes: passed, failed, unchecked, a green/amber/red light by the share of checked takes that passed, and the failed takes worst first with expected and heard), `block_fix` (mend a failed take in place: the sentences gone wrong voiced again alone and spliced in, a tail or a long gap cut; a whole-sound defect or a one-sentence block voiced again whole; the new take wins only if it checks better), `block_ab` (a blind comparison of two local engines on a block: a take each, the active one kept; the pick is a `project_update` of the block's engine, voice, active take and its `ab` votes), `project_check_takes` (check ready takes by recognition now - the unchecked ones or `ids` - whatever the project's settings), `project_enrich` (the director suggests expression tags by meaning, or with `mode` punctuation the punctuation for reading aloud, words kept; apply the after texts with `project_update`), `block_ref_set` (a block's own reference clip and its text instead of a voice; `whole` continues that clip's speech), `project_export` (one file
  with subtitles in SRT, VTT, ASS and LRC and a table of its lines in CSV (result `table`: time, chapter, speaker, text, take, engine, voice) and a `sha256sum` manifest of every file it built (result `manifest`), M4B with chapters, `part_minutes` for long books, `per_chapter` for a file per chapter levelled as one book (the chapters keep their levels against each other), `stems` for a ZIP beside it with a FLAC track per speaker that add up to the file (result `stems`), or `per_block`; `allow_duration` lets only the
  too-long takes through), `project_lint` (before voicing: what would go wrong block by block, errors and warnings), `project_exports`, `project_exports_all` (every built audio file of every project, newest first, with its recipe: how it was made), `project_timeline` (blocks on the export's time scale with overlaps; a
  block's `cut_previous` cuts the reply it interrupts, a previous block's `pause_ms` of 0 joins them), `project_delete`.
  A block's `pause_ms` is the pause after it; a negative one makes the next block interrupt. Inside a block, `[pause 500ms]` and `[rate:0.9]` work with every engine, and a pasted SSML subset turns into them (`<break>`, `<prosody rate>`, `<sub alias>`, `<say-as interpret-as="characters">`; `text_prepare` shows how). While a voicing runs:
  `project_queue`, `project_queue_remove`, `project_queue_clear`; `speech_live_watch` hears the blocks as they are
  voiced (each block's sound as a WAV file). `history_list` lists the takes voiced lately.
- **Projects from files**: `project_create_from_subtitles` (SRT, VTT, ASS), `book_preview` + `book_chapter` +
  `project_create_from_book` (an audiobook of the chosen chapters; `dialogue` puts direct speech under its own speaker; `into` rebuilds a project from the edited book keeping the voiced blocks of unchanged paragraphs, `dry_run` counts first), `project_package` / `project_import_package` (one
  `.speechproj` file to move a project), `listen_to_speech` (a recording's transcript).
- **Tracks of a book or play** (music, sounds, ambience beside the voices): `project_track_import` (sound files and
  folders of this computer onto a track, copied into the project's media, one after another), `project_track_upload`
  (one file by upload, as the window does), `project_media_file` and `project_media_peaks` (a clip's sound and its
  waveform), `music_studios` and `project_track_music` (music made in YuE2, MiniMax or ACE-Step Studio on this
  computer by its own song form, a job; the song stays in that studio's library and comes onto a track). The project's `tracks` - each with `id`, `name`, `gain_db`, `muted` and `clips` (`file`, `label`, `start`
  in seconds on the export's scale, `duration`, `cut`, `mix`) - are edited with `project_update`; the export mixes every
  track that is not muted under the voices (not when one chapter is exported alone), and the tracks by speaker give each
  track its own stem. No ducking: the levels are set by the tracks' and clips' gains.
- **Takes and exports as files**: `project_take_file`, `project_take_peaks` (a take's waveform, 25 peaks a second, as the track editor draws it), `project_export_file`, `project_export_save` (copy to a path),
  `project_exports_to_folder`, `project_export_reveal`, `project_export_save_as`.
- **Expression tags differ per engine** (take them from `engine_tags` or `engines_list`, never across engines):
  - `higgs`: `<|category:value|>`, 43 tags (`PROMPTING.md` of the model). Emotion, style and prosody speed, pitch and
    expressiveness go at the start of the sentence: `<|emotion:sadness|>Ну что, начнём.`; pause and sound go at the spot,
    a sound glued to its onomatopoeia with no space: `<|sfx:laughter|>Ха-ха, привет!`. Unknown tags are cut and named.
  - `drama3`: free text in square brackets at the spot (`[laughs]`, `[sighs]`, `[clears throat]`), a span in angle
    brackets (`<whisper>не говори ей</whisper>`, `<slow>`, `<emphasis>`); the block's `direction` becomes the line's
    style prefix. Its tags and directions work poorly in Russian.
  - `supertonic`: `<laugh>`, `<breath>`, `<sigh>` at the spot (the model card names these; it has ten, unpublished).
  - `qwen3-tts`: no tags — they would be read aloud and are cut; write laughter as onomatopoeia in the text (`ха-ха`),
    the delivery as the block's `direction`, which reaches only the model's own nine voices.
  - Pauses `[pause 500ms]` and tempo `[rate:1.2]` work with every engine.
- **Engines**: `engines_list` says what each engine understands (languages, sampling, tags, cloning, `needs_voice`) and
  why one is not available here (Drama 3 needs a 6 GB card, and 32 GB of RAM when the card is under 24 GB or the studio setting `drama_vram_gb` is below the whole model). Local engines: `higgs`, `drama3`, `supertonic` (its own ten
  voices), `qwen3-tts` (nine named voices, cloning, the block's direction as its instruction) and `qwen3-tts-ru` (Russian
  stress marks, a library voice of at most 10 s). A speaker's engine (`cast_engines`) and a block's (`engine`) override
  the project's; cloud engines are set for the whole project only. Cloud engines cost money: `openrouter`, `google` (Gemini; the project's
  `cloud_model` and `cloud_voice`; the studio setting `google_tts_mode`: `standard` - direct requests,
  `or_concurrency` at a time; `batch` - the whole job as one half-price batch that can take hours, safe to restart: a
  batch already sent is found again, not paid twice; `auto` - direct up to 10 phrases, batch from 11;
  `project_google_report` shows the batch waiting, the last job's result with each failed phrase and Google's reason,
  and past batches), `fal`, `runware`. Ask the user before a paid run.
- **Text**: `text_prepare` shows how a text will be read (dictionary, Russian normalisation, stress marks, the engine's
  text), `text_stress` gives stress marks and ё with unsure homographs, `text_direct_speech` cuts direct speech (a
  dialogue dash, quotations) apart from the author's words, `dictionary_get` / `dictionary_set` /
  `dictionary_import` (Balabolka .dic, .rex, .bxd), `dictionary_try` + `dictionary_try_file` (hear a rule before saving
  it), `project_dictionary_counts` (how often each rule fires in a project), `project_unknown_words` (the words no
  dictionary knows).
- **Director model**: `podcast_write` (topic and speakers with their manner -> a new project),
  `project_cast_characters` (one chapter split into the narrator and characters word for word).

## Listening: recognition

- **One call**: `transcribe_file` (path, format text/srt/vtt/ass/json, lang, speakers), `verify_speech` (path and the
  text it should say -> word error rate and each missed, extra or replaced word), `separate_file` (voice and music apart).
- **Recognition settings** (in `listen_update`): `engine` ("" as in Settings, `parakeet` - 25 European languages, and
  `nemotron` - Nemotron 3.5 ASR, 28 languages with punctuation, Chinese and Japanese among them, both on transcribe.cpp
  after `models_download` of `transcribe-engine` and `parakeet` (or `parakeet-ultra`) or `asr-gguf-nemotron`,
  `whisper`, `openrouter`, `fal`), `prompt` (a hint Whisper takes, with the corrections' spellings), `denoise` and
  `denoise_db` (DPDFNet before recognition), `vad` (Silero VAD cuts a long recording for Parakeet and Nemotron at
  pauses: `threshold`, `hysteresis`, `min_speech_ms`, `min_silence_ms`, `pad_ms`).
- **Projects**: `listen_create_from_file` (`listen_upload` uploads a copy),
  `listen_watch_start` / `listen_watch_status` / `listen_watch_stop` (a folder of recordings recognised in the background into files, once or watching it for new ones, going on after a restart), `listen_record_live` (what is heard, phrase by phrase, while the person records from the microphone in the window; the phrases become the recording's first transcript), `listen_record_stop` (ends that recording), `listen_audio` (its sound as a file), `listen_get`, `listen_update` (rename speakers, edit or hide lines, mark
  words `cut`, recognition settings, corrections), `listen_transcribe`, `listen_cut` (a new recording without the
  marked words), `listen_stats` (who speaks how much: talk time, share, words, pace, longest stretch), `listen_export` (with `digits` numbers said in words come out as digits - "двадцать пять" as 25, "сорок пятом" as 45-м - in ru, en, es, fr, pt, de, it, nl, da, the transcript itself unchanged; with `rules`, a broadcaster's subtitle rules from `listen_subtitle_rules`, srt, vtt and ass go onto screens by its line length, lines and longest showing; without it the lines are exported as recognised), `listen_check`, `listen_align` (the reference text laid over the recording, each word
  at the time it was heard; put the lines back with `listen_update`), `listen_to_speech` (the transcript as a Speech project),
  `listen_voices` (each speaker's best clean speech into the voice library). Who a speaker is, by voice:
  `listen_speakers_match` (the most alike named speaker or library voice, with its score and whether it is the same
  person), `listen_speaker_remember` (keep a speaker's voice print under its name so later recordings recognise the
  person), `speaker_profiles_list`, `speaker_profile_delete`.

## OpenAI-compatible API

The studio's own tools for it: `openai_api_status`, `openai_api_key_create` (a new key, shown once),
`openai_api_key_delete`, `openai_api_log`, `openai_api_log_clear`, and `openai_api_models`, `openai_api_speech`,
`openai_api_transcribe` to call it as a program would.

Programs that speak OpenAI's audio API can use the studio directly: `POST http://127.0.0.1:8797/v1/audio/speech`
(`model` tts-1 / higgs / drama3, `voice` a voice name of the library, `instructions`, `speed`, `response_format` mp3,
wav, flac, aac, pcm; beyond OpenAI's fields the project settings of the engine, each over its default: `seed`,
`temperature`, `top_k`, `top_p`, `max_tokens`, `steps` for Supertonic, `loudness` off, video, podcast, acx), `POST /v1/audio/transcriptions` (multipart `file`, `model` whisper-1, `language`,
`response_format` json, text, srt, vtt, verbose_json) and `GET /v1/models`. With a key (`SPEECH_STUDIO_API_TOKEN`, or
one the user made in Settings, API) send `Authorization: Bearer <key>`. Settings, API also shows examples and the latest
requests with their answers (`ui_open_settings` with section api).
