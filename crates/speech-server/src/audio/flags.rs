//! Checks of a take's sound carried over from Game Dub, where they caught the defects of Google's Gemini takes
//! (`tts-audit`, `chk/tts_flags.js`, `chk/tts_verdict.js`, `docs/gta5/QC_RECIPES.md`; the synthesis defect detector
//! is the studio's `tts_host::defect`, Dub Studio's `synth_defect`: local engines run it while voicing, cloud ones,
//! Google's among them, only here). Recognising every take is costly, so cheap measures of the sound run on all of
//! them and sort what they find three ways: a defect (the take is redone), a suspect (the take is recognised and the
//! verdict decides), a note (shown, nothing else). Each kind can be turned off in the project; the cheap ones are on
//! by default, the costly two (a repeat by self-similarity of the sound, a whistle in the spectrum) are off.

use realfft::RealFftPlanner;

/// Every kind, in the order the settings list them.
#[cfg(test)]
pub const KINDS: [&str; 10] = ["defect", "clipping", "length", "edge", "long_pause", "little_speech", "loudness", "pause_noise", "audio_repeat", "whistle"];
/// On unless the project turns them off.
pub const CHEAP: [&str; 8] = ["defect", "clipping", "length", "edge", "long_pause", "little_speech", "loudness", "pause_noise"];

/// What a raised flag means for the take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Wrong as it is: the take is redone.
    Defect,
    /// Maybe wrong: recognition of the take decides.
    Suspect,
    /// Worth a look, not a reason to redo: loudness off the voice's usual, noise in the pauses.
    Note,
}

/// The role of a raised flag (the names `flags` and `defect` give).
pub fn role(flag: &str) -> Role {
    match flag {
        "runaway" | "cutoff" | "silence" | "hum" | "click" | "clipping" | "whistle" => Role::Defect,
        "loud" | "quiet" | "pause_noise" => Role::Note,
        _ => Role::Suspect,
    }
}

/// What the sound of a take measures.
#[derive(Debug, Default, Clone)]
pub struct Features {
    pub raw_s: f64,
    /// Without silence at the edges.
    pub trimmed_s: f64,
    /// Share of 10 ms frames no quieter than the speech peak −35 dB.
    pub speech_frac: f64,
    /// Runs of three or more samples at full scale.
    pub clip_events: usize,
    /// Share of the samples at full scale.
    pub clip_share: f64,
    pub lufs: Option<f64>,
    /// The longest quiet stretch between the first and the last loud frame.
    pub max_pause_s: f64,
    /// The loudest of the first and last 20 ms against the peak, dB: an edge cut mid-word stays loud.
    pub edge_db: f64,
    /// The median level of the quiet frames inside speech, dB.
    pub noise_db: Option<f64>,
    /// Share of speech frames whose one narrow 2–10 kHz peak stands 30 dB over the band: a whistle.
    pub tonal_frac: f64,
    /// The longest stretch of speech heard again 0.4 s or more later.
    pub rep_s: f64,
}

fn db(x: f64) -> f64 {
    20.0 * x.max(1e-9).log10()
}

/// The take's sound measured; `whistle` and `repeat` ask for the costly passes.
pub fn measure(x: &[f32], sr: u32, whistle: bool, repeat: bool) -> Features {
    let mut f = Features { raw_s: x.len() as f64 / sr as f64, ..Default::default() };
    if x.is_empty() {
        return f;
    }
    f.trimmed_s = crate::audio::trim::trim_edges(x, sr).0.len() as f64 / sr as f64;
    let mut run = 0usize;
    f.clip_share = x.iter().filter(|v| v.abs() >= 0.999).count() as f64 / x.len() as f64;
    for v in x {
        if v.abs() >= 0.999 {
            run += 1;
            if run == 3 {
                f.clip_events += 1;
            }
        } else {
            run = 0;
        }
    }
    f.lufs = crate::audio::loudness::integrated_lufs(x, sr);
    let w = (sr as usize / 100).max(1);
    let rms: Vec<f64> = x.chunks(w).map(|c| (c.iter().map(|&v| (v as f64) * (v as f64)).sum::<f64>() / c.len() as f64).sqrt()).collect();
    let peak = rms.iter().cloned().fold(0.0f64, f64::max);
    let thr = peak * 10f64.powf(-35.0 / 20.0);
    let loud: Vec<bool> = rms.iter().map(|&v| v >= thr && v > 1e-6).collect();
    f.speech_frac = loud.iter().filter(|&&l| l).count() as f64 / loud.len().max(1) as f64;
    let mut quiet_frames: Vec<usize> = Vec::new();
    if let (Some(a), Some(b)) = (loud.iter().position(|&l| l), loud.iter().rposition(|&l| l)) {
        let (mut q, mut longest) = (0usize, 0usize);
        for (i, &l) in loud.iter().enumerate().take(b + 1).skip(a) {
            if l {
                q = 0;
            } else {
                q += 1;
                longest = longest.max(q);
                quiet_frames.push(i);
            }
        }
        f.max_pause_s = longest as f64 * w as f64 / sr as f64;
    }
    let n20 = (sr as usize / 50).min(x.len());
    let edge = |s: &[f32]| s.iter().fold(0.0f32, |m, v| m.max(v.abs())) as f64;
    let peak_abs = x.iter().fold(0.0f32, |m, v| m.max(v.abs())) as f64;
    // a word begins fast: an engine that gives no silence before the speech (Audio8) reaches −10 dB of its peak within
    // 20 ms of a natural onset, while a start cut mid-word is loud from its first milliseconds; so the start is judged
    // by its first 5 ms and the end, where speech dies away slowly, by its last 20
    let n5 = (sr as usize / 200).min(x.len());
    f.edge_db = db(edge(&x[x.len() - n20..]).max(edge(&x[..n5])) / peak_abs.max(1e-9));
    let mut qr: Vec<f64> = quiet_frames.iter().map(|&i| rms[i]).collect();
    qr.sort_by(|a, b| a.total_cmp(b));
    f.noise_db = (!qr.is_empty()).then(|| db(qr[qr.len() / 2]));
    if !whistle && !repeat {
        return f;
    }

    let nfft = 1024usize;
    let mut planner = RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(nfft);
    let hann: Vec<f64> = (0..nfft).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / nfft as f64).cos()).collect();
    let spectrum = |start: usize| -> Option<Vec<f64>> {
        if start + nfft > x.len() {
            return None;
        }
        let mut buf: Vec<f64> = (0..nfft).map(|i| x[start + i] as f64 * hann[i]).collect();
        let mut out = fft.make_output_vec();
        fft.process(&mut buf, &mut out).ok()?;
        Some(out.iter().map(|c| c.norm_sqr()).collect())
    };
    let hop = nfft / 2;
    let (lo, hi) = (((2000.0 * nfft as f64) / sr as f64) as usize, ((10000.0 * nfft as f64) / sr as f64).min(nfft as f64 / 2.0 - 1.0) as usize);
    let mut peaks: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut frames_n = 0usize;
    let mut mel_frames: Vec<Option<Vec<f64>>> = Vec::new();
    let mel = mel_bank(sr as f64, nfft, 40);
    let limit = x.len().min(sr as usize * 60);
    let mut start = 0usize;
    while start + nfft <= limit {
        if let Some(p) = spectrum(start) {
            let is_loud = loud.get((start + nfft / 2) / w).copied().unwrap_or(false);
            if whistle && is_loud && hi > lo {
                frames_n += 1;
                let band = &p[lo..=hi];
                let mut sorted = band.to_vec();
                sorted.sort_by(|a, b| a.total_cmp(b));
                let med = sorted[sorted.len() / 2].max(1e-12);
                let (bi, bv) = band.iter().enumerate().fold((0, 0.0f64), |m, (i, &v)| if v > m.1 { (i, v) } else { m });
                if 10.0 * (bv / med).log10() >= 30.0 {
                    *peaks.entry(bi / 2).or_default() += 1;
                }
            }
            if repeat {
                mel_frames.push(is_loud.then(|| {
                    let m: Vec<f64> = mel.iter().map(|band| band.iter().map(|&(b, wt)| p[b] * wt).sum::<f64>().max(1e-10).ln()).collect();
                    let mean = m.iter().sum::<f64>() / m.len() as f64;
                    let v: Vec<f64> = m.iter().map(|a| a - mean).collect();
                    let n = v.iter().map(|a| a * a).sum::<f64>().sqrt().max(1e-9);
                    v.iter().map(|a| a / n).collect()
                }));
            }
        }
        start += hop;
    }
    if frames_n > 0 {
        f.tonal_frac = peaks.values().copied().max().unwrap_or(0) as f64 / frames_n as f64;
    }
    if repeat {
        let frame_s = hop as f64 / sr as f64;
        let n = mel_frames.len();
        let min_lag = (0.4 / frame_s).ceil() as usize;
        let mut best = 0usize;
        for lag in min_lag..n {
            let mut run = 0usize;
            for i in 0..n - lag {
                let same = match (&mel_frames[i], &mel_frames[i + lag]) {
                    (Some(a), Some(b)) => a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>() >= 0.93,
                    _ => false,
                };
                run = if same { run + 1 } else { 0 };
                best = best.max(run);
            }
        }
        f.rep_s = best as f64 * frame_s;
    }
    f
}

fn mel_bank(sr: f64, nfft: usize, bands: usize) -> Vec<Vec<(usize, f64)>> {
    let to_mel = |f: f64| 2595.0 * (1.0 + f / 700.0).log10();
    let from_mel = |m: f64| 700.0 * (10f64.powf(m / 2595.0) - 1.0);
    let (lo, hi) = (to_mel(80.0), to_mel((sr / 2.0).min(8000.0)));
    let pts: Vec<f64> = (0..bands + 2).map(|i| from_mel(lo + (hi - lo) * i as f64 / (bands + 1) as f64)).collect();
    let bin = |f: f64| f * nfft as f64 / sr;
    (0..bands)
        .map(|b| {
            let (l, c, r) = (bin(pts[b]), bin(pts[b + 1]), bin(pts[b + 2]));
            ((l.floor() as usize)..=(r.ceil() as usize).min(nfft / 2))
                .filter_map(|k| {
                    let k = k as f64;
                    let wt = if k < c { (k - l) / (c - l) } else { (r - k) / (r - c) };
                    (wt > 0.0).then_some((k as usize, wt))
                })
                .collect()
        })
        .collect()
}

pub fn syllables(text: &str) -> usize {
    const VOWELS: &str = "aeiouyàáâãäåèéêëìíîïòóôõöùúûüýæœаеёиоуыэюяіїє";
    crate::text::markup::visible(text).to_lowercase().chars().filter(|&c| VOWELS.contains(c) || is_cjk(c)).count()
}

/// A Chinese or Japanese character or a Korean syllable block: each is a syllable said, and a word of its own when
/// texts are compared, since these scripts put no spaces between words.
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30ff | 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xac00..=0xd7af | 0xf900..=0xfaff)
}

fn text_repeats(text: &str) -> bool {
    let words: Vec<String> = crate::text::markup::visible(text)
        .to_lowercase()
        .replace('ё', "е")
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.chars().count() >= 3)
        .map(|w| w.chars().take(4).collect())
        .collect();
    words.len() != words.iter().collect::<std::collections::HashSet<_>>().len()
}

/// What the voice sounds like elsewhere in the project: seconds per syllable and loudness, medians of its checked takes.
#[derive(Debug, Default, Clone, Copy)]
pub struct Norm {
    pub secs_per_syllable: Option<f64>,
    pub lufs: Option<f64>,
}

/// Share of the samples at full scale from which a take is clipped: the level open-tts-eval fails a clip at (it warns
/// from 0.001). A loud voice touching full scale for a few samples is not heard as clipping.
pub const CLIP_SHARE: f64 = 0.01;

/// The flags of `want` the take raises, as issue names (`clipping`, `little_speech`, `long_pause`, `edge`,
/// `pause_noise`, `too_long`, `too_short`, `loud`, `quiet`, `whistle`, `audio_repeat`).
pub fn flags(f: &Features, want: &[String], text: &str, norm: Norm) -> Vec<String> {
    let on = |k: &str| want.iter().any(|w| w == k);
    let mut out = Vec::new();
    let syl = syllables(text);
    if on("clipping") && f.clip_share >= CLIP_SHARE {
        out.push("clipping".into());
    }
    if on("little_speech") && f.raw_s >= 3.0 && f.speech_frac < 0.3 {
        out.push("little_speech".into());
    }
    if on("long_pause") && f.max_pause_s >= 1.2 {
        out.push("long_pause".into());
    }
    if on("edge") && f.edge_db >= -20.0 {
        out.push("edge".into());
    }
    if on("pause_noise") && f.max_pause_s >= 0.25 && f.noise_db.is_some_and(|n| n >= -45.0) {
        out.push("pause_noise".into());
    }
    if let (true, Some(rate)) = (on("length"), norm.secs_per_syllable) {
        let expected = syl.max(1) as f64 * rate + 0.3;
        let ratio = f.trimmed_s / expected;
        if ratio >= 1.3 && f.trimmed_s - expected >= 0.8 && syl >= 4 {
            out.push("too_long".into());
        } else if ratio < 0.6 && expected >= 1.5 {
            out.push("too_short".into());
        }
    }
    if let (true, Some(a), Some(b)) = (on("loudness"), f.lufs, norm.lufs) {
        if (a - b).abs() >= 6.0 {
            out.push(if a > b { "loud" } else { "quiet" }.into());
        }
    }
    if on("whistle") && f.trimmed_s >= 3.0 && f.tonal_frac >= 0.3 {
        out.push("whistle".into());
    }
    if on("audio_repeat") && f.rep_s >= 0.2 && syl >= 4 && !text_repeats(text) {
        out.push("audio_repeat".into());
    }
    out
}

fn norm_words(s: &str) -> Vec<String> {
    s.to_lowercase().replace('ё', "е").replace("<unk>", "").split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
}

fn cyrillic(s: &str) -> bool {
    let letters: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
    !letters.is_empty() && letters.iter().filter(|c| ('а'..='я').contains(&c.to_lowercase().next().unwrap_or(' ')) || **c == 'ё' || **c == 'Ё').count() as f64 / letters.len() as f64 >= 0.6
}

/// A drawn-out interjection («а-а-а», «мм») heard as separate letters is no repeated word.
fn counted(w: &str) -> bool {
    w.chars().count() >= 2 && !(w.chars().all(|c| "аоуэиыеяюё".contains(c)) || w.chars().all(|c| c == 'м' || c == 'х'))
}

/// The model said a word twice: a word of the text (three letters on) that the text has once is heard more than once,
/// and more words are heard than written. Words the text itself repeats are delivery, not a fault.
pub fn heard_repeat(text: &str, heard: &str) -> bool {
    if !cyrillic(heard) {
        return false;
    }
    let (a, b) = (norm_words(text), norm_words(heard));
    if b.len() <= a.len() {
        return false;
    }
    let stem = |w: &String| w.chars().take(4).collect::<String>();
    let count = |v: &[String]| v.iter().filter(|w| counted(w)).map(stem).fold(std::collections::HashMap::<String, usize>::new(), |mut m, w| {
        *m.entry(w).or_default() += 1;
        m
    });
    let (ct, cb) = (count(&a), count(&b));
    cb.iter().any(|(w, n)| w.chars().count() >= 3 && ct.get(w) == Some(&1) && *n > 1)
}

fn letters(s: &str) -> Vec<char> {
    s.to_lowercase().replace('ё', "е").chars().filter(|c| ('а'..='я').contains(c)).collect()
}

fn levenshtein(x: &[char], y: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=y.len()).collect();
    for i in 1..=x.len() {
        let mut cur = vec![i; y.len() + 1];
        for j in 1..=y.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + usize::from(x[i - 1] != y[j - 1]));
        }
        prev = cur;
    }
    prev[y.len()]
}

/// Mush: heard in Cyrillic, but its letters, spaces aside, are less than half like the text's. A text with Latin
/// letters or digits (names, numbers said in words) is not judged; an empty or Latin answer is the model failing.
pub fn garbled(text: &str, heard: &str) -> bool {
    if text.chars().any(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    let t = letters(text);
    if t.len() < 6 || !cyrillic(heard) {
        return false;
    }
    let h = letters(heard);
    1.0 - (levenshtein(&t, &h) as f64) / (t.len().max(h.len()) as f64) < 0.5
}

/// The verdict on a suspect take from what recognition heard (one model, or two when a second is there): `repeat` —
/// a repeat heard by any; `garbled` — mush heard by every model that answered in Cyrillic and by at least two, or by
/// the only one; `listen` — mush by one while the other answered nothing usable; `None` — the text is there.
pub fn verdict(text: &str, heard: &[&str]) -> Option<&'static str> {
    if heard.iter().any(|h| heard_repeat(text, h)) {
        return Some("repeat");
    }
    let mush: Vec<bool> = heard.iter().map(|h| garbled(text, h)).collect();
    let usable = |h: &&str| cyrillic(h);
    match heard {
        [one] => mush[0].then_some(if usable(one) { "garbled" } else { "listen" }),
        [a, b] => match (mush[0], mush[1]) {
            (true, true) => Some("garbled"),
            (true, false) if !usable(b) => Some("listen"),
            (false, true) if !usable(a) => Some("listen"),
            _ => None,
        },
        _ => None,
    }
}

/// The middle value; `None` under `at_least` values, too few to say what is usual.
pub fn median(mut v: Vec<f64>, at_least: usize) -> Option<f64> {
    v.retain(|x| x.is_finite());
    if v.len() < at_least {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    Some(v[v.len() / 2])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 24_000;

    #[test]
    fn syllables_count_accented_vowels_and_one_per_ideograph() {
        assert_eq!(syllables("está café"), 4);
        assert_eq!(syllables("coração"), 4);
        assert_eq!(syllables("床前明月光"), 5);
        assert_eq!(syllables("Мама мыла раму"), 6);
    }

    #[test]
    fn synthesis_markup_does_not_add_syllables() {
        for (text, count) in [
            ("М+ама [pause 300ms] **мыла** sil<[200]> [[раму]] [rate:0,8] <|emotion:awe|>", 6),
            ("床前<[medium]>明月光 <|style:singing|>", 5),
            ("<whisper>está</whisper> [PAUSE 1,5s] c+afé", 4),
        ] {
            assert_eq!(syllables(text), count, "{text}");
        }
    }

    #[test]
    fn length_and_repeat_flags_use_words_outside_markup() {
        let features = Features { trimmed_s: 3.0, rep_s: 0.3, ..Default::default() };
        let norm = Norm { secs_per_syllable: Some(0.2), ..Default::default() };
        let want = ["length".into(), "audio_repeat".into()];
        assert_eq!(
            flags(&features, &want, "М+ама <|speaker:мама|> **мыла** [[раму]] [pause 300ms] [rate:0,8]", norm),
            ["too_long", "audio_repeat"]
        );
        assert_eq!(flags(&features, &want, "Мама [pause 300ms] **мама** [[раму]]", norm), ["too_long"]);
    }

    fn tone(secs: f64, amp: f32) -> Vec<f32> {
        (0..(secs * SR as f64) as usize).map(|i| amp * (i as f32 * 2.0 * std::f32::consts::PI * 220.0 / SR as f32).sin()).collect()
    }

    fn all() -> Vec<String> {
        KINDS.iter().map(|k| k.to_string()).collect()
    }

    #[test]
    fn a_take_cut_mid_word_with_a_long_pause_and_little_speech_is_flagged() {
        let mut x = vec![0.0f32; SR as usize / 2];
        x.extend(tone(0.5, 0.5));
        x.extend(vec![0.0f32; SR as usize * 3]);
        x.extend(tone(0.4, 0.5));
        let f = measure(&x, SR, false, false);
        let got = flags(&f, &all(), "Да.", Norm::default());
        assert!(got.contains(&"long_pause".to_string()) && got.contains(&"edge".to_string()) && got.contains(&"little_speech".to_string()), "{got:?}");
        assert!(flags(&f, &[], "Да.", Norm::default()).is_empty(), "a kind turned off raises nothing");
    }

    #[test]
    fn length_and_loudness_are_judged_against_the_voice() {
        let f = Features { trimmed_s: 6.0, lufs: Some(-10.0), ..Default::default() };
        let norm = Norm { secs_per_syllable: Some(0.15), lufs: Some(-20.0) };
        let got = flags(&f, &all(), "Привет, как дела у тебя сегодня?", norm);
        assert!(got.contains(&"too_long".to_string()) && got.contains(&"loud".to_string()), "{got:?}");
        assert_eq!(median(vec![1.0, 2.0], 5), None);
    }

    #[test]
    fn the_verdict_follows_game_dub_s_pair_of_models() {
        let text = "Ну и где ты был всё это время?";
        assert_eq!(verdict(text, &["Ну и где ты был всё это время?"]), None);
        assert_eq!(verdict(text, &["Ну и где ты был был всё это время время?"]), Some("repeat"));
        assert_eq!(verdict(text, &["Пыр мыр дыр бур хыр гыр"]), Some("garbled"));
        assert_eq!(verdict(text, &["Пыр мыр дыр бур хыр гыр", "Ну и где ты был всё это время"]), None, "one model heard the text");
        assert_eq!(verdict(text, &["Пыр мыр дыр бур хыр гыр", ""]), Some("listen"));
        assert_eq!(verdict("Нет, нет, нет!", &["Нет нет нет нет"]), None, "the text repeats it itself");
    }

    #[test]
    fn each_flag_has_its_role() {
        for d in ["runaway", "cutoff", "silence", "hum", "click", "clipping", "whistle"] {
            assert_eq!(role(d), Role::Defect, "{d}");
        }
        assert_eq!(role("too_long"), Role::Suspect);
        assert_eq!(role("loud"), Role::Note);
    }

    #[test]
    fn a_clipped_take_is_flagged_and_a_clean_one_is_not() {
        let mut x = tone(1.0, 0.5);
        let clean = flags(&measure(&x, SR, true, true), &["clipping".to_string(), "whistle".to_string(), "audio_repeat".to_string()], "Привет всем.", Norm::default());
        assert!(clean.is_empty(), "{clean:?}");
        x[100..110].iter_mut().for_each(|v| *v = 1.0);
        assert!(flags(&measure(&x, SR, false, false), &["clipping".to_string()], "Привет.", Norm::default()).is_empty(), "ten samples at full scale in a second are not heard");
        let n = x.len();
        x.iter_mut().step_by(50).take(n / 50).for_each(|v| *v = 1.0);
        assert_eq!(flags(&measure(&x, SR, false, false), &["clipping".to_string()], "Привет.", Norm::default()), vec!["clipping"]);
    }
}
