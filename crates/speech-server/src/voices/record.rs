//! Запись голоса с микрофона (нативный cpal + hound → mono 16-bit WAV). Порт Higgs recorder.rs, но
//! уровень отдаём через атомик (GET /record/level опрашивает), не Tauri-события. WAV идёт в референс
//! клонирования / голос в галерею.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};

struct RecState {
    stop_tx: Sender<()>,
    path: std::path::PathBuf,
    /// A microphone test: a recording that starts takes the microphone from it.
    test: bool,
    since: std::time::Instant,
}

fn recorder() -> &'static Mutex<Option<RecState>> {
    static R: OnceLock<Mutex<Option<RecState>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(None))
}

/// Where the microphone's sound also goes while it records, mono at `rate()`: live recognition listens there.
fn tap_cell() -> &'static Mutex<Option<Sender<Vec<f32>>>> {
    static T: OnceLock<Mutex<Option<Sender<Vec<f32>>>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(None))
}

fn rate_cell() -> &'static AtomicU32 {
    static R: OnceLock<AtomicU32> = OnceLock::new();
    R.get_or_init(|| AtomicU32::new(0))
}

/// Sends the recording's sound to `to` as it comes, or stops sending it.
pub fn tap(to: Option<Sender<Vec<f32>>>) {
    *tap_cell().lock().unwrap_or_else(|p| p.into_inner()) = to;
}

/// The sample rate the microphone records at, while it records.
pub fn rate() -> u32 {
    rate_cell().load(Ordering::Relaxed)
}

fn peak_cell() -> &'static AtomicU32 {
    static P: OnceLock<AtomicU32> = OnceLock::new();
    P.get_or_init(|| AtomicU32::new(0))
}

/// Текущий пик 0..1 (для метра уровня).
pub fn level() -> f32 {
    f32::from_bits(peak_cell().load(Ordering::Relaxed))
}

/// Имя микрофона для UI и выбора: FriendlyName устройства (WASAPI).
fn device_name(d: &cpal::Device) -> Option<String> {
    use cpal::traits::DeviceTrait;
    d.description().ok().map(|desc| desc.name().to_string())
}

/// Список микрофонов (первым — системный по умолчанию).
pub fn input_devices() -> Vec<String> {
    use cpal::traits::HostTrait;
    let host = cpal::default_host();
    let def = host.default_input_device().and_then(|d| device_name(&d));
    let mut names = Vec::new();
    if let Some(n) = &def {
        names.push(n.clone());
    }
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Some(n) = device_name(&d) {
                if Some(&n) != def.as_ref() {
                    names.push(n);
                }
            }
        }
    }
    names
}

/// Начать запись в WAV по пути `path`. `device` — имя микрофона (None = системный).
pub fn start(path: &std::path::Path, device: Option<String>) -> Result<(), String> {
    begin(path, device, false)
}

/// A microphone test (`start` for a recording that gives way): it holds the microphone until a real recording starts.
pub fn start_test(path: &std::path::Path, device: Option<String>) -> Result<(), String> {
    begin(path, device, true)
}

fn begin(path: &std::path::Path, device: Option<String>, test: bool) -> Result<(), String> {
    let mut guard = recorder().lock().map_err(|_| "recorder poisoned".to_string())?;
    if let Some(running) = guard.as_ref() {
        if test || !running.test {
            return Err(t!("record-busy"));
        }
        let _ = running.stop_tx.send(());
        *guard = None;
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let path_buf = path.to_path_buf();
    let thread_path = path_buf.clone();
    let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    if let Ok(file) = std::env::var("SPEECH_STUDIO_MIC_FILE") {
        let (samples, sr) = crate::audio::wavio::read_mono_f32(std::path::Path::new(&file))?;
        tracing::warn!("the microphone is the file {file} (SPEECH_STUDIO_MIC_FILE)");
        let spec = hound::WavSpec { channels: 1, sample_rate: sr, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut writer = hound::WavWriter::create(&thread_path, spec).map_err(|e| t!("record-create-wav", error = e.to_string()))?;
        rate_cell().store(sr, Ordering::Relaxed);
        std::thread::spawn(move || {
            let step = (sr / 10).max(1) as usize;
            let silence = vec![0.0f32; step];
            let mut at = 0;
            while stop_rx.try_recv().is_err() {
                let chunk = if at < samples.len() { &samples[at..(at + step).min(samples.len())] } else { &silence[..] };
                at += step;
                for &v in chunk {
                    let _ = writer.write_sample((v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
                }
                peak_cell().store(chunk.iter().fold(0f32, |m, v| m.max(v.abs())).min(1.0).to_bits(), Ordering::Relaxed);
                if let Some(to) = tap_cell().lock().ok().as_deref().and_then(Option::as_ref) {
                    let _ = to.send(chunk.to_vec());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = writer.finalize();
        });
        *guard = Some(RecState { stop_tx, path: path_buf, test, since: std::time::Instant::now() });
        return Ok(());
    }

    std::thread::spawn(move || {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let wanted = device;
        let device = match &wanted {
            Some(name) => host.input_devices().ok().and_then(|mut it| it.find(|d| device_name(d).is_some_and(|n| &n == name))),
            None => host.default_input_device(),
        };
        let Some(device) = device else {
            let missing = match &wanted {
                Some(name) => t!("record-mic-not-found", name = name.clone()),
                None => t!("record-no-mic"),
            };
            let _ = ready_tx.send(Err(missing));
            return;
        };
        let supported = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => { let _ = ready_tx.send(Err(t!("record-mic-config", error = e.to_string()))); return; }
        };
        let fmt = supported.sample_format();
        let ch = (supported.channels() as usize).max(1);
        let sr = supported.sample_rate();
        let config: cpal::StreamConfig = supported.into();
        let spec = hound::WavSpec { channels: 1, sample_rate: sr, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let writer = match hound::WavWriter::create(&thread_path, spec) {
            Ok(w) => Arc::new(Mutex::new(Some(w))),
            Err(e) => { let _ = ready_tx.send(Err(t!("record-create-wav", error = e.to_string()))); return; }
        };
        let emit_every = (sr / 20).max(1) as usize;
        rate_cell().store(sr, Ordering::Relaxed);

        macro_rules! cb {
            ($ty:ty, $to_f:expr) => {{
                let w = writer.clone();
                let mut peak = 0f32;
                let mut frames = 0usize;
                move |data: &[$ty], _: &cpal::InputCallbackInfo| {
                    let mut heard = Vec::with_capacity(data.len() / ch);
                    if let Ok(mut g) = w.lock() {
                        if let Some(wr) = g.as_mut() {
                            for frame in data.chunks(ch) {
                                let mono = frame.iter().map(|&s| $to_f(s)).sum::<f32>() / ch as f32;
                                peak = peak.max(mono.abs());
                                let _ = wr.write_sample((mono.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
                                heard.push(mono);
                            }
                        }
                    }
                    if let Some(to) = tap_cell().lock().ok().as_deref().and_then(Option::as_ref) {
                        let _ = to.send(heard);
                    }
                    frames += data.len() / ch;
                    if frames >= emit_every {
                        peak_cell().store(peak.min(1.0).to_bits(), Ordering::Relaxed);
                        peak = 0.0;
                        frames = 0;
                    }
                }
            }};
        }
        let err_fn = |e| eprintln!("microphone stream error: {e}");
        let built = match fmt {
            cpal::SampleFormat::F32 => device.build_input_stream(config, cb!(f32, |s: f32| s), err_fn, None),
            cpal::SampleFormat::I16 => device.build_input_stream(config, cb!(i16, |s: i16| s as f32 / i16::MAX as f32), err_fn, None),
            cpal::SampleFormat::U16 => device.build_input_stream(config, cb!(u16, |s: u16| (s as f32 - 32768.0) / 32768.0), err_fn, None),
            other => { let _ = ready_tx.send(Err(t!("record-format-unsupported", format = format!("{other:?}")))); return; }
        };
        let stream = match built {
            Ok(s) => s,
            Err(e) => { let _ = ready_tx.send(Err(t!("record-mic-open", error = e.to_string()))); return; }
        };
        if let Err(e) = stream.play() {
            let _ = ready_tx.send(Err(t!("record-mic-start", error = e.to_string())));
            return;
        }
        let _ = ready_tx.send(Ok(()));
        let _ = stop_rx.recv();
        drop(stream);
        if let Some(w) = writer.lock().ok().and_then(|mut g| g.take()) {
            let _ = w.finalize();
        }
    });

    match ready_rx.recv().map_err(|e| e.to_string())? {
        Ok(()) => { *guard = Some(RecState { stop_tx, path: path_buf, test, since: std::time::Instant::now() }); Ok(()) }
        Err(e) => Err(e),
    }
}

/// The microphone in use now: whether it is a microphone test, and how long it has been on.
pub fn in_use() -> Option<(bool, std::time::Duration)> {
    recorder().lock().ok()?.as_ref().map(|st| (st.test, st.since.elapsed()))
}

/// Остановить запись → путь к готовому WAV.
pub fn stop() -> Result<std::path::PathBuf, String> {
    let mut guard = recorder().lock().map_err(|_| "recorder poisoned".to_string())?;
    let st = guard.take().ok_or_else(|| t!("record-not-recording"))?;
    let _ = st.stop_tx.send(());
    std::thread::sleep(std::time::Duration::from_millis(200));
    peak_cell().store(0, Ordering::Relaxed);
    Ok(st.path)
}

/// Остановить запись, только если пишется именно `path`: чужая запись (проект Listening, разговор Live) идёт дальше.
pub fn stop_only(path: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let other = recorder().lock().map_err(|_| "recorder poisoned".to_string())?.as_ref().map(|st| st.path != path);
    match other {
        None => Err(t!("record-not-recording")),
        Some(true) => Err(t!("record-test-other")),
        Some(false) => stop(),
    }
}

/// Below −60 dBFS over the whole take: a microphone the system keeps closed or a muted one writes zeros, not an error.
pub const SILENT_PEAK: f32 = 0.001;

/// The loudest sample of a recorded WAV, 0..1.
pub fn peak(path: &std::path::Path) -> Result<f32, String> {
    let mut r = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(r.samples::<i16>().filter_map(Result::ok).fold(0f32, |m, s| m.max((s as f32 / i16::MAX as f32).abs())))
}

// Пак голосов VibeVoice (HF) закреплён коммитом: размер и SHA-256 (lfs.oid) — этой ревизии.
const VOICE_PACK_URL: &str = "https://huggingface.co/datasets/nerualdreming/VibeVoice/resolve/5c884b3cbdc932c39a5167951c4072120e1a0310/voice-pack.zip";
const VOICE_PACK_SIZE: u64 = 104_685_334;
const VOICE_PACK_SHA256: &str = "682e73c189dbe9bc66fc35bc8fb29438641155957a917a7cc914f2f176a65264";

// Датасет доп-голосов (mp3+txt пары) закреплён коммитом: список и файлы одной ревизии. У mp3 (LFS) есть SHA-256,
// у txt (обычные файлы git, по SHA-256 Hugging Face их не описывает) — точный размер.
const VOICES_DATASET: &str = "nerualdreming/russia_voices";
const VOICES_REVISION: &str = "0269d8b31e7f522a3c4ff4cb2e096720ad764c44";
/// Больше этого голос датасета не бывает (самый крупный ~10 МБ).
const VOICE_FILE_LIMIT: u64 = 64 * 1024 * 1024;

fn hf_client() -> Result<reqwest::blocking::Client, String> {
    dub_llm::net::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("http: {e}"))
}

fn voice_file_url(path: &str) -> String {
    format!("https://huggingface.co/datasets/{VOICES_DATASET}/resolve/{VOICES_REVISION}/{path}")
}

/// Следующая страница списка файлов HF: заголовок `Link: <…>; rel="next"`.
fn next_page(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let link = headers.get("link")?.to_str().ok()?;
    link.split(',').find_map(|part| {
        let (url, rel) = part.split_once(';')?;
        rel.contains("rel=\"next\"").then(|| url.trim().trim_start_matches('<').trim_end_matches('>').to_string())
    })
}

/// Каждый файл датасета -> (размер, SHA-256 для LFS): аудио (mp3/wav) лежит в LFS с SHA-256, txt — обычный файл git,
/// по SHA-256 не описан, только размер.
fn files_of(items: &[serde_json::Value]) -> std::collections::HashMap<&str, (u64, Option<&str>)> {
    let mut files = std::collections::HashMap::new();
    for it in items {
        let Some(path) = it.get("path").and_then(|p| p.as_str()) else { continue };
        if let Some(lfs) = it.get("lfs") {
            if let (Some(sha), Some(size)) = (lfs.get("oid").and_then(|o| o.as_str()), lfs.get("size").and_then(|s| s.as_u64())) {
                files.insert(path, (size, Some(sha)));
                continue;
            }
        }
        if let Some(size) = it.get("size").and_then(|s| s.as_u64()) {
            files.insert(path, (size, None));
        }
    }
    files
}

/// Голоса каталога: строки metadata.jsonl, обогащённые списком файлов датасета — размер и SHA-256 аудио (mp3 или wav)
/// и размер txt рядом. Строка без своего аудиофайла в датасете в каталог не попадает.
fn voices_of(items: &[serde_json::Value], meta: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let files = files_of(items);
    let mut voices: Vec<serde_json::Value> = meta
        .iter()
        .filter_map(|row| {
            let file = row.get("file")?.as_str()?;
            if file.contains('/') || file.contains('\\') || file.contains("..") {
                return None;
            }
            let (size, sha256) = files.get(file).copied()?;
            let name = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
            let txt_size = files.get(format!("{name}.txt").as_str()).map(|(s, _)| *s);
            let field = |k: &str| row.get(k).cloned().unwrap_or(serde_json::Value::Null);
            Some(serde_json::json!({
                "name": name,
                "file": file,
                "displayName": row.get("name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(name),
                "gender": field("gender"),
                "age_group": field("age_group"),
                "language": field("language"),
                "description": field("description"),
                "duration_s": field("duration_s"),
                "birth_year": field("birth_year"),
                "url": voice_file_url(file),
                "size": size,
                "sha256": sha256,
                "txt_size": txt_size,
            }))
        })
        .collect();
    voices.sort_by(|a, b| a["displayName"].as_str().cmp(&b["displayName"].as_str()));
    voices
}

/// Строки metadata.jsonl датасета (обычный файл git): по строке на голос. Битые строки пропускаются (ревизия
/// закреплена, но лучше отдать остальной каталог, чем уронить весь).
fn fetch_metadata(client: &reqwest::blocking::Client) -> Result<Vec<serde_json::Value>, String> {
    let url = voice_file_url("metadata.jsonl");
    let text = client
        .get(&url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("{url}: {e}"))?
        .text()
        .map_err(|e| format!("{url}: {e}"))?;
    Ok(metadata_rows(&text))
}

fn metadata_rows(text: &str) -> Vec<serde_json::Value> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!("voices catalog: a metadata line is not JSON, skipped: {e}");
                None
            }
        })
        .collect()
}

/// Фильтры каталога голосов (окно и MCP отдают один и тот же отбор): пол, возраст (adult/child), язык (ISO 639-1),
/// длительность в секундах и поиск по имени и описанию. Пустое поле — без ограничения по нему.
#[derive(Default)]
pub struct CatalogFilter {
    pub gender: Option<String>,
    pub age_group: Option<String>,
    pub language: Option<String>,
    pub query: Option<String>,
    pub min_duration: Option<f64>,
    pub max_duration: Option<f64>,
}

/// Отобрать голоса каталога по фильтрам. Регистр для языка/пола/возраста и поиска не важен.
pub fn filter_catalog(voices: &[serde_json::Value], f: &CatalogFilter) -> Vec<serde_json::Value> {
    let eq = |value: Option<&str>, want: &Option<String>| match want {
        Some(w) if !w.trim().is_empty() => value.is_some_and(|v| v.eq_ignore_ascii_case(w.trim())),
        _ => true,
    };
    let query = f.query.as_deref().map(str::trim).filter(|q| !q.is_empty()).map(str::to_lowercase);
    voices
        .iter()
        .filter(|v| {
            eq(v["gender"].as_str(), &f.gender)
                && eq(v["age_group"].as_str(), &f.age_group)
                && eq(v["language"].as_str(), &f.language)
                && f.min_duration.is_none_or(|lo| v["duration_s"].as_f64().is_some_and(|d| d >= lo))
                && f.max_duration.is_none_or(|hi| v["duration_s"].as_f64().is_some_and(|d| d <= hi))
                && query.as_ref().is_none_or(|q| {
                    [v["displayName"].as_str(), v["name"].as_str(), v["description"].as_str()]
                        .into_iter()
                        .flatten()
                        .any(|s| s.to_lowercase().contains(q))
                })
        })
        .cloned()
        .collect()
}

fn catalog_cache() -> &'static std::sync::Mutex<Option<serde_json::Value>> {
    static CACHE: OnceLock<std::sync::Mutex<Option<serde_json::Value>>> = OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(None))
}

/// Каталог доп-голосов из HF-датасета закреплённой ревизии: по строке metadata.jsonl на голос
/// ({name, file, displayName, gender, age_group, language, description, duration_s, birth_year, url, size, sha256,
/// txt_size}), размер и SHA-256 аудио — из списка файлов датасета. Ревизия не меняется, поэтому удачный ответ живёт
/// до конца процесса; неудачный не запоминается.
pub fn catalog() -> Result<serde_json::Value, String> {
    if let Some(v) = catalog_cache().lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return Ok(v);
    }
    let client = hf_client()?;
    let mut url = Some(format!("https://huggingface.co/api/datasets/{VOICES_DATASET}/tree/{VOICES_REVISION}?recursive=1"));
    let mut items: Vec<serde_json::Value> = Vec::new();
    while let Some(page) = url.take() {
        let resp = client.get(&page).send().and_then(|r| r.error_for_status()).map_err(|e| format!("{page}: {e}"))?;
        url = next_page(resp.headers());
        let chunk: Vec<serde_json::Value> = resp.json().map_err(|e| t!("voices-not-file-list", page = page.clone(), error = e.to_string()))?;
        items.extend(chunk);
        if items.len() > 100_000 {
            return Err(t!("voices-list-endless", page = page.clone()));
        }
    }
    let meta = fetch_metadata(&client)?;
    let v = serde_json::json!({ "voices": voices_of(&items, &meta) });
    *catalog_cache().lock().unwrap_or_else(|e| e.into_inner()) = Some(v.clone());
    Ok(v)
}

/// Скачать `url` в `dest` через .part: размер и (если задан) SHA-256 сверяются до того, как файл появится под своим
/// именем. `progress(получено)` — по ходу.
fn fetch_checked(
    client: &reqwest::blocking::Client,
    url: &str,
    dest: &std::path::Path,
    size: u64,
    sha256: Option<&str>,
    progress: &dyn Fn(u64),
) -> Result<(), String> {
    use std::io::{Read, Write};
    let part = dest.with_extension(format!("{}.part", dest.extension().and_then(|e| e.to_str()).unwrap_or("")));
    let result = (|| -> Result<(), String> {
        let mut resp = client.get(url).send().and_then(|r| r.error_for_status()).map_err(|e| format!("{url}: {e}"))?;
        let mut f = std::fs::File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        let mut buf = [0u8; 1 << 16];
        let mut got = 0u64;
        loop {
            let n = resp.read(&mut buf).map_err(|e| format!("{url}: {e}"))?;
            if n == 0 {
                break;
            }
            got += n as u64;
            if got > size {
                return Err(t!("voices-too-large", url = url.to_string(), size = size));
            }
            f.write_all(&buf[..n]).map_err(|e| format!("{}: {e}", part.display()))?;
            progress(got);
        }
        f.flush().map_err(|e| format!("{}: {e}", part.display()))?;
        if got != size {
            return Err(t!("voices-size-mismatch", url = url.to_string(), got = got, size = size));
        }
        if let Some(want) = sha256 {
            let got = crate::setup::sha256_file(&part, &|| false)
                .map_err(|e| format!("{}: {e}", part.display()))?
                .unwrap_or_default();
            if got != want {
                return Err(t!("voices-sha-mismatch", url = url.to_string(), got = got.clone(), want = want.to_string()));
            }
        }
        std::fs::rename(&part, dest).map_err(|e| format!("{}: {e}", dest.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

/// Скачать один голос датасета (mp3 + txt, если он есть) в каталог голосов, сверив с каталогом закреплённой ревизии.
pub fn fetch_voice(dir: &std::path::Path, name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(t!("voices-bad-name"));
    }
    let catalog = catalog()?;
    let entry = catalog["voices"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|v| v["name"] == name)
        .cloned()
        .ok_or_else(|| t!("voices-not-in-dataset", name = name.to_string(), dataset = VOICES_DATASET))?;
    let file = entry["file"].as_str().ok_or_else(|| t!("voices-not-in-dataset", name = name.to_string(), dataset = VOICES_DATASET))?;
    let size = entry["size"].as_u64().filter(|s| *s <= VOICE_FILE_LIMIT).ok_or_else(|| t!("voices-bad-size", name = name.to_string()))?;
    // mp3 lives in LFS and carries a SHA-256; the few wav do too; a file without one is checked by size alone
    let sha256 = entry["sha256"].as_str();
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let client = hf_client()?;
    let base = std::path::Path::new(name).file_name().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = std::path::Path::new(file).extension().and_then(|e| e.to_str()).unwrap_or("mp3");
    fetch_checked(&client, &voice_file_url(file), &dir.join(format!("{base}.{ext}")), size, sha256, &|_| {})?;
    if let Some(txt) = entry["txt_size"].as_u64() {
        if txt > VOICE_FILE_LIMIT {
            return Err(t!("voices-bad-size", name = format!("{name}.txt")));
        }
        fetch_checked(&client, &voice_file_url(&format!("{name}.txt")), &dir.join(format!("{base}.txt")), txt, None, &|_| {})?;
    }
    Ok(())
}

/// Скачать пак голосов (VibeVoice) закреплённой ревизии, сверить SHA-256 и распаковать в `dir` (плоско, .mp3+.txt
/// пары). Прогресс -> cb. Пак, который уже лежит у другой программы timoncool (Dub Studio), берётся у неё без закачки.
pub fn download_pack(dir: &std::path::Path, cb: &dyn Fn(serde_json::Value)) -> Result<serde_json::Value, String> {
    std::fs::create_dir_all(dir).map_err(|e| t!("common-create-dir", path = dir.display().to_string(), error = e.to_string()))?;
    if let Some(app) = take_pack_from_neighbor(dir) {
        let count = pack_files().count();
        let msg = if app.is_empty() { t!("voices-pack-done", count = count) } else { t!("voices-pack-taken", count = count, app = crate::setup::program_name(&app).to_string()) };
        cb(serde_json::json!({ "stage": "voicepack", "msg": msg, "pct": 100 }));
        return Ok(serde_json::json!({ "extracted": count, "taken_from": (!app.is_empty()).then_some(app) }));
    }
    cb(serde_json::json!({ "stage": "voicepack", "msg": t!("voices-pack-downloading"), "pct": 0 }));
    let client = dub_llm::net::builder()
        .timeout(None)
        .build()
        .map_err(|e| format!("http: {e}"))?;
    let zip_path = dir.join("_voice-pack.zip");
    let last = std::sync::Mutex::new(0u64);
    fetch_checked(&client, VOICE_PACK_URL, &zip_path, VOICE_PACK_SIZE, Some(VOICE_PACK_SHA256), &|got| {
        let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
        if got - *last >= 1 << 20 || got == VOICE_PACK_SIZE {
            *last = got;
            cb(serde_json::json!({ "stage": "voicepack", "msg": t!("voices-pack-downloading"), "pct": (got as f64 / VOICE_PACK_SIZE as f64 * 90.0) }));
        }
    })?;
    cb(serde_json::json!({ "stage": "voicepack", "msg": t!("voices-pack-unpacking"), "pct": 92 }));
    let zf = std::fs::File::open(&zip_path).map_err(|e| t!("voices-open-zip", error = e.to_string()))?;
    let mut zip = zip::ZipArchive::new(zf).map_err(|e| format!("zip: {e}"))?;
    let mut count = 0;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|e| format!("zip[{i}]: {e}"))?;
        if file.is_dir() { continue; }
        let name = std::path::Path::new(file.name())
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        let Some(name) = name else { continue };
        let out = dir.join(&name);
        let mut o = std::fs::File::create(&out).map_err(|e| t!("voices-create-file", name = name.clone(), error = e.to_string()))?;
        std::io::copy(&mut file, &mut o).map_err(|e| t!("voices-unpack-file", name = name.clone(), error = e.to_string()))?;
        count += 1;
    }
    drop(zip);
    if let Err(e) = std::fs::remove_file(&zip_path) {
        tracing::warn!("{}: the pack archive was not removed after unpacking: {e}", zip_path.display());
    }
    cb(serde_json::json!({ "stage": "voicepack", "msg": t!("voices-pack-done", count = count), "pct": 100 }));
    Ok(serde_json::json!({ "extracted": count }))
}


/// The files of the pack's archive: name, size and SHA-256 of each.
fn pack_files() -> impl Iterator<Item = (&'static str, u64, &'static str)> {
    include_str!("voice_pack.tsv").lines().filter_map(|l| {
        let mut f = l.split('\t');
        Some((f.next()?, f.next()?.parse().ok()?, f.next()?))
    })
}

/// The whole pack linked from the `voices` folder of another program that has every file of it (only the pack's own
/// files, each by its SHA-256: the rest of that folder is the person's voices and is not touched); the program's id,
/// empty when the pack is here already, or None when no program has all of it (a file already here and whole is kept).
fn take_pack_from_neighbor(dir: &std::path::Path) -> Option<String> {
    let whole = |p: &std::path::Path, size: u64, sha256: &str| p.metadata().is_ok_and(|m| m.len() == size) && matches!(crate::setup::sha256_file(p, &|| false), Ok(Some(h)) if h == sha256);
    let missing: Vec<_> = pack_files().filter(|(name, size, sha256)| !whole(&dir.join(name), *size, sha256)).collect();
    if missing.is_empty() {
        return Some(String::new());
    }
    for neighbor in studio_hub_client::neighbors::current() {
        let theirs = neighbor.data_dir.join("voices");
        if !missing.iter().all(|(name, size, _)| theirs.join(name).metadata().is_ok_and(|m| m.len() == *size)) {
            continue;
        }
        let taken = missing.iter().all(|(name, _, sha256)| {
            let placed = studio_hub_client::neighbors::take_file(&theirs.join(name), &dir.join(name), true, |p| whole(p, p.metadata().map_or(0, |m| m.len()), sha256));
            if let Err(e) = &placed {
                tracing::warn!("{name} from {} ({}): {e}", neighbor.app, theirs.display());
            }
            placed.is_ok()
        });
        if taken {
            tracing::info!("[OK] the voice pack taken from {} instead of downloading", neighbor.app);
            return Some(neighbor.app);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_downloads_are_pinned_to_a_commit() {
        for url in [VOICE_PACK_URL.to_string(), voice_file_url("RU_Female_x.mp3")] {
            assert!(!url.contains("/resolve/main/"), "{url}");
            let rev = url.split("/resolve/").nth(1).and_then(|r| r.split('/').next()).unwrap_or_default();
            assert!(rev.len() == 40 && rev.bytes().all(|b| b.is_ascii_hexdigit()), "{url}");
        }
        assert_eq!(VOICE_PACK_SHA256.len(), 64);
    }

    #[test]
    fn the_catalog_joins_metadata_with_the_file_list_and_follows_its_pages() {
        let items: Vec<serde_json::Value> = serde_json::from_str(r#"[
            {"type":"file","oid":"fd3a","size":6038690,"lfs":{"oid":"d4c4af1c2704a671f9352cd300b80bb92eee4d88b7642b84aa073009140cc710","size":6038690},"path":"RU_Female_abramova_oljga.mp3"},
            {"type":"file","oid":"517a","size":7928,"path":"RU_Female_abramova_oljga.txt"},
            {"type":"file","oid":"aa","size":10,"path":"EN_Male_no_lfs.wav"},
            {"type":"file","oid":"bb","size":5,"lfs":{"oid":"cc","size":5},"path":"RU_Child_x.mp3"}
        ]"#).unwrap();
        let meta = metadata_rows(concat!(
            "{\"file\":\"RU_Female_abramova_oljga.mp3\",\"name\":\"Ольга Абрамова\",\"gender\":\"female\",\"age_group\":\"adult\",\"language\":\"ru\",\"description\":\"диктор\",\"duration_s\":25.8,\"birth_year\":1970}\n",
            "{\"file\":\"EN_Male_no_lfs.wav\",\"name\":\"No Lfs\",\"gender\":\"male\",\"age_group\":\"adult\",\"language\":\"en\",\"duration_s\":9.0}\n",
            "{\"file\":\"RU_Child_x.mp3\",\"name\":\"Kid\",\"gender\":\"female\",\"age_group\":\"child\",\"language\":\"ru\",\"duration_s\":5.0}\n",
            "not json, skipped\n",
            "{\"file\":\"RU_Missing.mp3\",\"name\":\"Gone\",\"gender\":\"male\"}\n",
        ));
        let v = voices_of(&items, &meta);
        let by = |name: &str| v.iter().find(|x| x["name"] == name).unwrap_or_else(|| panic!("{name} not in {v:?}"));
        assert_eq!(v.len(), 3, "a metadata row whose audio is not in the dataset is dropped: {v:?}");
        let a = by("RU_Female_abramova_oljga");
        assert_eq!((a["size"].as_u64(), a["txt_size"].as_u64(), a["gender"].as_str()), (Some(6038690), Some(7928), Some("female")));
        assert_eq!((a["displayName"].as_str(), a["age_group"].as_str(), a["language"].as_str(), a["duration_s"].as_f64(), a["birth_year"].as_i64()), (Some("Ольга Абрамова"), Some("adult"), Some("ru"), Some(25.8), Some(1970)));
        assert!(a["url"].as_str().unwrap().contains(VOICES_REVISION) && a["url"].as_str().unwrap().ends_with(".mp3"));
        // a wav without a SHA-256 is offered, checked by size, and keeps its extension in the url
        let w = by("EN_Male_no_lfs");
        assert_eq!((w["sha256"].clone(), w["txt_size"].clone(), w["file"].as_str()), (serde_json::Value::Null, serde_json::Value::Null, Some("EN_Male_no_lfs.wav")));
        assert_eq!(by("RU_Child_x")["txt_size"], serde_json::Value::Null);
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("link", "<https://huggingface.co/api/datasets/nerualdreming/russia_voices/tree/x?recursive=1&cursor=abc>; rel=\"next\"".parse().unwrap());
        assert_eq!(next_page(&h).as_deref(), Some("https://huggingface.co/api/datasets/nerualdreming/russia_voices/tree/x?recursive=1&cursor=abc"));
        assert_eq!(next_page(&reqwest::header::HeaderMap::new()), None);
    }

    #[test]
    fn the_catalog_filters_by_the_metadata_fields() {
        let meta = metadata_rows(concat!(
            "{\"file\":\"RU_Female_adult.mp3\",\"name\":\"Анна\",\"gender\":\"female\",\"age_group\":\"adult\",\"language\":\"ru\",\"description\":\"спокойный диктор\",\"duration_s\":30.0}\n",
            "{\"file\":\"RU_Female_child.mp3\",\"name\":\"Катя\",\"gender\":\"female\",\"age_group\":\"child\",\"language\":\"ru\",\"description\":\"ребёнок\",\"duration_s\":8.0}\n",
            "{\"file\":\"EN_Male_adult.mp3\",\"name\":\"John\",\"gender\":\"male\",\"age_group\":\"adult\",\"language\":\"en\",\"description\":\"narrator\",\"duration_s\":60.0}\n",
        ));
        let items: Vec<serde_json::Value> = ["RU_Female_adult", "RU_Female_child", "EN_Male_adult"]
            .iter()
            .map(|n| serde_json::json!({"type":"file","size":5,"lfs":{"oid":"aa","size":5},"path":format!("{n}.mp3")}))
            .collect();
        let all = voices_of(&items, &meta);
        let names = |f: &CatalogFilter| filter_catalog(&all, f).iter().map(|v| v["name"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(names(&CatalogFilter { gender: Some("female".into()), age_group: Some("child".into()), language: Some("ru".into()), ..Default::default() }), ["RU_Female_child"]);
        assert_eq!(names(&CatalogFilter { min_duration: Some(10.0), ..Default::default() }), ["EN_Male_adult", "RU_Female_adult"]);
        assert_eq!(names(&CatalogFilter { query: Some("narrator".into()), ..Default::default() }), ["EN_Male_adult"]);
        assert_eq!(names(&CatalogFilter { query: Some("катя".into()), ..Default::default() }), ["RU_Female_child"]);
        assert_eq!(names(&CatalogFilter::default()).len(), 3);
    }

    #[test]
    fn a_download_is_checked_before_it_takes_its_name() {
        use dub_llm::test_http::{serve, Reply};
        use sha2::{Digest, Sha256};
        let _lang = crate::i18n::test_language("ru");
        let body = b"voice bytes".to_vec();
        let sha: String = Sha256::digest(&body).iter().map(|b| format!("{b:02x}")).collect();
        let server = serve(vec![Reply::bytes(200, "audio/mpeg", body.clone()), Reply::bytes(200, "audio/mpeg", body.clone()), Reply::bytes(200, "audio/mpeg", body.clone())]);
        let client = reqwest::blocking::Client::builder().no_proxy().build().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("v.mp3");
        let n = body.len() as u64;
        fetch_checked(&client, &format!("{}/v", server.base()), &dest, n, Some(&sha), &|_| {}).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        let other = dir.path().join("w.mp3");
        assert!(fetch_checked(&client, &format!("{}/v", server.base()), &other, n, Some(&"0".repeat(64)), &|_| {}).unwrap_err().contains("SHA-256"));
        let short = fetch_checked(&client, &format!("{}/v", server.base()), &other, n + 1, None, &|_| {}).unwrap_err();
        assert!(short.contains("закреплено"), "{short}");
        assert!(!other.exists(), "a file that failed its check never appears");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "no .part is left");
    }


    #[test]
    fn the_pack_lists_every_file_of_its_archive_with_size_and_hash() {
        let files: Vec<_> = pack_files().collect();
        assert_eq!(files.len(), 100);
        assert!(files.iter().all(|(name, size, sha)| !name.is_empty() && *size > 0 && sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit())));
        let names: std::collections::HashSet<_> = files.iter().map(|f| f.0).collect();
        assert_eq!(names.len(), files.len());
    }
}
