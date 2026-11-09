//! Лимитер пиков фразы дубляжа — перенос audio_post::limiter студий (лимитер Hyrax из matchering).
//! Обработка офлайн, поэтому «предпросмотр» даёт centred_max + filtfilt с нулевой фазой: гейн начинает
//! опускаться ДО пика, а не срезает его. Жёсткий гейн (hard) входит в итоговый максимум, так что ни один
//! сэмпл не выходит за порог. Моно-вариант `limit_mono` и речевой пресет `SPEECH` — адаптация под дубляж.

#[derive(Debug, Clone, Copy)]
pub struct LimiterConfig {
    pub attack_ms: f64,
    pub hold_ms: f64,
    pub release_ms: f64,
    pub attack_filter_coefficient: f64,
    pub hold_filter_coefficient: f64,
    pub release_filter_coefficient: f64,
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            attack_ms: 1.0,
            hold_ms: 1.0,
            release_ms: 3000.0,
            attack_filter_coefficient: -2.0,
            hold_filter_coefficient: 7.0,
            release_filter_coefficient: 800.0,
        }
    }
}

/// Речевой пресет. Срез release-фильтра = release_filter_coefficient / release_ms Гц: 800/400 = 2 Гц,
/// постоянная времени ≈ 80 мс — гейн возвращается между слогами, а не давит всю фразу после одного
/// пика (как 3000 мс мастеринга музыки). Атака 5 мс ловит взрывные согласные, hold 10 мс держит гейн
/// на пике периода голоса и не даёт модуляции на низких частотах.
pub const SPEECH: LimiterConfig = LimiterConfig {
    attack_ms: 5.0,
    hold_ms: 10.0,
    release_ms: 400.0,
    attack_filter_coefficient: -2.0,
    hold_filter_coefficient: 7.0,
    release_filter_coefficient: 800.0,
};

fn ms_to_samples(ms: f64, rate: u32) -> usize {
    (rate as f64 * ms * 1e-3) as usize
}

/// Maximum over a centred window of `size` (odd), edges clipped.
fn centred_max(x: &[f64], size: usize) -> Vec<f64> {
    let half = size / 2;
    let n = x.len();
    let mut out = vec![0.0; n];
    let mut deque: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut next = 0;
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = (i + half).min(n - 1);
        while next <= hi {
            while deque.back().is_some_and(|&b| x[b] <= x[next]) {
                deque.pop_back();
            }
            deque.push_back(next);
            next += 1;
        }
        let lo = i.saturating_sub(half);
        while deque.front().is_some_and(|&f| f < lo) {
            deque.pop_front();
        }
        *slot = x[*deque.front().unwrap()];
    }
    out
}

/// Maximum over the `size` samples ending at each one.
fn trailing_max(x: &[f64], size: usize) -> Vec<f64> {
    let mut out = vec![0.0; x.len()];
    let mut deque: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..x.len() {
        while deque.back().is_some_and(|&b| x[b] <= x[i]) {
            deque.pop_back();
        }
        deque.push_back(i);
        while deque.front().is_some_and(|&f| f + size <= i) {
            deque.pop_front();
        }
        // the window reaches before the start, where matchering pads with zeros
        out[i] = x[*deque.front().unwrap()].max(0.0);
    }
    out
}

/// First-order Butterworth low-pass, bilinear with pre-warping (SciPy `butter(1, fc, fs=rate)`).
fn butter1(cutoff: f64, rate: u32) -> ([f64; 2], [f64; 2]) {
    let k = (std::f64::consts::PI * cutoff / rate as f64).tan();
    let b = k / (1.0 + k);
    ([b, b], [1.0, (k - 1.0) / (k + 1.0)])
}

/// `lfilter` for a first-order section, from rest.
fn lfilter1(b: [f64; 2], a: [f64; 2], x: &[f64]) -> Vec<f64> {
    let mut y = Vec::with_capacity(x.len());
    let (mut x1, mut y1) = (0.0, 0.0);
    for &xi in x {
        let yi = b[0] * xi + b[1] * x1 - a[1] * y1;
        y.push(yi);
        x1 = xi;
        y1 = yi;
    }
    y
}

/// SciPy `filtfilt(b=[1-c], a=[1,-c], x)`: odd extension of 6 samples at both
/// ends, steady-state initial conditions, forward then backward.
fn filtfilt_one_pole(c: f64, x: &[f64]) -> Vec<f64> {
    let n = x.len();
    let pad = 6.min(n.saturating_sub(1));
    let mut ext = Vec::with_capacity(n + 2 * pad);
    for i in (1..=pad).rev() {
        ext.push(2.0 * x[0] - x[i]);
    }
    ext.extend_from_slice(x);
    for i in 1..=pad {
        ext.push(2.0 * x[n - 1] - x[n - 1 - i]);
    }
    let b0 = 1.0 - c;
    let run = |input: &[f64]| -> Vec<f64> {
        // lfilter_zi of this section is c: a unit step settles at 1 from sample one
        let mut state = c * input[0];
        input
            .iter()
            .map(|&xi| {
                let yi = b0 * xi + state;
                state = c * yi;
                yi
            })
            .collect()
    };
    let forward = run(&ext);
    let reversed: Vec<f64> = forward.into_iter().rev().collect();
    let backward = run(&reversed);
    let mut y: Vec<f64> = backward.into_iter().rev().collect();
    y.drain(..pad);
    y.truncate(n);
    y
}

/// Кривая гейна (множитель 0..1 на сэмпл) по выпрямленному пику `peak[i]`. None — ни один сэмпл не
/// выше порога, сигнал не трогаем.
fn gain_curve(peak: &[f64], rate: u32, threshold: f64, config: &LimiterConfig) -> Option<Vec<f32>> {
    let rectified: Vec<f64> = peak.iter().map(|&p| p.max(threshold) / threshold).collect();
    if rectified.iter().all(|&r| (r - 1.0).abs() <= 1e-8 + 1e-5) {
        return None;
    }
    let hard: Vec<f64> = rectified.iter().map(|&r| 1.0 - 1.0 / r).collect();

    let attack = ms_to_samples(config.attack_ms, rate).max(1);
    let odd = if attack.is_multiple_of(2) { attack + 1 } else { attack };
    let slided = centred_max(&hard, 2 * odd - 1);
    let c = (config.attack_filter_coefficient / attack as f64).exp();
    let gain_attack = filtfilt_one_pole(c, &slided);

    let hold = ms_to_samples(config.hold_ms, rate).max(1);
    let held = trailing_max(&slided, hold);
    let (b, a) = butter1(config.hold_filter_coefficient, rate);
    let hold_out = lfilter1(b, a, &held);
    let (b, a) = butter1(config.release_filter_coefficient / config.release_ms, rate);
    let release_in: Vec<f64> = held.iter().zip(&hold_out).map(|(&h, &o)| h.max(o)).collect();
    let release_out = lfilter1(b, a, &release_in);

    Some(
        (0..peak.len())
            .map(|i| (1.0 - hard[i].max(gain_attack[i]).max(hold_out[i].max(release_out[i]))) as f32)
            .collect(),
    )
}

/// Моно-вариант студийного `limit`: выпрямленный пик берётся по одному каналу, остальное то же. Возвращает число
/// сэмплов, которые без лимитера вышли бы за порог (телеметрия для журнала рендера). Кривая гейна стоит несколько
/// массивов f64 на сэмпл, поэтому длинный сигнал (аудиокнига) идёт кусками по `SEGMENT_SECS`.
pub fn limit_mono(x: &mut [f32], rate: u32, threshold: f64, config: &LimiterConfig) -> usize {
    limit_segmented(x, rate, threshold, config, (SEGMENT_SECS * rate as f64) as usize)
}

/// Самый длинный кусок, по которому кривая гейна строится разом.
const SEGMENT_SECS: f64 = 60.0;

/// Куски режутся только там, где перед разрезом столько секунд ни один сэмпл не выше порога — релиз (`SPEECH`:
/// фильтр 2 Гц, τ 80 мс) затух до ~1e-11, — и после него ещё `QUIET_AFTER_SECS`, дальше атаки и удержания: кривая
/// по обе стороны разреза та же, что по всему сигналу.
const QUIET_BEFORE_SECS: f64 = 2.0;
const QUIET_AFTER_SECS: f64 = 0.25;

fn limit_segmented(x: &mut [f32], rate: u32, threshold: f64, config: &LimiterConfig, segment: usize) -> usize {
    let over = x.iter().filter(|v| v.abs() as f64 > threshold).count();
    let mut start = 0;
    for end in piece_ends(x, rate, threshold, segment) {
        limit_piece(&mut x[start..end], rate, threshold, config);
        start = end;
    }
    over
}

/// Where each piece of `x` the limiter takes at once ends: at most two `segment`s long.
fn piece_ends(x: &[f32], rate: u32, threshold: f64, segment: usize) -> Vec<usize> {
    let segment = segment.max(1);
    let (mut ends, mut start) = (Vec::new(), 0);
    while start < x.len() {
        let end = match x.len() - start {
            left if left <= segment => x.len(),
            left => {
                let window = &x[start + segment..start + left.min(segment.saturating_mul(2))];
                quiet_cut(window, rate, threshold, left <= segment.saturating_mul(2)).map_or(x.len(), |at| start + segment + at)
            }
        };
        ends.push(end);
        start = end;
    }
    ends
}

/// Where in `x` to cut: the first place with `QUIET_BEFORE_SECS` before it and `QUIET_AFTER_SECS` after it where no
/// sample is above the threshold. A stretch over the threshold all the time (a loud preset on dense speech) has no such
/// place; then the cut goes where the quiet before it is longest, the release there decayed the most. None — `x` is
/// the signal's tail, to be limited with what comes before it.
fn quiet_cut(x: &[f32], rate: u32, threshold: f64, tail: bool) -> Option<usize> {
    let before = ((QUIET_BEFORE_SECS * rate as f64) as usize).max(1);
    let after = ((QUIET_AFTER_SECS * rate as f64) as usize).max(1);
    let (mut run, mut best) = (0, (0, x.len()));
    for (i, v) in x.iter().enumerate() {
        if v.abs() as f64 > threshold {
            run = 0;
            continue;
        }
        run += 1;
        if run == before + after {
            return Some(i + 1 - after);
        }
        if run > after && run - after > best.0 {
            best = (run - after, i + 1 - after);
        }
    }
    (!tail).then_some(best.1)
}

fn limit_piece(x: &mut [f32], rate: u32, threshold: f64, config: &LimiterConfig) {
    let peak: Vec<f64> = x.iter().map(|v| v.abs() as f64).collect();
    if let Some(gain) = gain_curve(&peak, rate, threshold, config) {
        for (v, g) in x.iter_mut().zip(gain) {
            *v *= g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_changes_below_the_threshold() {
        let mut l = vec![0.5f32; 1000];
        limit_mono(&mut l, 48_000, 0.998, &LimiterConfig::default());
        assert!(l.iter().all(|&v| v == 0.5));
    }

    #[test]
    fn peaks_come_down_to_the_threshold() {
        let n = 48_000;
        let mut l: Vec<f32> = (0..n).map(|i| 1.6 * ((i as f32) * 0.05).sin()).collect();
        limit_mono(&mut l, 48_000, 0.998, &LimiterConfig::default());
        let peak = l.iter().fold(0.0f32, |p, v| p.max(v.abs()));
        assert!(peak <= 0.999, "peak {peak}");
        assert!(peak > 0.8, "limited too hard: {peak}");
    }

    #[test]
    fn filtfilt_keeps_a_constant() {
        let x = vec![0.25; 300];
        let y = filtfilt_one_pole(0.96, &x);
        assert!(y.iter().all(|v| (v - 0.25).abs() < 1e-9));
    }

    const SR: u32 = 24_000;
    const PLOSIVE_AT: f32 = 1.0625;
    const PLOSIVE_LEN: f32 = 0.012;

    /// Речеподобная фраза 2 с: тон 180 Гц со слоговой огибающей 4 Гц, пик огибающей 0.5, и одна
    /// взрывная согласная — гладкий (ханн 12 мс) всплеск x`burst` на пике слога.
    fn phrase(amp: f32, burst: f32) -> Vec<f32> {
        (0..SR as usize * 2)
            .map(|i| {
                let t = i as f32 / SR as f32;
                let syll = 0.55 + 0.45 * (2.0 * std::f32::consts::PI * 4.0 * t).sin();
                let mut v = amp * syll * (2.0 * std::f32::consts::PI * 180.0 * t).sin();
                let ph = t - PLOSIVE_AT;
                if (0.0..PLOSIVE_LEN).contains(&ph) {
                    let h = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * ph / PLOSIVE_LEN).cos();
                    v *= 1.0 + (burst - 1.0) * h;
                }
                v
            })
            .collect()
    }

    fn rms_of(x: &[f32], keep: impl Fn(f32) -> bool) -> f64 {
        let (mut sum, mut n) = (0.0f64, 0usize);
        for (i, &v) in x.iter().enumerate() {
            if keep(i as f32 / SR as f32) {
                sum += (v as f64) * (v as f64);
                n += 1;
            }
        }
        (sum / n.max(1) as f64).sqrt()
    }

    #[test]
    fn speech_limiter_leaves_no_overs() {
        let mut x = phrase(0.5, 4.0); // всплеск до ~1.97 = +6 дБ над полкой 0.985
        let over = limit_mono(&mut x, SR, 0.985, &SPEECH);
        assert!(over > 0, "в тестовой фразе должны быть перегрузки");
        let peak = x.iter().fold(0.0f32, |p, v| p.max(v.abs()));
        assert!(peak <= 0.985 + 1e-6, "перегрузка после лимитера: {peak}");
    }

    #[test]
    fn speech_limiter_does_not_touch_a_quiet_phrase() {
        let orig = phrase(0.3, 1.0);
        let mut x = orig.clone();
        let over = limit_mono(&mut x, SR, 0.985, &SPEECH);
        assert_eq!(over, 0);
        assert_eq!(x, orig, "тихая фраза должна пройти без изменений");
    }

    #[test]
    fn speech_limiter_keeps_the_phrase_loudness() {
        let orig = phrase(0.5, 4.0);
        let mut x = orig.clone();
        limit_mono(&mut x, SR, 0.985, &SPEECH);
        // Вне всплеска и хвоста release (200 мс) фраза звучит как была.
        let body = |t: f32| !(PLOSIVE_AT - 0.02..PLOSIVE_AT + 0.2).contains(&t);
        let body_db = 20.0 * (rms_of(&x, body) / rms_of(&orig, body)).log10();
        assert!(body_db.abs() < 0.1, "лимитер изменил громкость фразы вне пика: {body_db:.3} дБ");
        // Целиком фраза теряет только энергию самого пика.
        let all_db = 20.0 * (rms_of(&x, |_| true) / rms_of(&orig, |_| true)).log10();
        assert!(all_db > -0.5, "лимитер съел громкость фразы: {all_db:.2} дБ");
    }

    #[test]
    fn speech_limiter_gain_has_no_steps() {
        // Щелчок = скачок гейна между соседними сэмплами. У жёсткого клипа гейн на пике прыгает на
        // каждом сэмпле; у лимитера кривая гладкая.
        let orig = phrase(0.5, 4.0);
        let peak: Vec<f64> = orig.iter().map(|v| v.abs() as f64).collect();
        let gain = gain_curve(&peak, SR, 0.985, &SPEECH).expect("есть перегрузки");
        let max_step = gain.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(max_step < 0.02, "скачок гейна лимитера {max_step}");
        let clip_gain: Vec<f64> = peak.iter().map(|&p| 0.985 / p.max(0.985)).collect();
        let clip_steps = clip_gain.windows(2).filter(|w| (w[1] - w[0]).abs() > 0.02).count();
        assert!(clip_steps > 0, "жёсткий клип на этой фразе должен давать скачки гейна");
    }

    /// How far the limited sound cut into pieces of `segment` samples is from the sound limited whole, and how many
    /// pieces there were.
    fn segmented_against_whole(x: &[f32], rate: u32, threshold: f64, segment: usize) -> (f32, usize) {
        let mut whole = x.to_vec();
        let mut pieces = x.to_vec();
        assert_eq!(limit_segmented(&mut whole, rate, threshold, &SPEECH, usize::MAX), limit_segmented(&mut pieces, rate, threshold, &SPEECH, segment));
        (whole.iter().zip(&pieces).fold(0.0f32, |m, (a, b)| m.max((a - b).abs())), piece_ends(x, rate, threshold, segment).len())
    }

    /// A long programme is limited piece by piece, cut only in its long pauses, and comes out as limited whole.
    #[test]
    fn a_long_programme_limited_in_pieces_is_the_programme_limited_whole() {
        let mut x: Vec<f32> = Vec::new();
        for n in 0..24 {
            x.extend(phrase(0.8, 2.2));
            x.extend(std::iter::repeat_n(0.0, SR as usize * if n % 4 == 3 { 5 } else { 1 } * 3 / 5));
        }
        let (diff, pieces) = segmented_against_whole(&x, SR, 0.985, SR as usize * 15);
        assert!(pieces >= 3, "{pieces} pieces");
        assert!(diff < 1e-6, "{diff}");
    }

    /// Speech over the threshold all the time has no quiet place to cut in; it is still limited in pieces, and the
    /// pieces are cut where the quiet before them is longest.
    #[test]
    fn dense_loud_speech_is_cut_where_it_is_quietest() {
        let x: Vec<f32> = (0..20).flat_map(|_| phrase(0.8, 2.2)).collect();
        let (diff, pieces) = segmented_against_whole(&x, SR, 0.985, SR as usize * 6);
        eprintln!("dense: {pieces} pieces, the largest difference {diff:e}");
        assert!(pieces >= 3, "{pieces} pieces");
        assert!(diff < 1e-3, "{diff}");
    }

    /// Sound over the threshold the whole window has no quiet place: the cut falls in it, and the gain on its two
    /// sides stays within half a decibel of the gain over the sound limited whole.
    #[test]
    fn a_cut_in_sound_over_the_threshold_leaves_no_audible_step() {
        let x: Vec<f32> = (0..SR as usize * 30).map(|i| 1.4 * (i as f32 * 2.0 * std::f32::consts::PI * 180.0 / SR as f32).sin() * (0.8 + 0.2 * (i as f32 / SR as f32 * 3.0).sin())).collect();
        let segment = SR as usize * 6;
        let ends = piece_ends(&x, SR, 0.985, segment);
        assert!(ends.len() >= 3, "{ends:?}");
        let mut whole = x.clone();
        let mut pieces = x.clone();
        limit_segmented(&mut whole, SR, 0.985, &SPEECH, usize::MAX);
        limit_segmented(&mut pieces, SR, 0.985, &SPEECH, segment);
        let db = |a: f32, b: f32| 20.0 * (a.abs().max(1e-9) / b.abs().max(1e-9)).log10();
        let step = x.iter().zip(whole.iter().zip(&pieces)).filter(|(v, _)| v.abs() > 0.1).map(|(_, (w, p))| db(*p, *w).abs()).fold(0.0f32, f32::max);
        eprintln!("cut in sound over the threshold: pieces end at {ends:?}, the largest gain difference {step:.3} dB");
        assert!(step < 0.5, "{step} dB");
    }

    /// The memory the limiter takes does not grow with the programme once it is longer than two pieces.
    #[test]
    fn the_limiter_s_memory_does_not_grow_with_the_programme() {
        let take = |minutes: usize| {
            let mut x: Vec<f32> = (0..minutes * 30).flat_map(|_| phrase(0.8, 2.2)).collect();
            crate::alloc_meter::peak_of(|| {
                limit_mono(&mut x, SR, 0.985, &SPEECH);
            })
        };
        let (short, long) = (take(3), take(9));
        eprintln!("limiter: 3 min {short} bytes, 9 min {long}");
        assert!(long < short + short / 5, "3 min {short} bytes, 9 min {long}");
    }

    /// The same on real takes: `TAKES_DIR` — a project's takes folder, its takes one after another with 300 ms
    /// between them, raised to `TAKES_LUFS` (-14) and limited at -1.5 dBFS, as a loud export levels them.
    #[test]
    #[ignore]
    fn real_takes_limited_in_pieces_are_the_takes_limited_whole() {
        let dir = std::path::PathBuf::from(std::env::var("TAKES_DIR").expect("TAKES_DIR"));
        let work = tempfile::tempdir().unwrap();
        let mut x: Vec<f32> = Vec::new();
        let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == "flac" || e == "wav")).collect();
        files.sort();
        for (i, f) in files.iter().enumerate() {
            let wav = work.path().join(format!("{i}.wav"));
            crate::audio::media::extract_audio(f, &wav, 24_000, 1).unwrap();
            x.extend(crate::audio::wavio::read_mono_f32(&wav).unwrap().0);
            x.extend(std::iter::repeat_n(0.0, 7_200));
        }
        let lufs: f64 = std::env::var("TAKES_LUFS").ok().and_then(|s| s.parse().ok()).unwrap_or(-14.0);
        let gain = 10f32.powf(((lufs - crate::audio::loudness::integrated_lufs(&x, 24_000).unwrap()) / 20.0) as f32);
        x.iter_mut().for_each(|v| *v *= gain);
        let threshold = 10f64.powf(-1.5 / 20.0);
        eprintln!("{:.2} % of the samples over the threshold", 100.0 * x.iter().filter(|v| v.abs() as f64 > threshold).count() as f64 / x.len() as f64);
        let (diff, pieces) = segmented_against_whole(&x, 24_000, threshold, 24_000 * 30);
        eprintln!("{} takes, {:.1} min, {pieces} pieces, the largest difference {diff:e}", files.len(), x.len() as f64 / 24_000.0 / 60.0);
        assert!(pieces >= 2, "{pieces} pieces");
        assert!(diff < 1e-6, "{diff}");
    }
}
