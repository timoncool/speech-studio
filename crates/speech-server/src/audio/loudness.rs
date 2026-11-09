//! Громкость речи: интегральная громкость ITU-R BS.1770 (замер Dub) и выравнивание к цели линейным усилением с
//! речевым лимитером пиков вместо компрессии; лимитер чуть снижает громкость, поэтому замер и усиление
//! повторяются, пока до цели не меньше 0,5 LU.

use crate::jobs::limiter;

/// Межсэмпловые пики выше выборочных: лимитер держит выборки на полдецибела ниже потолка true-peak.
const TRUE_PEAK_MARGIN_DB: f64 = 0.5;
/// Частота, до которой поднимается сигнал при замере истинного пика.
const TRUE_PEAK_RATE: u32 = 192_000;

/// Интегральная громкость ITU-R BS.1770 (LUFS) моно-сигнала: K-weighting (high-shelf + high-pass
/// биквады как в pyloudnorm) -> блоки 400мс с overlap 75% -> абсолютный гейт -70 + относительный -10.
/// None если блоков не осталось (слишком коротко/тихо).
pub fn integrated_lufs(x: &[f32], sr: u32) -> Option<f64> {
    let hs = biquad_high_shelf(1681.9744509555319, 0.7071752369554196, 4.0, sr as f64);
    let hp = biquad_high_pass(38.13547087613982, 0.5003270373253953, sr as f64);
    let y = apply_biquad(&apply_biquad(x, &hs), &hp);
    let block = (0.4 * sr as f64) as usize;
    let step = (0.1 * sr as f64) as usize;
    if block == 0 || step == 0 || y.len() < block {
        return None;
    }
    let mut zs: Vec<f64> = Vec::new();
    let mut i = 0;
    while i + block <= y.len() {
        let ms: f64 = y[i..i + block].iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>()
            / block as f64;
        zs.push(ms);
        i += step;
    }
    if zs.is_empty() {
        return None;
    }
    let loud = |z: f64| -0.691 + 10.0 * (z.max(1e-12)).log10();
    // абсолютный гейт -70 LUFS
    let abs_gated: Vec<f64> = zs.iter().copied().filter(|&z| loud(z) >= -70.0).collect();
    if abs_gated.is_empty() {
        return None;
    }
    let mean_abs = abs_gated.iter().sum::<f64>() / abs_gated.len() as f64;
    let rel_thr = loud(mean_abs) - 10.0;
    let rel_gated: Vec<f64> = abs_gated.into_iter().filter(|&z| loud(z) >= rel_thr).collect();
    if rel_gated.is_empty() {
        return None;
    }
    let mean_rel = rel_gated.iter().sum::<f64>() / rel_gated.len() as f64;
    Some(loud(mean_rel))
}

/// Биквад-фильтр прямой формы I (b/a нормированы на a0). Возвращает отфильтрованный сигнал.
fn apply_biquad(x: &[f32], c: &[f64; 5]) -> Vec<f32> {
    let [b0, b1, b2, a1, a2] = *c;
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut out = Vec::with_capacity(x.len());
    for &xn in x {
        let xn = xn as f64;
        let yn = b0 * xn + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        x2 = x1;
        x1 = xn;
        y2 = y1;
        y1 = yn;
        out.push(yn as f32);
    }
    out
}

/// High-shelf биквад (pyloudnorm K-weighting stage 1). Коэффы [b0,b1,b2,a1,a2] нормированы на a0.
fn biquad_high_shelf(fc: f64, q: f64, gain_db: f64, sr: f64) -> [f64; 5] {
    let a = 10f64.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f64::consts::PI * fc / sr;
    let (cw, sw) = (w0.cos(), w0.sin());
    let alpha = sw / (2.0 * q);
    let am = a - 1.0;
    let ap = a + 1.0;
    let sa = 2.0 * a.sqrt() * alpha;
    let b0 = a * (ap + am * cw + sa);
    let b1 = -2.0 * a * (am + ap * cw);
    let b2 = a * (ap + am * cw - sa);
    let a0 = ap - am * cw + sa;
    let a1 = 2.0 * (am - ap * cw);
    let a2 = ap - am * cw - sa;
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

/// High-pass биквад (pyloudnorm K-weighting stage 2, RLB). Коэффы нормированы на a0.
fn biquad_high_pass(fc: f64, q: f64, sr: f64) -> [f64; 5] {
    let w0 = 2.0 * std::f64::consts::PI * fc / sr;
    let (cw, sw) = (w0.cos(), w0.sin());
    let alpha = sw / (2.0 * q);
    let b0 = (1.0 + cw) / 2.0;
    let b1 = -(1.0 + cw);
    let b2 = (1.0 + cw) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cw;
    let a2 = 1.0 - alpha;
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
}

/// E2E-верификация капшенов без TTS/аудио: собрать ASS из Project и вжечь блюр+сабы в captioned.mp4.
/// Используется примером verify_captions (реальные кадры порт-vs-эталон). Порт captions.build+burn

/// Результат выравнивания: громкость до и после и сколько раз пришлось ограничить пики.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levelled {
    pub before: f64,
    pub after: f64,
}

/// Истинный пик в dBTP: сигнал передискретизируется до 192 кГц и выше (как BS.1770 для 48 кГц ×4; звуку 24 кГц нужно
/// ×8) FFT-ресемплером rubato с ограничением полосы, берётся наибольший модуль.
pub fn true_peak_db(x: &[f32], sr: u32) -> Result<f64, String> {
    use audioadapter::Adapter;
    use rubato::Resampler;
    if x.len() < 2 {
        return Ok(x.first().map_or(-120.0, |v| 20.0 * (v.abs() as f64).max(1e-6).log10()));
    }
    let input = vec![x.iter().map(|&s| s as f64).collect::<Vec<_>>()];
    let adapter = audioadapter_buffers::direct::SequentialSliceOfVecs::new(&input, 1, x.len()).map_err(|e| format!("true peak buffer: {e}"))?;
    let factor = (TRUE_PEAK_RATE / sr.max(1)).max(4) as usize;
    let mut up = rubato::Fft::<f64>::new(sr as usize, sr as usize * factor, 1024, 1, rubato::FixedSync::Both).map_err(|e| format!("true peak resampler: {e}"))?;
    let out = up.process_all(&adapter, x.len(), None).map_err(|e| format!("true peak resampling: {e}"))?;
    let peak = (0..out.frames()).filter_map(|i| out.read_sample(0, i)).fold(0.0f64, |m, v| m.max(v.abs()));
    Ok(if peak <= 0.0 { -120.0 } else { 20.0 * peak.log10() })
}

/// Потолок фразы после выравнивания (Dub `VOICE_CEILING`).
const PIECE_CEILING: f32 = 0.985;

/// Одна фраза к общей громкости перед склейкой, чтобы куски и голоса звучали ровно (Dub `normalize_voice`):
/// интегральная громкость BS.1770 к −14 LUFS, короткая или почти беззвучная по замеру — RMS громких отсчётов к
/// −16 dBFS; усиление не больше +40 дБ, пики к полке держит речевой лимитер. Громкость программы потом ставит
/// пресет экспорта.
pub fn level_piece(x: &mut [f32], sr: u32) {
    let peak = x.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
    if peak < 1e-4 {
        return;
    }
    let by_lufs = (x.len() >= (0.4 * sr as f64) as usize)
        .then(|| integrated_lufs(x, sr))
        .flatten()
        .filter(|l| l.is_finite() && *l > -60.0)
        .map(|l| 10f64.powf((-14.0 - l) / 20.0));
    let gain = by_lufs.unwrap_or_else(|| {
        let loud: Vec<f64> = x.iter().filter(|v| v.abs() > peak * 0.05).map(|&v| v as f64 * v as f64).collect();
        let rms = if loud.is_empty() { x.iter().map(|&v| v as f64 * v as f64).sum::<f64>() / x.len() as f64 } else { loud.iter().sum::<f64>() / loud.len() as f64 }.sqrt().max(1e-9);
        10f64.powf(-16.0 / 20.0) / rms
    });
    let gain = gain.min(10f64.powf(40.0 / 20.0));
    x.iter_mut().for_each(|v| *v = (*v as f64 * gain) as f32);
    limiter::limit_mono(x, sr, PIECE_CEILING as f64, &limiter::SPEECH);
    x.iter_mut().for_each(|v| *v = v.clamp(-PIECE_CEILING, PIECE_CEILING));
}

/// Моно, которое играют обе колонки, звучит на 3,01 дБ громче замера одного канала (`ffmpeg-normalize --dual-mono`).
pub const DUAL_MONO_DB: f64 = 3.0103;

/// Привести моно-сигнал к `target` LUFS с истинным пиком не выше `peak_db` dBTP: ограничитель работает по отсчётам,
/// поэтому после него пик меряется по BS.1770, и при выходе за потолок порог опускается на превышение. С `dual_mono`
/// громкость меряется как у моно на двух колонках.
pub fn level(x: &mut [f32], sr: u32, target: f64, peak_db: f64, dual_mono: bool) -> Result<Levelled, String> {
    // a programme shorter than one gating block of BS.1770 (0.4 s: a single «Да.») has no integrated loudness; it is
    // measured by the mean square of its loud samples on the same scale, as `level_piece` does, instead of refusing
    // the export as silent
    let integrated_lufs = |x: &[f32], sr: u32| {
        integrated_lufs(x, sr)
            .filter(|l| l.is_finite())
            .or_else(|| {
                let peak = x.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
                let loud: Vec<f64> = x.iter().filter(|v| peak >= 1e-4 && v.abs() > peak * 0.05).map(|&v| v as f64 * v as f64).collect();
                (!loud.is_empty()).then(|| -0.691 + 10.0 * (loud.iter().sum::<f64>() / loud.len() as f64).max(1e-12).log10())
            })
            .map(|l| if dual_mono { l + DUAL_MONO_DB } else { l })
    };
    let before = integrated_lufs(x, sr).ok_or_else(|| t!("media-loudnorm-silent"))?;
    let mut limit_db = peak_db - TRUE_PEAK_MARGIN_DB;
    let mut now = before;
    for _ in 0..8 {
        let gain = 10f32.powf(((target - now) / 20.0) as f32);
        x.iter_mut().for_each(|v| *v *= gain);
        limiter::limit_mono(x, sr, 10f64.powf(limit_db / 20.0), &limiter::SPEECH);
        now = integrated_lufs(x, sr).ok_or_else(|| t!("media-loudnorm-silent"))?;
        let over = true_peak_db(x, sr)? - peak_db;
        if over > 0.0 {
            limit_db -= over + 0.05;
            continue;
        }
        if (now - target).abs() < 0.5 {
            break;
        }
    }
    let over = true_peak_db(x, sr)? - peak_db;
    if over > 0.0 {
        let gain = 10f32.powf((-(over + 0.05) / 20.0) as f32);
        x.iter_mut().for_each(|v| *v *= gain);
        now = integrated_lufs(x, sr).ok_or_else(|| t!("media-loudnorm-silent"))?;
    }
    Ok(Levelled { before, after: now })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speechlike(secs: f64, sr: u32, amp: f32) -> Vec<f32> {
        (0..(secs * sr as f64) as usize)
            .map(|i| {
                let t = i as f32 / sr as f32;
                let env = if (t * 3.0) as u32 % 2 == 0 { 1.0 } else { 0.15 };
                amp * env * ((t * 220.0 * std::f32::consts::TAU).sin() * 0.6 + (t * 1330.0 * std::f32::consts::TAU).sin() * 0.3 + if i % 97 == 0 { 0.9 } else { 0.0 })
            })
            .collect()
    }


    /// A one-word programme shorter than a gating block is levelled, not refused as silent; real silence is refused.
    #[test]
    fn a_programme_of_one_short_word_is_levelled() {
        let sr = 24_000;
        let mut word = speechlike(0.3, sr, 0.05);
        let done = level(&mut word, sr, -14.0, -1.0, false).expect("a short word has a level");
        assert!(done.after > done.before, "brought up towards the target: {done:?}");
        assert!(word.iter().all(|v| v.abs() < 1.0));
        assert!(level(&mut vec![0.0f32; 4_000], sr, -14.0, -1.0, false).is_err(), "silence has nothing to level");
    }

    #[test]
    fn a_quiet_recording_reaches_the_target_without_passing_the_peak() {
        let sr = 24_000;
        let mut x = speechlike(8.0, sr, 0.05);
        let r = level(&mut x, sr, -14.0, -1.0, false).unwrap();
        assert!(r.before < -25.0, "{r:?}");
        assert!((r.after + 14.0).abs() < 0.6, "{r:?}");
        let tp = true_peak_db(&x, sr).unwrap();
        assert!(tp <= -1.0 + 1e-6, "true peak {tp}");
    }

    #[test]
    fn dual_mono_lands_three_decibels_lower_in_the_channel() {
        let sr = 24_000;
        let mut x = speechlike(8.0, sr, 0.05);
        let r = level(&mut x, sr, -16.0, -1.0, true).unwrap();
        assert!((r.after + 16.0).abs() < 0.6, "{r:?}");
        let channel = integrated_lufs(&x, sr).unwrap();
        assert!((channel - (-16.0 - DUAL_MONO_DB)).abs() < 0.6, "one channel {channel}");
    }

    #[test]
    fn silence_is_refused() {
        let mut x = vec![0.0f32; 48_000];
        assert!(level(&mut x, 24_000, -16.0, -1.0, false).is_err());
    }

    #[test]
    fn a_quiet_and_a_loud_piece_come_to_one_level_under_the_ceiling() {
        let sr = 24_000;
        let (mut quiet, mut loud) = (speechlike(3.0, sr, 0.03), speechlike(3.0, sr, 0.9));
        level_piece(&mut quiet, sr);
        level_piece(&mut loud, sr);
        let (q, l) = (integrated_lufs(&quiet, sr).unwrap(), integrated_lufs(&loud, sr).unwrap());
        assert!((q - l).abs() < 1.0, "{q} {l}");
        assert!(quiet.iter().chain(&loud).all(|v| v.abs() <= PIECE_CEILING));
        let mut silence = vec![0.0f32; 24_000];
        level_piece(&mut silence, sr);
        assert!(silence.iter().all(|v| *v == 0.0));
    }
}
