//! Проект Speech: `workspace/<pid>/project.json` вида `speech` — главы из блоков (спикер, голос, текст, указание,
//! пауза после, дубли), состав «спикер → голос» и настройки синтеза. Статус блока выводится, а не хранится: ключ
//! синтеза (текст для синтеза, голос и его файлы, движок, квант, параметры, язык) сравнивается с ключом выбранного
//! дубля — совпал `ready`, нет `stale`, дублей нет `pending`, последняя ошибка на этом ключе `failed`.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::engines::tts_host::Params;

pub(crate) use crate::platform::ids::new_id;
pub(crate) use crate::platform::time::unix_seconds as now;
pub(crate) use crate::platform::workspace::is_pid;

pub const KIND: &str = "speech";
/// Поднимать при смене правил текста или синтеза, меняющей звучание уже озвученного.
const KEY_VERSION: &str = "speech-key-v1";

#[cfg(test)]
mod boundary_tests;
/// Длиннее этого блок режется по предложениям: длинный текст Higgs читает хуже и дольше ждать ошибку.
pub const BLOCK_CHARS: usize = 400;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Take {
    pub n: u32,
    pub file: String,
    pub duration: f64,
    pub seed: u64,
    pub temperature: f64,
    pub key: String,
    pub created: u64,
    pub source: String,
    pub synth_secs: f64,
    pub retried: Vec<String>,
    pub qc: Option<crate::audio::qc::Check>,
    /// Текст блока, по которому озвучен дубль: к нему можно вернуться вместе с дублем.
    pub text: String,
    pub direction: String,
    /// Параметры, с которыми сделан дубль (для истории и повтора).
    pub top_k: u32,
    pub top_p: f64,
    pub max_tokens: u32,
    pub voice: String,
    pub engine: String,
    /// Подгонка под реплику: ускорение, на сколько секунд начат раньше, не влез и с ускорением.
    pub fit_stretch: f64,
    pub fit_lead: f64,
    pub fit_over: bool,
    /// Цена облачного дубля из ответа провайдера (USD), если он её назвал.
    pub cost: Option<f64>,
    /// Оценка цены облачного дубля до запуска (USD).
    pub estimate: Option<f64>,
    /// Сколько миллисекунд от начала синтеза до первого звука, у движков, что отдают звук потоком.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_ms: Option<u64>,
    /// A phrase said aloud: milliseconds from its text arriving to its first sound, the wait the person hears.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heard_ms: Option<u64>,
    /// Длина дубля в склейке с ровными паузами, с.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tidy: Option<f64>,
    /// The part of the take the project uses, start and end in seconds, when its edges were moved on the waveform;
    /// the take's file stays as the model made it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut: Option<[f64; 2]>,
    /// Размер файла дубля при записи; 0 — не записан (дубли старых проектов), файл тогда не сверяется.
    #[serde(skip_serializing_if = "is_zero")]
    pub bytes: u64,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct BlockError {
    pub key: String,
    pub text: String,
}

/// A clip's montage in the track editor (VideoSOS clip controls): gain in dB and fades in seconds.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Mix {
    pub gain_db: f64,
    pub fade_in: f64,
    pub fade_out: f64,
}

impl Mix {
    pub const GAIN_DB: std::ops::RangeInclusive<f64> = -40.0..=24.0;

    pub fn is_flat(&self) -> bool {
        *self == Mix::default()
    }

    /// The samples with the gain, then the fades: linear in amplitude over their length, each no longer than the clip.
    pub fn apply(&self, samples: &mut [f32], sr: u32) {
        if self.is_flat() {
            return;
        }
        let gain = 10f32.powf(self.gain_db as f32 / 20.0);
        let n = samples.len();
        let fade_in = ((self.fade_in * sr as f64) as usize).min(n);
        let fade_out = ((self.fade_out * sr as f64) as usize).min(n);
        for (k, s) in samples.iter_mut().enumerate() {
            let mut g = gain;
            if k < fade_in {
                g *= k as f32 / fade_in as f32;
            }
            if n - k <= fade_out {
                g *= (n - k - 1) as f32 / fade_out as f32;
            }
            *s *= g;
        }
    }
}

/// Свой реф реплики вместо голоса: звук в `refs/` проекта и что в нём сказано. `whole` — клип целиком в контекст
/// движка (продолжение речи), иначе он режется до предела рефа, как голос библиотеки.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct OwnRef {
    pub file: String,
    pub text: String,
    pub whole: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct WordNote {
    pub quote: String,
    pub note: String,
    pub marker: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Block {
    pub id: String,
    pub speaker: String,
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub own_ref: Option<OwnRef>,
    pub text: String,
    pub direction: String,
    pub pause_ms: Option<i64>,
    /// Свой движок блока; нет — движок персонажа или проекта.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// Перебивание (отрицательная пауза) обрывает предыдущую реплику, а не звучит поверх неё.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub cut_previous: bool,
    pub hidden: bool,
    pub takes: Vec<Take>,
    pub active: Option<u32>,
    pub pinned: Option<u32>,
    pub error: Option<BlockError>,
    /// Заметка к блоку: на синтез не влияет.
    pub note: String,
    /// Цвет маркера (`red`, `amber`, `green`, `blue`, `violet`) или пусто.
    pub marker: String,
    /// Blind comparisons of engines on this block: the two sides heard and which one was picked.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ab: Vec<AbVote>,
    /// Ревью блока: `approved` — утверждён (текст закрыт, «озвучить всё» его не трогает), `fix` — на исправление.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub review: String,
    /// Заметки к словам блока: цитата, заметка и цвет маркера (вслух не читаются).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub word_notes: Vec<WordNote>,
    /// Язык блока, когда он не язык проекта (вставка на другом языке); пусто — язык проекта.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lang: String,
    /// Свои параметры синтеза блока; нет — персонажа или проекта.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<crate::engines::tts_host::Params>,
    /// The voice effect of the block (`voice_fx::KINDS`): `none` takes its speaker's off, empty — its speaker's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub fx: String,
    /// The montage of the block's clip: its gain and fades, applied where it sounds, the take staying as it is.
    #[serde(skip_serializing_if = "Mix::is_flat")]
    pub mix: Mix,
    /// Время реплики субтитров, с: в проекте по таймкодам блок звучит в нём.
    pub start: Option<f64>,
    pub end: Option<f64>,
}

/// A side of a blind comparison: an engine and a voice (none — the block's own).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct AbSide {
    pub engine: String,
    pub voice: Option<String>,
    /// The take it made on the block.
    pub take: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct AbVote {
    pub sides: Vec<AbSide>,
    /// The side picked, by its place in `sides`; none — no difference heard.
    pub picked: Option<usize>,
}

pub const MARKERS: [&str; 5] = ["red", "amber", "green", "blue", "violet"];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct QcSettings {
    /// Распознать каждый новый дубль и сравнить с текстом.
    pub asr_check: bool,
    pub strictness: String,
    /// Сколько дублей на блок; больше одного — лучший выбирается проверкой.
    pub takes: u32,
    /// Сверять голос дубля с рефом (WeSpeaker): непохожий — повтор и пометка.
    pub similarity: bool,
    /// The kinds of `audio_flags` the sound of each new take is checked for; the cheap ones unless turned off.
    pub heuristics: Vec<String>,
}

impl Default for QcSettings {
    fn default() -> Self {
        QcSettings { asr_check: false, strictness: "normal".into(), takes: 1, similarity: false, heuristics: crate::audio::flags::CHEAP.iter().map(|k| k.to_string()).collect() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub lang: String,
    pub engine: String,
    pub params: Params,
    pub voice: Option<String>,
    pub pause_same_ms: i64,
    pub pause_switch_ms: i64,
    pub loudness: String,
    pub qc: QcSettings,
    /// Русский текст перед озвучкой проходит разметку ударений и «ё» (stress.rs); движок берёт из неё то, что читает.
    pub auto_stress: bool,
    /// Русский текст перед озвучкой проходит нормализацию: числа, даты, единицы словами (normalize_ru.rs).
    pub normalize: bool,
    /// Группы правил нормализации, которые не читать словами (`normalize_ru::GROUPS`): телефоны, коды…
    pub normalize_off: Vec<String>,
    /// Блоки звучат во времени своих реплик (проект из субтитров), а не друг за другом.
    pub timeline: bool,
    /// Предел ускорения клипа под реплику (как в Dub).
    pub max_stretch: f64,
    /// Не влезающая реплика сначала начинается раньше, в тишину перед ней (до 1 с), и только потом ускоряется.
    pub lead_into_silence: bool,
    /// Паузы склейки — тихий room tone, а не цифровой ноль (у пресета ACX — всегда, с тишиной по краям).
    pub room_tone: bool,
    /// В паузу 0,4–1,8 с между блоками перед следующим ставится тихий вдох (Dub `breath_on`).
    pub breaths: bool,
    /// Сквозная интонация: блок берёт рефом предыдущий дубль того же голоса с его текстом (продолжение речи Higgs).
    pub continuation: bool,
    /// Склейка берёт дубли без тишины по краям и без аномальных пауз внутри (дубль на диске остаётся как есть).
    pub tidy_pauses: bool,
    /// Склейка выравнивает каждый кусок к общей громкости до громкости программы (Dub `normalize_voice`).
    pub level_pieces: bool,
    /// The export's file name by a template (`speech_jobs::EXPORT_NAME_KEYS`); empty — the studio's own names.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub export_name: String,
    /// The per-block export's file names by a template (`speech_jobs::CLIP_NAME_KEYS`); empty — the studio's own.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub clip_name: String,
    /// Движок `openrouter`: модель и голос (пусто — из «Облака»).
    pub cloud_model: String,
    pub cloud_voice: String,
    /// Значения формы облачной модели fal или Runware (голос, скорость…), кроме текста.
    pub cloud_values: serde_json::Map<String, Value>,
    /// The voice of each language a block is written in when it is not the project's: Russian text with English
    /// passages read by an English voice. A block's own voice comes first.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lang_voices: BTreeMap<String, String>,
    /// One voice for the whole project: every block speaks with the project's engine, voice and sampling, while the
    /// cast, the blocks' own voices and the voices by language are kept for going back to all voices.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub one_voice: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { lang: "auto".into(), engine: "higgs".into(), params: Params::default(), voice: None, pause_same_ms: 300, pause_switch_ms: 600, loudness: "video".into(), qc: QcSettings::default(), auto_stress: true, normalize: true, normalize_off: Vec::new(), timeline: false, max_stretch: 1.25, lead_into_silence: true, room_tone: false, breaths: false, continuation: false, tidy_pauses: true, level_pieces: true, export_name: String::new(), clip_name: String::new(), cloud_model: String::new(), cloud_voice: String::new(), cloud_values: serde_json::Map::new(), lang_voices: BTreeMap::new(), one_voice: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct SpeechProject {
    pub kind: String,
    pub version: u32,
    pub name: String,
    pub created: u64,
    pub settings: Settings,
    pub cast: BTreeMap<String, String>,
    /// Движок персонажа, когда он не тот, что у проекта (только локальные движки: у облака одна настройка на проект).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub cast_engines: BTreeMap<String, String>,
    /// Параметры синтеза персонажа, когда они не те, что у проекта; блок может задать свои поверх.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub cast_params: BTreeMap<String, crate::engines::tts_host::Params>,
    /// What is known of each character for picking its voice: gender and age, from the director's registry or the user.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub characters: BTreeMap<String, Character>,
    /// The voice effect of each character that has one (`voice_fx::KINDS`): every line of theirs sounds through it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cast_fx: BTreeMap<String, String>,
    pub chapters: Vec<Chapter>,
    /// Словарь произношения проекта: срабатывает раньше глобального.
    pub dictionary: Vec<crate::text::pron_dict::Entry>,
    /// Проект из книги: название, автор и файл обложки в папке проекта — в метаданные и обложку аудиокниги.
    pub book: Option<BookMeta>,
    /// Ключ инструмента агента «одним вызовом»: повторный вызов с теми же входами находит этот проект.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub key: String,
    /// The book's or the play's own tracks beside the voices — music, sounds, ambience — mixed into the export.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracks: Vec<Track>,
}

/// A track of the book or play: its clips on the export's time scale, its gain over theirs, muted or not.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub gain_db: f64,
    pub muted: bool,
    pub clips: Vec<TrackClip>,
}

/// A sound on a track: a copy in the project's `media/`, where it starts in the export, the part of it kept and its
/// montage; a song made in a music studio keeps where it came from (`source`: the studio, the song, the form's values).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct TrackClip {
    pub id: String,
    pub file: String,
    pub label: String,
    pub start: f64,
    pub duration: f64,
    pub cut: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Mix::is_flat")]
    pub mix: Mix,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Value>,
}

impl TrackClip {
    /// The part of the file the clip plays, in seconds.
    pub fn kept(&self) -> [f64; 2] {
        self.cut.unwrap_or([0.0, self.duration])
    }
}

/// A character's gender (`female`, `male`) and age (`child`, `teen`, `adult`, `senior`), each empty when unknown.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Character {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub gender: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub age: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct BookMeta {
    pub title: String,
    pub author: String,
    pub cover: Option<String>,
}

impl Default for SpeechProject {
    fn default() -> Self {
        SpeechProject { kind: KIND.into(), version: 1, name: String::new(), created: 0, settings: Settings::default(), cast: BTreeMap::new(), cast_engines: BTreeMap::new(), cast_params: BTreeMap::new(), characters: BTreeMap::new(), cast_fx: BTreeMap::new(), chapters: Vec::new(), dictionary: Vec::new(), book: None, key: String::new(), tracks: Vec::new() }
    }
}

/// Проект по таймкодам из реплик субтитров: реплика — блок; говорящий — из файла или рассказчик.
pub fn from_cues(name: &str, cues: &[crate::text::subimport::Cue], narrator: &str) -> SpeechProject {
    let blocks = cues
        .iter()
        .map(|c| Block {
            id: new_id(),
            speaker: c.speaker.clone().unwrap_or_else(|| narrator.to_string()),
            text: c.text.clone(),
            start: Some(c.start),
            end: Some(c.end),
            ..Default::default()
        })
        .collect();
    let mut project = SpeechProject { name: name.to_string(), created: now(), ..Default::default() };
    project.settings.timeline = true;
    project.chapters.push(Chapter { id: new_id(), title: String::new(), blocks });
    project
}

pub struct EngineRule<'a> {
    pub id: &'a str,
    pub cloud: bool,
}

pub struct ValidationRules<'a> {
    pub engines: &'a [EngineRule<'a>],
    pub genders: &'a [&'a str],
    pub ages: &'a [&'a str],
}

impl SpeechProject {
    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.chapters.iter().flat_map(|c| c.blocks.iter())
    }

    pub fn block_mut(&mut self, id: &str) -> Option<&mut Block> {
        self.chapters.iter_mut().flat_map(|c| c.blocks.iter_mut()).find(|b| b.id == id)
    }

    /// Names the library voice `from` as `to` wherever the project speaks with it or made a take with it; whether it
    /// did anywhere.
    pub fn rename_voice(&mut self, from: &str, to: &str) -> bool {
        let mut changed = false;
        let mut swap = |v: &mut String| {
            if v == from {
                *v = to.to_string();
                changed = true;
            }
        };
        if let Some(v) = self.settings.voice.as_mut() {
            swap(v);
        }
        self.settings.lang_voices.values_mut().for_each(&mut swap);
        self.cast.values_mut().for_each(&mut swap);
        for b in self.chapters.iter_mut().flat_map(|c| c.blocks.iter_mut()) {
            if let Some(v) = b.voice.as_mut() {
                swap(v);
            }
            b.takes.iter_mut().for_each(|t| swap(&mut t.voice));
        }
        changed
    }

    /// Движок, что озвучивает блок: свой блока, персонажа, проекта; в режиме одного голоса — проекта.
    pub fn engine_of<'a>(&'a self, b: &'a Block) -> &'a str {
        if self.settings.one_voice {
            return &self.settings.engine;
        }
        b.engine.as_deref().filter(|e| !e.is_empty()).or_else(|| self.cast_engines.get(&b.speaker).map(String::as_str).filter(|e| !e.is_empty())).unwrap_or(&self.settings.engine)
    }

    /// Голос блока: свой, иначе по языку блока, иначе спикера по составу, иначе голос проекта; `None` — базовый голос
    /// движка. В режиме одного голоса — голос проекта.
    pub fn voice_of(&self, b: &Block) -> Option<String> {
        if self.settings.one_voice {
            return self.settings.voice.clone().filter(|v| !v.is_empty());
        }
        let own_lang = (!b.lang.is_empty() && b.lang != self.settings.lang).then(|| self.settings.lang_voices.get(&b.lang)).flatten();
        b.voice
            .clone()
            .filter(|v| !v.is_empty())
            .or_else(|| own_lang.cloned().filter(|v| !v.is_empty()))
            .or_else(|| self.cast.get(&b.speaker).cloned().filter(|v| !v.is_empty()))
            .or_else(|| self.settings.voice.clone().filter(|v| !v.is_empty()))
    }

    /// The sampling the block is voiced with: its own, else its speaker's, else the project's; with one voice the
    /// project's.
    pub fn params_of<'a>(&'a self, b: &'a Block) -> &'a crate::engines::tts_host::Params {
        if self.settings.one_voice {
            return &self.settings.params;
        }
        b.params.as_ref().or_else(|| self.cast_params.get(&b.speaker)).unwrap_or(&self.settings.params)
    }

    /// The voice effect the block sounds through: its own, else its speaker's; `None` — none.
    pub fn fx_of<'a>(&'a self, b: &'a Block) -> Option<&'a str> {
        let own = Some(b.fx.as_str()).filter(|f| !f.is_empty());
        own.or_else(|| self.cast_fx.get(&b.speaker).map(String::as_str)).filter(|f| *f != "none")
    }

    /// The block's language: its own when it has one, else the project's (`auto` included).
    pub fn lang_of<'a>(&'a self, b: &'a Block) -> &'a str {
        if b.lang.is_empty() { &self.settings.lang } else { &b.lang }
    }

    /// Ид блоков и глав уникальны, у каждого блока есть ид, пустые главы допустимы, ссылки на дубли верны.
    pub fn validate(&self, rules: &ValidationRules) -> Result<(), String> {
        if self.kind != KIND {
            return Err(format!("this is a {} project, not a speech one", self.kind));
        }
        self.settings.params.checked()?;
        let mut seen_tracks = std::collections::HashSet::new();
        for track in &self.tracks {
            if track.id.is_empty() || !seen_tracks.insert(track.id.as_str()) {
                return Err(format!("track id {:?} is empty or repeated", track.id));
            }
            if !Mix::GAIN_DB.contains(&track.gain_db) {
                return Err(format!("track {}: gain {} dB must be {}..{}", track.name, track.gain_db, Mix::GAIN_DB.start(), Mix::GAIN_DB.end()));
            }
            for clip in &track.clips {
                let [a, z] = clip.kept();
                let mix_ok = Mix::GAIN_DB.contains(&clip.mix.gain_db) && clip.mix.fade_in >= 0.0 && clip.mix.fade_out >= 0.0;
                if clip.id.is_empty() || !seen_tracks.insert(clip.id.as_str()) || !super::naming::safe_file(&clip.file) || !(clip.start >= 0.0) || !(a >= 0.0 && z > a && z <= clip.duration + 1e-3) || !mix_ok {
                    return Err(format!("track {}: clip {:?} ({}) needs its own id, a file of media/, a start not below 0, a cut inside its {} s and a gain {}..{} dB", track.name, clip.id, clip.file, clip.duration, Mix::GAIN_DB.start(), Mix::GAIN_DB.end()));
                }
            }
        }
        if let Some(b) = self.blocks().find(|b| !Mix::GAIN_DB.contains(&b.mix.gain_db) || !(b.mix.fade_in >= 0.0 && b.mix.fade_out >= 0.0 && b.mix.fade_in.is_finite() && b.mix.fade_out.is_finite())) {
            return Err(format!("block {}: gain {} dB must be {}..{} and fades {} s, {} s not below 0", b.id, b.mix.gain_db, Mix::GAIN_DB.start(), Mix::GAIN_DB.end(), b.mix.fade_in, b.mix.fade_out));
        }
        super::naming::name_keys(&self.settings.export_name, &super::naming::EXPORT_NAME_KEYS)?;
        super::naming::name_keys(&self.settings.clip_name, &super::naming::CLIP_NAME_KEYS)?;
        for (speaker, params) in &self.cast_params {
            params.checked().map_err(|e| format!("speaker {speaker}: {e}"))?;
        }
        if let Some((who, fx)) = self.cast_fx.iter().find(|(_, f)| !crate::voices::fx::KINDS.contains(&f.as_str())) {
            return Err(format!("speaker {who}: voice effect {fx} is not one of {:?}", crate::voices::fx::KINDS));
        }
        if let Some(b) = self.blocks().find(|b| !b.fx.is_empty() && b.fx != "none" && !crate::voices::fx::KINDS.contains(&b.fx.as_str())) {
            return Err(format!("block {}: voice effect {} is not one of {:?} or none", b.id, b.fx, crate::voices::fx::KINDS));
        }
        for (name, c) in &self.characters {
            if !rules.genders.contains(&c.gender.as_str()) || !rules.ages.contains(&c.age.as_str()) {
                return Err(format!("character {name}: gender is one of {:?} and age one of {:?}", rules.genders, rules.ages));
            }
        }
        if let Some(g) = self.settings.normalize_off.iter().find(|g| !crate::text::normalize_ru::GROUPS.contains(&g.as_str())) {
            return Err(format!("normalize_off: {g} is not one of {}", crate::text::normalize_ru::GROUPS.join(", ")));
        }
        if !(1.0..=2.0).contains(&self.settings.max_stretch) {
            return Err(format!("max_stretch {} is outside 1.0..2.0", self.settings.max_stretch));
        }
        crate::text::pron_dict::validate(&self.dictionary)?;
        let mut seen = std::collections::HashSet::new();
        for c in &self.chapters {
            if c.id.is_empty() || !seen.insert(format!("c:{}", c.id)) {
                return Err(format!("chapter id {:?} is empty or repeated", c.id));
            }
            for b in &c.blocks {
                if b.id.is_empty() || !seen.insert(format!("b:{}", b.id)) {
                    return Err(format!("block id {:?} is empty or repeated", b.id));
                }
                for n in [b.active, b.pinned].into_iter().flatten() {
                    if !b.takes.iter().any(|t| t.n == n) {
                        return Err(format!("block {} points at take {n}, which it does not have", b.id));
                    }
                }
                if let Some(t) = b.takes.iter().find(|t| t.cut.is_some_and(|[a, z]| !(a >= 0.0 && z > a + 0.05 && z <= t.duration + 0.05))) {
                    return Err(format!("block {} take {}: cut is a start and an end inside 0..{} seconds", b.id, t.n, t.duration));
                }
                if let (Some(a), Some(z)) = (b.start, b.end) {
                    if !(a >= 0.0 && z > a) {
                        return Err(format!("block {} cue {a}..{z} is not a forward span", b.id));
                    }
                }
                if !b.marker.is_empty() && !MARKERS.contains(&b.marker.as_str()) {
                    return Err(format!("block {} marker {:?} is not one of {MARKERS:?}", b.id, b.marker));
                }
                if !(b.lang.is_empty() || (2..=3).contains(&b.lang.len()) && b.lang.chars().all(|c| c.is_ascii_lowercase())) {
                    return Err(format!("block {} lang {:?} is not a language code such as en", b.id, b.lang));
                }
                if let Some(params) = &b.params {
                    params.checked().map_err(|e| format!("block {}: {e}", b.id))?;
                }
                if !matches!(b.review.as_str(), "" | "approved" | "fix") {
                    return Err(format!("block {} review {:?} is not approved, fix or none", b.id, b.review));
                }
                if let Some(w) = b.word_notes.iter().find(|w| w.quote.trim().is_empty() || (!w.marker.is_empty() && !MARKERS.contains(&w.marker.as_str()))) {
                    return Err(format!("block {} word note {:?}: a quote and a marker of {MARKERS:?} or none", b.id, w.quote));
                }
                if let Some(e) = b.engine.as_deref().filter(|e| !e.is_empty()) {
                    own_engine_ok(e, &self.settings.engine, rules).map_err(|why| format!("block {}: {why}", b.id))?;
                }
                if let Some(p) = b.pause_ms {
                    if !(-2000..=10_000).contains(&p) {
                        return Err(format!("block {} pause {p} ms is outside -2000..10000", b.id));
                    }
                }
            }
        }
        for (speaker, e) in self.cast_engines.iter().filter(|(_, e)| !e.is_empty()) {
            own_engine_ok(e, &self.settings.engine, rules).map_err(|why| format!("speaker {speaker}: {why}"))?;
        }
        for p in [self.settings.pause_same_ms, self.settings.pause_switch_ms] {
            if !(-2000..=10_000).contains(&p) {
                return Err(format!("pause {p} ms is outside -2000..10000"));
            }
        }
        if !matches!(self.settings.qc.strictness.as_str(), "strict" | "normal" | "lenient") {
            return Err(format!("strictness {} is not one of strict, normal, lenient", self.settings.qc.strictness));
        }
        if !(1..=20).contains(&self.settings.qc.takes) {
            return Err(format!("takes per block {} is outside 1..20", self.settings.qc.takes));
        }
        if !matches!(self.settings.loudness.as_str(), "off" | "video" | "podcast" | "acx") {
            return Err(format!("loudness preset {} is not one of off, video, podcast, acx", self.settings.loudness));
        }
        Ok(())
    }
}

/// Начало видимого текста блока для строки прогресса.
pub fn visible_snippet(text: &str) -> String {
    let v = crate::speech::text::visible(text);
    let mut out: String = v.chars().take(40).collect();
    if v.chars().count() > 40 {
        out.push('…');
    }
    out
}

static DIALOG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(?:\[([^\[\]\n]{1,40})\]\s+|([\p{L}][\p{L}\p{N} ._'-]{0,30}?)\s*(?::\s+|：\s*))(\S.*)$").expect("dialog"));
static SENTENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^.!?…]+[.!?…]+[»”\x22)]*\s*|[^.!?…]+$").expect("sentence"));

/// Длинный абзац — по предложениям в куски не длиннее `BLOCK_CHARS`.
fn chunk(text: &str) -> Vec<String> {
    let text = text.trim();
    if text.chars().count() <= BLOCK_CHARS {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for s in SENTENCE.find_iter(text).map(|m| m.as_str().trim()).filter(|s| !s.is_empty()) {
        if !cur.is_empty() && cur.chars().count() + 1 + s.chars().count() > BLOCK_CHARS {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(s);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Текст → блоки (спикер, текст). Диалог — когда не меньше половины строк начинаются с `Имя:` или `[Имя]` и
/// спикеров не больше восьми; иначе абзацы рассказчика, длинные — по предложениям. В диалоге строка без имени сразу
/// под репликой — её продолжение, а после пустой строки — абзац рассказчика.
pub fn split_text(text: &str, narrator: &str) -> Vec<(String, String)> {
    split_text_lines(text, narrator, None)
}

/// `split_text`, and with `lines` the direct speech of each narrator paragraph (`direct_speech`) goes to that speaker,
/// the author's words staying the narrator's.
pub fn split_text_lines(text: &str, narrator: &str, lines: Option<&str>) -> Vec<(String, String)> {
    if let Some(tags) = dialog_tags(text) {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut after_break = true;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                after_break = true;
                continue;
            }
            match tags.get(line).cloned().flatten() {
                Some((s, t)) => out.extend(chunk(&t).into_iter().map(|c| (s.clone(), c))),
                None => match out.last_mut() {
                    Some(last) if !after_break => {
                        last.1.push(' ');
                        last.1.push_str(line);
                    }
                    _ => out.extend(chunk(line).into_iter().map(|c| (narrator.to_string(), c))),
                },
            }
            after_break = false;
        }
        return out;
    }
    text.split("\n\n")
        .flat_map(|p| {
            let p = p.split_whitespace().collect::<Vec<_>>().join(" ");
            let parts = match lines {
                Some(who) => crate::text::direct_speech::split(&p).into_iter().map(|(speech, t)| (if speech { who } else { narrator }, t)).collect(),
                None => vec![(narrator, p)],
            };
            parts
                .into_iter()
                .flat_map(|(who, t)| chunk(&t).into_iter().filter(|c| !c.is_empty()).map(|c| (who.to_string(), c)).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Строки текста с их спикером, если текст — диалог (хотя бы две строки с `Имя:` или `[Имя]` и не меньше половины
/// непустых строк, не больше восьми спикеров): строка → `Some((спикер, реплика))` или `None` для строки без имени.
/// Одна строка «Слово: текст» — фраза, а не диалог («Внимание: поезд прибывает»).
fn dialog_tags(text: &str) -> Option<std::collections::HashMap<&str, Option<(String, String)>>> {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let tags: std::collections::HashMap<&str, Option<(String, String)>> = lines
        .iter()
        .map(|l| (*l, DIALOG.captures(l).map(|c| (c.get(1).or_else(|| c.get(2)).map(|m| m.as_str().trim().to_string()).unwrap_or_default(), c[3].trim().to_string()))))
        .collect();
    let tagged = lines.iter().filter(|l| tags.get(*l).is_some_and(Option::is_some)).count();
    let speakers: std::collections::BTreeSet<&str> = tags.values().flatten().map(|(s, _)| s.as_str()).collect();
    (tagged >= 2 && tagged * 2 >= lines.len() && speakers.len() <= 8).then_some(tags)
}

/// Новый проект из текста: одна глава, блоки по `blocks_of`.
pub fn from_text(name: &str, text: &str, narrator: &str) -> SpeechProject {
    SpeechProject {
        name: name.to_string(),
        created: now(),
        chapters: vec![Chapter { id: new_id(), title: String::new(), blocks: blocks_of(text, narrator, None) }],
        ..Default::default()
    }
}

/// Пакет: каждая непустая строка — отдельный клип (блок), без нарезки; в диалоге строка `Имя: …` — реплика этого
/// спикера, как в разборе по абзацам.
pub fn from_lines(name: &str, text: &str, narrator: &str) -> SpeechProject {
    let tags = dialog_tags(text);
    let blocks = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let (speaker, said) = tags.as_ref().and_then(|t| t.get(l).cloned().flatten()).unwrap_or_else(|| (narrator.to_string(), l.to_string()));
            Block { id: new_id(), speaker, text: said, ..Default::default() }
        })
        .collect();
    SpeechProject { name: name.to_string(), created: now(), chapters: vec![Chapter { id: new_id(), title: String::new(), blocks }], ..Default::default() }
}

/// Пакет из файлов: текст файла — блок, имя файла — заметка блока и имя его клипа при экспорте.
pub fn from_files(name: &str, files: &[(String, String)], narrator: &str) -> SpeechProject {
    let blocks = files
        .iter()
        .filter(|(_, t)| !t.trim().is_empty())
        .map(|(file, t)| Block { id: new_id(), speaker: narrator.to_string(), text: t.trim().to_string(), note: file.clone(), ..Default::default() })
        .collect();
    SpeechProject { name: name.to_string(), created: now(), chapters: vec![Chapter { id: new_id(), title: String::new(), blocks }], ..Default::default() }
}

/// Блок, чей дубль служит рефом продолжения: ближайший видимый выше с тем же голосом и выбранным дублем.
pub fn continuation_source<'a>(project: &'a SpeechProject, b: &Block) -> Option<(&'a Block, &'a Take)> {
    if !project.settings.continuation {
        return None;
    }
    let voice = project.voice_of(b);
    let blocks: Vec<&Block> = project.blocks().collect();
    let at = blocks.iter().position(|x| x.id == b.id)?;
    blocks[..at]
        .iter()
        .rev()
        .filter(|x| !x.hidden && !says_nothing(&x.text))
        .find(|x| project.voice_of(x) == voice)
        .and_then(|x| x.active.and_then(|n| x.takes.iter().find(|t| t.n == n)).map(|t| (*x, t)))
}

/// Ключ синтеза блока: всё, от чего зависит звук. `engine` — движок и его модель с квантом.
pub fn synth_key(project: &SpeechProject, b: &Block, voice_tag: &str, engine: &str, ref_limit: f64, prep: &str) -> String {
    let p = project.params_of(b);
    let seed = if p.seed_mode == "fixed" { p.seed.to_string() } else { "random".into() };
    let mut h = blake3::Hasher::new();
    for part in [
        KEY_VERSION,
        b.text.trim(),
        b.direction.trim(),
        voice_tag,
        engine,
        &format!("{:.3}|{}|{:.3}|{seed}|{ref_limit:.1}", p.temperature, p.top_k, p.top_p),
        project.lang_of(b),
    ] {
        h.update(part.as_bytes());
        h.update(b"\x1f");
    }
    if project.settings.auto_stress && crate::text::stress::is_russian(&b.text, &project.settings.lang) {
        h.update(b"auto-stress\x1f");
    }
    if let Some(r) = &b.own_ref {
        h.update(format!("own-ref:{}:{}:{}\x1f", r.file, r.text.trim(), r.whole).as_bytes());
    } else if let Some((src, take)) = continuation_source(project, b) {
        h.update(format!("continue:{}:{}\x1f", src.id, take.file).as_bytes());
    }
    if !prep.is_empty() {
        h.update(prep.as_bytes());
        h.update(b"\x1f");
    }
    if let (true, Some(a), Some(z)) = (project.settings.timeline, b.start, b.end) {
        h.update(format!("cue:{a:.3}:{z:.3}:{:.2}:{}\x1f", project.settings.max_stretch, project.settings.lead_into_silence).as_bytes());
    }
    h.finalize().to_hex()[..20].to_string()
}

pub fn status(b: &Block, key: &str) -> &'static str {
    if says_nothing(&b.text) || b.hidden {
        return "skipped";
    }
    if let Some(e) = &b.error {
        if e.key == key {
            return "failed";
        }
    }
    match b.active.and_then(|n| b.takes.iter().find(|t| t.n == n)) {
        None if b.takes.is_empty() => "pending",
        None => "stale",
        Some(t) if t.key == key => "ready",
        Some(_) => "stale",
    }
}

/// Whether a text has not a letter or a digit to say: empty, or a scene break such as `* * *` or `—`. Voicing makes
/// nothing of it, so it is never waited for.
pub fn says_nothing(text: &str) -> bool {
    !text.chars().any(char::is_alphanumeric)
}

/// The pause after the block before a scene break (`* * *`), which is no block of its own.
pub const SCENE_BREAK_MS: i64 = 1500;

/// The blocks of a text as `split_text_lines` cuts it; a line with nothing to say is no block: the block before it
/// keeps a scene's pause instead.
pub fn blocks_of(text: &str, narrator: &str, lines: Option<&str>) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for (speaker, text) in split_text_lines(text, narrator, lines) {
        if says_nothing(&text) {
            if let Some(prev) = out.last_mut() {
                prev.pause_ms = Some(prev.pause_ms.unwrap_or(0).max(SCENE_BREAK_MS));
            }
            continue;
        }
        out.push(Block { id: new_id(), speaker, text, ..Default::default() });
    }
    out
}

/// Проект для окна: сам проект и по каждому блоку его ключ и статус.
/// Движок блока или персонажа: известный и локальный — облачный только тот, что выбран у проекта.
fn own_engine_ok(engine: &str, project_engine: &str, rules: &ValidationRules) -> Result<(), String> {
    match rules.engines.iter().find(|e| e.id == engine) {
        None => Err(format!("engine {engine:?} is not one of the studio's engines")),
        Some(e) if e.cloud && engine != project_engine => Err(format!("cloud engine {engine:?} is set up for the whole project only")),
        Some(_) => Ok(()),
    }
}

/// Прогноз до синтеза для блока по таймкодам: `fits`, `tight` (уложится ускорением) или `impossible` — по
/// оценке длины текста при темпе языка (механизм `dub_core::fit::verdict`).
pub fn fit_forecast(project: &SpeechProject, b: &Block) -> Option<&'static str> {
    let (true, Some(start), Some(end)) = (project.settings.timeline, b.start, b.end) else { return None };
    let timed: Vec<&Block> = project.blocks().filter(|x| !x.hidden && x.start.is_some()).collect();
    let next = timed.iter().position(|x| x.id == b.id).and_then(|i| timed.get(i + 1)).and_then(|x| x.start).unwrap_or(end + 3600.0);
    let rules = dub_core::fit::FitRules { speech_rate_on: false, qc_duration: true, max_stretch: project.settings.max_stretch, lead_into_silence: false };
    let slot = dub_core::fit::slot(start, end, start, next, &rules);
    let lang = if project.settings.lang == "auto" { if crate::text::stress::is_russian(&b.text, "auto") { "ru" } else { "en" } } else { project.settings.lang.as_str() };
    let est = dub_core::fit::estimate(&crate::speech::text::visible(&b.text), dub_core::fit::table_cps(lang));
    Some(match dub_core::fit::verdict(est, &slot, &rules) {
        dub_core::fit::Verdict::Fits => "fits",
        dub_core::fit::Verdict::Tight => "tight",
        dub_core::fit::Verdict::Impossible => "impossible",
    })
}

pub fn with_status(project: &SpeechProject, keys: &BTreeMap<String, String>) -> Value {
    let mut v = serde_json::to_value(project).unwrap_or(Value::Null);
    let st: serde_json::Map<String, Value> = project
        .blocks()
        .map(|b| {
            let key = keys.get(&b.id).cloned().unwrap_or_default();
            let mut entry = json!({ "state": status(b, &key), "key": key });
            if let Some(fit) = fit_forecast(project, b) {
                entry["fit"] = json!(fit);
            }
            (b.id.clone(), entry)
        })
        .collect();
    v["status"] = Value::Object(st);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_voice_speaks_every_block_with_the_project_s_and_keeps_the_cast() {
        let mut p = SpeechProject::default();
        p.settings.voice = Some("Рассказчик".into());
        p.cast.insert("Олег".into(), "Ливанов".into());
        p.cast_engines.insert("Олег".into(), "drama3".into());
        p.cast_params.insert("Олег".into(), crate::engines::tts_host::Params { temperature: 0.6, ..Default::default() });
        let b = Block { id: "b".into(), speaker: "Олег".into(), voice: Some("Свой".into()), text: "Раз.".into(), ..Default::default() };
        assert_eq!((p.voice_of(&b).as_deref(), p.engine_of(&b), p.params_of(&b).temperature), (Some("Свой"), "drama3", 0.6));
        p.settings.one_voice = true;
        assert_eq!((p.voice_of(&b).as_deref(), p.engine_of(&b), p.params_of(&b).temperature), (Some("Рассказчик"), "higgs", 1.0));
        assert_eq!(p.cast.get("Олег").map(String::as_str), Some("Ливанов"));
    }

    #[test]
    fn sampling_comes_from_the_block_then_the_speaker_then_the_project_and_keys_the_take() {
        let mut p = SpeechProject::default();
        let mut b = Block { id: "b".into(), speaker: "Олег".into(), text: "Раз.".into(), ..Default::default() };
        let key = |p: &SpeechProject, b: &Block| synth_key(p, b, "v", "higgs", 12.0, "");
        let plain = key(&p, &b);
        assert_eq!(p.params_of(&b).temperature, 1.0);
        p.cast_params.insert("Олег".into(), crate::engines::tts_host::Params { temperature: 0.6, ..Default::default() });
        assert_eq!(p.params_of(&b).temperature, 0.6);
        let by_speaker = key(&p, &b);
        b.params = Some(crate::engines::tts_host::Params { temperature: 1.3, ..Default::default() });
        assert_eq!(p.params_of(&b).temperature, 1.3);
        assert!(plain != by_speaker && by_speaker != key(&p, &b));
    }

    #[test]
    fn a_block_in_another_language_speaks_with_that_language_s_voice() {
        let mut p = SpeechProject::default();
        p.settings.lang = "ru".into();
        p.settings.voice = Some("Анна".into());
        p.settings.lang_voices.insert("en".into(), "Emma".into());
        let mut b = Block { speaker: "Рассказчик".into(), text: "Hello there.".into(), lang: "en".into(), ..Default::default() };
        assert_eq!((p.voice_of(&b).as_deref(), p.lang_of(&b)), (Some("Emma"), "en"));
        b.voice = Some("Boris".into());
        assert_eq!(p.voice_of(&b).as_deref(), Some("Boris"));
        let ru = Block { speaker: "Рассказчик".into(), text: "Привет.".into(), ..Default::default() };
        assert_eq!((p.voice_of(&ru).as_deref(), p.lang_of(&ru)), (Some("Анна"), "ru"));
    }

    #[test]
    fn paragraphs_become_blocks_and_long_ones_are_cut_at_sentences() {
        let long = "Предложение номер один здесь. ".repeat(30);
        let blocks = split_text(&format!("Первый абзац.\n\nВторой абзац\nв две строки.\n\n{long}"), "Рассказчик");
        assert_eq!(blocks[0], ("Рассказчик".to_string(), "Первый абзац.".to_string()));
        assert_eq!(blocks[1].1, "Второй абзац в две строки.");
        assert!(blocks.len() > 3 && blocks[2..].iter().all(|(_, t)| t.chars().count() <= BLOCK_CHARS && t.ends_with('.')));
    }

    #[test]
    fn a_dialog_keeps_its_speakers() {
        let blocks = split_text("Анна: Привет!\n[Борис] Здравствуй.\nкак дела?\nАнна: Хорошо.", "Рассказчик");
        assert_eq!(blocks, vec![
            ("Анна".to_string(), "Привет!".to_string()),
            ("Борис".to_string(), "Здравствуй. как дела?".to_string()),
            ("Анна".to_string(), "Хорошо.".to_string()),
        ]);
    }

    /// A book's scene breaks (`* * *`, 31 of 3196 blocks of the book of issue #24) became blocks voicing made nothing
    /// of, and the book never got done: they are now a pause after the block before them, and such a block already in a
    /// project is skipped, not waited for.
    #[test]
    fn a_scene_break_is_a_pause_not_a_block_to_voice() {
        let blocks = blocks_of("Первая сцена кончилась.\n\n* * *\n\nВторая началась.\n\n—\n\nИ кончилась.", "Р", None);
        let texts: Vec<&str> = blocks.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(texts, ["Первая сцена кончилась.", "Вторая началась.", "И кончилась."]);
        assert_eq!(blocks[0].pause_ms, Some(SCENE_BREAK_MS));
        assert_eq!(blocks[1].pause_ms, Some(SCENE_BREAK_MS));
        assert_eq!(blocks[2].pause_ms, None);
        assert!(blocks_of("* * *\n\nНачало.", "Р", None).len() == 1, "a break before any block is the chapter's own pause");
        let old = Block { text: "* * *".into(), ..Default::default() };
        assert_eq!(status(&old, "k"), "skipped");
        assert_eq!(status(&Block { text: "1945".into(), ..Default::default() }, "k"), "pending", "a number is said");
    }

    #[test]
    fn one_line_with_a_colon_is_a_phrase_not_a_dialog() {
        assert_eq!(split_text("Проверка релиза: озвучка работает.", "Рассказчик"), vec![("Рассказчик".to_string(), "Проверка релиза: озвучка работает.".to_string())]);
        assert_eq!(split_text("Анна: Привет!\nБорис: Здравствуй.", "Рассказчик").len(), 2, "two named lines are still a dialog");
    }

    #[test]
    fn a_narrator_paragraph_after_a_line_is_the_narrator_s() {
        let blocks = split_text("Анна: Круассан? Ладно, прощаю.\n\nОлег улыбнулся и сел напротив.\n\nОлег: Спасибо.", "Рассказчик");
        assert_eq!(blocks, vec![
            ("Анна".to_string(), "Круассан? Ладно, прощаю.".to_string()),
            ("Рассказчик".to_string(), "Олег улыбнулся и сел напротив.".to_string()),
            ("Олег".to_string(), "Спасибо.".to_string()),
        ]);
    }

    #[test]
    fn a_line_by_line_dialog_keeps_its_speakers() {
        let p = from_lines("x", "Анна: Ты опоздал.\nОлег: Автобус.\nОна вздохнула.", "Рассказчик");
        let got: Vec<(String, String)> = p.chapters[0].blocks.iter().map(|b| (b.speaker.clone(), b.text.clone())).collect();
        assert_eq!(got, vec![
            ("Анна".to_string(), "Ты опоздал.".to_string()),
            ("Олег".to_string(), "Автобус.".to_string()),
            ("Рассказчик".to_string(), "Она вздохнула.".to_string()),
        ]);
        let prose = from_lines("x", "Встреча в 12:00.\nВзять с собой: паспорт.\nНе опаздывать.", "Р");
        assert!(prose.chapters[0].blocks.iter().all(|b| b.speaker == "Р" && !b.text.is_empty()));
        assert_eq!(prose.chapters[0].blocks[1].text, "Взять с собой: паспорт.");
    }

    #[test]
    fn a_chinese_dialog_with_full_width_colons_keeps_its_speakers() {
        let parts = split_text_lines("安娜：你听说了吗？\n博文：听说了。", "Narrator", None);
        assert_eq!(parts, vec![("安娜".to_string(), "你听说了吗？".to_string()), ("博文".to_string(), "听说了。".to_string())]);
        let latin = split_text_lines("See http://example.com for it.\nAnd more prose.", "Narrator", None);
        assert!(latin.iter().all(|(who, _)| who == "Narrator"), "{latin:?}");
    }

    #[test]
    fn a_colon_in_prose_does_not_make_a_dialog() {
        let blocks = split_text("Встреча в 12:00 у входа.\nВзять с собой: паспорт, билеты.\nНе опаздывать.\nВсё.", "Р");
        assert!(blocks.iter().all(|(s, _)| s == "Р"));
    }

    #[test]
    fn status_follows_the_key_of_the_active_take() {
        let mut b = Block { id: "b".into(), text: "Да".into(), ..Default::default() };
        assert_eq!(status(&b, "k1"), "pending");
        b.takes.push(Take { n: 1, key: "k1".into(), ..Default::default() });
        b.active = Some(1);
        assert_eq!(status(&b, "k1"), "ready");
        assert_eq!(status(&b, "k2"), "stale");
        b.error = Some(BlockError { key: "k2".into(), text: "x".into() });
        assert_eq!(status(&b, "k2"), "failed");
        b.hidden = true;
        assert_eq!(status(&b, "k2"), "skipped");
    }

    #[test]
    fn the_key_changes_with_what_changes_the_sound() {
        let mut p = from_text("t", "Привет.", "Р");
        let b = p.chapters[0].blocks[0].clone();
        let k = synth_key(&p, &b, "v1", "higgs:q8_0", 12.0, "");
        assert_eq!(k, synth_key(&p, &b, "v1", "higgs:q8_0", 12.0, ""));
        assert_ne!(k, synth_key(&p, &b, "v2", "higgs:q8_0", 12.0, ""));
        assert_ne!(k, synth_key(&p, &b, "v1", "higgs:q6_k", 12.0, ""));
        p.settings.params.temperature = 0.7;
        assert_ne!(k, synth_key(&p, &b, "v1", "higgs:q8_0", 12.0, ""));
        let mut b2 = b.clone();
        b2.pause_ms = Some(900);
        p.settings.params.temperature = 1.0;
        assert_eq!(k, synth_key(&p, &b2, "v1", "higgs:q8_0", 12.0, ""), "a pause is silence, it does not change the clip");
    }

    #[test]
    fn a_block_speaks_with_its_own_engine_then_its_speakers_then_the_projects() {
        let mut p = from_text("t", "Раз.\n\nДва.\n\nТри.", "Р");
        p.settings.engine = "higgs".into();
        p.chapters[0].blocks[1].speaker = "Анна".into();
        p.cast_engines.insert("Анна".into(), crate::engines::supertonic::ENGINE.into());
        p.chapters[0].blocks[2].speaker = "Анна".into();
        p.chapters[0].blocks[2].engine = Some(crate::engines::qwen_tts::ENGINE.into());
        let b = &p.chapters[0].blocks;
        assert_eq!((p.engine_of(&b[0]), p.engine_of(&b[1]), p.engine_of(&b[2])), ("higgs", crate::engines::supertonic::ENGINE, crate::engines::qwen_tts::ENGINE));
        assert!(crate::speech::validate(&p).is_ok());
        p.chapters[0].blocks[0].engine = Some("openrouter".into());
        assert!(crate::speech::validate(&p).is_err(), "a cloud engine is set up for the whole project only");
        p.chapters[0].blocks[0].engine = Some("nope".into());
        assert!(crate::speech::validate(&p).is_err());
    }

    #[test]
    fn validation_refuses_repeated_ids_and_dangling_takes() {
        let mut p = from_text("t", "Раз.\n\nДва.", "Р");
        assert!(crate::speech::validate(&p).is_ok());
        let id = p.chapters[0].blocks[0].id.clone();
        p.chapters[0].blocks[1].id = id;
        assert!(crate::speech::validate(&p).is_err());
        let mut p = from_text("t", "Раз.", "Р");
        p.chapters[0].blocks[0].active = Some(3);
        assert!(crate::speech::validate(&p).unwrap_err().contains("take 3"));
    }

    #[test]
    fn a_clips_mix_gains_then_fades_in_and_out_over_its_length() {
        let mut x = vec![0.5f32; 1000];
        Mix { gain_db: 6.0, fade_in: 0.1, fade_out: 0.2 }.apply(&mut x, 1000);
        let g = 10f32.powf(6.0 / 20.0) * 0.5;
        assert_eq!(x[0], 0.0);
        assert!((x[50] - g * 0.5).abs() < 1e-4, "{}", x[50]);
        assert!((x[500] - g).abs() < 1e-4);
        assert!((x[899] - g * 100.0 / 200.0).abs() < 1e-2, "{}", x[899]);
        assert_eq!(x[999], 0.0);
        let mut flat = vec![0.5f32; 10];
        Mix::default().apply(&mut flat, 1000);
        assert!(flat.iter().all(|v| *v == 0.5));
        let mut p = SpeechProject::default();
        p.chapters.push(Chapter { id: "c".into(), title: String::new(), blocks: vec![Block { id: "b".into(), text: "Да.".into(), mix: Mix { gain_db: 30.0, ..Mix::default() }, ..Default::default() }] });
        assert!(crate::speech::validate(&p).unwrap_err().contains("gain"));
    }
}
