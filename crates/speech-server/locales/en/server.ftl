asr-windowed = transcribing in windows on the GPU ({ $speakers } { $speakers ->
    [one] speaker
   *[other] speakers
})

## Separation, OCR, benchmark, casting library, cloud ASR and TTS

atomic-extract-separation-audio = extracting 44.1k audio for separation
atomic-separating = separating ({ $model })
atomic-separation-failed = separation: { $error }
ocr-detecting-burned-text = detecting burned-in text ({ $model })
bench-stage = ⏱ { $name }: { $seconds }s | GPU ~{ $gpu_avg }% (peak { $gpu_max }%) | CPU ~{ $cpu_avg }% | VRAM { $vram }MB | bound by: { $bound }
bench-total = ⏱ { $label } TOTAL: { $seconds }s
casting-no-casting-json = the project has no casting.json (casting has not been run)
casting-profile-delete-failed = deleting profile { $slug }: { $error }
casting-profile-not-found = profile “{ $slug }” not found
cloud-asr-no-key = cloud ASR is on, but no OpenRouter key is set
cloud-asr-no-model = no OpenRouter STT model is selected in the settings
cloud-asr-failed = cloud ASR: { $error }
cloud-asr-empty = cloud STT returned an empty transcript
cloud-tts-no-key = cloud TTS is on, but no OpenRouter key is set
cloud-tts-no-voice = no TTS voice is set in the settings (each model has its own voices)
cloud-tts-failed = cloud TTS: { $error }
cloud-tts-too-short = cloud TTS: the audio is too short ({ $bytes } { $bytes ->
    [one] byte
   *[other] bytes
})
cloud-tts-read-wav = reading the cloud wav: { $error }

## Shared messages, compositing, downloads, dub timing, endpoints, preview frame

common-read = reading { $path }: { $error }
common-parse = parsing { $path }: { $error }
common-ffmpeg-start = starting ffmpeg: { $error }
compose-summary = composite: titles={ $titles } (bbox), blur { $boxes } { $boxes ->
    [one] box
   *[other] boxes
}, sub_px={ $sub_px }
compose-taglines-no-mt = taglines: machine translation is unavailable ({ $error }) -> blur only
compose-taglines-translating = taglines: translating the title card text
compose-taglines-failed = taglines: the translation failed; blur only
downloads-interrupted = the download stopped when the app closed; what was fetched is kept in .part and resumes from there
downloads-nothing-to-download = none of the selected ids is a downloadable component
downloads-busy = a download is already running
downloads-thread-failed = the download thread did not start: { $error }
downloads-panicked = the download stopped on an internal error: { $error }
timing-words-not-recognized = word timings of the dub were not recognized for { $count } { $count ->
    [one] line
   *[other] lines
} ({ $examples }); their words are highlighted by length
openrouter-catalog-failed = OpenRouter catalogue: { $error }
openrouter-empty-key = the key is empty
llm-server-no-address = no server address is set
llm-server-no-answer = the server { $base } did not answer: { $reason }
llm-server-status = { $endpoint } answered { $status }
llm-server-not-json = { $endpoint } did not return JSON: { $error }
llm-server-not-model-list = { $endpoint } did not return an OpenAI model list (no data field)
remix-start = remixing { $count } { $count ->
    [one] line
   *[other] lines
} → { $instruction }
remix-no-llm = remix: the LLM is unavailable: { $error }
remix-lines-changed = remix: the lines changed while the remix ran; the project was left as it was, run the remix again
frame-empty-ass = empty ASS: { $error }
frame-read-preview = reading the preview frame: { $error }
frame-read-source = reading the source frame: { $error }

## Glossary, hardware, job records, LLM providers

glossary-bad-format = format “{ $format }”: json or tsv
glossary-series-unreadable = the series glossary is unreadable: { $error }
glossary-no-text = the project has no text: the glossary is built from recognized speech, so analyze first
glossary-lines = glossary: { $count } { $count ->
    [one] line
   *[other] lines
} of text
glossary-no-llm = glossary: the LLM is unavailable: { $error }
glossary-failed = glossary: { $error }
glossary-proposed = glossary: { $count } { $count ->
    [one] entry
   *[other] entries
} proposed
glossary-series-profile-missing = series profile “{ $slug }” not found; its glossary is not applied
glossary-series-slug-unreadable = the glossary of series “{ $slug }” is unreadable: { $error }
glossary-series-applied = series glossary “{ $slug }”: { $count } { $count ->
    [one] entry
   *[other] entries
}, { $added } added to the project
hw-no-nvidia = no NVIDIA GPU
jobs-serialize-record = serializing job.json: { $error }
jobs-record-missing = { $file } is not in { $dir }
llm-llama-server-missing = llama-server not found ({ $path })
llm-gemma-missing = the Gemma GGUF was not found ({ $path })
llm-mmproj-missing = the Gemma vision projector (mmproj) was not found ({ $path })
llm-chat-client = chat client: { $error }
llm-local-no-text-model = the local server ({ $url }) is chosen for translation, but no model is selected
llm-local-no-vision-model = the local server ({ $url }) is chosen for vision, but no model is selected
llm-local-client = local server client: { $error }
llm-local-label = local server { $url } · { $model }
llm-openrouter-no-key = OpenRouter is chosen, but no key is set
llm-agent-no-vision = the agent connected over MCP does not look at pictures; choose another provider for them
llm-agent-label = Agent (MCP)
llm-agent-no-port = the server's port is not known yet; try again in a moment
llm-agent-absent = the studio's model is the connected agent (Settings → Models), but no agent is connected now: connect it (Settings → Agent) or choose another model
llm-openrouter-no-text-model = OpenRouter is chosen for translation, but no model is selected
llm-openrouter-no-vision-model = OpenRouter is chosen for vision, but no model is selected
llm-openrouter-not-text = the OpenRouter model { $model } does not answer in text; choose another one for translation
llm-openrouter-not-vision = the OpenRouter model { $model } does not accept images; choose a vision model
llm-vision-missing = none ({ $reason })
llm-pair = translation: { $text }; vision: { $vision }

## Media tools and OCR

common-ffmpeg-exit = ffmpeg exit code { $code }:
    { $tail }
media-ffprobe-start = ffprobe failed to start: { $error }
media-ffprobe-exit = ffprobe exited with code { $code }: { $stderr }
media-ffprobe-no-streams = ffprobe: no streams
media-no-streams = the input has neither a video nor an audio stream
media-no-duration = the duration could not be determined
media-no-wav = ffmpeg did not create the wav
media-ffmpeg-hung = ffmpeg did not finish in { $seconds }s and was killed (it hung)
media-ffprobe-duration-exit = ffprobe duration exit code { $code }
media-env-filter-script = envelope filter script: { $error }
media-no-sample-rate = { $path }: the sample rate was not read ({ $stderr })
ocr-no-blur = { $error }; no blur
ocr-models-missing = the OCR models were not found
ocr-detection-failed = OCR detection failed ({ $error })
ocr-summary = OCR: { $regions } { $regions ->
    [one] region
   *[other] regions
}, { $localize } to localize, { $bands } band { $bands ->
    [one] span
   *[other] spans
}, sub_y={ $sub_y }

## Hardware presets

common-write = writing { $what }: { $error }
preset-rtx5090-title = RTX 5090 (32 GB)
preset-rtx4090-title = RTX 4090 (24 GB)
preset-top-subtitle = Maximum quality: the top quants locally
preset-gpu16-title = 16 GB GPU
preset-gpu16-subtitle = High quality (4080/4070 Ti and similar)
preset-gpu12-title = 12 GB GPU
preset-gpu12-subtitle = Balanced (3060/4070 and similar)
preset-gpu8-title = 8 GB GPU
preset-gpu8-subtitle = Economical: light quants (3060 Ti/4060)
preset-weak-nvidia-cloud-title = Weak NVIDIA + cloud
preset-weak-nvidia-cloud-subtitle = The heavy work (translation/vision/voicing) in OpenRouter, separation and ASR on your GPU
preset-cloud-title = CPU + cloud (no NVIDIA)
preset-cloud-subtitle = The heavy work in OpenRouter, the local work on the processor; runs without a graphics card (a key is needed)
preset-custom-title = Custom
preset-custom-subtitle = I will set every parameter myself
preset-reason-no-gpu = No NVIDIA GPU found, so the “CPU + cloud” mode: the heavy work in OpenRouter, the local work on the processor (slower, but it works)
preset-reason-top-card = { $gpu } detected: the maximum quants
preset-reason-by-vram = { $gpu } · { $vram } GB VRAM: { $preset }
preset-reason-low-vram = { $gpu } · { $vram } GB VRAM is little for local models; the cloud is more reliable
preset-unknown = unknown preset: { $id }

## Service port, shortening, frontend, launch defaults

common-corrupt = { $path } is damaged: { $error }
common-create-dir = folder { $path }: { $error }
service-bad-port = { $value }: not a port number (1..65535 expected)
service-exe-path = the service exe path: { $error }
service-request-not-sent = the connection was accepted, but the request was not sent: { $error }
service-no-health-answer = the connection was accepted, but /health did not answer: { $error }
service-not-http = the answer is not HTTP
service-health-status = an HTTP server; /health answered { $code }
service-health-not-dub-studio = an HTTP server; /health did not answer with a Speech Studio body
service-health-not-json = an HTTP server; /health did not answer with JSON
service-health-no-fields = /health calls itself { $app }, but without the service fields: { $error }
service-other-app = another application ({ $app })
service-health-no-app = an HTTP server; /health without an application name
service-port-reserved = the system does not give out the port, though nobody accepts connections on it (it may be in a range Windows reserves: { $command })
service-port-dub-studio = Speech Studio { $version } is on it ({ $executable })
service-port-other = another process holds it: { $what }
service-port-busy = Port 127.0.0.1:{ $port } has been busy for { $seconds } s: { $who }. Error: { $error }.

    Close the program that holds it, or set another port with the { $env } environment variable (for example { $env }={ $other_port }).
shorten-line-done = shortening { $n }/{ $total }: { $from } -> { $to } characters
shorten-line-rejected = shortening { $n }/{ $total }: the answer was not accepted ({ $reason })
shorten-line-no-answer = shortening { $n }/{ $total }: the LLM did not answer: { $error }
shorten-auto-start = { $count } { $count ->
    [one] line does
   *[other] lines do
} not fit the slot; shortening the translation and voicing only those
shorten-higgs-unloaded = Higgs is unloaded while the translation is shortened
shorten-auto-no-llm = shortening the translation was skipped: the LLM is unavailable: { $error }
shorten-none-shortened = shortening: none of the { $count } lines got shorter; they stay as voiced
shorten-done = { $count } of { $total } { $total ->
    [one] line
   *[other] lines
} shortened
shorten-nothing = nothing to shorten: every line fits its slot
shorten-no-llm = shortening: the LLM is unavailable: { $error }
shorten-start = shortening { $count } { $count ->
    [one] line
   *[other] lines
}: { $provider }
shorten-all-failed = shortening failed: the LLM answered none of the lines ({ $id }: { $error })
spa-not-built = the frontend is not built
settings-bad-speaker-count = speaker_count: a whole number from 0 to { $max } is expected (0 means automatic)
settings-bad-vo-gain = vo_gain_db={ $value }: a number from { $min } to { $max } dB is expected
settings-bad-src-lang = src_lang={ $value }: neither a language code nor "auto"
settings-bad-tgt-lang = tgt_lang={ $value }: not a language code
settings-bad-casting-ref = casting_ref={ $value }: not a casting profile slug
settings-style-too-long = tr_style_custom is longer than { $max } characters
settings-too-many-slots = { $name }: more than { $max } slots
settings-unknown-field = unknown launch defaults field: { $key }
settings-read-failed = reading the launch defaults: { $error }
settings-patch-not-object = the PATCH /settings/launch body is an object of fields
settings-write-failed = writing the launch defaults: { $error }

## Takes, translation, TTS text, MCP arguments

takes-delete-old = deleting the old take { $path }: { $error }
takes-missing = take { $take } is not in the history of line { $line }
translate-no-vision = the content type was not determined: vision is unavailable ({ $error })
translate-subs-already-translated = the subtitles are already in the target language -> no machine translation (voicing only)
translate-transcribe-only = transcribe: tgt=the source text, no translation
translate-same-language = same language -> no machine translation (tgt=the source)
translate-no-llm = translation failed: the LLM is unavailable: { $error }
translate-ctx-pass = context pass: vision layout/scene + translating the transcript
translate-failed = translation failed: { $error }
translate-untranslated = translation failed: { $left } of { $total } { $total ->
    [one] line
   *[other] lines
} stayed in the source language (details in the log and logs/llama-server.log)
translate-untranslated-auto = translation failed: { $left } of { $total } { $total ->
    [one] line
   *[other] lines
} stayed in the source language; if the speech in the video is already in the target language, set the original language and no translation is needed (details in the log and logs/llama-server.log)
translate-done = translation ready: { $done }/{ $total } lines, titles={ $titles }
translate-left-untranslated = { $left } of { $total } { $total ->
    [one] line
   *[other] lines
} stayed in the source language (details in logs/llama-server.log)
translate-coverage-retry = translation coverage: { $count } { $count ->
    [one] line
   *[other] lines
} without a translation; translating them again
translate-coverage-failed = translation coverage: translating again failed ({ $error })
translate-coverage-left = translation coverage: { $count } left without a translation
tts-silent-after-cleanup = no speech left after cleaning the text, so silence: { $count } { $count ->
    [one] line
   *[other] lines
} ({ $lines })
mcp-bad-speaker-count = speaker_count: a whole number from 0 to { $max } is expected
mcp-speakers-without-diarize = speaker_count above 1 does not go with diarize=false

## Routes: projects, voices, setup, translation, export, jobs, alignment

project-serialize = serializing project.json: { $error }
project-delete-failed = deleting the project { $path }: { $error }
setup-no-ids = ids is empty
setup-fetching-missing = Fetching the models this feature still needs…
setup-diarization-missing = The diarization model did not download ({ $error }); the analysis goes on without telling the speakers apart
setup-pick-model-files = Model file(s)
setup-pick-models-folder = Folder with ready models
setup-no-folder = there is no folder { $path }
voices-not-found = voice { $name } was not found in voices/
voices-no-vocals = no vocals to measure F0 on; analyze first
voices-speakers-changed = voices by slots: the speakers changed while the voice was measured; the project was left as it was, run it again
analyze-args-not-object = analyze args: an object was expected
cost-analyze = OpenRouter: ${ $spent } spent on the analysis (${ $total } used in total)
cost-run = OpenRouter: ${ $spent } spent on the run (${ $total } used in total)
translate-lines-to = Translating { $count } { $count ->
    [one] line
   *[other] lines
} → { $lang }
translate-note = translation: { $note }
translate-titles-failed = the titles were not translated ({ $error }); they stay in the source language in the video
translate-titles-error = translating the titles: { $error }
translate-lines-changed = translation: the lines or titles changed while the translation ran; the project was left as it was, run the translation again
export-pick-folder = Where to save the results
export-copy-failed = copying to { $path }: { $error }
jobs-wait-not-number = wait: a number of seconds was expected
jobs-unknown-kind = unknown job kind in job.json: { $kind }
jobs-not-resumable = a { $kind } job cannot be resumed
align-no-source-text = aligning to speech: the lines have no text in the original language (the subtitles were imported in the target language)
align-no-vocals = aligning to speech: the project has no vocal track; analyze first
align-recognizing = aligning to speech: recognizing the words
align-recognition-failed = aligning to speech: recognition: { $error }
align-no-speech = aligning to speech: no speech was recognized; the timings were not changed
align-mismatch = aligning to speech: the lines did not match the speech ({ $share }% matched); the timings were not changed
align-lines-changed = aligning to speech: the lines changed during recognition; the timings were not changed, run the alignment again
align-done = aligned to speech: { $share }% of the lines by words, timing changed for { $changed }; offset { $offset } s

## Analysis

analyze-serialize = serializing { $what }: { $error }
analyze-read-model = reading the model { $path }: { $error }
analyze-stage-cache-unreadable = the cache of stage { $stage } is unreadable ({ $error }); recomputing
analyze-checkpoint-not-saved = the checkpoint of stage { $stage } was not saved: { $error }
analyze-diarize-continuous = diarization: { $speakers } { $speakers ->
    [one] speaker
   *[other] speakers
} expected, one continuous pass over the whole recording without resetting labels at hour boundaries
analyze-diarize-fragments = diarization: { $speakers } { $speakers ->
    [one] speaker
   *[other] speakers
} expected, matching voices across fragments
analyze-wespeaker-needed = WeSpeaker is needed for a set number of speakers: { $error }
analyze-align-skipped = aligning to speech skipped: the subtitles are in the target language and the speech in the original one
analyze-align-recognizing = aligning the subtitles to speech: recognizing the words
analyze-align-recognition-failed = aligning the subtitles: speech recognition: { $error }
analyze-align-no-speech = aligning to speech: no speech was recognized; the file’s timings are kept
analyze-align-mismatch = the subtitles did not match the speech ({ $share }% of the lines matched); the file’s timings are kept
analyze-align-done = the subtitles are aligned to speech: { $share }% of the lines by words, the rest shifted with their neighbours; file offset { $offset } s
analyze-more = { $count } more
analyze-window-plan = a long track of { $duration }s: a plan of { $windows } { $windows ->
    [one] window
   *[other] windows
} (the first ~{ $first }s); windowed ASR is not active yet, processing in one piece
analyze-audio-cached = audio from the cache (the source did not change); skipping ffmpeg
analyze-extracting-audio = extracting the audio (ffmpeg -> 16k mono)
analyze-stems-stale = the stems were made from an earlier extraction; separating again
analyze-separating = separating the vocals ({ $model }): a clean voice for diarization/ASR
analyze-separation-cached = separation from the cache (the stems are already made)
analyze-separation-failed = separation failed ({ $error }); diarization/ASR on the raw audio
analyze-separator-missing = { $model } was not found; diarization/ASR on the raw audio
analyze-diarizing = diarization ({ $model })
analyze-diarization-cached = diarization from the cache
analyze-diarization-count-failed = diarization with a set number of speakers: { $error }
analyze-diarization-failed = diarization failed ({ $error }); going on with a single speaker
analyze-diarization-model-missing-count = the { $model } model was not found: the set number of speakers cannot be applied
analyze-subs-no-diarization = subtitles: no diarization (the whole clip as one piece)
analyze-diarization-model-missing = the diarization model was not found; going on with a single speaker
analyze-fewer-speakers = { $expected } { $expected ->
    [one] speaker was
   *[other] speakers were
} expected, { $found } told apart: the missing voices were not added
analyze-speakers-matched = voices matched across fragments: { $speakers } { $speakers ->
    [one] speaker
   *[other] speakers
}
analyze-transcript-cached = transcript from the cache: { $segments } { $segments ->
    [one] segment
   *[other] segments
}
analyze-read-subs = reading the subtitles { $path }: { $error }
analyze-subs-empty = the subtitles are unrecognized or empty: { $path }
analyze-subs-imported = subtitles imported: { $lines } { $lines ->
    [one] line
   *[other] lines
}, { $speakers } { $speakers ->
    [one] speaker
   *[other] speakers
}
analyze-cloud-asr = transcribing in the cloud (OpenRouter STT)
analyze-cloud-stt-failed = cloud STT: { $error }
analyze-cloud-done = cloud: { $lines } { $lines ->
    [one] line
   *[other] lines
}, { $speakers } { $speakers ->
    [one] speaker
   *[other] speakers
}
analyze-hallucination-filter = ASR hallucination filter: { $error }
analyze-asr-language = Parakeet does not transcribe "{ $lang }": it knows 25 European languages. Choose CrisperWhisper in Settings → Models (99 languages); picking the source language in the window switches it for you.
analyze-asr-no-words = The video has { $speech } s of speech and Parakeet heard { $words } words in it: Parakeet knows only 25 European languages. Pick the source language in the window instead of Auto — for a language outside Europe the studio switches recognition to CrisperWhisper.
analyze-hidden-hallucinations = ASR hallucination lines hidden (no voice): { $count }: { $lines }
analyze-hidden-by-text = ASR titles and sounds hidden by their text (there is sound in the interval, the voice is not separated from the music): { $count }: { $lines }
analyze-voiced-suspects = look like hallucinations, but there is a voice, so they are kept with a mark: { $count }: { $lines }
analyze-merged-fragments = merging fragments: { $before } -> { $after } segments
analyze-characters-by-voice = characters by voice: { $count }
analyze-segments-speakers = { $segments } { $segments ->
    [one] segment
   *[other] segments
}, { $speakers } { $speakers ->
    [one] speaker
   *[other] speakers
}
analyze-project-unparsable = project.json cannot be parsed ({ $error }): it holds the project glossary, so the analysis stopped to keep it
analyze-glossary-fixed = glossary: { $count } term recognition { $count ->
    [one] error
   *[other] errors
} fixed
analyze-no-speech-nodub = no speech segments; keeping the original track (nodub)
analyze-auto-nodub = auto: no speech worth dubbing -> NODUB (the original + localized on-screen text)
analyze-casting-style = character descriptions from the casting profile -> translation style ({ $chars } chars)
analyze-empty-removed = segments without words removed: { $count }
analyze-translation-cached = translation from the cache
analyze-ocr-cached = on-screen text detection from the cache
analyze-audio-no-ocr = audio mode: no video, so no on-screen text detection is needed
analyze-ocr-off = burned-in text detection is off (the checkbox)
analyze-content-type = content type (auto): { $kind }
analyze-casting-cached = casting from the cache
analyze-casting-checkpoint = the casting checkpoint was not saved: { $error }
analyze-profile-voices-missing = the profile voices were not found in voices/ -> clone: { $voices }
analyze-profile-voices-applied = the carried-over profile voices are applied to the dub ({ $count } { $count ->
    [one] character
   *[other] characters
})
analyze-audio-no-casting = audio mode: no video, so no character casting is needed
analyze-cache-not-saved = cache.json could not be saved: { $error } (not critical)

## Casting, recording, voice library

casting-relabel = relabelling by voice: { $count } { $count ->
    [one] character
   *[other] characters
} by voice (diarization found { $found })
casting-skipped-no-speech = casting skipped: no speech segments
casting-skipped-no-speakers = casting skipped: no speakers
casting-speakers-ranked = speaking characters: { $count } (ranked by speaking time)
casting-faces-bound = faces bound to speakers: { $count } of { $total }
casting-bit-part-skipped = { $character }: speaker { $speaker } has 1 line and neither a face nor a voice -> skipped (a bit part not worth casting)
casting-character-face = { $character }: speaker { $speaker }, { $lines } { $lines ->
    [one] line
   *[other] lines
}, { $seconds }s of speech, face: yes
casting-character-no-face = { $character }: speaker { $speaker }, { $lines } { $lines ->
    [one] line
   *[other] lines
}, { $seconds }s of speech, face: no
casting-profile-other-type = the profile is of another type ({ $previous } ≠ { $current }); cross-matching skipped
casting-cross-episode = across episodes: names/voices carried over: { $count }
casting-saved = casting.json is ready: { $count } { $count ->
    [one] character
   *[other] characters
}
casting-save-failed = casting.json could not be written: { $error }
casting-faces-collected = faces collected: { $faces } ({ $samples } samples)
casting-face-clusters = face identities (clusters by vector): { $count }
casting-anime-detector-missing = the anime detector was not found ({ $path }); no avatars
casting-anime-detector-failed = the anime detector did not load: { $error }; no avatars
casting-scrfd-missing = SCRFD was not found; no avatars (casting by voice)
casting-scrfd-failed = SCRFD did not load: { $error }; no avatars
casting-embedder-missing = { $model } was not found ({ $path }); avatars without an embedding
casting-embedder-failed = { $model } did not load: { $error }; avatars without an embedding
casting-no-frame = ffmpeg did not extract the frame
casting-voice-no-vocals = voice embedding skipped: no clean vocals
casting-voice-no-model = voice skipped: no WeSpeaker model ({ $path })
casting-voice-model-failed = WeSpeaker did not load: { $error }; voice skipped
casting-voice-trim-failed = voice sample { $speaker }: trimming failed: { $error }
casting-voice-embedding-failed = voice { $speaker }: embedding failed: { $error }
casting-voice-embeddings = voice embeddings: { $count }
casting-applying-profile = applying the library profile: { $slug }
casting-library-profile-missing = the library profile “{ $slug }” was not found; nothing applied
record-busy = Recording is already running
record-mic-not-found = Microphone “{ $name }” was not found: it is unplugged or renamed; choose another one
record-no-mic = No microphone found
record-mic-config = microphone configuration: { $error }
record-create-wav = creating the wav: { $error }
record-format-unsupported = format { $format } is not supported
record-mic-open = opening the microphone: { $error }
record-mic-start = starting the microphone: { $error }
record-not-recording = Nothing is being recorded
record-silent = The microphone recorded only silence, so the voice was not saved: check that the system lets programs use the microphone (Windows: Settings → Privacy & security → Microphone), that it is not muted, and that the right microphone is chosen.
voices-not-file-list = { $page }: not a file list: { $error }
voices-list-endless = { $page }: the dataset file list does not end
voices-too-large = { $url }: more than the pinned { $size } bytes
voices-size-mismatch = { $url }: { $got } bytes arrived, { $size } are pinned
voices-sha-mismatch = { $url }: SHA-256 { $got } does not match the pinned { $want }
voices-bad-name = a bad name
voices-not-in-dataset = voice { $name } is not in the dataset { $dataset }
voices-bad-size = { $name }: the size in the catalogue is not valid
voices-no-sha = { $name }: no SHA-256 in the catalogue
voices-pack-downloading = downloading the voice pack
voices-pack-unpacking = unpacking
voices-open-zip = opening the zip: { $error }
voices-create-file = creating { $name }: { $error }
voices-unpack-file = unpacking { $name }: { $error }
voices-pack-done = done: { $count } { $count ->
    [one] file
   *[other] files
}
voices-pack-taken = taken from { $app }: { $count } { $count ->
    [one] file
   *[other] files
}

## Downloads by link

url-no-fetch = there is no download { $id }
url-stopped = stopped
url-already-downloading = this link is already downloading: { $id }
url-probe-not-json = the yt-dlp -J answer is not JSON: { $error }; { $stderr }
url-interrupted = the download stopped when the studio closed; what was fetched is in its folder and resumes from there
url-not-a-link = “{ $url }” is not a link: { $error }
url-not-http = “{ $url }” is not an http(s) link
url-bad-subs-lang = “{ $lang }” is not a subtitle language code
url-cookies-not-file = { $path } is not a cookies.txt file of up to 1 MB
url-cookies-empty-or-large = cookies.txt is empty or larger than 1 MB
url-thread-failed = the download thread did not start: { $error }
url-cookies-gone = this download’s cookies.txt is gone from its folder; start the download again with cookies
url-disk-space = about { $need } MB is needed, { $free } MB is free
url-no-subs = the video has no human-made “{ $lang }” subtitles (there are: { $have })
url-no-subs-none = the video has no human-made “{ $lang }” subtitles (there are none at all)
url-no-media-file = yt-dlp finished, but there is no video file in { $path }: { $stderr }
url-subs-failed = the “{ $lang }” subtitles did not download ({ $code }): { $error }
url-ytdlp-start = starting yt-dlp: { $error }
url-ytdlp-wait = waiting for yt-dlp: { $error }
url-still-downloading = { $id } is still downloading
url-cancelled-removed = { $id } was cancelled and what was fetched is removed: start a new download
url-still-stopping = { $id } is still stopping; try again in a couple of seconds
url-cancel-first = { $id } is still downloading; cancel it first
url-no-subs-file = yt-dlp finished without a subtitle file in { $path }: { $stderr }
url-no-parent = { $path } has no parent folder
url-subs-empty = the subtitles { $path } have no lines at all
url-ytdlp-missing = the ytdlp component is not downloaded
url-bad-quality = quality “{ $quality }”: best, 1080, 720, 480 or audio

## yt-dlp

ytdlp-update-missing = the yt-dlp update { $version } ({ $path }) is missing; the pinned { $pinned } runs
ytdlp-http-client = HTTP client: { $error }
ytdlp-tag-not-version = the yt-dlp release tagged “{ $tag }” is not a version
ytdlp-no-checksum = { $url } has no { $file }
ytdlp-new-says = the new yt-dlp { $tag } calls itself “{ $said }”; { $current } stays
ytdlp-new-failed = the new yt-dlp { $tag } did not start: { $error }; { $current } stays
ytdlp-too-large = { $url }: more than { $limit } bytes
ytdlp-sha-mismatch = { $url }: SHA-256 { $got } does not match SHA2-256SUMS { $want }
ytdlp-exit-code = exit code { $code }: { $stderr }
ytdlp-component-missing = the ytdlp component (yt-dlp and deno) is not downloaded
ytdlp-no-ffmpeg = no ffmpeg: neither the ffmpeg component nor ffmpeg.exe in PATH
ytdlp-start = starting { $program }: { $error }
ytdlp-timeout = yt-dlp did not answer in { $seconds } s: { $stderr }
ytdlp-playlist = the link is a playlist or a channel ({ $count } { $count ->
    [one] video
   *[other] videos
}), not a single video
ytdlp-redirect-only = the link has no video itself, only a link onward
ytdlp-live = a live stream ({ $status })
ytdlp-thumbnail-not-image = the thumbnail is not an image ({ $mime })
ytdlp-thumbnail-too-large = the thumbnail is larger than 4 MB
ytdlp-failed-silently = yt-dlp failed without a message

## Render: voicing, QC, mix, mux

render-input = input { $width }x{ $height } dur={ $duration }s
render-nodub-original = nodub: the original audio track
render-done-audio = done (audio only) -> { $path }
render-building-ass = building the ASS (titles + dubbed subtitles)
render-burning = burning in the subtitles + blur (ffmpeg + libass, NVENC)
render-burn-off = subtitles/titles are off (subs.burn=off)
render-encode-cpu-mode = the studio runs on the processor, so the video is encoded on the processor (x264), slower than on the graphics card
render-encode-cpu-driver = NVIDIA driver { $have } is older than { $need }, which encoding on the graphics card needs, so the video is encoded on the processor (x264), slower. Update the driver from NVIDIA's site to encode on the card
render-muxing = muxing video + audio
render-track-dub = { $lang } (dub)
render-track-original = { $lang } (original)
render-two-tracks = two tracks: { $dub } + { $original } -> { $container }
render-multitrack-failed = the multi-track mux failed ({ $error }) -> one track
render-subtitle-tracks = subtitles as mkv tracks: { $tracks }
render-subtitle-tracks-failed = subtitles as mkv tracks: { $error }
render-mp4-companion-failed = the mp4 companion was not built ({ $error }); the player opens the mkv (VLC works)
render-done = done -> { $path }
render-dub-audio-done = the dub audio is ready
render-synth-thread-ended = the synthesis thread ended without a result
render-synth-timeout = { $engine }: synthesis timed out after >{ $seconds }s; cancelled, the engine is free
render-engine-stuck = { $engine }: synthesis does not cancel after >{ $seconds }s; the render is stopped (the engine hung in the DLL)
render-higgs-gpu-old = The local Higgs voice cannot run on { $card }: it needs an NVIDIA card from Turing on (compute capability { $min }, GTX 16 and RTX 20 or newer). Choose the "Google · direct" or "OpenRouter" voice.
render-higgs-driver-old = The local Higgs voice needs NVIDIA driver { $min } or newer, and this one is older: { $card }. Update the driver or choose the "Google · direct" or "OpenRouter" voice.
render-higgs-no-gpu = The local Higgs voice needs an NVIDIA card with CUDA, and none was found ({ $card }). Choose the "Google · direct" or "OpenRouter" voice.
render-higgs-model-missing = Higgs Audio v3 is not downloaded ({ $path }): download it in Settings → Models → Voicing.
render-defect-runaway = runaway
render-defect-cutoff = cut off
render-defect-silence = silence
render-defect-hum = hum
render-recognition-no-answer = recognition returned no answer
render-second-pass-overflow = the second voicing pass asked for shortening, which is off in it
render-no-translated-lines = no translated lines -> silence, the original track
render-takes-of-removed-lines = take histories of removed lines cleared: { $count }
render-extracting-audio = extracting the audio (ffmpeg 44.1k stereo)
render-separator-missing = the separation engine was not found -> no background (keep_music off)
render-ref-from-mix = the clone reference comes from the unseparated mix: the original background sounds in it too
render-emotion-ref-failed = segment { $segment }: the emotion reference was not cut ({ $error }); using the speaker’s identity reference
render-cloud-voices = cloud voices by speaker: { $voices }
render-synth-keys-reset = { $error }; the synthesis keys start over
render-synthesizing = synthesizing { $count } of { $total } { $total ->
    [one] segment
   *[other] segments
}
render-voicing-cached = voicing from the cache: { $count } { $count ->
    [one] segment
   *[other] segments
}
render-cloud-tts-parallel = cloud TTS: { $count } { $count ->
    [one] segment
   *[other] segments
} in { $threads } parallel threads
render-cloud-tts-ready = cloud TTS: pre-synthesis ready ({ $count } { $count ->
    [one] segment
   *[other] segments
})
render-takes-quarantined = { $error }; the take history of line { $line } is set aside in { $path } and started over
render-pinned-take = line { $line }: the pinned take plays; new voicing does not replace it
render-take-unpinned = line { $line }: the take is unpinned because the line’s text changed
render-selected-take = line { $line }: the chosen take { $take } plays; new voicing does not replace it
render-write-cloud-segment = writing the cloud seg{ $line }: { $error }
render-cloud-tts-failed = ⚠ segment { $line }: cloud TTS failed ({ $error }); the original is kept
render-loading-higgs = loading Higgs
render-failures-kept-generated = ⚠ segment { $line }: { $attempts } synthesis failures ({ $error }); the generated voicing is used (range { $range } dB)
render-failures-kept-original = ⚠ segment { $line }: { $attempts } synthesis failures/timeouts ({ $error }); the original line is kept
render-regenerating = segment { $line }: { $error }; regenerating ({ $attempt }/{ $attempts })
render-defects-kept-generated = ⚠ segment { $line }: all { $attempts } attempts have a defect ({ $defect }); the generated voicing is used (range { $range } dB)
render-silent-kept-original = ⚠ segment { $line }: { $attempts } attempts without sound; the original is used
render-retry-alt-ref = another reference
render-retry-temperature = higher temperature
render-defect-regenerating = segment { $line }: a synthesis defect ({ $defect }); regenerating ({ $via } { $attempt }/{ $attempts })
render-write-segment = writing seg{ $line }: { $error }
render-too-many-artifacts = TTS: too many hum artifacts ({ $in_a_row } in a row, { $retries } retries in total); regenerating does not help. The problem is likely the setup (model/VRAM) or the voice reference clips. Stopped at segment { $line }.
render-multi-take = segment { $line }: multi-take; the take closest to the slot is chosen ({ $deviation }s off)
render-stretch-over-cap = segment { $line }: needs a stretch of x{ $needed } (slot { $slot }s), cap x{ $cap }; the text is faster than normal
render-lead-into-silence = starting in the silence: { $lines } { $lines ->
    [one] line starts
   *[other] lines start
} earlier, { $seconds } s in all, instead of being sped up
render-silence-trimmed = trimming TTS silence: { $seconds } s cut from { $lines } { $lines ->
    [one] line
   *[other] lines
} ({ $pauses } s of it pauses); trimming brought the speed-up within the cap for { $into_cap } { $into_cap ->
    [one] line
   *[other] lines
}
render-fit-summary = fitting: { $over }/{ $total } segments above the cap ({ $share }%)
render-fit-summary-drift = fitting: { $over }/{ $total } segments above the cap ({ $share }%), sync caught up on { $drift }
render-qc-start = QC: checking { $count } { $count ->
    [one] line
   *[other] lines
} by transcription
render-qc-unheard = QC: { $count } of { $total } lines were not checked; recognition failed: { $reason }
render-qc-mismatch = QC: { $count } { $count ->
    [one] line does
   *[other] lines do
} not match the translation; resynthesizing
render-qc-resynthesized = QC: segment { $line } resynthesized (attempt { $attempt })
render-qc-unconfirmed = ⚠ QC: segment { $line } (“{ $text }”) could not be confirmed; check the line by hand
render-qc-kept-mismatch = ⚠ QC: segment { $line } does not match the translated text; the generated voicing is kept
render-qc-resynth-unheard = QC: { $count } resynthesized { $count ->
    [one] line was
   *[other] lines were
} not checked; recognition failed: { $reason }
render-qc-summary = QC result: { $fixed }/{ $total } fixed, { $flagged } still flagged, { $unheard } not checked
render-qc-all-confirmed = QC: every line is confirmed by transcription ✓
render-qc-rest-confirmed = QC: the other lines are confirmed by transcription
render-laying-out = laying the dub out on the timeline
render-peak-limiter = peak limiter: { $lines } { $lines ->
    [one] line
   *[other] lines
}, { $samples } samples above the { $ceiling } ceiling brought down without clipping
render-tempo-fit = tempo fit of the whole track x{ $factor }
render-voiceover-envelope = voiceover: the original at { $db } dB UNDER the translation, full in the pauses (dynamic envelope, { $blocks } blocks)
render-voiceover-flat = voiceover: the envelope is unavailable -> flat attenuation
render-mix-no-ducking = mixing: instrumental + dub vocals (ducking OFF, full background)
render-mix-ducking = mixing: instrumental + dub vocals (ducking ON, envelope, { $blocks } blocks)
render-mix-sidechain = the envelope is unavailable -> sidechain ducking
render-mix-plain = sidechain is unavailable -> a plain mix
render-loudness-off = loudness evening is off: the mix as it is
render-loudness-normalizing = normalizing loudness (EBU R128, true peak)
render-loudnorm-skipped = loudnorm skipped ({ $error })
render-track-gain = track gain { $db } dB
render-dub-timings-not-written = dub timings for the subtitles were not written: the layout ({ $lines } lines, { $spans } spans) did not match the segments ({ $segments }); the subtitles follow the original timings
render-dub-timing-mismatch = dub timings: line { $line } of the synthesis view does not match project segment #{ $index }
render-word-timings = word timings of the subtitles: recognizing { $count } dub { $count ->
    [one] line
   *[other] lines
}
render-refs-unchecked = checking references: { $count } { $count ->
    [one] candidate is
   *[other] candidates are
} accepted unchecked; recognition failed: { $error }
render-refs-all-failed = ⚠ speaker { $speaker }: no reference candidate passed the check (heard: “{ $heard }”); taking the best by score
render-speaker-ref-failed = speaker { $speaker } reference: { $error }
render-speaker-ref = speaker { $speaker } reference: “{ $text }” ({ $seconds }s, { $candidates } { $candidates ->
    [one] candidate
   *[other] candidates
}, check ok)
render-speaker-ref-unchecked = speaker { $speaker } reference: “{ $text }” ({ $seconds }s, { $candidates } { $candidates ->
    [one] candidate
   *[other] candidates
}, check ⚠ failed)

## Setup: components

setup-comp-higgs-purpose = Voices a text, clones a voice from a short sample and carries emotion
setup-comp-higgs-engine-name = Voice and recognition engine (audio.cpp)
setup-comp-higgs-engine-purpose = The program that runs Higgs voicing and CrisperWhisper and Nemotron 3.5 recognition
setup-comp-audiocpp-build-name = audio.cpp engine: { $build } build
setup-comp-audiocpp-build-purpose = The engine built for another device: downloaded when voicing or recognition is set to it in Settings → Models
setup-comp-gemma-purpose = The director: writes a podcast on a topic and casts voices to a book’s characters
setup-comp-gemma-q5-0-purpose = The script assistant, more accurate than q4_0
setup-comp-gemma-q6-k-purpose = The script assistant, more accurate still
setup-comp-gemma-q8-0-purpose = The script assistant, the most accurate and the heaviest
setup-comp-parakeet-purpose = Recognises speech with the time of every word: checking takes by ear and transcribing recordings
setup-comp-higgs-q6-k-purpose = Voices — a lighter variant than Q8_0
setup-comp-higgs-q4-k-m-purpose = Voices — the lightest variant
setup-comp-sortformer-name = Who is speaking (up to 8 voices)
setup-comp-sortformer-purpose = Tells apart who speaks in a recording
setup-comp-roformer-purpose = Separates the voice from music: cleans a voice sample and a recording before recognition
setup-comp-roformer-q5-purpose = Voice separation — a lighter variant than Q8_0
setup-comp-roformer-q4-purpose = Voice separation — the lightest variant
setup-comp-wespeaker-name = Voice similarity (WeSpeaker)
setup-comp-wespeaker-purpose = Measures how close a voicing sounds to its reference voice, for the voicing check
setup-comp-voice-gate-name = Voice gate (ReDimNet2-B3)
setup-comp-voice-gate-purpose = Recognises your own voice so a conversation in Live starts only when you speak
setup-comp-wake-word-name = Wake word (PhoneticXeus)
setup-comp-wake-word-purpose = Universal phone recogniser so Live wakes on a phrase you set, in any language
setup-comp-ruaccent-purpose = Puts stress marks and the letter ё into Russian text and resolves homographs by context
setup-comp-ruaccent-omograph-big-name = Large RUAccent homograph model
setup-comp-ruaccent-omograph-big-purpose = RUAccent's big_poetry model in int8 for reading homographs by meaning (замо́к/за́мок, мука́/му́ка): more right answers than the built-in tiny2 (29 of 33 against 24 on the test sentences); once downloaded it is used instead
setup-comp-silero-vad-name = Silero VAD v6.2
setup-comp-silero-vad-purpose = Finds where speech is in a recording, so a long one is cut into pieces at the pauses for recognition, not mid-word (1.2 MB)
setup-comp-langid-purpose = Tells the language of a recording set to Auto, so a model that knows it recognises it (107 languages)
setup-comp-dpdfnet-name = DPDFNet noise suppression
setup-comp-dpdfnet-purpose = Removes noise from voice references and recordings, with a strength control so speech does not sound sterile (10.5 MB)
denoise-missing = The DPDFNet noise suppression model is not downloaded: download it in Settings → Models.
setup-comp-bsroformer-engine-name = Voice separation engine (NVIDIA)
setup-comp-bsroformer-engine-purpose = The program that separates the voice on the graphics card
setup-comp-bsroformer-engine-cpu-name = Voice separation engine (processor)
setup-comp-bsroformer-engine-cpu-purpose = For a computer without NVIDIA: runs on the processor, slower
setup-comp-llama-name = Assistant server (llama.cpp)
setup-comp-llama-purpose = The program that runs Gemma for the script assistant
setup-llama-vulkan-glibc = The assistant runs on the card through Vulkan: the CUDA build of llama.cpp needs glibc { $need } or newer, this system has { $glibc }
setup-comp-onnxruntime-purpose = Needed for recognising speech and speakers
setup-comp-onnxruntime-gpu-purpose = So that speech and speaker recognition run on the graphics card, not the processor
setup-comp-ffmpeg-purpose = Reads and writes audio and video
setup-comp-ytdlp-name = Download by link (yt-dlp + deno)
setup-comp-ytdlp-purpose = Fetch a recording by link (YouTube and the other yt-dlp sites)
setup-comp-cuda-runtime-purpose = NVIDIA libraries the engines use on the graphics card; no separate CUDA install needed
setup-comp-cudnn-purpose = Needed for speech and speaker recognition on the graphics card
setup-comp-vcruntime-purpose = Windows system libraries for the engines (included)
setup-comp-nvidia-driver-name = NVIDIA driver
setup-comp-nvidia-driver-purpose = Needed to work on the graphics card; installed from NVIDIA's site, not by the app

## Setup: downloads and installation

setup-http-status = { $url }: status { $status }
setup-write = writing: { $error }
setup-not-zip = not a zip: { $error }
setup-zip-entry = zip entry: { $error }
setup-open = opening { $path }: { $error }
setup-verifying = Checking the SHA-256 of { $file }…
setup-install-record = the install record: { $error }
setup-disk-space = not enough space: { $need } GB needed, { $free } GB free ({ $path })
setup-unknown-component = there is no component { $id }
setup-not-removable = { $id } is not installed by the app
setup-component-busy = the component is downloading now; pause the download
setup-paused = the download is paused
setup-rate-limited = { $url }: the server has answered { $status } for { $minutes } min
setup-proxy-scheme = proxy { $proxy }: the scheme { $scheme } does not suit downloads (http, https, socks4, socks5)
setup-proxy-no-host = proxy { $proxy }: no host
setup-proxy-no-port = proxy { $proxy }: no port
setup-proxy-credentials = proxy { $proxy }: model downloads (ureq) cannot pass such a login or password to the proxy: it has / ? #, a space, non-ASCII or (for SOCKS5) a colon in the password; cloud requests through this proxy work, downloads need a password without these characters
setup-start-failed = { $url }: could not start in { $retries } attempts: { $error }
setup-chunk-manifest = the chunk manifest: { $error }
setup-range-incomplete = an incomplete range: { $got }/{ $want } bytes
setup-range-failed = range { $start }-{ $end } after { $retries } attempts: { $error }
setup-range-status = range { $start }-{ $end }: status { $status } (206 expected)
setup-range-read = reading the range: { $error }
setup-create = creating { $path }: { $error }
setup-read = reading: { $error }
setup-download-failed = { $url } after { $retries } attempts: { $error }
setup-size-mismatch = { $file }: { $got } bytes downloaded, { $want } pinned; the file is removed and the next attempt starts over
setup-hash-mismatch = { $file }: SHA-256 { $got } does not match the pinned { $want }; the file is removed and the next attempt starts over
setup-unpacking = Unpacking { $file }…
setup-rename = renaming { $path }: { $error }
setup-waiting-other = Waiting for another download of the same components…
setup-delete = deleting { $path }: { $error }
setup-downloading = Downloading the models…
setup-taken-from = { $file } taken from { $app } instead of downloading
setup-source-changed = { $url }: the server gives { $got } bytes, { $want } are pinned; the source changed
setup-manifest-write = manifest { $path }: { $error }
setup-archive-no-files = the archive { $path } has none of the needed files
setup-wheel-not-zip = the wheel is not a zip: { $error }
setup-wheel-entry = wheel entry: { $error }
setup-archive-no-dll = the archive { $path } has no libraries
setup-unpack = unpacking { $path }: { $error }
setup-finalize = finishing { $path }: { $error }

## Settings applied after analysis

post-analyze-bad-vo-gain = vo_gain: a number of dB was expected, { $value } came
post-analyze-bad-flag = { $name }: 0 or 1 was expected, { $value } came
post-analyze-bad-container = container: mp4 or mkv was expected, { $value } came
post-analyze-bad-voice-slots = voice_slots: an object {"{"}male:[…], female:[…]{"}"} was expected
post-analyze-edit-failed = the setting after analysis { $edit }: { $error }

## Messages of the engines and libraries: glossary, LLM, ASR, translation, separation, faces, TTS, captions, OCR

glossary-over-limit = the glossary has { $total } entries, more than { $max }
glossary-empty-term = entry { $entry } has no term
glossary-field-too-long = entry { $entry } (“{ $term }”): a field is longer than { $max } characters
glossary-duplicate = the term “{ $term }” is listed twice (entries { $first } and { $second })
glossary-tsv-keep = line { $line }: keep is “{ $value }”, 1 or 0 was expected
glossary-tsv-empty-term = line { $line } has no term
glossary-one-of = send either entries or TSV, not both
glossary-nothing = nothing to save: no entries and no TSV
llm-spawn-failed = llama-server did not start: { $error }
llm-gguf-missing = the GGUF model was not found ({ $path })
llm-log-file = llama-server log { $path }: { $error }
llm-exited-early = llama-server exited before it was ready ({ $status }); stderr: { $stderr }
llm-not-ready = llama-server did not come up in { $secs } s (port { $port }); stderr: { $stderr }
llm-no-gpu-device = the local Gemma needs a graphics card, and the llama.cpp build sees none ({ $detail }). Choose a local server, OpenRouter or the agent for the script assistant in Settings → Models, or check the graphics driver.
llm-http = the request to the model failed: { $error }
llm-api = the model API: { $error }
llm-rejected = the model API refused the request ({ $status }): { $body }
llm-empty-answer = the model { $model } returned an empty answer (no reason given)
llm-empty-answer-reason = the model { $model } returned an empty answer (finish_reason={ $reason })
llm-cut-short = the model { $model } hit the limit of { $max_tokens } { $max_tokens ->
    [one] token
   *[other] tokens
} (finish_reason=length): the answer is incomplete; it probably spends the budget on reasoning — choose a model without mandatory thinking
llm-prompt-cut = the server read only { $read } { $read ->
    [one] token
   *[other] tokens
} of a request of { $chars } { $chars ->
    [one] character
   *[other] characters
} and dropped its start — its context is too small: raise num_ctx in Ollama or the model's Context Length in LM Studio
asr-engine = speech recognition: { $error }
asr-wav-read = could not read the wav { $path }: { $error }
asr-resample = resampling: { $error }
asr-speaker-count = the number of speakers must be from 1 to { $max }
speakers-embedding = voice of speaker { $speaker }: { $error }
speakers-no-sample = speaker { $speaker }: no speech piece of at least 0.3 seconds to match the voice
speakers-too-many-voices = { $given } { $given ->
    [one] speaker
   *[other] speakers
} given, but the piece has { $found } different { $found ->
    [one] voice
   *[other] voices
}: they could not be merged safely by voice
speakers-bad-embedding = the voice model returned empty or invalid voice features
speakers-dimension-changed = the size of the voice features changed
speakers-more-voices = the piece has more voices than the given number of speakers
speakers-unmatched = the voices of the piece could not be matched reliably with the given number of speakers
translate-format-json = translation: the model answers with a JSON object by schema ({ $model })
translate-format-json-probe = translation: trying answers by JSON schema ({ $model }); if refused, numbered lines
translate-format-numbered = translation: numbered lines; the model { $model } does not declare structured_outputs in the OpenRouter catalogue
translate-schema-ignored = translation: { $model } accepted the JSON schema but answered with numbered lines; numbered lines from now on
translate-schema-ignored-server = translation: the server accepted the JSON schema but answered with numbered lines; numbered lines from now on
translate-schema-refused = translation: { $model } refused answers by JSON schema ({ $status }: { $body }); numbered lines from now on
translate-schema-refused-server = translation: the server refused answers by JSON schema ({ $status }: { $body }); numbered lines from now on
translate-line-flawed = translation: line { $line } keeps a translation with a remark: { $reason }
translate-line-failed = translation: line { $line } is not translated: { $reason }
translate-line-reason = { $line }: { $reason }
translate-lines-rejected = translation: { $bad } of { $total } { $total ->
    [one] line
   *[other] lines
} failed the check ({ $reasons })
translate-batch-stopped = translation: the batch of lines { $first }..{ $last } failed ({ $error }); the translation stopped
translate-batch-failed = translation: the batch of lines { $first }..{ $last } failed ({ $error })
translate-layout-no-vision = frame layout: skipped (no vision model is chosen or available)
translate-layout-not-needed = frame layout: skipped (subtitles are not burned in, no layout is needed)
translate-layout = frame layout: sub_style={ $sub_style } titles={ $titles } brands={ $brands }
translate-layout-failed = frame layout: skipped ({ $error })
translate-scene-failed = scene context: skipped ({ $error })
translate-scene-no-vision = scene context: skipped (no vision model is chosen or available)
translate-audio-failed = audio context: skipped ({ $error })
translate-context-trimmed = translation: the context block of { $chars } { $chars ->
    [one] character
   *[other] characters
} is cut to { $budget } (n_ctx guard)
translate-names-skipped = translation: the automatic name glossary is skipped ({ $error })
translate-chunks = translation: { $lines } { $lines ->
    [one] line
   *[other] lines
} -> { $chunks } { $chunks ->
    [one] chunk
   *[other] chunks
} (glossary: { $terms } { $terms ->
    [one] term
   *[other] terms
}, { $names } { $names ->
    [one] name
   *[other] names
})
translate-pass-done = translation: done, { $translated } translated ({ $flawed } with a remark), { $untranslated } left in the source language
glossary-pass = glossary: model pass { $pass }/{ $passes }
glossary-schema-refused = glossary: the server refused answers by JSON schema ({ $status }); asking for JSON as text
content-type-decided = content type: { $decided ->
    [anime] animation
   *[other] live action
} ({ $votes } { $votes ->
    [one] clear answer
   *[other] clear answers
} of { $frames } frames)
line-missing = missing from the answer
line-cut = the answer was cut by the token limit
line-untranslated = not in the target language
line-echo = repeats the source
line-too-short = too short ({ $got } < { $min })
line-too-long = too long ({ $got } > { $max })
line-loop = loops on “{ $gram }”
line-term-missing = the glossary term “{ $term }” is missing
line-gender = the speaker speaks of themselves as the other gender: “{ $form }”
line-latin = the source word “{ $word }” is left untranslated
translate-frame = extracting a video frame: { $error }
translate-audio = audio context: { $error }
translate-empty = the model translated none of { $lines } { $lines ->
    [one] line
   *[other] lines
}; the last cause: { $reason }
translate-empty-no-reason = the model translated none of { $lines } { $lines ->
    [one] line
   *[other] lines
}
translate-contract = the model's answer: { $problem }
translate-contract-answer = the model's answer: { $problem }; the answer: { $answer }
answer-no-json-object = it has no JSON object
answer-not-json = it is not valid JSON ({ $error })
answer-no-terms = it has no terms list
remix-failed = remix: { $error }
sep-engine-missing = the separation engine was not found ({ $path })
sep-model-missing = the separation model was not found ({ $path })
sep-spawn = starting the separation engine: { $error }
sep-engine-failed = the separation engine failed (exit code { $code }): { $tail }
sep-fell-back-to-cpu = the voice separation build for the graphics card found no card and ran on the processor ({ $why }). Choose «CPU» or «Auto» for voice cleaning in Settings → Models, or check the graphics driver.
sep-engine-killed = the separation engine was stopped without an exit code: { $tail }
sep-no-output = the separation engine made no vocal stem ({ $path })
sep-audio-io = separation audio: { $error }
faces-model-missing = the model was not found ({ $path })
ffmpeg-exit = ffmpeg exit code { $code }: { $tail }
ffmpeg-killed = ffmpeg was stopped without an exit code: { $tail }
onnx-runtime = the ONNX runtime: { $error }
onnx-runtime-load = the ONNX runtime { $file } did not load ({ $error }). Most often an old Microsoft Visual C++ runtime keeps it from starting: install the current Visual C++ 2015–2022 Redistributable (x64) from Microsoft and restart the studio
files-error = files: { $error }
model-no-outputs = the model returned no outputs
faces-output-shape = { $model }: the model output has an unexpected shape { $shape }
faces-output-count = { $model }: { $expected } model outputs were expected, { $got } came
faces-crop-size = the face embedding expects a 112x112 aligned crop, got { $width }x{ $height }
faces-sample-rate = the voice clip must be { $expected } Hz, it is { $got } Hz
faces-clip-too-short = the voice clip is too short to analyse
captions-write-ass = writing the subtitle file: { $error }
captions-filter-script = writing the ffmpeg filter script: { $error }
captions-ffmpeg-wait = waiting for ffmpeg: { $error }
captions-ffmpeg-timeout = ffmpeg did not finish in { $secs } s and was stopped (it hung): { $tail }
captions-burn-failed = burning the subtitles in failed: { $tail }
captions-nvenc-driver-old = the NVIDIA driver is too old for video encoding on the graphics card (NVENC): version { $need } or newer is needed. Update the driver from NVIDIA's site and render again
captions-frame-failed = the preview frame failed: { $tail }
ocr-no-dictionary = the text recognition model has no character dictionary
ocr-internal = text recognition: internal error ({ $error })
update-title = Speech Studio update
quit-title = Close Speech Studio?
quit-running = The studio is still working: { $n } task(s) queued or running. Closing now stops them; the takes already made are saved, and the rest can be voiced later with “Voice changed”.
quit-button-close = Close
quit-button-stay = Keep open
update-open-page = Version { $version } is out. Open the download page?
update-install-now = Version { $version } is out. Update now? The app restarts.
update-button-open = Open
update-button-install = Update
update-button-later = Later
update-open-failed = Could not open { $url }: { $error }
update-failed = The update did not install: { $error }

## Voice library
voice-missing = There is no voice named { $name }.
voice-transcribing = Recognising the reference text of { $name }…
voice-no-speech = No speech was recognised in { $name }: the recording may be silent or music only.
voice-clean-missing = Voice cleaning needs the vocal separation engine and its model: download them in Settings → Models → Voice cleaning, or use the voice without cleaning.
voice-cleaning = Cleaning { $name } of music and noise…
voice-denoising = Removing noise from { $name }…
voice-design-missing = The Qwen3-TTS VoiceDesign model is not downloaded: download it in Settings → Models.
voice-designing = Making a voice from the description: { $n } of { $total }
voice-design-gone = These candidates are gone: make the voice again.
voice-design-empty = Describe the voice and write a sample text.
voice-design-name = Designed voice
setup-comp-qwen3-tts-design-name = Qwen3-TTS VoiceDesign (voice design)
setup-comp-qwen3-tts-design-purpose = A voice from a text description — gender, age, timbre, manner: several candidates, the best kept in the library as a reference
voice-bad-audio = { $name } is not a sound file the studio takes: WAV, MP3, FLAC, M4A or OGG.
voice-default-name = My voice
voice-lang-needs-whisper = { $name } is in a language Parakeet does not know ({ $lang }): its text is recognised by CrisperWhisper. Download CrisperWhisper in Settings → Models → Speech recognition.
voice-no-words = { $name } has no word timings yet: recognise its reference text first.

## Speech
tts-defective = The voice came out damaged in every attempt ({ $defects }): try another voice, a shorter text or a lower temperature.
speech-block-silent = Nothing in this block is spoken: it is only pauses and markup.
speech-synth-block = Voicing { $n } of { $total }: { $text }
speech-synth-all-failed = No block was voiced: { $error }
speech-export-not-ready = { $count } blocks keep the export back (not voiced, out of date, a take file lost or changed, a take longer than its cue): the export check names them.
speech-export-empty = There is nothing to export: every block is empty or hidden.
speech-export-block = Joining { $n } of { $total }…
speech-export-loudness = Bringing the loudness to the preset…
speech-name-template-same = The name template “{ $template }” gives all { $count } files one name: add {"{"}n{"}"} or {"{"}chapter{"}"}
speech-export-stems = Tracks by speaker: { $count }
speech-by-speaker = by speaker
speech-untitled = Untitled
speech-narrator = Narrator
speech-lines = Lines
speech-no-project = There is no project { $pid }.
media-loudnorm-silent = The audio is silent: there is nothing to bring to a loudness.
media-bad-format = { $format } is not an export format: WAV, FLAC, MP3, OGG or M4A.
speech-qc-block = Checking { $n } of { $total } by ear…
speech-qc-failed = The check did not pass: recognised as “{ $heard }”. Listen to it, choose another take or voice it again.
stress-model-missing = The Russian stress model is not downloaded yet: get RUAccent in Models
stress-failed = Could not place stress marks: { $detail }
dict-bad-id = A dictionary entry has an empty or repeated id: { $id }
dict-bad-kind = Unknown kind of dictionary entry: { $kind }
dict-empty-find = A dictionary entry has nothing to find
dict-bad-pattern = The pattern of a dictionary entry is broken: { $detail }
dict-import-unknown = { $name } is not a Balabolka dictionary (.dic, .rex or .bxd)
speech-engine-unknown = Unknown synthesis engine: { $engine }
speech-lang-unsupported = { $engine } does not speak { $lang }: choose another language or Auto in the project settings
speech-no-voice-blocks = These blocks have no voice, and their engine speaks only with a voice from the library: { $blocks }. Choose a voice for the project, the speaker or the block.
speech-no-voice-library-empty = These blocks have no voice, and their engine speaks only with a voice from the library, which has no voices yet: { $blocks }. Add a voice in the Voices tab (from a file, a recording or a pack) or choose an engine with voices of its own.
speech-unknown-voice-blocks = These blocks have a voice that is neither in the library nor one of their engine's own voices (often a built-in voice of another engine): { $blocks }. Choose a voice for the project, the speaker or the block.
speechproj-not-a-package = This is not a Speech Studio project file (.speechproj)
speechproj-bad-path = The project file has a path outside its folder: { $path }
speechproj-bad-manifest = The project file description is damaged: { $detail }
speechproj-too-new = The project file was made by a newer Speech Studio (format { $version }, app { $app }): update the studio
speechproj-engine-missing = The project needs the { $engine } engine, which this studio does not have
speechproj-asset-missing = The project file lacks { $path }
speechproj-asset-damaged = { $path } in the project file is damaged (size or SHA-256 does not match)
speech-subs-empty = No lines with timing were found in { $name } (SRT, VTT and ASS are read)
book-unknown-format = { $name }: books are read from EPUB, FB2, PDF, DOCX, HTML, MD and TXT
book-empty = No text was found in { $name }
book-damaged = The book file is damaged or not what its name says: { $detail }
speech-chapter-n = Chapter { $n }
export-save-as = Save as
speech-qc-unlike = The voice of the take does not sound like the reference (similarity { $similarity }, needed 0.5 or more): voice it again or check the reference
listen-speaker-n = Speaker { $n }
listen-no-source = the project has no recording
listen-whisper-missing = CrisperWhisper is chosen, but it is not installed: download it in Settings → Models
listen-live-downloading = Downloading the recogniser; live recognition starts when it is done
say-hushed = The person stopped the speech in the window: wait for their next phrase before saying more
live-metrics-bad-range = Not a line number: { $value }
say-no-floor = Nobody is speaking over the studio now: there is nothing to go on after
echo-guard-no-owner = The echo protection by your voice needs your voice recorded (Settings, voice gate): until then the studio keeps from hearing itself by loudness and by its own words only
voice-gate-stale = Your voice was recorded by an earlier version and is not used: record it again (Settings, voice gate). Until then the voice gate lets every phrase through
say-mic-off = The microphone is off: the person turns it on in Live, then voice_listen hears them
mic-only-person = Only the person turns the microphone on, with its button in the studio
listen-gguf-missing = The recognition model "{ $engine }" is not downloaded: get it and the transcribe.cpp engine in Settings → Models.
listen-gguf-lang = The model "{ $engine }" does not recognise "{ $lang }": choose another model or CrisperWhisper.
listen-separate-missing = voice separation needs the Roformer engine and model: download them in Settings → Models
listen-diar-missing = speaker detection needs the diarization model: download it in Settings → Models, or set one speaker
listen-step-audio = Preparing the audio
listen-step-separate = Separating the voice from music and noise
listen-step-diarize = Finding the speakers
listen-step-asr = Recognising speech
listen-step-langid = Language of the recording: { $lang }
listen-step-asr-piece = Recognising speech: piece { $n } of { $total }
listen-step-denoise = Removing noise
listen-no-speech = No speech found in the recording: if there is quiet speech, lower the speech threshold (now { $threshold }) in Recognition → Cutting at pauses.
listen-step-filter = Checking for recogniser hallucinations
listen-step-done = Done: { $lines } lines
listen-nothing-marked = no words are marked for removal
listen-cut-everything = every word is marked: nothing would be left
listen-step-cut = Cutting { $n } fragments out
listen-cut-name = { $name } (edited)
listen-step-cut-done = The edited recording is ready
listen-export-format = unknown transcript format { $format }
listen-not-media = { $file } is not an audio or video file
listen-not-recording = nothing is being recorded
listen-mic-name = Recording { $time }
listen-mic-origin = microphone
listen-empty = the transcript has no lines
listen-transcribe-first = recognise the recording first
listen-align-no-text = Paste the text the recording should say.
drama-needs-cuda = The Drama 3 voice needs an NVIDIA card with CUDA, and none was found ({ $card }). Choose Higgs.
drama-needs-vram = Drama 3 needs a graphics card with 6 GB or more, and this one has { $gb } GB ({ $card }). Choose Higgs.
drama-needs-ram = On a card under 24 GB, Drama 3 keeps its experts (18 GB) in RAM: it needs 32 GB of RAM, and this computer has { $gb } GB. Choose Higgs.
render-drama-missing = Drama 3 is chosen but not fully downloaded: download it in Models (voice), or choose Higgs.
render-drama-load-failed = loading Drama 3: { $error }
render-drama-vram-limit-low = The video memory limit for Drama 3 ({ $gb } GB in Settings → Models) leaves no room for its expert cache on this machine: set it to Auto or raise it ({ $error })
render-drama-vram-busy = Drama 3 found no room for its expert cache: other programs hold the graphics card's memory. Close them and voice again ({ $error })
render-loading-drama = loading Drama 3
render-drama-no-ref-text = the voice reference has no transcript, and Drama 3 cannot clone a voice without one
drama-voice-no-text = The voice «{ $name }» has no reference text, and Drama 3 does not clone a voice without it: open the voice in the «Voice library» and type its «Reference text» or press «Recognise the reference text».
drama-voice-no-piece = The voice «{ $name }» is longer than { $secs } s, and no piece of it up to { $secs } s with whole words was found: open the voice in the «Voice library», press «Find the best pieces» and use one, or trim the reference.
setup-comp-drama3-purpose = Fish Audio Drama 3: a second voice engine with expressive cloning; graphics cards from 6 GB, with 32 GB of RAM on cards under 24 GB
drama-needs-voice = Drama 3 clones a voice from its reference: choose a voice for the block or the project
setup-comp-parakeet-ultra-purpose = Speech recognition — a fine-tuned version with fewer mistakes
setup-comp-transcribe-engine-purpose = The transcribe.cpp recognition engine on the GPU: Parakeet Ultra, language identification and Nemotron 3.5 in a live conversation
setup-comp-transcribe-engine-cpu-purpose = The transcribe.cpp recognition engine on the processor, for a computer without NVIDIA: Parakeet Ultra, language identification and Nemotron 3.5 in a live conversation, slower
setup-comp-transcribe-engine-vulkan-purpose = The transcribe.cpp recognition engine on Vulkan, for an AMD or Intel card and an NVIDIA one CUDA 13 does not run on: Parakeet Ultra, language identification and Nemotron 3.5 in a live conversation
setup-comp-asr-gguf-nemotron-purpose = Nemotron 3.5: 28 languages with punctuation, Russian, Chinese and Japanese among them (NVIDIA)
render-defect-clipping = clipping
render-defect-click = a click at the end
director-failed = the director model did not answer: { $error }
director-not-json = the director model answered not in the agreed form: { $answer }
director-empty = the director model wrote no lines
director-not-verbatim = the director model changed the book text in the part starting “{ $start }” twice; mark that part by hand
director-need-topic = a topic and at least two named speakers are needed
director-starting = Starting the director model
director-writing = { $model } writes the script
director-casting = Marking speakers: part { $n } of { $total }
director-done = Done
cloud-catalog-empty = the provider listed no speech models
cloud-unknown-model = { $provider } has no model { $model } in the catalogue
cloud-no-schema = the model { $model } has no form description
cloud-unknown-provider = { $provider } is not a cloud provider here (fal, runware)
cloud-limit-unpriced = the daily cloud limit for agents is { $limit } USD and this run has no price before it runs
cloud-limit-reached = the daily cloud limit for agents is reached: { $used } of { $limit } USD
cloud-no-key = { $provider } is chosen but has no key: add it in Settings, Cloud
cloud-failed = { $provider }: { $error }
cloud-no-audio = the cloud model answered without audio
cloud-no-text-field = the model { $model } takes no text
cloud-no-audio-field = the model { $model } takes no audio file
cloud-key-one-line = the key must be one non-empty line
cloud-no-model-chosen = choose a { $provider } model in the project settings
google-no-voice = choose a Google Gemini voice in the project settings
google-batch-sending = Google Batch: sending { $n } phrases as one batch
google-batch-waiting = Google Batch: { $state }, { $done } of { $total } phrases ready
google-batch-dead = Google Batch { $name } ended { $state } without results ({ $error }); voicing again sends the same phrases as a new batch
google-batch-unknown = Google Batch { $name } was sent but its answer was lost and Google does not list it yet; run the voicing again later (journal { $journal })
google-batch-many = { $n } Google batches carry the name { $name }; resolve them in Google AI Studio (journal { $journal })
google-batch-queue-full = the Google Batch queue for this model is full, nothing was created; run again when a running batch ends: { $error }
google-batch-rejected = Google refused the batch, nothing was created and nothing is paid; it is safe to run again: { $error }
google-batch-no-row = the batch result has no line for this phrase
google-retry = Google: { $what } failed (attempt { $attempt } of { $of }), again in { $secs } s: { $error }
google-read-failed = Google: { $what } failed { $attempts } times in a row: { $error }
google-phrase-failed = Google did not voice this phrase: { $error }
google-direct-sending = Google Gemini: { $n } phrases, { $threads } requests at a time
google-direct-progress = Google Gemini: { $done } of { $total } phrases
google-err-prepayment = the Google Gemini API prepayment has run out — top up the key's project in Google AI Studio and start the voicing again
google-err-cap = the Google key's project has hit its spending cap — raise the cap in Google AI Studio or wait for the next month
google-err-key = Google did not accept the key — check it in Settings → Cloud
google-err-access = the Google key has no access to this model — check the key's project in Google AI Studio or pick another model
google-err-rate = Google limited the request rate — lower the parallel requests in the settings or start later
google-err-filter = Google refused to voice this phrase under its content rules — rephrase it or voice it with another engine
chat-empty = write the assistant a message
chat-no-model = choose the assistant's model
chat-too-many-turns = the assistant took { $turns } steps and stopped; say whether to go on
assistant-no-agent = the assistant is the connected agent, but no agent is connected now: connect one (Settings, Agent) or make an OpenRouter model the assistant
assistant-no-answer = the agent did not answer within { $minutes } min
dict-try-local-only = the try by ear works with the engines on this computer (Higgs, Drama 3); the project's cloud engine is paid for every run
dict-try-empty = enter a phrase to try
dict-try-before = Try by ear: as written
dict-try-after = Try by ear: with the rule
speech-part-n = part { $n }
speech-export-encoding = Encoding part { $n } of { $total }

render-supertonic-missing = Supertonic 3 is chosen but not downloaded: download it in Models (voicing) or choose another engine.
render-supertonic-load-failed = Supertonic 3 did not load: { $error }
render-audio8-missing = Audio8 TTS is chosen but not downloaded: download it in Models (voicing) or choose another engine.
render-audio8-load-failed = Audio8 TTS did not load: { $error }
render-audio8-model-missing = Audio8 TTS is not downloaded ({ $path }): download it in Settings → Models → Voicing.
setup-comp-audio8-purpose = Compact multilingual voice with zero-shot cloning; an alternative voicing engine.
speech-voice-not-preset = The voice “{ $voice }” is from the library, and { $engine } speaks only in its own voices: choose one of them for the project, the speaker or the block.
speech-voice-preset-needed = This engine needs one of its own voices.
setup-comp-supertonic3-purpose = Supertonic 3: fast voicing in ten ready voices and 31 languages, no cloning
render-qwen-missing = Qwen3-TTS is chosen but not fully downloaded (the engine or the weights of the chosen quantisation): download it in Models (voicing) or choose another engine.
render-qwen-load-failed = Qwen3-TTS did not load: { $error }
setup-comp-qwen3-tts-purpose = Qwen3-TTS 1.7B: nine ready voices (CustomVoice) and cloning of library voices (Base), ten languages with Russian, streamed audio
render-qwen-clone-missing = Voice cloning in Qwen3-TTS runs on the Base model of the chosen quantisation, and its files are missing: download Qwen3-TTS again in Models or pick one of the model's own voices.
render-qwen-stress-missing = Qwen3-TTS with Russian stress is chosen but not fully downloaded (the engine, the Base model of the chosen quantisation or the stress adapter): download it in Models (voicing) or choose another engine.
render-qwen-stress-needs-voice = Qwen3-TTS with Russian stress only clones a voice: choose a library voice for the block or the project.
setup-comp-qwen3-tts-ru-stress-name = Russian stress for Qwen3-TTS
setup-comp-qwen3-tts-ru-stress-purpose = A LoRA adapter for Qwen3-TTS Base (siriusfreak): the cloned voice follows the stress marks the studio places, 96% of words against 17% without it
setup-comp-qwen3-tts-engine-name = Qwen3-TTS engine
setup-comp-qwen3-tts-engine-purpose = The program and the audio codec every Qwen3-TTS quantisation shares
project-missing = project { $pid } is gone: it was deleted or not created yet
media-bad-rate = { $rate } Hz does not suit { $format }
media-bad-bits = { $bits }-bit does not suit { $format } (WAV: 16, 24, 32; FLAC: 16, 24)
media-bad-kbps = { $kbps } kbps does not suit { $format } (MP3, OGG, M4A: 64 to 320)
speech-own-ref-missing = the own reference file { $file } is not in the project's refs folder - upload the reference again
speech-own-ref-no-file = no reference file came
speech-no-block = the project has no block { $id }
director-no-tags = { $engine } reads no expression tags - there is nothing to put in
director-enriching = The director puts in tags: part { $n } of { $total }
speaker-too-little = speaker { $name } speaks too little to remember the voice: at least 2 seconds are needed
book-name-note = a name from the book: put the stress mark + and mark it when checked by ear
director-punctuating = The director punctuates: part { $n } of { $total }
tracks-empty-folder = The folder { $path } has no sound files
tracks-not-audio = { $path } is neither a sound file nor a folder
tracks-no-track = The project has no track { $track }
tracks-stem = track
tracks-no-files = No sound file was chosen
music-no-job = { $studio } started no song job
music-no-song = { $studio } gave no song
music-unknown = No music studio { $studio }
music-asking = Sending to { $studio }…
music-making = { $studio } is making the song: { $detail }
music-placing = Putting the song on the track…
music-track = Music
listen-export-rules = No subtitle rules “{ $rules }”; there are: { $known }
itn-no-lang = Numbers as digits are not known for { $lang }: only ru, en, es, fr, pt, de, it, nl, da
listen-whisper-for-lang = Parakeet does not know { $lang } — it needs CrisperWhisper, which is not installed: download it in Settings → Models or pick a cloud recogniser
listen-live-name = live recognition
listen-live-no-vad = Live recognition needs Silero VAD — download it in Settings → Models
listen-live-broke = Live recognition broke down — the recording is kept, recognise it as usual
watch-no-folder = There is no folder { $folder }
watch-no-formats = Choose at least one format
watch-bad-format = No format { $format }; there are: { $known }
watch-broke = The folder's work broke down — start it again, nothing done is lost
watch-busy = The recording is being recognised by another job ({ $kind }) — it is tried on the next round
speech-corpus-name = Hard phrases — engine check
speech-corpus-unknown = There is no phrase set { $corpus }. Only ru_hard exists.
speech-qc-defects = Sound defect: { $list }. Listen to it, pick another take or voice it again.
qc-issue-click = a click at the end
qc-issue-clipping = clipping
qc-issue-cutoff = cut off
qc-issue-gap = a gap inside
qc-issue-garbled = mush instead of the text
qc-issue-hum = hum
qc-issue-noise = music or noise instead of speech
qc-issue-repeat = repeated phrase
qc-issue-runaway = the model ran on
qc-issue-silence = silence
qc-issue-stretched = a stretched word
qc-issue-tail = sound after the last word
qc-issue-whistle = whistle
speech-ab-two = A blind comparison takes exactly two engines.
speech-ab-engine = Engine { $engine } cannot be compared: it takes a known local engine (cloud ones cost money).
speech-ab-side = Voicing variant { $n } of 2…
speech-fix-no-take = The block has no take to mend: voice it first.
speech-fix-sentence = Voicing sentence { $n } again…
speech-restore-gone = The project is not in the recycle bin: it was emptied or the project was deleted elsewhere.
speech-restore-finder = On macOS a deleted project is taken back from the Bin in Finder: select it there and choose Put Back.
speech-restore-exists = The project is already in place.
speak-aloud-no-window = There is nowhere to say it aloud: the Speech Studio window is not open. Open it, and the sound plays from the studio as the phrase is voiced.
say-session-name = Conversation { $time }
talk-widget-title = Speech Studio Live
say-no-session = There is no speech session “{ $session }”: start a new one.
listen-live-not-running = Live recognition is not running: turn the microphone on in Live first.
listen-live-engine = The recogniser "{ $engine }" cannot recognise live: take parakeet, nemotron or whisper.
say-no-text = The text is empty: write what to tell the agent.
record-test-other = Another recording is running, not a microphone test.
record-test-none = There has been no microphone test yet.
voice-gate-too-short = The recording is too short to learn your voice — say the phrase again.
voice-gate-not-heard = I couldn't make out your voice — say the phrase clearly and try again.
setup-comp-crisperwhisper-purpose = Speech recognition in 99 languages with timestamps, beyond Parakeet's 25 European ones. Licence: nyra health Non-Commercial (LICENSE.md beside the weights)
acpp-no-server = The audio.cpp engine is not installed ({ $path }): download it in Settings → Models
acpp-no-libraries = The audio.cpp engine cannot start: its CUDA libraries are missing ({ $files }). Download them in Settings → Models
acpp-no-cuda-build = The graphics card (CUDA) is chosen, but this computer has no NVIDIA card the audio.cpp engine runs on: it needs Maxwell or newer with driver 525 or later. Choose Vulkan or the processor in Settings → Models
acpp-no-model = The model «{ $model }» of the audio.cpp engine is not downloaded: download it in Settings → Models
acpp-error = audio.cpp engine: { $error }
acpp-out-of-memory = The audio.cpp engine ran out of video memory: choose a lighter Higgs quantization in Settings → Models, shorten the block or close programs that use the card ({ $error })
acpp-no-device = This computer has no device the audio.cpp engine was set to: choose another device for voicing or recognition in Settings → Models ({ $error })
update-reason-transcribe-own-ggml = Recognition no longer fails after voicing with Qwen3-TTS: the engine's libraries got names of their own
update-reason-newer = A newer version
