# Changelog

What changed, newest first. Dates are release dates; the app is versioned by its Windows
build. Every change a user can see is written here in the commit that makes it, and the
release notes on GitHub are taken from the release's section.

## Unreleased

### Changed

- **ONNX Runtime 1.31.1 and llama.cpp b12248**: recognition, diarization, noise suppression, stresses and the voice gate run on ONNX Runtime 1.31.1, the assistant on llama.cpp b12248, downloads by link on yt-dlp 2026.10.26; the start screen offers them as a components update and removes ONNX Runtime 1.28.2. Noise suppression, speech marking and Russian numbers in words are the same code as Dub Studio's.
- **A model server that cannot be reached is named**: when every attempt to reach a cloud or a local model server fails to connect, the error names its host and points to Settings → Network, and a provider's 409 conflict is tried again as a rate limit is.

### Fixed

- **A clone from a quiet voice speaks at speech level** ([#34](https://github.com/timoncool/speech-studio/issues/34)): Higgs speaks at the level of its reference, so a voice recorded far below speech level is raised to −23 dBFS before cloning (its peaks kept under full scale); the voice file itself stays as it is.
- **An ONNX Runtime that does not load is told, not a crash of the task**: when the library cannot be loaded (a missing Visual C++ runtime, error 1114), recognition, noise suppression and stresses say which file and what to install instead of failing inside the engine.
- **«Stop» is not an error**: stopping a voicing, the casting or the director no longer shows a red «job cancelled» banner with «Report a problem» nor marks the block it was voicing as failed; the activity log says it was cancelled, in the window's language.
- **Dragging at an interface scale other than 100 %**: a clip on the project timeline and a floating panel follow the mouse at 90–150 % instead of running ahead of it or lagging behind.

## 0.5.0 — 2026-11-10

### Added

- **Audio8 TTS Realistic-Air — a compact multilingual voice with zero-shot cloning** (Settings → Models → Voicing): a new voicing engine beside Higgs and Supertonic — the Audio8 Realistic-Air 0.6B model (Q8_0, 1.5 GB) on the audio.cpp server. It speaks 23 languages, Russian among them, with its own stress, clones a voice from a reference clip and its exact transcript, and answers a phrase at a time at 44.1 kHz. Download it in Models; Higgs stays the default.
- **Roles in Live: a trainer for any conversation** — pick who the agent plays (Live → Role): an interviewer, an investor, a client after a discount, a sceptic, a boss who refuses a raise, a difficult colleague, an audience with a question, a language partner, a debate opponent. A role is a ready idea, not a frame: the agent may change it, and you two can make up your own just by talking — a hard conversation, a diction or language drill, an improvisation, a scene of a play — and keep it for next time (`live_role_list`, `live_role_set`). Every role is a file in the studio's `roles` folder that you can edit or add to. After the conversation the agent's review (`live_role_review`) gives your pace, pauses, filler words and how soon you answered, three of your phrases, three things to change and one goal, compared with your last time in the same role.
- **Live: the end of your phrase heard by a model** (Live → Microphone and sound → Agent's voice → «The end of your phrase»): Smart Turn v3.2 (32 MB, downloaded when you choose it) listens whether a phrase sounds finished, so a pause for breath cuts you off mid-sentence less often — at the cost of waiting a little longer for some ends. By the pause, as before, unless you choose it.
- **The director inside an open project** ([#29](https://github.com/timoncool/speech-studio/issues/29)): «Continue the script» writes what comes next for the same speakers, from the end of what is written, with an optional wish where it goes — the new blocks are added after the last one; «Rewrite the selected» words the chosen blocks anew as you ask and shows each before and after to keep or drop. Undo takes either back; agents have `project_extend` and `project_enrich` with mode `rewrite`.
- **Quiet voices are no longer quiet** ([#34](https://github.com/timoncool/speech-studio/issues/34)): a recording from the microphone is levelled as soon as it stops, a voice shows the loudness of its reference with a «Level the loudness» button for a quiet one (the original is kept), and in Live and while listening as it is voiced a quiet voice plays as loud as the others.
- **Speech metrics for the agent in Live** — `voice_metrics` gives the person's words a minute, pauses of 500 and 1000 ms, filler words (Russian and English, more in fillers.json), how soon they answer and their speech and silence; `voice_lines` gives their phrases with each word's time; a Live phrase carries word timings when the recogniser gives them.
- **The agent sees when its speech stalls** — `say_state` gives for each phrase the silence heard before it because it was not ready in time (`queue_wait_ms`) and how many such pauses were over 300 ms.
- **A benchmark of Live for the agent** — `live_bench` measures and keeps each run as a JSON file in the studio's bench folder, to compare two attempts: the waits of the conversation (median and 95th percentile), the rule that tells an interruption from a «mhm» on a marked set of phrases, and the word error rate of the recogniser on your recordings with their texts. The same from the command line: `cargo run -p speech-server --example live-bench`.
- **Echo protection in Live** — the agent no longer hears itself from the speakers: by default the microphone is not heard while the studio's sound is far louder than it, and you speaking over it are heard at once; or by your voice (voice gate), or by words only.

### Changed

- **Voicing no longer starts into a known failure**: with a cloud engine that has no key or model, an engine that is not downloaded, or a graphics card the engine cannot run on, the window says so before anything starts and opens the settings that mend it; a block of pauses alone is skipped instead of failing; a voice pack download that broke off goes on from where it stopped.
- **Higgs on a slow card is no longer cut off as hung**: a phrase waits for the engine as long as that engine takes on this computer, the server is started again once when it ends before a phrase's first sound, and a clone from a short reference or one at 44.1/48 kHz is no longer refused by the codec.
- **An update of a component is shown as an update** — when a new version replaces an engine you already have, the start screen says «Components update» and names what is updated and why, instead of «First start».
- **In Live the agent decides when you speak over it** — the studio turns its voice down and the agent stops or goes on (`voice_floor`, `say_continue`); with no word from it within 1.2 s the studio stops by itself. The setting "When you speak over the agent" in Live's voice tab chooses the agent, an immediate stop or none. A phrase cut off while it played is shown as interrupted.

### Fixed

- **A block you approved by ear is exported** ([#35](https://github.com/timoncool/speech-studio/issues/35)): a take the check turned down kept its block out of the export even after you listened to it and marked it «approved» — the export said hundreds of voiced blocks were errors. Your approval now outranks the check: the block counts as voiced, passes in the quality light and is exported as it is; the export window says so. The «Export without the unvoiced» checkbox shows its number again.
- **A Russian take is checked in Russian**: in a project with the language «Auto» the check guessed the language from the sound, and a Russian phrase in a voice with an accent was heard as Ukrainian and failed («сборки → збурки»). The check now hears a take in the language its text is read in. A new project on «Auto» also gets a library voice of its text's language instead of the first by name.
- **Downloads through a proxy whose password has special characters** (`/ ? # :`, a space, non-Latin letters) now work instead of being refused; the libraries the studio is built on are updated.
- **Recognition no longer fails after voicing with Qwen3-TTS** ([#28](https://github.com/timoncool/speech-studio/issues/28)): «transcribe.dll: LoadLibraryExW failed» came when the recognition engine was loaded after Qwen3-TTS — both carried a ggml library of the same name, and Windows gave the second engine the first one's. The recognition engine (transcribe.cpp 0.2.1) now names its own, so the two no longer meet in either order, on Windows and Linux.
- **Blocks no longer stay unvoiced or keep a project from export because of a too strict sound check**: a take was called «clipped» for a single run of three samples at full scale — now from 1 % of the samples, the level speech-quality tools fail a clip at — and such a take kept the whole project from export («blocks keep the export back»). A click at the end of a take is faded out instead of the take being thrown away. A phrase counts as «drawn out» only when it is twice as slow as normal reading (slow voices were turned away on every attempt), and the limits follow the speed you asked for. When every attempt still comes out with a defect, the least damaged take is kept and marked for you to listen to, instead of «No block was voiced». Every take turned away is written to the log with what it was measured by.
- **The check of a take no longer mistakes a fast start or spoken numbers for a defect**: an engine that begins speaking at once (Audio8) had nearly every take sent to a second recognition as «cut at the edge», which made checking a project take minutes; and a phrase with numbers said in words («восемнадцать тридцать») was called «mush instead of the text» when the recogniser wrote them in digits.
- **A book with scene breaks voices to the end and a book of many hours exports** ([#24](https://github.com/timoncool/speech-studio/issues/24)): a line of «* * *» became a block with nothing to say, so the count stopped short of the end (3165 of 3196) — it is now a pause after the block before it, and such blocks in projects already made are skipped. Exporting a whole book ran out of memory (an hour of sound took 8 GB): MP3 and FLAC, the loudness, the true peak and the limiter now work piece by piece.
- **A book of thousands of blocks no longer freezes the window** ([#31](https://github.com/timoncool/speech-studio/issues/31)): a project from 200 blocks draws only the blocks near the screen, the rest keep their place and are drawn as you scroll, search or jump to them. On 5200 blocks a click and a typed letter answer within a frame (the page did not answer for over 45 s before); dragging a block and the keyboard reorder work as before.
- **A take sounds in the window as loud as its exported file** ([#27](https://github.com/timoncool/speech-studio/issues/27)): the window played the model's own take, which with some voices and engines is 10–17 dB quieter than the export brought to the project's loudness — «quiet in the app, normal on disk». The player, the take history and the blind comparison now play a take at the project's loudness preset; the take's file stays as the model made it.
- **An updated engine leaves no files of its earlier version behind**: a download unpacked over the version before it removes that version's files the new one no longer has.
- **Voicing blocks that have no voice no longer fails as a job**: with an engine that speaks only a library voice the window opens the choice of a voice at once, and says to add one when the library is empty; a new project on such an engine starts with a library voice, and a voice of another language or without its reference text is no longer passed over.
- **Russian speech the language detector takes for Belarusian is recognised in Russian** by Parakeet, without asking to download CrisperWhisper.
- **Live: the agent can be spoken over in a quiet voice** — the echo protection measures how loud the studio's own voice comes back into the microphone (the window's volume and the microphone's gain are in it) and hears the person 12 dB over it; a muted microphone and a phrase cut by a stop are no longer taken for echo; a phrase that failed is shown as failed, not as sounding or interrupted.
- **A take kept with a defect after every attempt is marked for listening** whatever checks the project has on, and a failed export no longer erases the file exported before under the same name.
- **Stresses, the voice gate and noise removal work on a computer without working CUDA**: they take the processor build of ONNX Runtime there; when the runtime does not load, the error names its file and says to install the current Visual C++ 2015–2022 Redistributable.
- **A project of one short word exports**: «The sound is quiet: nothing to bring to the loudness» came for a programme shorter than 0.4 s.
- **The voice gate recognises its owner again** — your voice print was taken together with the silence around the recording and turned most of your phrases away; it is now taken from your speech only. Record your voice again in Settings, voice gate: an old print is not used until you do.
- **Live no longer loses what the person says** — the agent gets every phrase heard (`heard` is the number of lines there are to get); the studio's own voice heard back through the microphone is marked as echo, told by when its phrase sounded rather than by shared words, and a stop pressed in the window ends with the conversation.
- **Qwen3-TTS downloads whole from the Models screen**: the engine's build for the video card is shown beside the engine, so the engine no longer stays "not fully downloaded" after its files are there.
- **The first start on a Mac no longer stops with an error** before the recognition runtime is downloaded.

## 0.4.0 — 2026-11-03

### Added

- **A voice gate for Live — start a conversation hands-free** (Settings → Voice gate): the studio listens in the background and begins talking to the agent by itself, by **your own voice** (record it once; a speaker-recognition model, ReDimNet2-B3, recognises you and starts only when you speak) and/or by a **wake phrase you set, in any language** («Эй Клод», «Привет Анифса», «Hey Claude»): a universal phone recogniser (PhoneticXeus) and eSpeak-ng spot the phrase, so there is no training and no fixed word list. Recording your voice finishes by itself once enough is heard. Everything is off by default; the voiceprint and the wake model stay on the device.
- **Start Speech Studio on login** (Settings → Appearance): the studio launches with the computer, so Live is ready at once. Off by default.
- **Talk to the agent from Claude Code by voice**: a companion mod, [studio-voice](https://github.com/timoncool/speech-studio-voice), shows the studio's status above the prompt, speaks the agent's replies aloud as they finish (optionally shortened by Haiku to a spoken line) and feeds your speech back as the next prompt.
- **A volume control on the player** ([#22](https://github.com/timoncool/speech-studio/issues/22)): mute and set the level of what is voiced, kept in sync with Settings.
- **Export can skip the blocks that are not voiced** ([#24](https://github.com/timoncool/speech-studio/issues/24)): a project with empty blocks still exports the rest.
- **Make library voices from a recording's speakers, and a Logs folder** ([#20](https://github.com/timoncool/speech-studio/issues/20)).

### Fixed

- **A cough, a laugh or a short noise no longer interrupts the agent in Live**: only real, continuing speech takes the floor (semantic barge-in, as voice agents do it), and the agent's speech is stopped at once when it does.
- **A block that comes back empty is voiced again on the engine's own recommended settings** ([#18](https://github.com/timoncool/speech-studio/issues/18)), instead of staying unvoiced.
- **A transient «LoadLibraryExW failed» on the third re-voicing or recognition is retried** ([#21](https://github.com/timoncool/speech-studio/issues/21)) instead of failing the take.
- **The agent connects to MCP again** ([#23](https://github.com/timoncool/speech-studio/issues/23)).
- **No more double dot in an export's file name** ([#19](https://github.com/timoncool/speech-studio/issues/19)).
- **The voice catalogue and library search every field** — name, description, language, gender, age and year — not just the name and description.
- **The changelog tab shows the changelog again**: its heading format had drifted from what the window parsed, so it had gone blank.

## 0.3.5 — 2026-10-28

### Added

- **The interface can be scaled** ([#16](https://github.com/timoncool/speech-studio/issues/16)): Settings → Appearance → Interface scale draws the whole studio from 90 to 150 % and keeps it for the next start; Ctrl + plus and Ctrl + minus, or Ctrl with the mouse wheel, change it too, and Ctrl + 0 returns to 100 %.
- **The voice catalogue filters, searches and plays before you download**: the extra voices come with their name, gender, age (adult or child), language, description, length and year of birth, so the library's "Open cloud voices" filters by gender, age, language and minimum length, searches the name and description, sorts by name, length or age, plays a sample, and downloads the ones picked or everything the filters show, with the sample's transcript where the collection has one. A downloaded voice keeps all of it: the library shows it under its own name with its description, length and year of birth. The catalogue holds 1,636 voices: 1,394 Russian and 242 in 31 other languages (English 75, French 28, Italian 16, German 14, Portuguese 13, Spanish 13, Polish, Arabic and Turkish 9 each, Japanese 7 and more); 1,068 male, 568 female, 60 children. If you know a good collection of popular voices in another language, [open an issue](https://github.com/timoncool/speech-studio/issues/new/choose) and we will add it.
- **The library finds voices as the catalogue does**: the same search over names, descriptions and tags, filters by gender, age, language, source, length and favourites, sorting by name, length or age, a count of what is shown and «Reset filters»; a project's voice field finds a voice by its name or description.
- **A voice without its reference text is heard when it is voiced**: the reference's text is recognised once, before voicing, for every engine that clones, so a catalogue voice without a transcript (about half of them) or an imported one needs no «Recognise» first; Drama 3 no longer refuses such a voice.
- **The API takes the engine's settings** ([#17](https://github.com/timoncool/speech-studio/issues/17)): `POST /v1/audio/speech` and the MCP tool `speak_text` accept, beyond OpenAI's fields, the project settings of the local engines — `seed`, `temperature`, `top_k`, `top_p`, `max_tokens`, `steps` (Supertonic) and `loudness` (off, video, podcast, acx) — each over the engine's default, as Project settings set them in the window.

### Fixed

- **Every engine runs on any x86-64 processor, with or without AVX2, FMA or BMI2**: Drama 3 starts again as it did before 0.3.0, which refused it with «Drama 3 needs a processor with AVX2…», and the audio.cpp engine (Higgs, Supertonic, CrisperWhisper, Parakeet), Qwen3-TTS, the voice separator and recognition (transcribe.cpp) now come with a processor build for every instruction level and load the one this processor runs best, instead of the AVX2 build alone. The new builds are downloaded once after the update.
- **A voice reference in a language the recogniser does not know is heard anyway**: under Auto, a Chinese (or other) reference recognised by the default Parakeet, which does not know it, came back empty and was refused as «no speech was recognised»; now the reference's language is heard by VoxLingua107 and a recogniser that knows it (CrisperWhisper) is used, so its transcript, its cleaning and its voicing with Drama 3 go through.
- **Auto only guesses a language a recogniser can read**: the language id (VoxLingua107) names a few languages CrisperWhisper has no token for (Esperanto, Guarani, Interlingua) and a recording guessed as one of them was refused; Auto now takes the likeliest language a recogniser here can read instead.
- **The voice-similarity check runs on the processor when there is no usable graphics card**, instead of stopping a voicing with an ONNX Runtime CUDA error: its model asks for the card only when one can run a CUDA build, as under Auto with no card.

## 0.3.0 — 2026-10-27

### Changed

- **Speech Studio for macOS on Apple silicon** (macOS 13 and newer, a .dmg): First run downloads the macOS builds of recognition (transcribe.cpp), Qwen3-TTS, separation, the local LLM, ONNX Runtime, ffmpeg and yt-dlp, each with Metal where it has it, and the audio.cpp engine with Metal for Higgs, Supertonic, CrisperWhisper, Parakeet and Nemotron; the script assistant, voices and separation run on Metal too. Drama 3 runs on NVIDIA cards only. A deleted project is taken back from the Bin in Finder (Put Back): macOS lets no program do it.
- **Studio notices come as a window too**: a notice of the studio's feed can open as a window over the studio, once ready, after its delay and only while nothing is being voiced, recorded or said aloud; its buttons open a link, the news, Settings → Models or About. A strip or a window shows only in the sections it is meant for (making a work, the voice library, the news, the tools, Settings), and an advertisement carries its label and erid.
- **News from the feed have their picture and buttons**, and a button can open a link, Settings or the news.
- **The anonymous report says how fast voicing goes on the card**: seconds of speech voiced and seconds the engine worked for them, for local engines (Settings → Privacy says so). The window's own polling (microphone state, recording level, resource monitor, queue) is no longer counted as using the studio.
- **Whisper is CrisperWhisper 2.0 turbo in the audio.cpp engine**, as in Dub Studio: 99 languages with word times, the language of Auto heard by VoxLingua107, the Listening prompt passed to it. Faster-Whisper, its six models and its CUDA 11 libraries are gone, and this program's copies of them are removed at start (a model folder shared with another program loses only its link).
- **Higgs Audio v3 runs in the audio.cpp server** that CrisperWhisper runs in, as in Dub Studio, instead of its own engine library: the setup screen downloads the server once (131 MB, «Voice and recognition engine (audio.cpp)»), and the old library is removed. It has machine code for RTX 50 cards too.
- **Higgs runs on graphics cards from GTX 900 and GTX 10 to Volta and on drivers 525-579**: the audio.cpp engine comes in a CUDA 12.9 build for the cards and drivers CUDA 13 left, with its CUDA 12.9 runtime; before, Higgs said such a card is too old. The engine's CUDA libraries are checked before it starts and a missing one is named.
- **Cloud voicing works without choosing a model or a voice first**: OpenRouter starts with Gemini 3.1 Flash TTS (it speaks Russian) and its first adult voice, Google with Gemini 3.8 Flash Lite TTS, and Settings show them; the projects and settings that chose their own keep them.
- **Downloads trust the certificates Windows trusts**: behind a proxy or an antivirus that checks HTTPS with its own certificate, components failed with «invalid peer certificate: UnknownIssuer».
- **A graphics card to run on, when a computer has more than one NVIDIA card** (Settings → Models → Graphics card; from the next start): the studio and every engine it starts see that card alone. The card the studio names, its memory and the resource monitor are now the card CUDA really uses, found by its PCI bus id, not the first card NVML lists. A chosen card that is no longer in the computer is said in Settings and by every stage that needs it, instead of the work landing on another card (#10).
- **The studio has its own volume** (Settings → Language and sounds → Studio volume): everything it plays — voicing, voices, recordings and notification sounds — at that level, so switching to another window no longer brings a sudden loud take (#11).
- **A click outside «Voice from a description» no longer closes it while it holds a description, a sample or made voices**, and does not close the voice library behind it either; the window still closes with its cross or Esc (#6).
- **The detached resource monitor follows the light theme**: its panel, bars and figures were drawn in dark colours (#7).
- **Voicing names every block that has no voice before any model loads**, when the engine of those blocks speaks only with a library voice, instead of failing block by block.
- **An engine that only clones gets a voice to start with**: Drama 3 or Qwen3-TTS with Russian stress chosen for a project, speaker or block that has no voice takes a library voice of its language (a favourite first, then one of the pack; for Drama 3 one with its reference text), and Project settings say so with a link to the voice library when the library has none.
- **Recognition runs on Vulkan** on an AMD or Intel card and on an NVIDIA card CUDA 13 does not run on (Auto), and Vulkan can be chosen by hand in Settings → Models → Recognition; before, such a computer recognised on the processor.

### Fixed

- **Supertonic 3 and speaker diarization run in the audio.cpp engine of this release** (server-6d6cb6a5 on Windows and Linux), which reads Supertonic's int8 model and Nemotron 3 Diarization in ONNX.
- **An engine taken from another program counts as installed**: the record of the build it replaced stayed, so First run asked for the same engine again after taking it.
- **Recognition engines of transcribe.cpp 0.1.4 on every platform**, the Mac with Metal included (it had 0.1.1): the studio takes their pins from the shared recognition crate, as Dub Studio does.
- **A file another program has on another drive no longer removes this program's own copy first**: the studio's file is replaced only by a finished link or copy.
- **First run names its button as it is** («Download checked» in Russian, Spanish and Portuguese), and a component taken from another install of Speech Studio says so instead of «taken from Speech Studio».
- **A program downloaded on Linux or macOS runs** even when its archive carries no execute bit (the audio.cpp server of Higgs, CrisperWhisper and Parakeet answered «Permission denied»): after unpacking, a component's program is made executable.
- **Voicing finishes on Linux with the system's ffmpeg**: the AppImage gave ffmpeg, ffprobe, yt-dlp and the file manager its own libraries, and a newer system's ffmpeg stopped with «undefined symbol vaMapBuffer2»; programs of the computer now start without them.
- **The script assistant starts on Linux older than glibc 2.38 (Ubuntu 22.04 and the like)**: the CUDA builds of llama.cpp need glibc 2.38, so there the assistant runs on the card through the Vulkan build, and Settings → Models says why.
- **A window 768 to 1023 pixels wide no longer scrolls sideways**: the top bar shows Speech, Listening and Live by their icons there, with their names from 1024 pixels on.
- **The status line names the voicing model the studio really uses** outside Speech too: Higgs with its quantization, or the OpenRouter or Google model; it showed Higgs whatever was chosen.
- **The status line names the assistant the studio really uses**: Gemma's file, the OpenRouter or local server model, or the agent (MCP); it showed Gemma whatever was chosen, and now follows a preset or a change by the agent at once.
- **A hardware preset moves every stage and stays shown**: «CPU + cloud» left voicing, recognition and separation on the card when they had a device of their own, and Settings → Models showed the recommended preset again once reopened; now a preset sets the device of each stage, and the list shows the preset the settings are on, or Custom.
- **Linux and macOS know the free space of the models folder**: First run said the free space is unknown, and a download too big for the disk was not stopped before it began.
- **Nemotron under Auto keeps the first word of what is said** («Победа пришла…» was heard as «Пришла…»): Live and Listening tell it the language VoxLingua107 hears among its own, the first two seconds of speech in Live, before it recognises anything.
- **Recognised Chinese, Japanese and Thai text has no spaces between its characters** in Live, Listening and the take check (it read «今天 天 气 很好 ,»), and no text has a space before a comma or a full stop.
- **ONNX Runtime sends Microsoft no usage telemetry**: the studio sets its environment with telemetry off before any model of it loads (stress marks, noise removal, speaker voices), and does not start if it cannot.
- **Casting a book keeps the book's words whatever the model writes** («the director model changed the book's text twice»): the model only says who speaks and where each line begins, and every line's text is cut from the book itself; a chunk fails only when the model's answer is another text.
- **Higgs stops a phrase that does not end a second past its runaway length**: the cap counted 75 tokens a second where audio.cpp counts 25 codec frames, so a phrase that would not stop ran three times longer than meant before it was judged and voiced again.
- **Speech detection fetches what it lacks**: Live, Listening and the take check download the transcribe.cpp build of the recognition device or the Silero VAD model with the job's progress. Before, Live asked for Silero VAD, which was there, and kept refusing to start, and the take check and Listening stopped with «Silero VAD is not downloaded» when it was the engine build that was missing.
- **Renaming a library voice no longer breaks the projects that speak with it** («There is no voice named …»): they follow the new name, and the blocks voiced with it stay voiced.
- **Chinese, Japanese and Korean text in a project of language Auto is no longer failed as «runaway»**: its length was judged by the pace of Russian letters, so a long sentence of ideographs counted as speech that would not stop on every attempt; the language is now read from the letters when the project leaves it to Auto.
- **A window opened over the editor pauses what the editor plays** (#14): the voice library, the settings or any other window covered the player while a project or a recording went on playing with no way to stop it; it pauses where it was, and Play goes on from there. Pausing right after Play no longer shows «The play() request was interrupted by a call to pause()».
- **Another engine's built-in voice no longer fails voicing block by block** («There is no voice named F1»): choosing Higgs for a block or a speaker drops a Supertonic or Qwen voice it cannot speak, changing the project's engine clears such a voice from the voices by language too, and voicing names every block whose voice is neither in the library nor its engine's own before any model loads.
- **A failed download tells the anonymous statistics which component failed and on which site** (for example `network asr-gguf-nemotron huggingface.co`), so a cause behind "network" can be found; nothing about the computer or the person is added.
- **A recording of only silence is not saved as a voice**: when the system keeps the microphone closed or it is muted, recording a voice ends with what to check instead of a voice that recognition later finds empty.
- **The director splits a chapter without blank lines into pieces the model can hold**: a book whose paragraphs are separated by single line breaks went to the model as one piece, which overran a local model's context or cut its answer short; a long paragraph now splits by lines, a long line by sentences (Chinese and Japanese full stops too), a sentence by its last space.
- **Out of video memory in the Higgs engine says what to do**: a lighter quantization, a shorter block or closing the programs on the card, instead of the engine's line alone.
- **Drama 3 says when its video memory limit is the cause**: a limit set by hand that leaves its expert cache no room is named with the setting, and a card filled by other programs is said as such, instead of the engine's numbers alone.
- **Recognition no longer closes the program on processors without AVX-512** (Intel Core before the 11th generation and the 12th–14th, most AMD before Zen 4): transcribe.cpp engine-0.1.3 is built for AVX2, FMA and F16C instead of the build machine's own instructions, and a processor without them gets a message instead of a crash. The take check, a voice's reference text, cloning from a long reference and Listening all went through it.
- **Voices of the pack with their text in a `.lab` file have their reference text** (English_Female, Arabic_Female and six more): the studio read only `.txt`, so Drama 3 refused them for a missing transcript and the studio tried to recognise text it already had.
- **Drama 3 says which voice lacks its reference text and where to add it** (Voice library → Reference text, or Recognise the reference text), and does so before loading the model; a long voice with no piece of whole words up to the reference length is named as such.
- **Cleaning a voice works on any sample rate**: a voice recorded at 48 or 24 kHz went to the separator as it was, and the separator, which takes 44.1 kHz only, failed with exit code 1.
- **The log is kept on disk and survives a crash** (`logs/speech-studio.log` in the data folder, the previous one beside it as `speech-studio.old.log` past 8 MB): every step of a job, every failed job with its whole error, a panic of any thread and, on Windows, an exception that ends the process with its code and the module it came from. *Report a problem* packs both files.
- **Checking the SHA-256 after a download moves the bar** (bytes checked of the bytes to check) instead of leaving it at 100 %; a failure inside the download thread ends the download with its cause instead of leaving it spinning.
- **A synthesis that timed out or hung names its engine.**
- **The director's verbatim check takes full-width digits and letters (３, Ａ) as the ordinary ones.**
- **Drama 3, Qwen3-TTS and the separator check the processor before they load** (AVX2, FMA, F16C, BMI2) and name what is missing instead of closing the program.
- **Cleaning a voice works in a folder with Cyrillic or other non-ASCII letters in its path**, and a separator failure shows its error line.
- **Higgs voices in a data folder with Cyrillic or other non-ASCII letters in its path**: its engine library could not open such a path and said the model folder does not exist.
- **An engine left over from an earlier version is downloaded again**: an engine or program counts as installed only when its SHA-256 is the pinned one, not when a file of its name and size is there (a build with other processor flags can be the same size).
- **The separator's graphics card build that found no card stops with that cause** instead of cleaning on the processor unnoticed, and the local Gemma build that sees no card says so before loading.
- **CrisperWhisper is not offered Vulkan**: Settings → Models → Recognition shows Vulkan only for Parakeet and Nemotron, and CrisperWhisper with Vulkan chosen earlier says it has no Vulkan build instead of running on CUDA or the processor.
- **The status line at the bottom follows the settings**: a recogniser, a device or a voice model chosen in Settings shows there at once, not after the next start.
- **The diagnostics check the transcribe.cpp builds' libraries too.**
- **Stop ends the engines a job started at once** (separator, llama-server, ffmpeg, yt-dlp): on Windows they were ended through taskkill, which waits as long as WMI hangs, so a stopped job could keep running to its end.
- **An edited reference text is used at once**: the piece cut from a voice was kept by the audio's size and date alone, so a corrected text or new word timings came in only after the audio changed.
- **Assistant streaming.** Text remains intact across network packets, malformed model events report errors, and provider-reported costs are retained when a stream fails.
- **Assistant tool batches.** A batch cannot bypass the chat's tool restrictions and start a recursive assistant request.
- **Link import retries.** A failed Listening-project import preserves the downloaded media for retry and cleans up the unfinished project directory.
- **Stress model errors.** Missing, malformed or non-finite RUAccent outputs now report an error instead of panicking or silently skipping stress marks.
- **Dictionary import errors.** Malformed multipart uploads now report an error even when an earlier part contained a valid dictionary.
- **Noise suppression errors.** Invalid DPDFNet metadata or output sizes now return an error instead of panicking during processing.
- **DLL diagnostics.** Malformed import addresses and unsupported PE headers report an error instead of panicking or passing the check.
- **Credential saves.** Failed key and proxy password replacements clean up their temporary files and preserve the previous value.
- **Reliable file saves.** Concurrent saves of a voice's card, transcript and image no longer share a temporary file.
  Failed writes and blocked replacements preserve the previous file.

## 0.2.4 — 2026-10-21

### Added

- **The time of every step for agents** (`studio_timings`): how long each phrase took to recognise and how soon after the end of your speech its text came, how soon after the agent's text its first sound played, and how long a voice's engine took to load, so the agent can compare recognisers and voices with you.
- **The recogniser that hears is named**: the top bar shows it next to Microphone on, and an agent asked to change it switches it at once, without restarting the microphone, while the Live window shows the new one.

### Fixed

- **Sounds no longer cut the agent off.** A cough, a laugh or a sigh from Drama 3's tags, or a short "uh-huh" reaching the microphone from the speakers, was taken for you interrupting the agent; a phrase of at least two words now interrupts, as in Pipecat and LiveKit.
- **The cross cuts speech off**: ending the conversation also stops what the agent is saying.
- **The voice an agent chose shows in the window**: the Agent's voice panel follows the voice the agent set at your request.
- **The installed app writes its log again**; the diagnostics report showed "0 lines".

## 0.2.3 — 2026-10-21

### Added

- **The top bar shows while the microphone records**: Microphone on, what it is on for (Live, a Listening recording, a voice recording, the microphone test) and for how long, with a cross that turns it off from any screen.
- **Pause in Live** (M): the agent falls silent and does not hear you; press again and the conversation goes on.

### Changed

- **Only the person turns the microphone on.** Live, a Listening recording, a voice recording and the microphone test start only on your own click or key in the studio's window. An agent connected over MCP has no tool that turns the microphone on, and the window does not take its clicks on those buttons; the agent can only pause the conversation or end it.
- **No window, no recording**: when no window of the studio has been open for five seconds, the microphone turns off by itself.

## 0.2.2 — 2026-10-21

### Added

- **Live says the voice is loading.** While the engine of a newly chosen voice loads, the window reads "Loading the voice…"; the engine loads at once on the change, not at the next phrase.
- **Live warns about Parakeet and a chosen language.** When a language is chosen and Parakeet recognises, the window says Parakeet detects the language itself and may write some words in another one, and suggests Nemotron 3.5, which keeps to the chosen language.

### Fixed

- **No strange pauses in Live.** A long phrase of the agent is said whole up to 300 characters instead of being cut at a comma after 120, which gave Qwen3-TTS pauses and a new intonation mid-sentence; a pause an engine leaves inside a phrase lasts at most 0.6 s, as in an export with Even pauses.
- **The aura has no square edge**, and the wave follows the loudness smoothly instead of in steps.

## 0.2.1 — 2026-10-20

### Changed

- **The chosen recognition model is downloaded on first use.** In Listening jobs, take checks and voices it is a step of the job with its progress; in Live it downloads in the background with progress in the top bar, and recognition starts by itself when it is done.
- **A recogniser, a language or a microphone chosen during a conversation applies at once**: the microphone starts again with the new choice.
- **A new voice in a conversation loads its engine at once**, not at the next phrase.
- **Parakeet leaves video memory** after a Listening job and a take check; in Live it stays while Live is on.
- **Turning the microphone off answers the agent briefly**: the recording's number and how many phrases were heard.

### Fixed

- **The agent's phrase is said once**: a defective take is no longer voiced again aloud, and no check after the phrase voices it a second time.
- **An agent turning the microphone on while it is already on in the window joins it** instead of getting "Already recording"; the microphone test in the window gives the microphone up to the conversation.
- **Two studio windows no longer take every connection**: the main window and the widget each hold one event stream, so buttons no longer hang.
- **A phrase the engine cannot say** (a language it does not know, for one) is marked failed with its reason instead of waiting for ever.
- **The bottom line names the model that actually recognises**: its quantization and the graphics card or the processor.
- **Nemotron 3.5 knows 28 languages**; the window said 35.

## 0.2.0 — 2026-10-20

### Live

- **Live mode: talk with an agent by voice.** An agent connected over MCP speaks each finished phrase at once
  (`say_start`, `say_text`), the person answers into the microphone and is recognised phrase by phrase; speaking over
  the agent stops it. Mute with M or Ctrl+Shift+Space from any window (hold to talk), type to the agent, choose the
  recogniser and the agent's voice, the agent's name beside its lines. Ctrl+3 and the command palette open Live, as
  Ctrl+1 and Ctrl+2 open Speech and Listening.
- A new conversation never starts muted: the window drops the last one's mute at the start, and M, the mute button
  and the widget ask the studio to toggle rather than set what the window last saw.
- **The sound window**: microphone, agent's voice and speaker tabs, level meters with the peak held, hear yourself, a
  test chime, the agent's volume.
- **Five looks** (aura, wave, bars, radial, matrix) and a widget — a frameless bar over every other window.

### Speech

- **Drama 3 from 6 GB cards.** When the model does not fit the card, its experts (18 GB) live in RAM (32 GB needed)
  and the card keeps a cache of the ones in use; the sound is the same. Settings → Models → *VRAM for Drama 3*: Auto
  (the whole model when it fits the free VRAM) or 5 to 24 GB by hand. Measured on an RTX 4090: whole model about 8×
  faster than real time, 12 GB 4.7×, 8 GB 3×, 6 GB 2.2×. Engine drama-rs engine-0.1.3.
- **Whisper keeps int8 weights on the card** (int8_float16 by default; only the int8 computes are offered, a saved float16/float32 choice falls back to the default), and a card that fails says so instead of recognising on the processor in silence.
- **A single line with a colon is voiced whole** («Внимание: поезд прибывает»): only two lines or more like «Anna: …» make a dialog, a single one lost its words before the colon as a speaker's name.
- **A stress model that failed to load no longer stops voicing for the rest of the session.** The failure kept a lock
  closed and every later block failed with "stress lock"; it now says why the load failed, and the next block tries
  again. The load itself failed when ONNX Runtime was downloaded after the studio started (on the first-run screen):
  Windows then found another program's older onnxruntime.dll (1.17 in System32); the studio now always names its own.
- **On an NVIDIA card the GPU ONNX Runtime and cuDNN are required.** Skipped on first run, the studio loaded the CPU
  build of ONNX Runtime, which a running process cannot swap, and Supertonic on the card failed until a restart; first
  run now fetches them with the other required parts.
- **The voice pack comes from Dub Studio when it has it.** *Download the voice pack* links the pack's 100 files from
  another program of timoncool's that already has them, each checked by its hash, and downloads only when none has
  the whole pack; that program's own voices are not touched.
- **Voice-pack voices clone in Drama 3.** A reference longer than the engine's limit and without word timings got cut
  without its text, and Drama 3 refused it; the timings are now recognized once, before voicing, and saved beside the
  voice. The typed transcript is kept.
- **Cloud voices play while they voice.** OpenRouter, Gemini, fal and Runware blocks reach the window phrase by phrase
  as each one arrives, as the local engines do, and their takes keep the time to the first sound.
- **Gemini TTS** in parallel standard requests or as a restart-safe Batch at half price, with an auto mode; Game Dub's
  checks of Google takes (whistle, repeats, padding) redo a bad take, each kind switched in the project.
- **Editing**: a director's note on a take voices the next one, *Edit the line* from the player, take edges on the
  waveform, A/B of takes, blind comparison of two engines, mending a failed take where it went wrong, approved blocks
  kept by voice-all, notes on words, words heard differently shown under the block.
- **Text**: an SSML subset, prosody cues coloured over the words, phones, e-mail, sites and codes read right, groups of
  normalisation rules left as written, a passage in another language with its own voice, the director's punctuation
  and expression tags with words kept, a check of the script before voicing.
- **Books and plays**: direct speech apart from the author's words, a character registry with voices picked by gender,
  age and language, one voice for the whole project, footnotes and addresses dropped at import, the book's names into
  the dictionary, a project rebuilt from its edited book keeping what was voiced, a book player page beside the export.
- **Montage**: a track editor on the project's time scale (move with snapping, trim, fades, gain), tracks of music and
  sounds under the voices from files, folders or a song made in YuE2, MiniMax Music3 or ACE-Step Studio; the timeline
  at the bottom, above the player.
- **Export**: subtitles in SRT, VTT, ASS and LRC, sample rate, bit depth and bitrate, voice effects (radio, phone,
  loudspeaker), even piece levels, a file per chapter levelled as one book, tracks per speaker in a ZIP, a CSV of the
  lines, file name templates, a SHA-256 manifest and a recipe of how each file was made.
- **Voices**: record a voice from the picker, trim a reference by its waveform, a character's variants, a voice from
  attributes, a voice's license, rotating pieces of a long reference, what each engine needs from a reference.
- **Quality**: a quality light and report for the project, a corpus of hard Russian phrases as an engine check, a
  runaway limit per language's speech rate, numbers compared as digits in six languages.

### Listening

- **Recognition on transcribe.cpp** (GGUF Q8_0 on the GPU, or its processor build on a computer without an NVIDIA
  card; Settings → Models → Recognition, switched without a restart): Parakeet-TDT v3 and Parakeet Ultra move off ONNX Runtime,
  and **Nemotron 3.5 ASR** (28 languages with punctuation) joins them; the model stays loaded between jobs instead of
  loading for each one. Whisper stays on faster-whisper for its word times. The engine and the models are the ones
  Dub Studio uses, so either program takes them from the other.
- The ONNX recognition models of 0.1.0 (Parakeet v3 and Ultra, Silero VAD) are removed at start: transcribe.cpp does not read them.
- **Live recognition** from the microphone phrase by phrase, a **watched folder** recognised in the background,
  **quick recognition** of a file without the editor, a batch of files saved in several formats at once.
- Subtitles by a broadcaster's rules (Netflix, BBC and others), who talks how much, a repeated phrase in the player,
  numbers as digits in the export, the recogniser chosen by the recording's language.

### Window and agents

- **First run asks only what to have**: voices, recognition (one Whisper model or none) and extras; the engines, runtimes and the build for this computer's card come with them by themselves, named in one line with their size.
- The hardware preset names a card's memory as its box does: an RTX 4090 is 24 GB, not 26 (memory is counted in
  2³⁰-byte gigabytes, as the resource monitor already did); so is the diagnostics report.
- The first-run screen's component ticks and Download buttons carry the component's name, so an agent and a screen reader
  know which one they are.
- Every route and window action is an MCP tool; the agent can be the studio's language model, as in YuE2 and MiniMax.
- A command palette (Ctrl+K), own shortcuts, a simple mode with steps, inputs as tabs, built files on the home screen,
  undo of a project deletion, hints on every control in six languages, button texts that say what happens, a phone
  layout, lists in the theme's colours.
- Models another program of timoncool's has are taken from it before downloading; the hub counts model downloads.
- A project page in six languages with examples of every engine.

## 0.1.0 — 2026-10-15

The first release.

### Speech

- **Five local voice engines**: Higgs Audio v3 (cloning, emotions, sounds), Fish Audio Drama 3 (24 GB cards), Supertonic 3 (ten voices, CPU or GPU), Qwen3-TTS 1.7B (nine voices, cloning, an instruction per block) and Qwen3-TTS with Russian stress (a LoRA applied at run time); cloud voices of OpenRouter, Google Gemini (with half-price Batch), fal.ai and Runware behind a confirmation.
- **Projects**: paragraphs become blocks and `Anna:` lines a dialog; takes, stale tracking, undo, drag to reorder, regex search and replace, notes and markers; an engine and a voice per character and per block.
- **Text for Russian**: number normalisation with cases, stress, «ё» and homographs by RUAccent (the big_poetry model in int8 as an option), a pronunciation dictionary with Balabolka import and a check by ear, SpeechKit-compatible markup.
- **Quality**: every take recognised back and compared word by word, defect checks (clipping, silence, cut-off, end click, repeated phrase, tail, music or noise instead of speech), voice similarity, several takes with the best one picked.
- **Podcasts and books**: the director writes a podcast for N speakers and casts a book chapter by character; EPUB, FB2, PDF, DOCX, HTML, Markdown and TXT import; M4B with chapters, parts for long books, ACX check.
- **Subtitles as a project**: SRT, VTT and ASS cues voiced into their time with the tempo raised ahead, a retry, stretching and the silence before the line.
- **Timeline** of the export with overlaps of interruptions to cut, join or keep.
- **Export**: WAV, FLAC, MP3, OGG, M4A, M4B, subtitles and synced text, per-block files, loudness presets with a true-peak limiter, room tone and breaths, and a check before export (missing, stale, failed, changed file, too long).
- **Voice by description** with Qwen3-TTS VoiceDesign: candidates to hear, one kept in the library.
- **Streaming**: Higgs and Drama 3 play while they voice, with the time to the first sound.

### Listening

- **Recognition** by Parakeet-TDT or Whisper (with a hint), up to eight speakers, long recordings cut at pauses by Silero VAD v6, noise removed by DPDFNet, music by Roformer; several files at once.
- **The transcript**: speakers, word times, karaoke player, search, cut by text, hallucination filter, corrections, export to SRT, VTT, ASS, TXT and JSON.
- **Check a voicing** against its text, **lay a known text** over the recording, **voices from speakers**.

### Voices

- **Library**: a voice pack, 743 Russian voices from the cloud catalogue, microphone recording, files and ZIPs, photo and card, reference transcript recognised once; cleaning by Roformer or DPDFNet with the original kept; the best piece of a reference.

### Everywhere

- **First run and the model manager** with resumable downloads checked against SHA-256, import from a folder, and models taken from the author's other studios.
- **Agents**: MCP with every function and the window, an assistant panel, an OpenAI-compatible audio API.
- **Diagnostics**, a bug-report package without keys, six languages, light and dark theme, a question before closing while tasks run, auto-update, anonymous statistics that can be turned off.
