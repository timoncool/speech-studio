//! Мини WAV I/O для сборки таймлайна дубляжа (assemble.timeline): чтение WAV в mono f32 + запись
//! mono f32. hound читает PCM16 и float WAV, стерео сводим в mono усреднением.

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

/// Прочитать WAV -> (mono f32 сэмплы, sample_rate).
pub fn read_mono_f32(path: &Path) -> Result<(Vec<f32>, u32), String> {
    read_mono_f32_head(path, usize::MAX)
}

/// Первые `frames` кадров WAV -> (mono f32 сэмплы, sample_rate); остальное не читается.
pub fn read_mono_f32_head(path: &Path, frames: usize) -> Result<(Vec<f32>, u32), String> {
    let mut r = WavReader::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let spec = r.spec();
    let ch = spec.channels.max(1) as usize;
    let take = frames.saturating_mul(ch);
    let interleaved: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => r
            .samples::<f32>()
            .take(take)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("read f32: {e}"))?,
        SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>()
                .take(take)
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("read int: {e}"))?
        }
    };
    let mono: Vec<f32> = if ch <= 1 {
        interleaved
    } else {
        interleaved
            .chunks(ch)
            .map(|fr| fr.iter().sum::<f32>() / ch as f32)
            .collect()
    };
    Ok((mono, spec.sample_rate))
}

/// Записать mono f32 -> WAV IEEE-float32.
pub fn write_mono_f32(path: &Path, data: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut w = WavWriter::create(path, spec).map_err(|e| format!("create {}: {e}", path.display()))?;
    for &s in data {
        w.write_sample(s).map_err(|e| format!("write: {e}"))?;
    }
    w.finalize().map_err(|e| format!("finalize: {e}"))?;
    Ok(())
}


/// PCM f32 as a PCM16 WAV in memory (a basic PCM header, which every engine of the studio reads as a reference).
pub fn encode_wav(samples: &[f32], sample_rate: i32, channels: i32) -> Vec<u8> {
    let spec = WavSpec { channels: channels.max(1) as u16, sample_rate: sample_rate.max(1) as u32, bits_per_sample: 16, sample_format: SampleFormat::Int };
    let mut out = std::io::Cursor::new(Vec::with_capacity(44 + samples.len() * 2));
    {
        let mut w = WavWriter::new(&mut out, spec).expect("a WAV header into memory");
        for &x in samples {
            w.write_sample((x.clamp(-1.0, 1.0) * 32767.0) as i16).expect("a sample into memory");
        }
        w.finalize().expect("a WAV into memory");
    }
    out.into_inner()
}

/// Linear resampling to 16 kHz: enough for the speech detector and the language id, the recogniser gets the same.
pub fn to_16k(x: &[f32], sr: u32) -> Vec<f32> {
    if sr == 16_000 || x.is_empty() {
        return x.to_vec();
    }
    let n = (x.len() as f64 * 16_000.0 / sr as f64) as usize;
    (0..n)
        .map(|i| {
            let at = i as f64 * sr as f64 / 16_000.0;
            let k = at as usize;
            let f = (at - k as f64) as f32;
            let a = x.get(k).copied().unwrap_or(0.0);
            let b = x.get(k + 1).copied().unwrap_or(a);
            a + (b - a) * f
        })
        .collect()
}

#[cfg(test)]
mod tests {
}
