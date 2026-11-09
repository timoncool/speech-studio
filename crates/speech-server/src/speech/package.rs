//! Файл проекта для переноса `.speechproj` (zip): `manifest.json` (формат, версия, нужные движки и модели, ассеты с
//! SHA-256), `project.json`, `voices.zip` — голоса, на которые ссылается проект, и по желанию `audio/` — дубли.
//! Импорт сверяет формат, движки и хэши и отказывает с названием причины; занятые имена голосов переименовываются,
//! ссылки проекта следуют за ними.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::speech::jobs::SpeechPaths;
use crate::speech::project as sp;
use crate::speech::project::SpeechProject;

pub const FORMAT: &str = "speechproj";
pub const VERSION: u32 = 1;

#[cfg(test)]
mod boundary_tests;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EngineNeed {
    pub id: String,
    /// Движок с моделью и квантом (`qwen3-tts:Q4_K_M`), как его пишет дубль; пусто в файлах до версий.
    #[serde(default)]
    pub version: String,
}

/// Движки проекта, его персонажей и блоков и его дублей с версиями, каждый один раз.
fn engines_of(project: &SpeechProject, current: String) -> Vec<EngineNeed> {
    let mut seen: Vec<EngineNeed> = vec![EngineNeed { id: project.settings.engine.clone(), version: current }];
    for t in project.blocks().flat_map(|b| b.takes.iter()).filter(|t| !t.engine.is_empty()) {
        let need = EngineNeed { id: t.engine.split(':').next().unwrap_or_default().to_string(), version: t.engine.clone() };
        if !seen.contains(&need) {
            seen.push(need);
        }
    }
    for b in project.blocks() {
        let own = project.engine_of(b);
        if !seen.iter().any(|e| e.id == own) {
            seen.push(EngineNeed { id: own.to_string(), version: String::new() });
        }
    }
    seen
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Asset {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub app_version: String,
    pub created: u64,
    pub name: String,
    pub engines: Vec<EngineNeed>,
    pub with_audio: bool,
    pub voices: Vec<String>,
    pub assets: Vec<Asset>,
}

fn sha(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Голоса, на которые ссылается проект: голос проекта, состав и голоса блоков.
pub fn voices_of(project: &SpeechProject) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.extend(project.settings.voice.iter().cloned());
    set.extend(project.cast.values().cloned());
    set.extend(project.blocks().filter_map(|b| b.voice.clone()));
    set.into_iter().filter(|v| !v.is_empty()).collect()
}

/// Собрать `.speechproj` проекта в его папку экспорта; ответ — имя файла и размер.
pub fn pack(p: &SpeechPaths, pid: &str, with_audio: bool) -> Result<(String, u64), String> {
    let mut project = crate::speech::load(&p.workspace, pid)?;
    let voices: Vec<String> = voices_of(&project).into_iter().filter(|v| crate::voices::audio_path(&p.voices_dir, v).is_some()).collect();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    if !voices.is_empty() {
        files.push(("voices.zip".into(), crate::voices::http::export_zip(&p.voices_dir, &voices)?));
    }
    if with_audio {
        let takes = p.workspace.join(pid).join("takes");
        for b in project.blocks() {
            for t in &b.takes {
                let path = takes.join(&t.file);
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                files.push((format!("audio/{}", t.file), bytes));
            }
        }
    } else {
        for c in &mut project.chapters {
            for b in &mut c.blocks {
                b.takes.clear();
                b.active = None;
                b.pinned = None;
                b.error = None;
            }
        }
    }
    files.push(("project.json".into(), serde_json::to_vec_pretty(&project).map_err(|e| e.to_string())?));
    let manifest = Manifest {
        format: FORMAT.into(),
        version: VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created: sp::now(),
        name: project.name.clone(),
        engines: engines_of(&project, crate::speech::jobs::engine_tag(p, &project, &project.settings.engine)),
        with_audio,
        voices,
        assets: files.iter().map(|(path, b)| Asset { path: path.clone(), size: b.len() as u64, sha256: sha(b) }).collect(),
    };
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let deflate = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("manifest.json", deflate).map_err(|e| e.to_string())?;
        zip.write_all(&serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        for (path, bytes) in &files {
            let opts = if path.starts_with("audio/") || path.ends_with(".zip") { stored } else { deflate };
            zip.start_file(path.as_str(), opts).map_err(|e| e.to_string())?;
            zip.write_all(bytes).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    let bytes = buf.into_inner();
    let dir = p.workspace.join(pid).join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = format!("{}.speechproj", crate::speech::jobs::file_stem(&project.name, None));
    std::fs::write(dir.join(&name), &bytes).map_err(|e| format!("{name}: {e}"))?;
    Ok((name, bytes.len() as u64))
}

/// Разобрать `.speechproj` в новый проект; ответ — его id и голоса, добавленные в библиотеку.
pub fn unpack(p: &SpeechPaths, bytes: &[u8]) -> Result<(String, Vec<String>), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| t!("speechproj-not-a-package"))?;
    let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().map_err(|e| e.to_string())?.replace('\\', "/");
        if name.contains("..") || name.starts_with('/') {
            return Err(t!("speechproj-bad-path", path = name));
        }
        let mut data = Vec::new();
        f.read_to_end(&mut data).map_err(|e| e.to_string())?;
        entries.insert(name, data);
    }
    let manifest: Manifest = entries
        .get("manifest.json")
        .ok_or_else(|| t!("speechproj-not-a-package"))
        .and_then(|b| serde_json::from_slice(b).map_err(|e| t!("speechproj-bad-manifest", detail = e.to_string())))?;
    if manifest.format != FORMAT {
        return Err(t!("speechproj-not-a-package"));
    }
    if manifest.version > VERSION {
        return Err(t!("speechproj-too-new", version = manifest.version as i64, app = manifest.app_version.clone()));
    }
    for e in &manifest.engines {
        if crate::engines::get(&e.id).is_none() {
            return Err(t!("speechproj-engine-missing", engine = e.id.clone()));
        }
    }
    for a in &manifest.assets {
        let data = entries.get(&a.path).ok_or_else(|| t!("speechproj-asset-missing", path = a.path.clone()))?;
        if data.len() as u64 != a.size || sha(data) != a.sha256 {
            return Err(t!("speechproj-asset-damaged", path = a.path.clone()));
        }
    }
    let mut project: SpeechProject = serde_json::from_slice(entries.get("project.json").ok_or_else(|| t!("speechproj-asset-missing", path = "project.json".to_string()))?)
        .map_err(|e| t!("speechproj-bad-manifest", detail = e.to_string()))?;
    super::validate(&project)?;
    let mut added = Vec::new();
    if let Some(zip) = entries.get("voices.zip") {
        let (mapped, _same) = crate::voices::http::import_zip_mapped(&p.voices_dir, zip)?;
        let rename: BTreeMap<String, String> = mapped.into_iter().filter(|(from, to)| from != to).collect();
        added = rename.values().cloned().collect();
        let swap = |v: &mut Option<String>| {
            if let Some(name) = v.as_ref().and_then(|n| rename.get(n)) {
                *v = Some(name.clone());
            }
        };
        swap(&mut project.settings.voice);
        for v in project.cast.values_mut() {
            if let Some(name) = rename.get(v) {
                *v = name.clone();
            }
        }
        for c in &mut project.chapters {
            for b in &mut c.blocks {
                swap(&mut b.voice);
            }
        }
    }
    let pid = sp::new_id();
    let takes = p.workspace.join(&pid).join("takes");
    for (path, data) in entries.iter().filter(|(k, _)| k.starts_with("audio/")) {
        let file = path.trim_start_matches("audio/");
        if file.contains('/') {
            continue;
        }
        std::fs::create_dir_all(&takes).map_err(|e| format!("{}: {e}", takes.display()))?;
        std::fs::write(takes.join(file), data).map_err(|e| format!("{file}: {e}"))?;
    }
    crate::speech::save(&p.workspace, &pid, &project)?;
    Ok((pid, added))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(root: &std::path::Path) -> SpeechPaths {
        SpeechPaths {
            repo_root: root.to_path_buf(),
            workspace: root.join("workspace"),
            voices_dir: root.join("voices"),
            models_root: root.join("models"),
        }
    }

    #[test]
    fn a_package_comes_back_as_a_new_project_and_a_damaged_one_is_refused() {
        let a = tempfile::tempdir().unwrap();
        let pa = paths(a.path());
        std::fs::create_dir_all(&pa.voices_dir).unwrap();
        crate::audio::wavio::write_mono_f32(&pa.voices_dir.join("Anna.wav"), &[0.1; 2400], 24_000).unwrap();
        let mut project = sp::from_text("Тест", "Привет.\n\nПока.", "Рассказчик");
        project.settings.voice = Some("Anna".into());
        crate::speech::save(&pa.workspace, "p1", &project).unwrap();
        let (file, _) = pack(&pa, "p1", false).unwrap();
        let bytes = std::fs::read(pa.workspace.join("p1").join("exports").join(&file)).unwrap();

        let b = tempfile::tempdir().unwrap();
        let pb = paths(b.path());
        std::fs::create_dir_all(&pb.voices_dir).unwrap();
        crate::audio::wavio::write_mono_f32(&pb.voices_dir.join("Anna.wav"), &[0.5; 2400], 24_000).unwrap();
        let (pid, added) = unpack(&pb, &bytes).unwrap();
        let back = crate::speech::load(&pb.workspace, &pid).unwrap();
        assert_eq!(back.blocks().count(), 2);
        assert_eq!(added, vec!["Anna 2".to_string()]);
        assert_eq!(back.settings.voice.as_deref(), Some("Anna 2"));

        let mut broken = bytes.clone();
        let at = broken.windows(5).position(|w| w == b"\xd0\x9f\xd1\x80\xd0").unwrap_or(broken.len() / 2);
        broken[at] ^= 0xff;
        assert!(unpack(&pb, &broken).is_err());
        assert!(unpack(&pb, b"not a zip").is_err());
    }

    #[test]
    fn the_manifest_names_each_engine_with_its_version_once() {
        let mut project = sp::from_text("Тест", "Привет.\n\nПока.", "Рассказчик");
        project.settings.engine = "qwen3-tts".into();
        let tags = ["higgs:q8_0", "qwen3-tts:Q4_K_M"];
        for (b, tag) in project.chapters[0].blocks.iter_mut().zip(tags) {
            b.takes.push(sp::Take { n: 1, engine: tag.into(), ..Default::default() });
            b.takes.push(sp::Take { n: 2, engine: tag.into(), ..Default::default() });
        }
        let need = engines_of(&project, "qwen3-tts:Q4_K_M".into());
        let got: Vec<(&str, &str)> = need.iter().map(|e| (e.id.as_str(), e.version.as_str())).collect();
        assert_eq!(got, vec![("qwen3-tts", "qwen3-tts:Q4_K_M"), ("higgs", "higgs:q8_0")]);
    }
}
