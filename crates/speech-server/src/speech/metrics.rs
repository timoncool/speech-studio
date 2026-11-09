//! The person's speech in a Live conversation measured for the agent: pace, pauses, fillers and how soon they answer
//! the studio. Counted from the word timings the recogniser gave, never from the text spread over a line.

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::app::refuse;
use crate::AppState;

/// A silence this long inside the person's speech is a pause.
pub const PAUSE_MS: u64 = 500;
/// A pause this long is a long one.
pub const LONG_PAUSE_MS: u64 = 1000;
/// The studio's phrases this close one after another are one reply of it.
const REPLY_GAP_SECS: f64 = 1.0;

/// A line of the person's measured: its number, where its speech starts and ends, its words with where each starts and
/// ends (None: the recogniser gave no timings), its text.
struct Spoken {
    n: u64,
    start: f64,
    end: f64,
    words: Option<Vec<(String, f64, f64)>>,
    text: String,
}

impl Spoken {
    fn from(line: &Value) -> Option<Self> {
        let words = line["words"].as_array().map(|ws| ws.iter().filter_map(|w| Some((w["text"].as_str()?.to_string(), w["start"].as_f64()?, w["end"].as_f64()?))).collect::<Vec<_>>()).filter(|ws| !ws.is_empty());
        let (start, end) = match &words {
            Some(ws) => (ws[0].1, ws[ws.len() - 1].2),
            None => (line["start"].as_f64()?, line["end"].as_f64()?),
        };
        Some(Spoken { n: line["n"].as_u64()?, start, end, words, text: line["text"].as_str().unwrap_or_default().to_string() })
    }

    fn word_count(&self) -> usize {
        match &self.words {
            Some(ws) => ws.len(),
            None => crate::audio::qc::normalize(&self.text).len(),
        }
    }

    /// The line's normalised words with where each starts; without timings, all at the line's start.
    fn tokens(&self) -> Vec<(String, f64)> {
        match &self.words {
            Some(ws) => ws.iter().flat_map(|(w, a, _)| crate::audio::qc::normalize(w).into_iter().map(move |t| (t, *a))).collect(),
            None => crate::audio::qc::normalize(&self.text).into_iter().map(|t| (t, self.start)).collect(),
        }
    }
}

fn ms(secs: f64) -> i64 {
    (secs * 1000.0).round() as i64
}

fn round2(x: f64) -> f64 {
    // an empty sum of floats is -0.0; adding zero writes it without its sign
    (x * 100.0).round() / 100.0 + 0.0
}

fn median(xs: &[i64]) -> Option<i64> {
    let mut v = xs.to_vec();
    v.sort_unstable();
    match v.len() {
        0 => None,
        n if n % 2 == 1 => Some(v[n / 2]),
        n => Some((v[n / 2 - 1] + v[n / 2]) / 2),
    }
}

/// Seconds the `spans` (start, end) cover, a stretch two of them share counted once.
fn covered(mut spans: Vec<(f64, f64)>) -> f64 {
    spans.retain(|(s, e)| e > s);
    spans.sort_by(|x, y| x.0.total_cmp(&y.0));
    let (mut busy, mut reach) = (0.0f64, f64::NEG_INFINITY);
    for (s, e) in spans {
        if e > reach {
            busy += e - s.max(reach);
            reach = e;
        }
    }
    busy
}

/// The metrics of the person's `lines` (as `voice_lines` gives them), the studio's speech sounding over `studio`
/// (seconds of the recording, in order), fillers from `fillers`. Lines of the studio's own voice heard back and typed
/// lines are not speech of the person's and are only counted.
pub fn measure(lines: &[Value], studio: &[(f64, f64)], fillers: &[Vec<String>]) -> Value {
    let echo = lines.iter().filter(|l| l["echo"] == true).count();
    let typed = lines.iter().filter(|l| l["typed"] == true && l["echo"] != true).count();
    let mut spoken: Vec<Spoken> = lines.iter().filter(|l| l["echo"] != true && l["typed"] != true).filter_map(Spoken::from).collect();
    spoken.sort_by(|a, b| a.start.total_cmp(&b.start));
    let timed: Vec<&Spoken> = spoken.iter().filter(|s| s.words.is_some()).collect();
    let words: usize = spoken.iter().map(Spoken::word_count).sum();
    let speech_secs: f64 = covered(spoken.iter().map(|s| (s.start, s.end)).collect());
    // the pace counts the lines with word timings only when there are any: a line's own edges hold the pause around it
    let (paced, pace_secs, wpm_source) = if timed.is_empty() {
        (words, speech_secs, "lines")
    } else {
        (timed.iter().map(|s| s.word_count()).sum(), timed.iter().map(|s| (s.end - s.start).max(0.0)).sum(), "words")
    };
    let wpm = (pace_secs > 0.0 && !spoken.is_empty()).then(|| round2(paced as f64 * 60.0 / pace_secs));

    let mut pauses: Vec<Value> = Vec::new();
    let mut pause = |from: f64, to: f64, between_words: bool, line: u64| {
        let gap = ms(to - from);
        if gap >= PAUSE_MS as i64 {
            pauses.push(json!({ "start": round2(from), "end": round2(to), "ms": gap, "between_words": between_words, "long": gap >= LONG_PAUSE_MS as i64, "line": line }));
        }
    };
    for (i, s) in spoken.iter().enumerate() {
        if let Some(ws) = &s.words {
            for w in ws.windows(2) {
                pause(w[0].2, w[1].1, true, s.n);
            }
        }
        // between two lines a pause is the person's only when the studio did not speak in it
        if let Some(next) = spoken.get(i + 1) {
            if !studio.iter().any(|(a, z)| *a < next.start && *z > s.end) {
                pause(s.end, next.start, false, next.n);
            }
        }
    }
    let long = pauses.iter().filter(|p| p["long"] == true).count();

    let tokens: Vec<(String, f64)> = spoken.iter().flat_map(Spoken::tokens).collect();
    let found = crate::audio::fillers::find(&tokens, fillers);
    let filler_count: usize = found.iter().map(|(_, at)| at.len()).sum();

    // the studio's replies: its phrases close one after another taken together
    let mut replies: Vec<(f64, f64)> = Vec::new();
    for &(a, z) in studio {
        match replies.last_mut() {
            Some(last) if a - last.1 <= REPLY_GAP_SECS => last.1 = last.1.max(z),
            _ => replies.push((a, z)),
        }
    }
    // a reply is answered by the person's first line after it, unless the studio spoke again before that line
    let turns: Vec<Value> = replies
        .iter()
        .enumerate()
        .filter_map(|(i, &(a, z))| {
            let line = spoken.iter().find(|s| s.end > z && s.start >= a - REPLY_GAP_SECS)?;
            let overtaken = replies.get(i + 1).is_some_and(|next| next.0 < line.start);
            (!overtaken).then(|| json!({ "studio_end": round2(z), "line": line.n, "ms": ms(line.start - z) }))
        })
        .collect();
    let latencies: Vec<i64> = turns.iter().filter_map(|t| t["ms"].as_i64()).collect();

    let window = spoken.first().zip(spoken.last()).map(|(f, l)| (f.start, spoken.iter().map(|s| s.end).fold(l.end, f64::max)));
    // neither speaking: the window less the union of the person's lines and the studio's speech, an overlap counted once
    let silence_secs = window.map(|(a, z)| {
        let busy = covered(spoken.iter().map(|s| (s.start, s.end)).chain(studio.iter().map(|&(sa, sz)| (sa.max(a), sz.min(z)))).filter(|(s, e)| e > s).collect());
        round2((z - a - busy).max(0.0))
    });

    json!({
        "lines": spoken.len(),
        "echo_lines": echo,
        "typed_lines": typed,
        "untimed_lines": spoken.len() - timed.len(),
        "words": words,
        "wpm": wpm,
        "wpm_source": wpm_source,
        "speech_secs": round2(speech_secs),
        "silence_secs": silence_secs,
        "pauses": pauses,
        "pauses_500": pauses.len(),
        "pauses_1000": long,
        "fillers": found.iter().map(|(w, at)| json!({ "word": w, "count": at.len(), "at": at.iter().map(|a| ms(*a)).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "filler_count": filler_count,
        "fillers_per_100_words": (words > 0).then(|| round2(filler_count as f64 * 100.0 / words as f64)),
        "start_latency": { "last_ms": latencies.last(), "median_ms": median(&latencies), "turns": turns },
    })
}

/// The person's lines numbered from `from` to `to` (both counted, from 1), all of them when not given.
fn range(q: &HashMap<String, String>) -> Result<(Vec<Value>, Value), Response> {
    let state = crate::listening::recognition::state_with_words();
    let (lines, _) = super::say::person_lines(&state);
    let number = |k: &str| q.get(k).filter(|v| !v.is_empty()).map(|v| v.parse::<usize>().map_err(|_| refuse(StatusCode::BAD_REQUEST, "bad_range", t!("live-metrics-bad-range", value = v.clone())))).transpose();
    let from = number("from")?.unwrap_or(1).max(1);
    let to = number("to")?.unwrap_or(lines.len());
    let picked = lines.into_iter().filter(|l| l["n"].as_u64().is_some_and(|n| n as usize >= from && n as usize <= to)).collect();
    Ok((picked, state["running"].clone()))
}

/// The studio's speech said aloud in the running conversation, as seconds of its recording, in order.
fn studio_speech() -> Vec<(f64, f64)> {
    super::live::aired_blocks()
        .iter()
        .filter_map(|a| Some((crate::listening::recognition::recording_secs(a.from)?, crate::listening::recognition::recording_secs(a.to)?)))
        .filter(|(_, z)| *z > 0.0)
        .collect()
}

/// GET /speech/listen/metrics?from=N&to=M — the person's pace, pauses, fillers and how soon they answered, over their
/// lines from N to M.
pub async fn metrics(State(st): State<AppState>, Query(q): Query<HashMap<String, String>>) -> Response {
    let (lines, listening) = match range(&q) {
        Ok(r) => r,
        Err(e) => return e,
    };
    let fillers = match crate::audio::fillers::list(&st.workspace) {
        Ok(f) => f,
        Err(e) => return refuse(StatusCode::INTERNAL_SERVER_ERROR, "bad_fillers", e),
    };
    let mut out = measure(&lines, &studio_speech(), &fillers);
    out["listening"] = listening;
    out["from"] = json!(lines.first().map(|l| &l["n"]));
    out["to"] = json!(lines.last().map(|l| &l["n"]));
    Json(out).into_response()
}

/// GET /speech/listen/lines?from=N&to=M&words=false — the person's lines from N to M, with each word's timing unless
/// words is false.
pub async fn lines(Query(q): Query<HashMap<String, String>>) -> Response {
    let (mut lines, listening) = match range(&q) {
        Ok(r) => r,
        Err(e) => return e,
    };
    if q.get("words").map(String::as_str) == Some("false") {
        for l in lines.iter_mut() {
            if let Some(o) = l.as_object_mut() {
                o.remove("words");
            }
        }
    }
    Json(json!({ "lines": lines, "listening": listening })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(n: u64, words: &[(&str, f64, f64)]) -> Value {
        json!({ "n": n, "text": words.iter().map(|w| w.0).collect::<Vec<_>>().join(" "), "start": words[0].1 - 0.15, "end": words[words.len() - 1].2 + 0.15, "echo": false, "typed": false,
            "words": words.iter().map(|(w, a, z)| json!({ "text": w, "start": a, "end": z })).collect::<Vec<_>>(), "words_source": "asr" })
    }

    fn fillers() -> Vec<Vec<String>> {
        crate::audio::fillers::list(std::path::Path::new("no such folder")).unwrap()
    }

    /// Pauses known to the millisecond come out within 50 ms.
    #[test]
    fn pauses_inside_and_between_lines_match_the_known_ones() {
        let lines = [
            line(1, &[("Ну", 1.0, 1.2), ("я", 1.3, 1.4), ("думаю", 2.05, 2.5), ("что", 2.55, 2.7), ("да", 3.9, 4.1)]),
            line(2, &[("А", 6.0, 6.1), ("почему", 6.2, 6.6)]),
            line(3, &[("Потому", 12.0, 12.4), ("что", 12.45, 12.6)]),
        ];
        let m = measure(&lines, &[(7.0, 11.0)], &fillers());
        let got: Vec<(f64, i64, bool)> = m["pauses"].as_array().unwrap().iter().map(|p| (p["start"].as_f64().unwrap(), p["ms"].as_i64().unwrap(), p["between_words"] == true)).collect();
        let want = [(1.4, 650, true), (2.7, 1200, true), (4.1, 1900, false)];
        assert_eq!(got.len(), want.len(), "{m}");
        for (g, w) in got.iter().zip(want) {
            assert!((g.0 - w.0).abs() < 0.05 && (g.1 - w.1).abs() <= 50 && g.2 == w.2, "{g:?} against {w:?}");
        }
        assert_eq!(m["pauses_1000"], 2);
        assert_eq!(m["words"], 9);
        assert_eq!(m["wpm_source"], "words");
        let wpm = m["wpm"].as_f64().unwrap();
        assert!((wpm - 9.0 * 60.0 / (3.1 + 0.6 + 0.6)).abs() < 0.1, "{wpm}");
        assert_eq!(m["fillers"][0]["word"], "ну");
        assert_eq!(m["fillers"][0]["at"][0], 1000);
        assert_eq!(m["start_latency"]["turns"][0]["ms"], 1000, "the studio ended at 11 s, the person began at 12 s: {m}");
        assert_eq!(m["start_latency"]["turns"][0]["line"], 3);
    }

    /// Lines of a recogniser without word timings, echo and typed lines are measured without failing and
    /// without timings made up for them.
    #[test]
    fn lines_without_words_echo_and_typed_ones_do_not_break_the_count() {
        let lines = [
            line(1, &[("Привет", 1.0, 1.4), ("всем", 1.5, 1.9)]),
            json!({ "n": 2, "text": "это строка без слов", "start": 3.0, "end": 5.0, "echo": false, "typed": false, "words": null, "words_source": "none" }),
            json!({ "n": 3, "text": "голос студии", "start": 6.0, "end": 7.0, "echo": true, "typed": false, "words": null, "words_source": "none" }),
            json!({ "n": 4, "text": "напечатано", "start": 7.0, "end": 7.0, "echo": false, "typed": true, "words": null, "words_source": "typed" }),
        ];
        let m = measure(&lines, &[], &fillers());
        assert_eq!(m["lines"], 2);
        assert_eq!(m["echo_lines"], 1);
        assert_eq!(m["typed_lines"], 1);
        assert_eq!(m["untimed_lines"], 1);
        assert_eq!(m["words"], 6);
        assert!(m["wpm"].as_f64().unwrap().is_finite());
        assert_eq!(m["pauses"].as_array().unwrap().len(), 1, "between the lines only, none made up inside the untimed one: {m}");
        let empty = measure(&[], &[], &fillers());
        assert_eq!(empty["wpm"], Value::Null);
        assert_eq!(empty["words"], 0);
    }

    #[tokio::test]
    async fn the_lines_of_the_conversation_come_with_their_words_and_a_range() {
        let _slot = crate::listening::recognition::TEST_SLOT.lock().unwrap_or_else(|p| p.into_inner());
        let zero = std::time::Instant::now() - std::time::Duration::from_secs(30);
        crate::listening::recognition::conversation_for_test(zero, &[(1.0, 2.0, "раз два", false, true), (3.0, 4.0, "три", false, true), (5.0, 6.0, "четыре", false, true)]);
        crate::listening::recognition::words_for_test(1, &[("три", 3.1, 3.6)]);
        let q = HashMap::from([("from".to_string(), "2".to_string()), ("to".to_string(), "3".to_string())]);
        let got: Value = serde_json::from_slice(&axum::body::to_bytes(lines(Query(q)).await.into_body(), usize::MAX).await.unwrap()).unwrap();
        let picked = got["lines"].as_array().unwrap();
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0]["n"], 2);
        assert_eq!(picked[0]["words"][0]["start"], 3.1);
        assert_eq!(picked[0]["words_source"], "asr");
        assert_eq!(picked[1]["words"], Value::Null);
        assert_eq!(picked[1]["words_source"], "none");
        let bad = HashMap::from([("from".to_string(), "x".to_string())]);
        assert_eq!(lines(Query(bad)).await.status(), StatusCode::BAD_REQUEST);
        crate::listening::recognition::end_for_test();
    }

    /// A conversation with no line of the person's says zero seconds of speech, not «-0.0» (an empty sum of floats).
    #[test]
    fn no_lines_is_zero_seconds_without_a_sign() {
        let m = measure(&[], &[], &fillers());
        assert_eq!(m["speech_secs"].to_string(), "0.0");
        let typed = measure(&[json!({ "n": 1, "text": "привет", "typed": true })], &[], &fillers());
        assert_eq!(typed["speech_secs"].to_string(), "0.0");
    }

    /// A reply the studio followed with another before the person spoke went unanswered: it is no turn, and the person's
    /// line answers the last reply only.
    #[test]
    fn a_reply_overtaken_by_the_next_is_not_answered() {
        let m = measure(&[line(1, &[("ну", 40.0, 40.3), ("да", 40.4, 40.7)])], &[(10.0, 12.0), (14.0, 16.0)], &fillers());
        let turns = m["start_latency"]["turns"].as_array().unwrap();
        assert_eq!(turns.len(), 1, "{m}");
        assert_eq!(turns[0]["studio_end"], 16.0);
        assert_eq!(turns[0]["ms"], 24000, "from the reply's end to the person's first word");
    }

    /// The person speaking over the studio is one stretch of speech, not two: the silence is what neither of them filled.
    #[test]
    fn speaking_over_the_studio_is_not_counted_twice_out_of_the_silence() {
        let m = measure(&[line(1, &[("раз", 0.15, 3.85)]), line(2, &[("два", 9.15, 9.85)])], &[(3.0, 6.0)], &fillers());
        // speech 0.15–3.85 and 9.15–9.85, the studio 3–6: 9.7 s, of them 6.55 s someone speaks
        assert_eq!(m["silence_secs"], 3.15, "{m}");
        assert_eq!(m["speech_secs"], 4.4);
    }
}
