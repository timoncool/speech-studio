//! ffmpeg/ffprobe-обёртки для analyze. Порт нужных кусков dubengine/media.py: probe (длительность,
//! видеопоток, fps, кодек) и extract_audio -> wav 16k mono. Тяжёлого ничего: только вызовы бинарей.

use serde_json::Value;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

#[cfg(windows)]
const FFMPEG: &str = "ffmpeg.exe";
#[cfg(windows)]
const FFPROBE: &str = "ffprobe.exe";
#[cfg(not(windows))]
const FFMPEG: &str = "ffmpeg";
#[cfg(not(windows))]
const FFPROBE: &str = "ffprobe";

/// Сводка probe: длительность файла.
#[derive(Debug, Clone, Default)]
pub struct MediaMeta {
    pub duration: f64,
}

/// ffprobe -show_format -show_streams (json) -> MediaMeta. Ошибка если нет видеопотока/длительности.
pub fn probe(input: &Path) -> Result<MediaMeta, String> {
    let out = dub_core::proc::host_command(FFPROBE)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(input)
        .output()
        .map_err(|e| t!("media-ffprobe-start", error = e.to_string()))?;
    if !out.status.success() {
        return Err(t!(
            "media-ffprobe-exit",
            code = format!("{:?}", out.status.code()),
            stderr = String::from_utf8_lossy(&out.stderr).into_owned()
        ));
    }
    let v: Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("ffprobe json: {e}"))?;
    let streams = v
        .get("streams")
        .and_then(|s| s.as_array())
        .ok_or_else(|| t!("media-ffprobe-no-streams"))?;
    // Видеопоток ОПЦИОНАЛЕН: чистый аудио-вход (WAV/mp3/…) поддерживается в аудио-режиме (без видео).
    // Нет видео -> width/height/fps=0 (сигнал audio-only), кодек берём из аудиопотока.
    let vstream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("video"));
    let astream = streams
        .iter()
        .find(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("audio"));
    if vstream.is_none() && astream.is_none() {
        return Err(t!("media-no-streams"));
    }
    let duration = v
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
        .or_else(|| {
            // некоторые WAV не имеют format.duration -> берём из аудиопотока
            astream
                .and_then(|s| s.get("duration"))
                .and_then(|d| d.as_str())
                .and_then(|d| d.parse::<f64>().ok())
        })
        .ok_or_else(|| t!("media-no-duration"))?;
    Ok(MediaMeta { duration })
}

/// Первый фильтр каждого извлечения звука, которое идёт в синхронные стадии (ASR, диаризация, сепарация,
/// микс, рендер, волна таймлайна): сэмплы стоят по своим меткам времени (дыра в pts заполнена тишиной,
/// наложение срезано), и поток начинается с t=0 контейнера, как картинка.
pub const SYNC_AF: &str = "aresample=async=1:first_pts=0";

fn os_args(parts: &[&OsStr]) -> Vec<OsString> {
    parts.iter().map(|p| p.to_os_string()).collect()
}

// ─── Рендер-хелперы (порт media.py: extract_audio/duration/time_stretch/mix/mux/trim) ─────────

fn run_ff(args: &[&std::ffi::OsStr]) -> Result<(), String> {
    let out = dub_core::proc::output(dub_core::proc::host_command(FFMPEG).args(args)).map_err(|e| t!("common-ffmpeg-start", error = e.to_string()))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: String = err.chars().rev().take(1500).collect::<String>().chars().rev().collect();
        return Err(t!("common-ffmpeg-exit", code = format!("{:?}", out.status.code()), tail = tail));
    }
    Ok(())
}

/// Mono samples through an ffmpeg audio filter chain, at the same rate.
pub fn filter_samples(samples: &[f32], sr: u32, af: &str) -> Result<Vec<f32>, String> {
    let tmp = tempfile::Builder::new().prefix("fx-").tempdir().map_err(|e| e.to_string())?;
    let (src, dst) = (tmp.path().join("in.wav"), tmp.path().join("out.wav"));
    crate::audio::wavio::write_mono_f32(&src, samples, sr)?;
    run_ff(&[OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(), OsStr::new("-af"), OsStr::new(af), OsStr::new("-ar"), OsStr::new(&sr.to_string()), OsStr::new("-ac"), OsStr::new("1"), OsStr::new("-c:a"), OsStr::new("pcm_f32le"), dst.as_os_str()])?;
    Ok(crate::audio::wavio::read_mono_f32(&dst)?.0)
}

fn run_ff_args(args: &[OsString]) -> Result<(), String> {
    let refs: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
    run_ff(&refs)
}

fn extract_audio_args(video: &Path, out_wav: &Path, sr: u32, ac: u32) -> Vec<OsString> {
    let (sr, ac) = (sr.to_string(), ac.to_string());
    os_args(&[
        OsStr::new("-y"), OsStr::new("-i"), video.as_os_str(),
        OsStr::new("-vn"), OsStr::new("-af"), OsStr::new(SYNC_AF), OsStr::new("-ac"), OsStr::new(&ac),
        OsStr::new("-ar"), OsStr::new(&sr), out_wav.as_os_str(),
    ])
}

/// Извлечь аудио в WAV sr/ac (порт media.extract_audio). Для сепарации: sr=44100, ac=2.
/// Пишет атомарно: недописанный файл не появится под целевым именем.
pub fn extract_audio(video: &Path, out_wav: &Path, sr: u32, ac: u32) -> Result<(), String> {
    dub_core::atomic::write_with(out_wav, |tmp| run_ff_args(&extract_audio_args(video, tmp, sr, ac)))
}

fn to_16k_mono_args(src: &Path, dst: &Path) -> Vec<OsString> {
    os_args(&[
        OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(),
        OsStr::new("-vn"), OsStr::new("-af"), OsStr::new(SYNC_AF), OsStr::new("-ac"), OsStr::new("1"),
        OsStr::new("-ar"), OsStr::new("16000"), dst.as_os_str(),
    ])
}

/// WAV/медиа -> 16k mono (порт media.to_16k_mono). Пишет атомарно.
pub fn to_16k_mono(src: &Path, dst: &Path) -> Result<(), String> {
    dub_core::atomic::write_with(dst, |tmp| run_ff_args(&to_16k_mono_args(src, tmp)))
}

/// Длительность файла в секундах (ffprobe format.duration). Порт media.duration.
pub fn duration(path: &Path) -> Result<f64, String> {
    let out = dub_core::proc::host_command(FFPROBE)
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
        .arg(path)
        .output()
        .map_err(|e| t!("media-ffprobe-start", error = e.to_string()))?;
    if !out.status.success() {
        return Err(t!("media-ffprobe-duration-exit", code = format!("{:?}", out.status.code())));
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    match text.parse::<f64>() {
        Ok(seconds) => Ok(seconds),
        // контейнер без format.duration (ffprobe печатает N/A): длительность есть у потоков
        Err(_) => probe(path).map(|meta| meta.duration).map_err(|e| format!("{}: {e}", path.display())),
    }
}

/// atempo-цепочка для factor вне [0.5,2.0] (порт media._atempo_chain).
fn atempo_chain(factor: f64) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut f = factor;
    while f > 2.0 {
        parts.push("atempo=2.0".into());
        f /= 2.0;
    }
    while f < 0.5 {
        parts.push("atempo=0.5".into());
        f /= 0.5;
    }
    parts.push(format!("atempo={:.6}", f));
    parts.join(",")
}

/// factor>1 ускоряет (укорачивает); <1 замедляет. Порт media.time_stretch.
pub fn time_stretch(src: &Path, dst: &Path, factor: f64) -> Result<(), String> {
    let chain = atempo_chain(factor);
    run_ff(&[
        OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(),
        OsStr::new("-filter:a"), OsStr::new(&chain), dst.as_os_str(),
    ])
}

/// Частота промежуточных стадий микса: частота audio_hq/стемов сепарации, чтобы по цепочке
/// mix -> loudnorm -> gain не было лишних пересчётов частоты. loudnorm внутри работает на 192 кГц,
/// поэтому частота выхода задаётся явно.
const MIX_SR: &str = "44100";

/// Кодек и частота промежуточного файла микса: float32 WAV без потерь (сумма голоса и фона может
/// выйти за 0 dBFS — float хранит это до loudnorm без клипа). Сжатие с потерями одно — в mux.
/// `-rf64 auto`: многочасовой стерео-float переходит границу RIFF в 4 ГБ.
fn lossless_out() -> [&'static OsStr; 6] {
    [
        OsStr::new("-c:a"), OsStr::new("pcm_f32le"),
        OsStr::new("-ar"), OsStr::new(MIX_SR),
        OsStr::new("-rf64"), OsStr::new("auto"),
    ]
}

     // длина подъёма

/// Финальная нормализация программы по EBU R128 (ffmpeg loudnorm): интегральная громкость к I LUFS и
/// true-peak лимитер к TP dBTP. Решение юзера (best-practice, НЕ питон): ставится последним шагом на
/// смиксованную дорожку — держит целевую громкость соцсетей и ловит межфразовые суммы и микс с фоном
/// (пики отдельной фразы до этого опускает пофразный лимитер normalize_voice). Выход без потерь: после
/// loudnorm уже нет перекодирования, которое сдвинуло бы true-peak.
pub fn loudnorm(src: &Path, dst: &Path, i: f64, tp: f64, lra: f64) -> Result<(), String> {
    let af = format!("loudnorm=I={i}:TP={tp}:LRA={lra}");
    let mut args: Vec<&OsStr> = vec![
        OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(),
        OsStr::new("-af"), OsStr::new(&af),
    ];
    args.extend(lossless_out());
    args.push(dst.as_os_str());
    run_ff(&args)
}

fn preview_aac_args(src: &Path, dst: &Path) -> Vec<OsString> {
    os_args(&[
        OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(),
        OsStr::new("-vn"), OsStr::new("-af"), OsStr::new(SYNC_AF),
        OsStr::new("-ac"), OsStr::new("2"), OsStr::new("-ar"), OsStr::new(MIX_SR),
        OsStr::new("-c:a"), OsStr::new("aac"), OsStr::new("-b:a"), OsStr::new("192k"), dst.as_os_str(),
    ])
}

/// Дорожка для прослушивания в редакторе (<audio> WebView): одно кодирование AAC из несжатого
/// микса. Источник может быть и видео (nodub: звук оригинала), поэтому -vn.
pub fn encode_preview_aac(src: &Path, dst: &Path) -> Result<(), String> {
    run_ff_args(&preview_aac_args(src, dst))
}

/// Фейды 15 мс на обоих краях рефа: разворот клипа даёт фейду в конце ту же точку отсчёта, что в начале,
/// без знания длины.
const REF_FADES: &str = "afade=t=in:d=0.015:curve=hsin,areverse,afade=t=in:d=0.015:curve=hsin,areverse";

/// Самая высокая частота рефа: выше неё wav-мюксер ffmpeg пишет заголовок EXTENSIBLE, который Higgs не
/// читает (голос, записанный на 96 или 192 кГц).
const REF_MAX_RATE: u32 = 48_000;

fn trim_ref_args(src: &Path, dst: &Path, start: f64, end: f64, rate: Option<u32>) -> Vec<OsString> {
    let (ss, to, af) = (format!("{start:.3}"), format!("{end:.3}"), format!("{SYNC_AF},{REF_FADES}"));
    let mut args = os_args(&[
        OsStr::new("-y"), OsStr::new("-ss"), OsStr::new(&ss), OsStr::new("-to"), OsStr::new(&to),
        OsStr::new("-i"), src.as_os_str(), OsStr::new("-vn"), OsStr::new("-af"), OsStr::new(&af),
        OsStr::new("-ac"), OsStr::new("1"),
    ]);
    if let Some(rate) = rate {
        args.extend([OsString::from("-ar"), OsString::from(rate.to_string())]);
    }
    args.extend([OsString::from("-c:a"), OsString::from("pcm_s16le"), dst.as_os_str().to_os_string()]);
    args
}

/// Частота первого звукового потока файла (ffprobe).
pub fn audio_rate(src: &Path) -> Result<u32, String> {
    let out = dub_core::proc::host_command(FFPROBE)
        .args(["-v", "error", "-select_streams", "a:0", "-show_entries", "stream=sample_rate", "-of", "csv=p=0"])
        .arg(src)
        .output()
        .map_err(|e| t!("media-ffprobe-start", error = e.to_string()))?;
    let text = String::from_utf8_lossy(&out.stdout);
    match (out.status.success(), text.trim().parse::<u32>()) {
        (true, Ok(rate)) if rate > 0 => Ok(rate),
        _ => Err(t!("media-no-sample-rate", path = src.display().to_string(), stderr = String::from_utf8_lossy(&out.stderr).trim().to_string())),
    }
}

/// Реф клона: [start,end] источника моно на его собственной частоте (не выше 48 кГц), PCM16 с простым
/// заголовком (float WAV и заголовок EXTENSIBLE Higgs не читает), фейды 15 мс на краях. Частоту движок
/// приводит к своей сам. Пишет атомарно.
pub fn trim_ref(src: &Path, dst: &Path, start: f64, end: f64) -> Result<(), String> {
    let rate = audio_rate(src)?;
    let out_rate = (rate > REF_MAX_RATE).then_some(REF_MAX_RATE);
    dub_core::atomic::write_with(dst, |tmp| run_ff_args(&trim_ref_args(src, tmp, start, end, out_rate)))
}

/// Samples of Higgs's codec hop at 24 kHz: a reference a whole number of them long gives its acoustic and semantic
/// encoders the same frame count.
pub const HIGGS_HOP: u64 = 960;

/// The number of 24 kHz samples a reference of `secs` is brought to for Higgs: a whole number of hops, a second at least
/// (the codec pads a shorter one on its acoustic side only).
pub fn higgs_ref_samples(secs: f64) -> u64 {
    let n = (secs.max(0.0) * 24_000.0).ceil() as u64;
    n.div_ceil(HIGGS_HOP).max(24_000 / HIGGS_HOP) * HIGGS_HOP
}

/// A reference as Higgs's codec takes it: mono 24 kHz PCM16, `higgs_ref_samples` long (the end padded with silence).
/// Its server refuses a reference whose length gives its two encoders different frame counts.
pub fn ref_for_higgs(src: &Path, dst: &Path) -> Result<(), String> {
    let n = higgs_ref_samples(duration(src)?);
    let af = format!("aresample=24000,atrim=end_sample={n},apad=whole_len={n}");
    let args = os_args(&[
        OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(), OsStr::new("-vn"), OsStr::new("-af"), OsStr::new(&af),
        OsStr::new("-ac"), OsStr::new("1"), OsStr::new("-ar"), OsStr::new("24000"), OsStr::new("-c:a"), OsStr::new("pcm_s16le"),
    ]);
    dub_core::atomic::write_with(dst, |tmp| {
        let mut a = args.clone();
        a.push(tmp.as_os_str().to_os_string());
        run_ff_args(&a)
    })
}

// ─── Оконная нарезка для полнометражного пайплайна (#79) ──────────────────────────────────────────
// Новые хелперы (не трогают существующие): вырезать ОДНО окно вокала в отдельный WAV (RAM O(окна)),
// либо нарезать весь вокал segment-muxer'ом одним проходом. `-reset_timestamps 1` даёт локальный t=0 в
// каждом окне (чистый клип для BSRoformer/Sortformer/ASR) — обратный сдвиг window_offset делает
// вызывающий при сшивке диаризации/ASR (dub_asr::Window::offset). Хардненные флаги для длинных входов
// (BORROWINGS #19): +discardcorrupt / avoid_negative_ts / max_muxing_queue_size — против timestamp-drift
// и 'Too many packets buffered' на часовых файлах.

/// Вырезать окно [start,end) вокала в отдельный WAV @ sr Гц (mono). Локальный t=0 (accurate seek:
/// -ss ПОСЛЕ -i для точного реза по сэмплу). Для стадийной обработки одного окна — RAM O(окна).
// allow(dead_code): вызывается оркестратором оконного пайплайна (интеграция #79 идёт отдельно).
#[allow(dead_code)]
pub fn slice_window(src: &Path, dst: &Path, start: f64, end: f64, sr: u32) -> Result<(), String> {
    let ss = format!("{:.3}", start);
    let to = format!("{:.3}", end);
    let ar = sr.to_string();
    run_ff(&[
        OsStr::new("-y"),
        OsStr::new("-fflags"), OsStr::new("+discardcorrupt"),
        OsStr::new("-i"), src.as_os_str(),
        // accurate seek внутри уже открытого потока (после -i) — точная граница окна
        OsStr::new("-ss"), OsStr::new(&ss), OsStr::new("-to"), OsStr::new(&to),
        OsStr::new("-ac"), OsStr::new("1"), OsStr::new("-ar"), OsStr::new(&ar),
        OsStr::new("-avoid_negative_ts"), OsStr::new("make_zero"),
        OsStr::new("-c:a"), OsStr::new("pcm_s16le"),
        dst.as_os_str(),
    ])
}

/// Нарезать весь вокал на окна фикс. длины `win_sec` segment-muxer'ом за ОДИН проход (RAM O(окна),
/// не O(файла)). Файлы пишутся по шаблону `pattern` с `%03d` (например `win_%03d.wav`).
/// `-reset_timestamps 1` -> каждый сегмент стартует с t=0; window_offset = idx*win_sec прибавляет
/// вызывающий при ре-базинге. Дешёвый фолбэк к min-cut нарезке, когда важна только RAM-локальность.
// allow(dead_code): вызывается оркестратором оконного пайплайна (интеграция #79 идёт отдельно).
#[allow(dead_code)]
pub fn segment_wav(src: &Path, out_dir: &Path, pattern: &str, win_sec: f64, sr: u32) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    let seg_time = format!("{:.3}", win_sec.max(0.1));
    let ar = sr.to_string();
    let out_tpl = out_dir.join(pattern);
    run_ff(&[
        OsStr::new("-y"),
        OsStr::new("-fflags"), OsStr::new("+discardcorrupt"),
        OsStr::new("-i"), src.as_os_str(),
        OsStr::new("-ac"), OsStr::new("1"), OsStr::new("-ar"), OsStr::new(&ar),
        OsStr::new("-c:a"), OsStr::new("pcm_s16le"),
        OsStr::new("-f"), OsStr::new("segment"),
        OsStr::new("-segment_time"), OsStr::new(&seg_time),
        OsStr::new("-reset_timestamps"), OsStr::new("1"),
        OsStr::new("-max_muxing_queue_size"), OsStr::new("2048"),
        out_tpl.as_os_str(),
    ])
}

#[cfg(test)]
mod lossless_tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("dub_media_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sine_wav(path: &Path, sr: u32, channels: u16, amp: f32, secs: f32) {
        let spec = hound::WavSpec { channels, sample_rate: sr, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..(sr as f32 * secs) as usize {
            let v = amp * (2.0 * std::f32::consts::PI * 220.0 * i as f32 / sr as f32).sin();
            for _ in 0..channels {
                w.write_sample(v).unwrap();
            }
        }
        w.finalize().unwrap();
    }

    fn read(path: &Path) -> (hound::WavSpec, Vec<f32>) {
        let mut r = hound::WavReader::open(path).unwrap();
        let spec = r.spec();
        let s: Vec<f32> = r.samples::<f32>().map(|x| x.unwrap()).collect();
        (spec, s)
    }

    #[test]
    fn loudnorm_stays_lossless_float_at_the_mix_rate() {
        let d = tmp("loudnorm");
        let music = d.join("instrumental.wav");
        sine_wav(&music, 44_100, 2, 0.8, 1.0);
        let normed = d.join("final_audio.wav");
        loudnorm(&music, &normed, -14.0, -1.0, 11.0).unwrap();
        let (spec, s) = read(&normed);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);
        assert_eq!(spec.sample_rate, 44_100, "loudnorm без явной частоты отдаёт 192 кГц");
        assert!(s.iter().all(|v| v.abs() <= 1.0));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sync_filter_leads_every_audio_extraction() {
        assert_eq!(SYNC_AF, "aresample=async=1:first_pts=0");
        let (i, o) = (Path::new("in.mp4"), Path::new("out.wav"));
        let all = [
            ("extract_audio", extract_audio_args(i, o, 44_100, 2)),
            ("to_16k_mono", to_16k_mono_args(i, o)),
            ("trim_ref", trim_ref_args(i, o, 1.0, 2.0, None)),
            ("trim_ref 96k", trim_ref_args(i, o, 1.0, 2.0, Some(REF_MAX_RATE))),
            ("encode_preview_aac", preview_aac_args(i, o)),
        ];
        for (name, args) in all {
            let at: Vec<usize> = args.iter().enumerate().filter(|(_, a)| *a == "-af").map(|(k, _)| k).collect();
            assert_eq!(at.len(), 1, "{name}: одна цепочка -af {args:?}");
            let chain = args[at[0] + 1].to_string_lossy();
            assert!(chain.starts_with(SYNC_AF), "{name}: aresample первым в цепочке: {chain}");
        }
    }

    fn rms_of(s: &[f32]) -> f32 {
        (s.iter().map(|v| v * v).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    #[test]
    fn extracted_audio_follows_timestamps_across_a_gap() {
        let d = tmp("gap");
        let src = d.join("gap.mkv");
        // 2.5 с звука, с первой секунды его метки сдвинуты на 0.5 с вперёд (дыра в pts); картинка 3 с.
        run_ff(&[
            OsStr::new("-v"), OsStr::new("error"), OsStr::new("-y"),
            OsStr::new("-f"), OsStr::new("lavfi"), OsStr::new("-i"), OsStr::new("sine=f=440:d=2.5:r=48000"),
            OsStr::new("-f"), OsStr::new("lavfi"), OsStr::new("-i"), OsStr::new("color=c=black:s=64x64:r=25:d=3"),
            OsStr::new("-filter_complex"), OsStr::new("[0:a]asetpts='if(gte(T,1),PTS+0.5/TB,PTS)'[a]"),
            OsStr::new("-map"), OsStr::new("1:v"), OsStr::new("-map"), OsStr::new("[a]"),
            OsStr::new("-c:v"), OsStr::new("rawvideo"), OsStr::new("-c:a"), OsStr::new("pcm_s16le"),
            src.as_os_str(),
        ])
        .unwrap();
        let container = probe(&src).unwrap().duration;
        let frame = 1.0 / 25.0;
        assert!((container - 3.0).abs() <= frame, "контейнер {container}");

        let hq = d.join("hq.wav");
        extract_audio(&src, &hq, 44_100, 2).unwrap();
        let m16 = d.join("m16.wav");
        to_16k_mono(&src, &m16).unwrap();
        for w in [&hq, &m16] {
            let got = duration(w).unwrap();
            assert!((got - container).abs() <= frame, "{}: {got} против {container}", w.display());
        }
        // Дыра стоит на своём месте: 1.0–1.5 с тишина, звук после неё начинается с 1.5 с.
        let (s, sr) = crate::audio::wavio::read_mono_f32(&m16).unwrap();
        let at = |t: f64| (t * sr as f64) as usize;
        assert!(rms_of(&s[at(1.1)..at(1.4)]) < 1e-3);
        assert!(rms_of(&s[at(1.6)..at(1.9)]) > 0.05);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn clone_ref_is_mono_pcm16_at_the_source_rate_with_edge_fades() {
        let d = tmp("ref");
        for (sr, want) in [(44_100u32, 44_100u32), (16_000, 16_000), (96_000, 48_000)] {
            let src = d.join(format!("vocals_{sr}.wav"));
            sine_wav(&src, sr, 2, 0.5, 3.0);
            let out = d.join(format!("ref_{sr}.wav"));
            trim_ref(&src, &out, 1.0, 2.5).unwrap();
            let bytes = std::fs::read(&out).unwrap();
            assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 1, "WAVE_FORMAT_PCM, а не EXTENSIBLE ({sr})");
            let sr = want;
            let mut r = hound::WavReader::open(&out).unwrap();
            let spec = r.spec();
            assert_eq!(spec.sample_rate, sr, "частота источника, не выше 48 кГц");
            assert_eq!((spec.channels, spec.bits_per_sample, spec.sample_format), (1, 16, hound::SampleFormat::Int));
            let s: Vec<f32> = r.samples::<i16>().map(|v| v.unwrap() as f32 / 32768.0).collect();
            assert!((s.len() as f64 / sr as f64 - 1.5).abs() < 0.01, "длина {}", s.len());
            let ms = |t: f64| (t * sr as f64 / 1000.0) as usize;
            let peak = |a: &[f32]| a.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let body = peak(&s[ms(20.0)..ms(40.0)]);
            assert!(body > 0.2, "за фейдом полный уровень: {body}");
            assert!(peak(&s[..ms(2.0)]) < 0.1 * body, "фейд на входе");
            assert!(peak(&s[s.len() - ms(2.0)..]) < 0.1 * body, "фейд на выходе");
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// Форматы экспорта и их кодеки: `acx` — MP3 192 кбит/с CBR 44,1 кГц, как требует ACX.
pub const EXPORT_FORMATS: [&str; 6] = ["wav", "flac", "mp3", "ogg", "m4a", "m4b"];

/// Что записать в файл экспорта: теги, главы (начало, конец в секундах, название) и обложка.
#[derive(Default, Debug, Clone)]
pub struct AudioMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub comment: String,
    pub genre: String,
    pub chapters: Vec<(f64, f64, String)>,
    pub cover: Option<PathBuf>,
    /// Synced text (LRC, word by word where timings are known): FLAC's LYRICS, MP4's ©lyr, MP3's USLT.
    pub lyrics: String,
}

/// Значение ffmetadata: `= ; # \` и перевод строки экранируются обратной косой.
fn ffmeta_escape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        if matches!(c, '=' | ';' | '#' | '\\' | '\n') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn ffmetadata(meta: &AudioMeta) -> String {
    let mut s = String::from(";FFMETADATA1\n");
    for (k, v) in [("title", &meta.title), ("artist", &meta.artist), ("album", &meta.album), ("comment", &meta.comment), ("genre", &meta.genre), ("lyrics", &meta.lyrics)] {
        if !v.is_empty() {
            s.push_str(&format!("{k}={}\n", ffmeta_escape(v)));
        }
    }
    for (a, z, title) in &meta.chapters {
        s.push_str(&format!("[CHAPTER]\nTIMEBASE=1/1000\nSTART={}\nEND={}\ntitle={}\n", (a * 1000.0).round() as u64, (z * 1000.0).round() as u64, ffmeta_escape(title)));
    }
    s
}

pub fn encode_audio(src: &Path, dst: &Path, format: &str, acx: bool) -> Result<(), String> {
    encode_audio_meta(src, dst, format, acx, &AudioMeta::default(), &Quality::default())
}

/// The export's sample rate, bit depth (WAV, FLAC) and bitrate (MP3, OGG, M4A); none set — the format's own.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Quality {
    pub rate: Option<u32>,
    pub bits: Option<u16>,
    pub kbps: Option<u32>,
}

pub const EXPORT_RATES: [u32; 6] = [16_000, 22_050, 24_000, 32_000, 44_100, 48_000];

impl Quality {
    /// Refuses what the format cannot carry instead of quietly writing something else.
    pub fn check(&self, format: &str) -> Result<(), String> {
        if let Some(r) = self.rate {
            if !EXPORT_RATES.contains(&r) || (format == "mp3" && !matches!(r, 32_000 | 44_100 | 48_000)) {
                return Err(t!("media-bad-rate", rate = r as i64, format = format.to_string()));
            }
        }
        if let Some(b) = self.bits {
            let ok = match format { "wav" => matches!(b, 16 | 24 | 32), "flac" => matches!(b, 16 | 24), _ => false };
            if !ok {
                return Err(t!("media-bad-bits", bits = b as i64, format = format.to_string()));
            }
        }
        if let Some(k) = self.kbps {
            if !matches!(format, "mp3" | "ogg" | "m4a" | "m4b") || !(64..=320).contains(&k) {
                return Err(t!("media-bad-kbps", kbps = k as i64, format = format.to_string()));
            }
        }
        Ok(())
    }
}

/// Звук `src` из кусков `keep` (секунды) встык — FLAC. Граф фильтров идёт файлом: кусков бывают сотни, а командная
/// строка Windows короче.
pub fn keep_audio_ranges(src: &Path, dst: &Path, keep: &[(f64, f64)]) -> Result<(), String> {
    if keep.is_empty() {
        return Err("nothing to keep".into());
    }
    let mut graph = String::new();
    for (i, (a, b)) in keep.iter().enumerate() {
        graph.push_str(&format!("[0:a:0]atrim=start={a:.3}:end={b:.3},asetpts=PTS-STARTPTS[k{i}];"));
    }
    for i in 0..keep.len() {
        graph.push_str(&format!("[k{i}]"));
    }
    graph.push_str(&format!("concat=n={}:v=0:a=1[out]", keep.len()));
    let script = tempfile::Builder::new().prefix("cut-").suffix(".txt").tempfile().map_err(|e| e.to_string())?;
    std::fs::write(script.path(), graph).map_err(|e| format!("{}: {e}", script.path().display()))?;
    dub_core::atomic::write_with(dst, |tmp| {
        run_ff_args(&os_args(&[
            OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(),
            OsStr::new("-/filter_complex"), script.path().as_os_str(),
            OsStr::new("-map"), OsStr::new("[out]"), OsStr::new("-c:a"), OsStr::new("flac"), OsStr::new("-f"), OsStr::new("flac"), tmp.as_os_str(),
        ]))
    })
}

/// Закодировать звук в формат экспорта с тегами; у M4B и M4A — ещё главы и обложка.
pub fn encode_audio_meta(src: &Path, dst: &Path, format: &str, acx: bool, meta: &AudioMeta, q: &Quality) -> Result<(), String> {
    let kbps = q.kbps.map(|k| format!("{k}k"));
    let mut codec: Vec<String> = match format {
        "wav" => vec!["-c:a".into(), match q.bits { Some(24) => "pcm_s24le", Some(32) => "pcm_f32le", _ => "pcm_s16le" }.into()],
        "flac" => vec!["-c:a".into(), "flac".into(), "-sample_fmt".into(), if q.bits == Some(16) { "s16" } else { "s32" }.into()],
        "mp3" if acx => vec!["-c:a".into(), "libmp3lame".into(), "-b:a".into(), "192k".into(), "-ar".into(), "44100".into()],
        "mp3" => match &kbps { Some(k) => vec!["-c:a".into(), "libmp3lame".into(), "-b:a".into(), k.clone()], None => vec!["-c:a".into(), "libmp3lame".into(), "-q:a".into(), "2".into()] },
        "ogg" => match &kbps { Some(k) => vec!["-c:a".into(), "libvorbis".into(), "-b:a".into(), k.clone()], None => vec!["-c:a".into(), "libvorbis".into(), "-q:a".into(), "6".into()] },
        "m4a" | "m4b" => vec!["-c:a".into(), "aac".into(), "-b:a".into(), kbps.clone().unwrap_or_else(|| "128k".into()), "-movflags".into(), "+faststart".into()],
        other => return Err(t!("media-bad-format", format = other.to_string())),
    };
    if let Some(r) = q.rate.filter(|_| !(acx && format == "mp3")) {
        codec.extend(["-ar".to_string(), r.to_string()]);
    }
    let work = tempfile::Builder::new().prefix("meta-").tempdir().map_err(|e| e.to_string())?;
    let meta_file = work.path().join("meta.txt");
    std::fs::write(&meta_file, ffmetadata(meta)).map_err(|e| e.to_string())?;
    let mp4 = matches!(format, "m4a" | "m4b");
    let cover = meta.cover.as_ref().filter(|c| c.is_file() && matches!(format, "m4a" | "m4b" | "mp3" | "flac"));
    dub_core::atomic::write_with(dst, |tmp| {
        let mut args: Vec<&OsStr> = vec![OsStr::new("-y"), OsStr::new("-i"), src.as_os_str(), OsStr::new("-i"), meta_file.as_os_str()];
        if let Some(c) = cover {
            args.extend([OsStr::new("-i"), c.as_os_str()]);
        }
        args.extend([OsStr::new("-map"), OsStr::new("0:a"), OsStr::new("-map_metadata"), OsStr::new("1"), OsStr::new("-map_chapters"), OsStr::new("1")]);
        if cover.is_some() {
            args.extend([OsStr::new("-map"), OsStr::new("2:v"), OsStr::new("-c:v"), OsStr::new(if mp4 { "copy" } else { "mjpeg" }), OsStr::new("-disposition:v:0"), OsStr::new("attached_pic")]);
        }
        args.extend([OsStr::new("-af"), OsStr::new("aformat=channel_layouts=mono")]);
        args.extend(codec.iter().map(|c| OsStr::new(c.as_str())));
        args.extend([OsStr::new("-f"), OsStr::new(match format { "m4a" | "m4b" => "ipod", "ogg" => "ogg", f => f })]);
        args.push(tmp.as_os_str());
        run_ff(&args)
    })
}

#[cfg(test)]
mod quality_tests {
    use super::Quality;

    #[test]
    fn export_quality_refuses_what_the_format_cannot_carry() {
        let q = |rate, bits, kbps| Quality { rate, bits, kbps };
        assert!(q(Some(44_100), Some(24), None).check("wav").is_ok());
        assert!(q(Some(48_000), None, Some(320)).check("mp3").is_ok());
        assert!(q(Some(22_050), None, None).check("mp3").is_err());
        assert!(q(None, Some(32), None).check("flac").is_err());
        assert!(q(None, Some(24), None).check("mp3").is_err());
        assert!(q(None, None, Some(192)).check("wav").is_err());
        assert!(q(Some(11_025), None, None).check("ogg").is_err());
    }
}
