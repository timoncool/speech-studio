//! Проверка аудиокниги на требования ACX (Audible) и room tone вместо цифровой тишины. Пороги — по разбору
//! Auphonic «RMS loudness normalization for Audible ACX»: RMS всего файла −23…−18 дБ, истинный пик не выше −3 дБ, шумовой
//! пол не выше −60 дБ, в начале 0,5–1 с и в конце 1–5 с тишины с room tone, пауза внутри не длиннее 5 с, глава не
//! длиннее 120 минут.

use serde::Serialize;

pub const RMS_RANGE: (f64, f64) = (-23.0, -18.0);
pub const PEAK_MAX: f64 = -3.0;
pub const NOISE_MAX: f64 = -60.0;
pub const HEAD_RANGE: (f64, f64) = (0.5, 1.0);
pub const TAIL_RANGE: (f64, f64) = (1.0, 5.0);
pub const INNER_SILENCE_MAX: f64 = 5.0;
pub const LENGTH_MAX: f64 = 120.0 * 60.0;
/// Уровень room tone: ниже шумового пола ACX с запасом, но не ноль.
pub const ROOM_TONE_DB: f64 = -68.0;
/// Что тише этого — тишина для подсчёта пауз и краёв.
const SILENCE_DB: f64 = -50.0;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Check {
    pub name: &'static str,
    pub value: f64,
    pub passed: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Report {
    pub passed: bool,
    pub checks: Vec<Check>,
}

fn db(x: f64) -> f64 {
    if x <= 1e-12 { -120.0 } else { 20.0 * x.log10() }
}

fn rms(x: &[f32]) -> f64 {
    super::live::rms(x) as f64
}

/// Окна по 50 мс: тихие ли они.
fn quiet_windows(x: &[f32], sr: u32) -> Vec<bool> {
    let w = (sr as usize / 20).max(1);
    x.chunks(w).map(|c| db(rms(c)) < SILENCE_DB).collect()
}

pub fn report(x: &[f32], sr: u32) -> Result<Report, String> {
    let secs = x.len() as f64 / sr as f64;
    let peak_db = crate::audio::loudness::true_peak_db(x, sr)?;
    let half = (sr as usize / 2).max(1);
    let noise = x.chunks(half).filter(|c| c.len() == half).map(|c| db(rms(c))).fold(f64::INFINITY, f64::min);
    let quiet = quiet_windows(x, sr);
    let step = 0.05;
    let head = quiet.iter().take_while(|q| **q).count() as f64 * step;
    let tail = quiet.iter().rev().take_while(|q| **q).count() as f64 * step;
    let (mut longest, mut run) = (0usize, 0usize);
    let first = quiet.iter().position(|q| !q).unwrap_or(quiet.len());
    let last = quiet.iter().rposition(|q| !q).unwrap_or(0);
    for q in quiet.iter().take(last + 1).skip(first) {
        run = if *q { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let rms_db = db(rms(x));
    let noise = if noise.is_finite() { noise } else { -120.0 };
    let checks = vec![
        Check { name: "rms", value: rms_db, passed: (RMS_RANGE.0..=RMS_RANGE.1).contains(&rms_db) },
        Check { name: "peak", value: peak_db, passed: peak_db <= PEAK_MAX + 1e-6 },
        Check { name: "noise_floor", value: noise, passed: noise <= NOISE_MAX && noise > -110.0 },
        Check { name: "head", value: head, passed: head >= HEAD_RANGE.0 - 1e-6 && head <= HEAD_RANGE.1 + step },
        Check { name: "tail", value: tail, passed: tail >= TAIL_RANGE.0 - 1e-6 && tail <= TAIL_RANGE.1 + step },
        Check { name: "inner_silence", value: longest as f64 * step, passed: longest as f64 * step <= INNER_SILENCE_MAX },
        Check { name: "length", value: secs, passed: secs <= LENGTH_MAX },
    ];
    Ok(Report { passed: checks.iter().all(|c| c.passed), checks })
}

/// Детерминированный розовато-белый шум для room tone (без генератора случайных чисел, чтобы файл повторялся).
fn noise(n: usize, level_db: f64, seed: u64) -> Vec<f32> {
    let amp = 10f64.powf(level_db / 20.0) * 3f64.sqrt();
    let mut state = seed | 1;
    let mut prev = 0.0f64;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let white = (state as f64 / u64::MAX as f64) * 2.0 - 1.0;
            prev = 0.7 * prev + 0.3 * white;
            (prev * amp * 1.8) as f32
        })
        .collect()
}

/// Room tone: цифровая тишина (паузы склейки) заполняется тихим шумом, по краям — нужная ACX тишина.
pub fn room_tone(x: &mut Vec<f32>, sr: u32, head: f64, tail: f64) {
    let floor = noise(x.len(), ROOM_TONE_DB, 0x5eed);
    for (s, n) in x.iter_mut().zip(floor) {
        if s.abs() < 1e-5 {
            *s += n;
        }
    }
    let head_n = (head * sr as f64) as usize;
    let tail_n = (tail * sr as f64) as usize;
    let mut out = noise(head_n, ROOM_TONE_DB, 0xbeef);
    out.append(x);
    out.extend(noise(tail_n, ROOM_TONE_DB, 0xcafe));
    *x = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 16_000;

    fn tone(secs: f64, db_level: f64) -> Vec<f32> {
        let amp = 10f64.powf(db_level / 20.0) * 2f64.sqrt();
        (0..(secs * SR as f64) as usize).map(|i| (amp * (i as f64 * 0.12).sin()) as f32).collect()
    }

    #[test]
    fn a_book_made_with_room_tone_passes_and_digital_silence_does_not() {
        let mut x = tone(10.0, -20.0);
        x.extend(vec![0.0; SR as usize]);
        x.extend(tone(10.0, -20.0));
        let bare = report(&x, SR).unwrap();
        assert!(!bare.passed);
        assert!(!bare.checks.iter().find(|c| c.name == "noise_floor").unwrap().passed, "digital zero is not a noise floor");
        room_tone(&mut x, SR, 0.8, 2.0);
        let r = report(&x, SR).unwrap();
        assert!(r.passed, "{:?}", r.checks);
    }

    #[test]
    fn loud_peaks_and_long_pauses_are_named() {
        let mut x = tone(5.0, -6.0);
        x.extend(vec![0.0; 6 * SR as usize]);
        x.extend(tone(5.0, -6.0));
        let r = report(&x, SR).unwrap();
        let failed: Vec<&str> = r.checks.iter().filter(|c| !c.passed).map(|c| c.name).collect();
        assert!(failed.contains(&"peak") && failed.contains(&"inner_silence") && failed.contains(&"rms"), "{failed:?}");
    }
}
