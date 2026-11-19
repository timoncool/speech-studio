//! Read-only component availability; no active selection or engine construction.

use super::*;

pub(super) fn archive_installed(repo_root: &Path, f: &FileSpec) -> bool {
    read_record(repo_root, f).is_some_and(|r| {
        // Both the original Windows CUDA archive and the official workaround recorded the original hash.
        (r.sha256 == f.sha256
            || (f.sha256 == "4c033a5088254f167e49c4f2eb90e2d0d77667b2b9400e3607e8e15a5c9fa970"
                && r.sha256 == "4b7a2d01a3cc96b12d06c8266af2c8f42c96365c4a0100d45fd874c71b4a2e19"
                && r.url == "https://github.com/microsoft/onnxruntime/releases/download/v1.28.2/onnxruntime-win-x64-gpu_cuda13-1.28.2.zip"))
            && !r.files.is_empty()
            && r.files.iter().all(|x| file_ok(&repo_root.join(&x.path), x.size))
    })
}

/// A program in one file (engine, CLI): builds with other flags can match in size, only sha256 tells them apart.
pub(super) fn is_program(f: &FileSpec) -> bool {
    f.extract == Extract::None && (f.dest_rel.starts_with("tools/") || [".dll", ".so", ".dylib", ".exe"].iter().any(|ext| f.dest_rel.ends_with(ext)))
}

/// A program counts when it has the pinned size and its record names the pinned sha256. A file without a record
/// (laid down before records or taken from a neighbouring program) is hashed once and the record keeps its real
/// sha256, so a foreign build stays not installed and is not hashed again.
fn program_installed(repo_root: &Path, f: &FileSpec) -> bool {
    let dest = repo_root.join(f.dest_rel);
    if !file_ok(&dest, f.size) {
        return false;
    }
    if let Some(r) = read_record(repo_root, f) {
        return r.sha256 == f.sha256;
    }
    match sha256_file(&dest, &|| false) {
        Ok(Some(h)) => {
            if h != f.sha256 {
                tracing::warn!("{} is not the pinned build (sha256 {h}, pinned {}); it counts as not installed", f.dest_rel, f.sha256);
            }
            if let Err(e) = write_record_of(repo_root, f, &h, &[dest]) {
                tracing::error!("[ERROR] the install record of {}: {e}", f.dest_rel);
                return false;
            }
            h == f.sha256
        }
        Ok(None) => false,
        Err(e) => {
            tracing::error!("[ERROR] {}: {e}", f.dest_rel);
            false
        }
    }
}

pub(super) fn file_installed(repo_root: &Path, f: &FileSpec) -> bool {
    match f.extract {
        Extract::None if is_program(f) => program_installed(repo_root, f),
        Extract::None => file_ok(&repo_root.join(f.dest_rel), f.size),
        _ => archive_installed(repo_root, f),
    }
}

/// Статус компонента на диске.
pub fn component_status(repo_root: &Path, c: &Component) -> ComponentStatus {
    let system = if FOUND_IN_SYSTEM_DIR.contains(&c.id) { system_dir() } else { None };
    status_with_system_dir(repo_root, c, system.as_deref())
}

/// `system` — системный каталог, где засчитываются маркеры компонентов FOUND_IN_SYSTEM_DIR.
pub(super) fn status_with_system_dir(repo_root: &Path, c: &Component, system: Option<&Path>) -> ComponentStatus {
    let mut missing: Vec<String> = Vec::new();
    let mut bytes_on_disk = 0u64;
    let mut space_needed = 0u64;
    for f in c.files {
        if file_installed(repo_root, f) {
            bytes_on_disk += f.size;
        } else {
            missing.push(f.dest_rel.to_string());
            bytes_on_disk += resumed_bytes(&part_path(repo_root, f), f.size);
            space_needed += file_space_needed(repo_root, f);
        }
    }
    let mut from_system = false;
    for m in c.markers {
        if marker_ok(repo_root, m) || missing.iter().any(|x| x == m.rel) {
            continue;
        }
        if system.is_some_and(|dir| in_system_dir(dir, m)) {
            from_system = true;
        } else {
            missing.push(m.rel.to_string());
        }
    }
    if c.delivery == Delivery::Bundled {
        bytes_on_disk = c.markers.iter().map(|m| file_len(&repo_root.join(m.rel))).sum();
    }
    let (installed, detail) = match c.delivery {
        Delivery::External => {
            let gpu = crate::engines::hw::gpu_report();
            (gpu.cuda13_ok, gpu.summary())
        }
        _ if c.id == "ffmpeg" && !missing.is_empty() && ffmpeg_on_path() => {
            // ffmpeg из PATH годится пайплайну (Command::new("ffmpeg") его найдёт) — закачку не навязываем.
            missing.clear();
            space_needed = 0;
            (true, Some("PATH".to_string()))
        }
        _ => (missing.is_empty(), from_system.then(|| "system".to_string())),
    };
    ComponentStatus {
        id: c.id.to_string(),
        name: c.name.to_string(),
        purpose: c.purpose.to_string(),
        requirement: c.requirement,
        delivery: c.delivery,
        size: c.size,
        installed,
        bytes_on_disk,
        space_needed,
        missing,
        detail,
        external_url: c.external_url.map(|s| s.to_string()),
        vram: vram_estimate(c.id),
        fits_vram: None,
        taken_from: lock(taken()).get(c.id).cloned(),
    }
}
