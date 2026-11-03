import { BASE, coded, postCoded, sendCoded, getCoded, getCodedPolled, type VoiceAge, type VoiceGender } from "./api";

export type Take = {
  n: number; file: string; duration: number; seed: number; temperature: number; key: string; created: number;
  source: string; synth_secs: number; retried: string[];
  qc: QcCheck | null; cost: number | null; estimate: number | null; text: string; direction: string; fit_stretch: number; fit_lead: number; fit_over: boolean; first_ms?: number;
  /** The part of the take the project uses when its edges were moved on the waveform; the file stays whole. */
  cut?: [number, number] | null;
};
export type QcCheck = { text_checked: boolean; wer: number; words: number; inserted: number; deleted: number; substituted: number; heard: string; passed: boolean; similarity: number | null; similar: boolean | null; issues: QcIssue[];
  /** What does not redo the take: loudness off the voice, noise in pauses, `listen`, suspects recognition cleared. */
  notes?: QcNote[]; off?: WordOp[];
  /** The words heard, each with its start and end in the take. */
  timed?: [string, number, number][] };
/** A word the check found said otherwise: missed (`deleted`), extra (`inserted`) or another (`substituted`), as the server compared them. */
export type WordOp = { op: "same" | "inserted" | "deleted" | "substituted"; expected: string | null; heard: string | null };
export type QcNote = "loud" | "quiet" | "pause_noise" | "listen" | "too_long" | "too_short" | "edge" | "long_pause" | "little_speech" | "audio_repeat";
export type QcIssue = "tail" | "stretched" | "gap" | "repeat" | "noise" | "garbled" | "runaway" | "cutoff" | "silence" | "hum" | "clipping" | "whistle" | "click";
export type QcSettings = { asr_check: boolean; strictness: "strict" | "normal" | "lenient"; takes: number; similarity: boolean; heuristics?: string[] };
/** The checks of a take's sound (Game Dub's on Google takes), in the order the settings list them; the last two are costly. */
export const SOUND_CHECKS = ["defect", "clipping", "length", "edge", "long_pause", "little_speech", "loudness", "pause_noise", "audio_repeat", "whistle"] as const;
export const COSTLY_SOUND_CHECKS: readonly string[] = ["audio_repeat", "whistle"];
export type Block = {
  id: string; speaker: string; voice: string | null; text: string; direction: string; pause_ms: number | null; cut_previous?: boolean; engine?: string | null;
  hidden: boolean; takes: Take[]; active: number | null; pinned: number | null; error: { key: string; text: string } | null;
  note: string; marker: Marker | ""; start: number | null; end: number | null;
  /** Blind comparisons of engines on this block. */
  ab?: AbVote[];
  /** The block's own reference clip in the project's refs instead of a voice; whole continues the clip's speech. */
  own_ref?: OwnRef | null;
  /** Notes on words of the block: the quoted words, the note and a marker colour; never read aloud. */
  word_notes?: WordNote[];
  /** Review: approved (its text closed, "voice all again" keeps it) or to fix; empty — not reviewed. */
  review?: Review;
  /** The block's language when it is not the project's; empty — the project's. */
  lang?: string;
  /** The block's own sampling, over its speaker's and the project's. */
  params?: SpeechParams | null;
  /** The block's voice effect: one of VOICE_FX, `none` to take its speaker's off, empty for its speaker's. */
  fx?: string;
  /** The clip's montage from the track editor; the take file stays as it is. */
  mix?: Mix;
};
/** Gain in dB and fades in seconds over the part of the take the clip keeps. */
export type Mix = { gain_db: number; fade_in: number; fade_out: number };
export const FLAT_MIX: Mix = { gain_db: 0, fade_in: 0, fade_out: 0 };
export const MIX_GAIN_DB = [-40, 24] as const;
export const isFlat = (m: Mix | undefined) => !m || (m.gain_db === 0 && m.fade_in === 0 && m.fade_out === 0);
export type Review = "" | "approved" | "fix";
export type WordNote = { quote: string; note: string; marker: Marker | "" };
export type OwnRef = { file: string; text: string; whole: boolean };
export type Marker = "red" | "amber" | "green" | "blue" | "violet";
export const MARKERS: Record<Marker, string> = { red: "#ff6b6b", amber: "#ffc46b", green: "#7ee0c3", blue: "#4cc2ff", violet: "#b8a4ff" };
export type Chapter = { id: string; title: string; blocks: Block[] };
export type SeedMode = "fixed" | "random" | "inc" | "dec";
export type SpeechParams = { temperature: number; top_k: number; top_p: number; seed_mode: SeedMode; seed: number; max_tokens: number; steps: number; speed: number };
export type ParamSpec = { key: "temperature" | "top_k" | "top_p" | "max_tokens" | "steps" | "speed"; kind: "float" | "int"; min: number; max: number; step: number; default: number };
export type EnginePreset = { id: string; name: string; gender: "female" | "male"; native: string; description: string };
export type EngineCaps = {
  id: string; name: string; languages: string[]; params: ParamSpec[]; seed_modes: SeedMode[]; tag_format: string;
  /** Expression tags by category; `scope` wraps a span (`<whisper>…</whisper>`), the rest go at a position. */
  tags: [string, string[]][];
  /** Tag categories and values that go at the start of the sentence; the rest go at a position. */
  sentence_tags: string[];
  /** Tag categories followed, with no space, by the sound's onomatopoeia: `<|sfx:laughter|>Ha-ha,`. */
  glued_tags: string[];
  /** The block's direction reaches the engine. */
  direction: boolean;
  /** The direction reaches the engine only with its own voices (`presets`). */
  direction_presets_only: boolean;
  clone: string; continuation: boolean; streaming: boolean; deterministic_seed: boolean; max_chars: number;
  reads: { stress: boolean; yo: boolean; phonemes: boolean; accent: boolean };
  presets: EnginePreset[];
  needs_voice: boolean;
  /** Clones only from a reference with its text. */
  ref_text_required: boolean;
  /** Its own longest reference, below the studio's setting; null — the setting. */
  ref_max_secs: number | null;
  unavailable: string | null;
};
export type Loudness = "off" | "video" | "podcast" | "acx";
export type SpeechSettings = {
  lang: string; engine: string; params: SpeechParams; voice: string | null;
  pause_same_ms: number; pause_switch_ms: number; loudness: Loudness; qc: QcSettings; auto_stress: boolean; normalize: boolean;
  /** Groups of the normalizer left as written: numbers, dates, time, money, phones, web, codes, roman. */
  normalize_off: string[];
  timeline: boolean; max_stretch: number; lead_into_silence: boolean; room_tone: boolean; breaths: boolean; continuation: boolean; tidy_pauses: boolean; level_pieces: boolean; export_name?: string; clip_name?: string; cloud_model: string; cloud_voice: string; cloud_values: Record<string, unknown>;
  /** The voice of each language a block is in when it is not the project's. */
  lang_voices?: Record<string, string>;
  one_voice?: boolean;
};
export type AcxReport = { passed: boolean; checks: { name: "rms" | "peak" | "noise_floor" | "head" | "tail" | "inner_silence" | "length"; value: number; passed: boolean }[] };
export type DictKind = "word" | "regex" | "phonemes";
export type DictEntry = {
  id: string; enabled: boolean; kind: DictKind; find: string; say: string; case_sensitive: boolean; whole_word: boolean;
  lang: string; voice: string; engine: string; note: string; source: string;
  /** Checked by ear: approved, fix, or empty when not checked. */
  review?: "" | "approved" | "fix";
};
export type PrepLayer = "ssml" | "dictionary" | "normalizer" | "stress" | "engine";
export type PrepStep = { layer: PrepLayer; rule: string; from: string; to: string };
export type Prepared = {
  text: string; engine_text: string; steps: PrepStep[]; unknown: string[];
  ignored: { accent: boolean; phonemes: boolean; stress: boolean; unknown_tags: string[] };
  ignored_phonemes: string[]; broken: [string, string][];
};
export type UnknownWord = { word: string; count: number; why: "unread" | "no_stress" };
export type BlockState = "pending" | "ready" | "stale" | "failed" | "skipped";
export type SpeechProject = {
  kind: "speech"; version: number; name: string; created: number; settings: SpeechSettings;
  cast: Record<string, string>; cast_engines?: Record<string, string>;
  /** A speaker's own sampling, over the project's. */
  cast_params?: Record<string, SpeechParams>; chapters: Chapter[]; dictionary: DictEntry[];
  /** What is known of each character for picking its voice. */
  characters?: Record<string, { gender?: VoiceGender; age?: VoiceAge }>;
  /** Each character's voice effect, when it has one. */
  cast_fx?: Record<string, string>;
  /** A project made from a book: its title, author and cover file. */
  book?: { title: string; author: string; cover: string | null } | null;
  status?: Record<string, { state: BlockState; key: string; fit?: "fits" | "tight" | "impossible" }>;
  /** The book's or the play's own tracks beside the voices, mixed into the export. */
  tracks?: Track[];
};
/** A sound on a track: a file of the project's media, its start on the export's scale, the part kept, its montage. */
export type TrackClip = { id: string; file: string; label: string; start: number; duration: number; cut: [number, number] | null; mix?: Mix; source?: { studio: string; song: string; values: Record<string, unknown> } };
export type Track = { id: string; name: string; gain_db: number; muted: boolean; clips: TrackClip[] };
export type MusicStudio = { id: string; name: string; page: string; running: boolean; fields: import("./cloud").CloudField[] };
export type ProjectCard = { pid: string; kind: string; name: string; mtime: number; created?: number; blocks?: number; ready?: number };
export type ExportFormat = "wav" | "flac" | "mp3" | "ogg" | "m4a" | "m4b";
/** The export's sample rate, bit depth (WAV, FLAC) and bitrate (MP3, OGG, M4A); unset — the format's own. */
export type ExportQuality = { sample_rate?: number; bits?: number; kbps?: number };
export const EXPORT_FORMATS: ExportFormat[] = ["wav", "flac", "mp3", "ogg", "m4a", "m4b"];
export type StressWord = {
  start: number; end: number; word: string; source: "manual" | "dictionary" | "model" | "omograph" | "yo";
  variants?: [string, number][]; doubt: boolean; yo_choice?: string;
};
export type StressResult = { text: string; words: StressWord[]; russian: boolean };
export type BookPreview = {
  id: string; title: string; author: string; lang: string; cover: boolean;
  chapters: { index: number; title: string; chars: number; est_secs: number; head: string; skip: "cover" | "front" | "toc" | "empty" | null }[];
  /** Footnote marks and site addresses taken out of the text. */
  removed: { notes: number; links: number };
  /** The names the text shows, the most frequent first. */
  names: { name: string; count: number }[];
};
export type HistoryTake = {
  pid: string; project: string; block: string; speaker: string; text: string; n: number; file: string; created: number;
  duration: number; synth_secs: number; rtf: number; seed: number; temperature: number; top_k: number; top_p: number; max_tokens: number;
  voice: string; engine: string; source: string; wer: number | null; active: boolean; first_ms: number | null;
};
export type QueuedSynth = { n: number; added: number; scope: { ids: string[] | null; from: string | null; force: boolean; seed: number | null } };
/** The engine that voices a block: its own, its speaker's, the project's. */
/** The voice effects carried over from Game Dub. */
export const VOICE_FX = ["radio", "phone", "loudspeaker"] as const;
/** The effect a block sounds through: its own, else its speaker's; null — none. */
export const fxOf = (p: SpeechProject, b: Block): string | null => {
  const fx = b.fx || p.cast_fx?.[b.speaker] || "";
  return fx && fx !== "none" ? fx : null;
};
export const engineOf = (p: SpeechProject, b: Block) => (p.settings.one_voice ? p.settings.engine : b.engine || p.cast_engines?.[b.speaker] || p.settings.engine);
/** What rebuilding a book project from its edited source keeps, adds and drops. */
export type RebuildStats = { kept: number; added: number; dropped: number; dropped_voiced: number };
/** A built audio file of a Speech project, with the recipe of how it was made when one is beside it. */
export type Render = { pid: string; project: string; file: string; bytes: number; mtime: number; recipe: string | null };
/** What a recipe says about one block of a render. */
export type RecipeBlock = { block: string; speaker: string; text: string; take: number; engine: string; voice: string; seed: number; temperature: number; wer: number | null; start?: number };

/** Something that would go wrong voicing a block (`error`) or not be read as written (`warn`). */
export type LintIssue = { block: string; level: "error" | "warn"; kind: string; detail: unknown };

/** The voice a block speaks with: its own, its language's, its speaker's, the project's; with one voice the project's. */
export const voiceOf = (p: SpeechProject, b: Block) => p.settings.one_voice ? p.settings.voice || null :
  b.voice || (b.lang && b.lang !== p.settings.lang ? p.settings.lang_voices?.[b.lang] : null) || p.cast[b.speaker] || p.settings.voice || null;
export type TimelineBlock = { block: string; speaker: string; start: number; end: number; voiced: boolean; overlap: { with: string; secs: number } | null; cut_previous: boolean;
  /** The block before it in the export, its take file, the take's whole length, the part kept and the clip's montage. */
  prev: string | null; take: string | null; take_secs: number | null; cut: [number, number] | null; mix: Mix };
export type ExportProblem = { block: string; state: BlockState | "missing" | "modified" | "duration" };

/** How the voicing sounds by the check of the active takes; `light` by the share of checked takes that passed. */
/** A side of a blind comparison: an engine, a voice (null — the block's) and the take it made. */
export type AbSide = { engine: string; voice: string | null; take: number };
/** `picked` is the index of the side chosen, null when no difference was heard. */
export type AbVote = { sides: AbSide[]; picked: number | null };
export type QualityLight = "green" | "amber" | "red" | "none";
export type QualityTake = { block: string; chapter: string; speaker: string; take: number; text: string; heard: string; off?: WordOp[]; text_checked: boolean; wer: number; passed: boolean; issues: QcIssue[]; notes: QcNote[]; similarity: number | null; similar: boolean | null };
export type QualityReport = { total: number; ready: number; checked: number; passed: number; failed: number; unchecked: string[]; light: QualityLight; green: number; amber: number; worst: QualityTake[] };

export const speechApi = {
  list: () => getCoded<{ projects: ProjectCard[] }>("/projects"),
  podcast: (body: { topic: string; speakers: { name: string; manner: string }[]; minutes: number; lang: string; name?: string }) => postCoded<{ job_id: string }>("/projects/podcast", body),
  castChapter: (pid: string, chapter: string) => postCoded<{ job_id: string }>(`/projects/${pid}/cast`, { chapter }),
  enrich: (pid: string, ids: string[] | null, mode: "tags" | "punctuation" = "tags") => postCoded<{ job_id: string }>(`/projects/${pid}/enrich`, ids ? { ids, mode } : { mode }),
  create: (body: { name?: string; text: string; corpus?: "ru_hard"; voice?: string | null; lang?: string; split?: "paragraphs" | "lines"; files?: { name: string; text: string }[]; engine?: string; key?: string }) => postCoded<{ project_id: string; project: SpeechProject }>("/projects", body),
  get: (pid: string) => getCoded<SpeechProject>(`/projects/${pid}`),
  put: (pid: string, project: SpeechProject) => {
    const { status: _status, ...body } = project;
    void _status;
    return sendCoded<SpeechProject>("PUT", `/projects/${pid}`, body);
  },
  remove: (pid: string) => sendCoded<{ deleted: string }>("DELETE", `/projects/${pid}`),
  /** Takes back from the recycle bin the project deleted last under this id. */
  restore: (pid: string) => postCoded<{ restored: string }>(`/projects/${pid}/restore`, {}),
  synth: (pid: string, scope: { ids?: string[]; from?: string; force?: boolean; seed?: number }) => postCoded<{ job_id: string; queued?: number }>(`/projects/${pid}/synth`, scope),
  queue: (pid: string) => getCodedPolled<{ queue: QueuedSynth[] }>(`/projects/${pid}/queue`),
  unqueue: (pid: string, n?: number) => sendCoded<{ removed: number }>("DELETE", n == null ? `/projects/${pid}/queue` : `/projects/${pid}/queue/${n}`),
  history: (pid?: string) => getCoded<{ takes: HistoryTake[] }>(pid ? `/history?pid=${pid}` : "/history"),
  lint: (pid: string) => getCoded<{ issues: LintIssue[] }>(`/projects/${pid}/lint`),
  autoCast: (pid: string) => getCoded<{ picks: { speaker: string; voice: string | null }[] }>(`/projects/${pid}/auto-cast`),
  exportCheck: (pid: string) => getCoded<{ problems: ExportProblem[] }>(`/projects/${pid}/export-check`),
  /** Mends the block's failed take where it went wrong, or voices the block again whole. */
  fix: (pid: string, bid: string) => postCoded<{ job_id: string }>(`/projects/${pid}/blocks/${bid}/fix`, {}),
  /** A take of the block by each side's engine and voice, the active take kept; the job's result names the takes. */
  ab: (pid: string, bid: string, sides: { engine: string; voice: string | null }[]) => postCoded<{ job_id: string }>(`/projects/${pid}/blocks/${bid}/ab`, { sides: sides.map((s) => (s.voice ? s : { engine: s.engine })) }),
  quality: (pid: string) => getCoded<QualityReport>(`/projects/${pid}/quality`),
  /** Recognition checks the ready takes now: `ids`, or every one whose words were not checked. */
  check: (pid: string, ids?: string[]) => postCoded<{ job_id: string }>(`/projects/${pid}/check`, ids ? { ids } : {}),
  timeline: (pid: string) => getCoded<{ blocks: TimelineBlock[] }>(`/projects/${pid}/timeline`),
  export: (pid: string, format: ExportFormat, chapter?: string, perBlock = false, partMinutes = 0, allowDuration = false, quality: ExportQuality = {}, perChapter = false, stems = false, allowUnvoiced = false) => postCoded<{ job_id: string }>(`/projects/${pid}/export`, { format, chapter, per_block: perBlock, part_minutes: partMinutes || undefined, per_chapter: perChapter || undefined, stems: stems || undefined, allow_duration: allowDuration || undefined, allow_unvoiced: allowUnvoiced || undefined, ...quality }),
  exportsToFolder: (pid: string, files: string[], folder?: string) => postCoded<{ folder: string | null; copied?: number }>(`/projects/${pid}/exports-to-folder`, folder ? { files, folder } : { files }),
  fromSubtitles: (file: File, name?: string) => {
    const form = new FormData();
    if (name) form.append("name", name);
    form.append("file", file, file.name);
    return fetch(`${BASE}/projects/from-subtitles`, { method: "POST", body: form }).then(coded<{ project_id: string; project: SpeechProject }>);
  },
  /** Sound files onto a track (or a new one called `name`), one after another from `at` or after its last clip. */
  tracksUpload: (pid: string, files: File[], track: string | null, name: string, at: number | null) => {
    const form = new FormData();
    for (const f of files) form.append("file", f, f.name);
    if (track) form.append("track", track);
    if (name) form.append("name", name);
    if (at != null) form.append("at", String(at));
    return fetch(`${BASE}/projects/${pid}/tracks/upload`, { method: "POST", body: form }).then(coded<SpeechProject>);
  },
  musicStudios: () => getCoded<{ studios: MusicStudio[] }>("/music/studios"),
  trackMusic: (pid: string, body: { studio: string; values: Record<string, unknown>; track?: string | null; name?: string; at?: number | null }) => postCoded<{ job_id: string }>(`/projects/${pid}/tracks/music`, body),
  mediaUrl: (pid: string, file: string) => `${BASE}/projects/${pid}/media/${encodeURIComponent(file)}`,
  mediaPeaks: (pid: string, file: string) => getCoded<{ secs: number; per_second: number; peaks: number[] }>(`/projects/${pid}/media/${encodeURIComponent(file)}/peaks`),
  setOwnRef: (pid: string, block: string, file: File, whole: boolean) => {
    const form = new FormData();
    form.append("file", file, file.name);
    if (whole) form.append("whole", "1");
    return fetch(`${BASE}/projects/${pid}/blocks/${block}/ref`, { method: "POST", body: form }).then(coded<SpeechProject>);
  },
  bookPreview: (file: File) => {
    const form = new FormData();
    form.append("file", file, file.name);
    return fetch(`${BASE}/books/preview`, { method: "POST", body: form }).then(coded<BookPreview>);
  },
  bookChapter: (id: string, index: number) => getCoded<{ title: string; text: string }>(`/books/${id}/chapters/${index}`),
  fromBook: (body: { id: string; name?: string; voice: string | null; chapters: { index: number; title?: string; text?: string }[]; names?: string[]; dialogue?: boolean; into?: string; dry_run?: boolean }) =>
    postCoded<{ project_id: string; project?: SpeechProject; stats?: RebuildStats; backup?: string }>("/projects/from-book", body),
  renders: () => getCoded<{ renders: Render[] }>("/renders"),
  exports: (pid: string) => getCoded<{ files: { file: string; bytes: number; mtime: number }[]; dir: string }>(`/projects/${pid}/exports`),
  revealExport: (pid: string, file: string) => postCoded<{ ok: boolean }>(`/projects/${pid}/exports/${encodeURIComponent(file)}/reveal`, {}),
  saveExportAs: (pid: string, file: string) => postCoded<{ path: string | null; bytes?: number }>(`/projects/${pid}/exports/${encodeURIComponent(file)}/save-as`, {}),
  pack: (pid: string, audio: boolean) => postCoded<{ file: string; bytes: number }>(`/projects/${pid}/package`, { audio }),
  importPackage: (file: File) => {
    const form = new FormData();
    form.append("file", file, file.name);
    return fetch(`${BASE}/speechproj/import`, { method: "POST", body: form }).then(coded<{ project_id: string; voices_added: string[] }>);
  },
  engines: () => getCoded<{ engines: EngineCaps[] }>("/engines"),
  stress: (text: string, lang: string) => postCoded<StressResult>("/text/stress", { text, lang }),
  directSpeech: (text: string) => postCoded<{ parts: { speech: boolean; text: string }[]; speaker: string }>("/text/direct-speech", { text }),
  prepare: (text: string, pid?: string, block?: string) => postCoded<{ prepared: Prepared; stress_applied: boolean; stress_missing: boolean }>("/text/prepare", { text, pid, block }),
  dictionary: () => getCoded<{ entries: DictEntry[] }>("/dictionary"),
  saveDictionary: (entries: DictEntry[]) => sendCoded<{ entries: DictEntry[] }>("PUT", "/dictionary", { entries }),
  importDictionary: (file: File) => {
    const form = new FormData();
    form.append("file", file, file.name);
    return fetch(`${BASE}/dictionary/import`, { method: "POST", body: form }).then(coded<{ entries: DictEntry[]; source: string }>);
  },
  dictionaryCounts: (pid: string) => getCoded<{ counts: Record<string, number> }>(`/projects/${pid}/dictionary-counts`),
  unknownWords: (pid: string) => getCoded<{ words: UnknownWord[]; stress_checked: boolean }>(`/projects/${pid}/unknown-words`),
  tryRule: (pid: string, body: { phrase: string; rule: DictEntry; voice: string | null }) => postCoded<{ job_id: string }>(`/projects/${pid}/dictionary/try`, body),
  tryUrl: (pid: string, file: string) => `${BASE}/projects/${pid}/tries/${encodeURIComponent(file)}`,
  /** A take as it sounds in the project: with its voice effect, and with the clip's gain and fades over the kept part. */
  takeUrl: (pid: string, file: string, fx?: string | null, mix?: Mix, cut?: [number, number] | null) => {
    const q = new URLSearchParams();
    if (fx) q.set("fx", fx);
    if (mix && !isFlat(mix)) {
      q.set("gain", String(mix.gain_db)); q.set("fin", String(mix.fade_in)); q.set("fout", String(mix.fade_out));
      if (cut) { q.set("a", String(cut[0])); q.set("z", String(cut[1])); }
    }
    const query = q.toString();
    return `${BASE}/projects/${pid}/takes/${encodeURIComponent(file)}${query ? `?${query}` : ""}`;
  },
  takePeaks: (pid: string, file: string) => getCoded<{ secs: number; per_second: number; peaks: number[] }>(`/projects/${pid}/takes/${encodeURIComponent(file)}/peaks`),
  exportUrl: (pid: string, file: string) => `${BASE}/projects/${pid}/exports/${encodeURIComponent(file)}`,
};

export const activeTake = (b: Block): Take | null => (b.active == null ? null : b.takes.find((t) => t.n === b.active) ?? null);

export const newId = () => crypto.randomUUID().replace(/-/g, "").slice(0, 10);

// readable on both themes: each hue is mixed with the theme's text colour
export const SPEAKER_TINTS = ["color-mix(in oklab, #4cc2ff 60%, var(--color-text))", "color-mix(in oklab, #f7a8c8 60%, var(--color-text))", "color-mix(in oklab, #7ee0c3 60%, var(--color-text))", "color-mix(in oklab, #ffc46b 60%, var(--color-text))", "color-mix(in oklab, #b8a4ff 60%, var(--color-text))", "color-mix(in oklab, #ff8b6b 60%, var(--color-text))", "color-mix(in oklab, #a5b8ff 60%, var(--color-text))", "color-mix(in oklab, #9be36e 60%, var(--color-text))"];

/** Words of a stress pass a person may want to choose: every homograph (the unsure ones first) and е/ё pairs. */
export const stressDoubts = (words: StressWord[]) =>
  words.filter((w) => (w.variants?.length ?? 0) > 1 || !!w.yo_choice).sort((a, b) => Number(b.doubt) - Number(a.doubt) || a.start - b.start);

/** `variant` (lower case, maybe with `+`) in the letter case of `word`. */
export function withCase(word: string, variant: string): string {
  const letters = Array.from(word).filter((c) => c !== "+");
  let i = 0;
  return Array.from(variant).map((c) => {
    if (c === "+") return c;
    const src = letters[i++];
    return src && src !== src.toLowerCase() ? c.toUpperCase() : c;
  }).join("");
}

/** Replace the characters `start..end` of `text` (counted in code points, as the server counts them). */
export function spliceChars(text: string, start: number, end: number, insert: string): string {
  const chars = Array.from(text);
  return [...chars.slice(0, start), ...Array.from(insert), ...chars.slice(end)].join("");
}

export const newEntry = (find = "", say = ""): DictEntry => ({
  id: newId(), enabled: true, kind: "word", find, say, case_sensitive: false, whole_word: true, lang: "", voice: "", engine: "", note: "", source: "manual",
});

/** Seconds as m:ss.s (or h:mm:ss.s) for cue times. */
export function cueTime(t: number): string {
  const h = Math.floor(t / 3600);
  const m = Math.floor((t % 3600) / 60);
  const sec = (t % 60).toFixed(1).padStart(4, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${sec}` : `${m}:${sec}`;
}
