//! Checks (default) or rewrites (`--write`) the download tables of README.md and its five translations against what
//! the app downloads on Windows: `speech_server::windows_downloads`, which reads manifest() of setup.rs and the pins of
//! the Dub crates. A table sits between `<!-- downloads:start -->` and `<!-- downloads:end -->`.
//!
//!     cargo run -p speech-server --example readme-downloads [-- --write]

#[cfg(not(windows))]
fn main() {
    eprintln!("[ERROR] the README tables are of the Windows downloads: run this on Windows");
    std::process::exit(1);
}

#[cfg(windows)]
fn main() {
    use speech_server::{windows_downloads, Extract, Requirement};
    use std::collections::BTreeSet;
    use std::path::Path;

    const START: &str = "<!-- downloads:start -->";
    const END: &str = "<!-- downloads:end -->";
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let write = std::env::args().any(|a| a == "--write");
    let downloads = windows_downloads();
    let mut failed = false;

    for (lang, file) in FILES {
        let path = root.join(file);
        let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
        let crlf = raw.contains("\r\n");
        let text = raw.replace("\r\n", "\n");
        let (Some(a), Some(b)) = (text.find(START), text.find(END)) else {
            eprintln!("[ERROR] {file}: no {START} ... {END} block");
            failed = true;
            continue;
        };
        let a = a + START.len();
        let t = text_of(lang);
        let mut rows = vec![format!("| {} |", t.head.join(" | ")), format!("|{}|", vec!["---"; t.head.len()].join("|"))];
        for d in &downloads {
            let Some(name) = name_of(d.id) else {
                eprintln!("[ERROR] component \"{}\" has no name in scripts/readme-downloads.rs", d.id);
                std::process::exit(1);
            };
            let links: Vec<String> = d
                .files
                .iter()
                .map(|f| {
                    let file_name = f.url.rsplit('/').next().unwrap_or(f.url);
                    match f.extract {
                        Extract::None => format!("[{file_name}]({}) → `{}`", f.url, f.dest_rel.replace('/', "\\")),
                        _ => format!("[{file_name}]({})", f.url),
                    }
                })
                .collect();
            let mut places: Vec<String> = Vec::new();
            for f in d.files {
                let folder = f.dest_rel.rfind('/').map_or(String::new(), |i| f.dest_rel[..=i].replace('/', "\\"));
                let place = match f.extract {
                    Extract::None => t.none.to_string(),
                    Extract::Flat => format!("{} `{folder}`", t.flat),
                    Extract::Pick => format!("{} `{folder}`", t.pick),
                    Extract::Tree => format!("{} `{folder}`", t.tree),
                    Extract::Libs => format!("{} `{folder}`", t.libs),
                };
                if !places.contains(&place) {
                    places.push(place);
                }
            }
            let need = match d.requirement {
                Requirement::Required => t.need[0],
                Requirement::Recommended => t.need[1],
                Requirement::Optional => t.need[2],
            };
            let size: u64 = d.files.iter().map(|f| f.size).sum();
            rows.push(format!("| {name} | {need} | {} | {} | {} |", links.join("<br>"), human(size), places.join("<br>")));
        }
        let fresh = format!("\n{}\n", rows.join("\n"));
        if write {
            let out = format!("{}{fresh}{}", &text[..a], &text[b..]);
            std::fs::write(&path, if crlf { out.replace('\n', "\r\n") } else { out }).unwrap_or_else(|e| panic!("{file}: {e}"));
            println!("[OK] {file}: table written");
            continue;
        }
        let have = &text[a..b];
        if have != fresh {
            let urls = |s: &str| -> BTreeSet<String> {
                s.split("](").skip(1).filter_map(|p| p.split(')').next()).filter(|u| u.starts_with("http")).map(String::from).collect()
            };
            let (present, wanted) = (urls(have), urls(&fresh));
            eprintln!("[ERROR] {file}: the table differs from what the app downloads; run with --write");
            for u in wanted.difference(&present) {
                eprintln!("  missing in README: {u}");
            }
            for u in present.difference(&wanted) {
                eprintln!("  not downloaded: {u}");
            }
            failed = true;
        } else {
            println!("[OK] {file}: {} components match", downloads.len());
        }
    }
    std::process::exit(i32::from(failed));
}

#[cfg(windows)]
const FILES: [(&str, &str); 6] = [
    ("en", "README.md"),
    ("ru", "README.ru.md"),
    ("zh", "README.zh.md"),
    ("es", "README.es.md"),
    ("fr", "README.fr.md"),
    ("pt", "README.pt.md"),
];

#[cfg(windows)]
struct Text {
    head: [&'static str; 5],
    /// required, recommended, optional
    need: [&'static str; 3],
    none: &'static str,
    flat: &'static str,
    tree: &'static str,
    pick: &'static str,
    libs: &'static str,
}

#[cfg(windows)]
fn text_of(lang: &str) -> Text {
    match lang {
        "ru" => Text {
            head: ["Компонент", "Нужен", "Файлы (прямые ссылки)", "Размер", "Куда положить"],
            need: ["обязателен", "рекомендуется", "по желанию"],
            none: "как есть, по пути после стрелки",
            flat: "распаковать файлы без подпапок в",
            tree: "распаковать с деревом папок в",
            pick: "взять ffmpeg.exe и ffprobe.exe из архива в",
            libs: "взять все .dll из архива в",
        },
        "zh" => Text {
            head: ["组件", "必要性", "文件（直接链接）", "大小", "存放位置"],
            need: ["必需", "推荐", "可选"],
            none: "原样放入，路径见箭头后",
            flat: "解压文件（不含子文件夹）到",
            tree: "保留文件夹结构解压到",
            pick: "从压缩包中取出 ffmpeg.exe 和 ffprobe.exe 放入",
            libs: "取出压缩包中所有 .dll 放入",
        },
        "es" => Text {
            head: ["Componente", "Necesidad", "Archivos (enlaces directos)", "Tamaño", "Dónde ponerlo"],
            need: ["obligatorio", "recomendado", "opcional"],
            none: "tal cual, en la ruta tras la flecha",
            flat: "descomprimir los archivos, sin subcarpetas, en",
            tree: "descomprimir con su árbol de carpetas en",
            pick: "tomar ffmpeg.exe y ffprobe.exe del archivo y ponerlos en",
            libs: "tomar todos los .dll del archivo y ponerlos en",
        },
        "fr" => Text {
            head: ["Composant", "Besoin", "Fichiers (liens directs)", "Taille", "Où le placer"],
            need: ["obligatoire", "recommandé", "facultatif"],
            none: "tel quel, au chemin indiqué après la flèche",
            flat: "décompresser les fichiers, sans sous-dossiers, dans",
            tree: "décompresser avec son arborescence dans",
            pick: "prendre ffmpeg.exe et ffprobe.exe de l'archive et les mettre dans",
            libs: "prendre tous les .dll de l'archive et les mettre dans",
        },
        "pt" => Text {
            head: ["Componente", "Necessidade", "Arquivos (links diretos)", "Tamanho", "Onde colocar"],
            need: ["obrigatório", "recomendado", "opcional"],
            none: "como está, no caminho após a seta",
            flat: "descompactar os arquivos, sem subpastas, em",
            tree: "descompactar com a árvore de pastas em",
            pick: "pegar ffmpeg.exe e ffprobe.exe do arquivo e colocar em",
            libs: "pegar todos os .dll do arquivo e colocar em",
        },
        _ => Text {
            head: ["Component", "Need", "Files (direct links)", "Size", "Put it in"],
            need: ["required", "recommended", "optional"],
            none: "as is, to the path after the arrow",
            flat: "unzip the files, without subfolders, into",
            tree: "unzip with its folder tree into",
            pick: "take ffmpeg.exe and ffprobe.exe from the archive into",
            libs: "take every .dll of the archive into",
        },
    }
}

#[cfg(windows)]
fn name_of(id: &str) -> Option<&'static str> {
    Some(match id {
        "higgs" => "Higgs Audio v3 Q8_0",
        "higgs-engine" => "audio.cpp server, CUDA 13 (Higgs, Audio8, Supertonic, Parakeet, Nemotron, CrisperWhisper, diarization)",
        "audiocpp-cuda12" => "audio.cpp server, CUDA 12.9 (cards and drivers CUDA 13 left)",
        "audiocpp-vulkan" => "audio.cpp server, Vulkan",
        "audiocpp-cpu" => "audio.cpp server, CPU",
        "gemma" => "Gemma-4 12B QAT q4_0 + vision",
        "gemma-q5_0" => "Gemma-4 12B Q5_K_M + vision",
        "gemma-q6_k" => "Gemma-4 12B Q6_K + vision",
        "gemma-q8_0" => "Gemma-4 12B Q8_0 + vision",
        "parakeet" => "Parakeet-TDT 0.6B v3 Q8_0",
        "higgs-q6_k" => "Higgs Audio v3 Q6_K",
        "higgs-q4_k_m" => "Higgs Audio v3 Q4_K_M",
        "parakeet-ultra" => "Parakeet Ultra 0.6B Q8_0 (Moondream, transcribe.cpp)",
        "crisperwhisper-turbo" => "CrisperWhisper 2.0 turbo (audio.cpp)",
        "sortformer" => "Nemotron 3 Diarization",
        "roformer" => "Mel-Band Roformer voc_fv6 Q8_0",
        "roformer-q5" => "Mel-Band Roformer voc_fv6 Q5_0",
        "roformer-q4" => "Mel-Band Roformer voc_fv6 Q4_0",
        "bsroformer-engine" => "BSRoformer.cpp (CUDA 13)",
        "bsroformer-engine-cuda12" => "BSRoformer.cpp (CUDA 12.9)",
        "bsroformer-engine-vulkan" => "BSRoformer.cpp (Vulkan)",
        "bsroformer-engine-cpu" => "BSRoformer.cpp (CPU)",
        "llama" => "llama.cpp server (CUDA 13)",
        "llama-cuda12" => "llama.cpp server (CUDA 12.4)",
        "llama-vulkan" => "llama.cpp server (Vulkan)",
        "llama-cpu" => "llama.cpp server (CPU)",
        "onnxruntime" => "ONNX Runtime",
        "onnxruntime-gpu" => "ONNX Runtime GPU (CUDA)",
        "ffmpeg" => "FFmpeg (static build)",
        "ytdlp" => "yt-dlp + deno",
        "cuda-runtime" => "CUDA 13 runtime (cudart, cuBLAS, cuFFT)",
        "cuda12-runtime" => "CUDA 12.9 runtime (cudart, cuBLAS, cuFFT)",
        "cudnn" => "cuDNN 9",
        "drama3" => "Fish Audio Drama 3 Q4 + drama_engine (6 GB+ GPU, 32 GB RAM under 24 GB)",
        "audio8" => "Audio8 TTS Realistic-Air Q8_0",
        "supertonic3" => "Supertonic 3 int8",
        "qwen3-tts-engine" => "Qwen3-TTS engine (qwen.dll) and codec",
        "qwen3-tts-cuda13" => "Qwen3-TTS CUDA 13 module",
        "qwen3-tts-cuda12" => "Qwen3-TTS CUDA 12.9 module",
        "qwen3-tts-vulkan" => "Qwen3-TTS Vulkan module",
        "qwen3-tts" => "Qwen3-TTS 1.7B Q4_K_M (CustomVoice + Base)",
        "qwen3-tts-q8_0" => "Qwen3-TTS 1.7B Q8_0 (CustomVoice + Base)",
        "qwen3-tts-ru-stress" => "Qwen3-TTS Russian stress LoRA",
        "qwen3-tts-design" => "Qwen3-TTS VoiceDesign Q4_K_M",
        "qwen3-tts-design-q8_0" => "Qwen3-TTS VoiceDesign Q8_0",
        "ruaccent" => "RUAccent (stress, «ё», homographs)",
        "ruaccent-omograph-big" => "RUAccent big_poetry homographs int8",
        "transcribe-engine" => "transcribe.cpp engine (CUDA 13)",
        "transcribe-engine-cpu" => "transcribe.cpp engine (CPU)",
        "transcribe-engine-vulkan" => "transcribe.cpp engine (Vulkan)",
        "asr-gguf-nemotron" => "Nemotron 3.5 ASR 0.6B Q8_0 (NVIDIA)",
        "wespeaker" => "WeSpeaker ResNet34-LM (voice similarity)",
        "silero-vad" => "Silero VAD v6.2",
        "langid" => "VoxLingua107 language identification Q8_0",
        "dpdfnet" => "DPDFNet 48 kHz (noise suppression)",
        "voice-gate" => "Voice gate (ReDimNet2-B3)",
        "wake-word" => "Wake word (PhoneticXeus int8)",
        "smart-turn" => "Smart Turn v3.2 (end of phrase in Live)",
        _ => return None,
    })
}

#[cfg(windows)]
fn human(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{} MB", (b / 1e6).round())
    } else {
        format!("{} KB", (b / 1e3).round().max(1.0))
    }
}
