//! PE import inspection and dependency resolution against explicitly supplied file specifications.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Имена DLL из таблицы импорта PE (PE32 и PE32+), включая отложенную загрузку (delay-load): CUDA-провайдер
/// onnxruntime берёт cuDNN и cuFFT именно так, и без неё проверка их не видит.
pub fn imported_libraries(binary: &Path) -> Result<Vec<String>, String> {
    let data = std::fs::read(binary).map_err(|e| format!("{}: {e}", binary.display()))?;
    let at = |offset: usize| -> Result<u32, String> {
        let b = data.get(offset..).and_then(|bytes| bytes.get(..4)).ok_or("truncated PE header")?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let short = |offset: usize| -> Result<u16, String> {
        let b = data.get(offset..).and_then(|bytes| bytes.get(..2)).ok_or("truncated PE header")?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    };
    let pe = at(0x3c)? as usize;
    if data.get(pe..).and_then(|bytes| bytes.get(..4)) != Some(b"PE\0\0") {
        return Err(format!("{} is not a PE file", binary.display()));
    }
    let sections = short(pe + 6)? as usize;
    let optional_size = short(pe + 20)? as usize;
    let optional = pe + 24;
    let directory_offset = match short(optional)? {
        0x10b => 96,
        0x20b => 112,
        magic => return Err(format!("unsupported PE optional header: {magic:#x}")),
    };
    if optional_size < directory_offset || data.get(optional..).and_then(|bytes| bytes.get(..optional_size)).is_none() {
        return Err("truncated PE optional header".into());
    }
    let directories = optional + directory_offset;
    let directory_count = at(directories - 4)? as usize;
    let headers: Vec<(u32, u32, u32)> = (0..sections)
        .map(|i| {
            let base = optional + optional_size + i * 40;
            Ok((at(base + 12)?, at(base + 16)?, at(base + 20)?))
        })
        .collect::<Result<_, String>>()?;
    let offset_of = |rva: u32| -> Option<usize> {
        headers.iter().find_map(|(va, size, raw)| {
            let delta = rva.checked_sub(*va)?;
            if delta >= *size {
                return None;
            }
            raw.checked_add(delta).map(|offset| offset as usize)
        })
    };
    let name_at = |rva: u32| -> Result<String, String> {
        let start = offset_of(rva).ok_or("import name outside the file")?;
        let end = data
            .get(start..)
            .ok_or("import name outside the file")?
            .iter()
            .position(|b| *b == 0)
            .ok_or("import name without a terminator")?
            + start;
        Ok(String::from_utf8_lossy(&data[start..end]).into_owned())
    };
    let mut names = Vec::new();
    // (каталог данных, размер дескриптора, смещение RVA имени в дескрипторе): импорт (1) и отложенный импорт (13).
    for (dir_index, descriptor, name_field) in [(1usize, 20usize, 12usize), (13, 32, 4)] {
        if dir_index >= directory_count {
            continue;
        }
        if directory_offset + (dir_index + 1) * 8 > optional_size {
            return Err("truncated PE data directories".into());
        }
        let table_rva = at(directories + dir_index * 8)?;
        if table_rva == 0 {
            continue;
        }
        let mut entry = offset_of(table_rva).ok_or("import table outside the file")?;
        loop {
            let d = data
                .get(entry..)
                .and_then(|bytes| bytes.get(..descriptor))
                .ok_or("truncated import table")?;
            if d.iter().all(|b| *b == 0) {
                break;
            }
            let f = &d[name_field..name_field + 4];
            names.push(name_at(u32::from_le_bytes([f[0], f[1], f[2], f[3]]))?);
            entry += descriptor;
        }
    }
    Ok(names)
}

/// Библиотеки, которые есть в любой Windows 10/11 или приходят с драйвером видеокарты. Всё прочее приложение
/// должно привезти само.
pub fn is_provided_by_the_system(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("api-ms-win-")
        || name.starts_with("ext-ms-win-")
        // Библиотека самого драйвера NVIDIA: по правилам NVIDIA не распространяется, её ставит драйвер.
        || name == "nvcuda.dll"
        // the Vulkan loader comes with every current graphics driver (NVIDIA, AMD, Intel)
        || name == "vulkan-1.dll"
        || [
            "kernel32.dll", "kernelbase.dll", "ntdll.dll", "user32.dll", "gdi32.dll", "advapi32.dll", "shell32.dll",
            "shlwapi.dll", "ole32.dll", "oleaut32.dll", "combase.dll", "ws2_32.dll", "crypt32.dll", "bcrypt.dll",
            "ncrypt.dll", "bcryptprimitives.dll", "secur32.dll", "rpcrt4.dll", "setupapi.dll", "cfgmgr32.dll", "version.dll", "dbghelp.dll",
            "powrprof.dll", "psapi.dll", "userenv.dll", "winmm.dll", "msvcrt.dll", "comdlg32.dll", "comctl32.dll",
            "imm32.dll", "iphlpapi.dll", "winhttp.dll", "wininet.dll", "dxgi.dll", "d3d11.dll", "d3d12.dll",
            "dxcore.dll", "mfplat.dll", "mf.dll", "mfreadwrite.dll", "avrt.dll", "ksuser.dll", "vfw32.dll",
            "avicap32.dll", "strmiids.dll", "dwmapi.dll", "uxtheme.dll", "winspool.drv", "normaliz.dll", "wldap32.dll",
            "d2d1.dll", "dwrite.dll", "usp10.dll", "dnsapi.dll", "msimg32.dll",
        ]
        .contains(&name.as_str())
}

/// Итог проверки одной точки входа.
#[derive(Debug, Default)]
pub struct Resolution {
    /// Не найдено ни в каталогах поиска, ни в системе.
    pub missing: Vec<String>,
    /// Найдено в каталогах поиска: (имя, путь).
    pub found: Vec<(String, PathBuf)>,
}

/// Обход импортов от точки входа (exe → ggml.dll → ggml-cuda.dll → cuBLAS…): каждая библиотека ищется в
/// каталоге точки входа, затем в каталогах поиска.
pub fn resolve(entry: &Path, dirs: &[PathBuf]) -> Result<Resolution, String> {
    let mut search: Vec<PathBuf> = entry.parent().map(|p| vec![p.to_path_buf()]).unwrap_or_default();
    search.extend(dirs.iter().cloned());
    let mut seen: Vec<String> = Vec::new();
    let mut out = Resolution::default();
    let mut queue: VecDeque<PathBuf> = VecDeque::from([entry.to_path_buf()]);
    while let Some(bin) = queue.pop_front() {
        for import in imported_libraries(&bin)? {
            let lowered = import.to_ascii_lowercase();
            if is_provided_by_the_system(&lowered) || seen.contains(&lowered) {
                continue;
            }
            seen.push(lowered);
            match search.iter().map(|d| d.join(&import)).find(|p| p.is_file()) {
                Some(p) => {
                    out.found.push((import, p.clone()));
                    queue.push_back(p);
                }
                None => out.missing.push(import),
            }
        }
    }
    Ok(out)
}

/// Supplied files may be absent from a staging root, but count only in a binary's search directories.
pub(crate) fn check_binaries(
    root: &Path,
    entries: &[(&str, &str)],
    dirs: &[PathBuf],
    provided: &[(String, PathBuf)],
    seen: &mut dyn FnMut(&str, &(String, PathBuf)),
) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for (component, rel) in entries {
        let entry = root.join(rel);
        if !entry.is_file() {
            problems.push(format!("{component}: no {rel} after installation"));
            continue;
        }
        let r = resolve(&entry, dirs)?;
        for hit in &r.found {
            seen(rel, hit);
        }
        for name in r.missing {
            let shipped = provided
                .iter()
                .any(|(n, d)| n.eq_ignore_ascii_case(&name) && (entry.parent() == Some(d.as_path()) || dirs.contains(d)));
            if !shipped {
                problems.push(format!(
                    "{rel} imports {name}: it is neither next to it, nor in Windows, nor in the manifest"
                ));
            }
        }
    }
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn pe(imports: &[&str], delayed: &[&str], plus: bool) -> Vec<u8> {
        let mut data = vec![0; 0xa00];
        data[..2].copy_from_slice(b"MZ");
        put32(&mut data, 0x3c, 0x80);
        data[0x80..0x84].copy_from_slice(b"PE\0\0");
        data[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        let optional = 0x98;
        let (magic, optional_size, directory_offset): (u16, u16, usize) = if plus { (0x20b, 240, 112) } else { (0x10b, 224, 96) };
        data[0x94..0x96].copy_from_slice(&optional_size.to_le_bytes());
        data[optional..optional + 2].copy_from_slice(&magic.to_le_bytes());
        let directories = optional + directory_offset;
        put32(&mut data, directories - 4, 16);
        let section = optional + usize::from(optional_size);
        put32(&mut data, section + 12, 0x1000);
        put32(&mut data, section + 16, 0x800);
        put32(&mut data, section + 20, 0x200);
        let mut name = 0x600;
        for (names, index, table, stride, field) in [(imports, 1, 0x200, 20, 12), (delayed, 13, 0x300, 32, 4)] {
            if names.is_empty() {
                continue;
            }
            put32(&mut data, directories + index * 8, (0x1000 + table - 0x200) as u32);
            put32(&mut data, directories + index * 8 + 4, ((names.len() + 1) * stride) as u32);
            for (i, library) in names.iter().enumerate() {
                let descriptor = table + i * stride;
                if index == 13 {
                    put32(&mut data, descriptor, 1);
                }
                put32(&mut data, descriptor + field, (0x1000 + name - 0x200) as u32);
                data[name..name + library.len()].copy_from_slice(library.as_bytes());
                name += library.len() + 1;
            }
        }
        data
    }

    fn write_pe(root: &Path, name: &str, imports: &[&str], delayed: &[&str]) -> PathBuf {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, pe(imports, delayed, true)).unwrap();
        path
    }

    #[test]
    fn normal_and_delayed_imports_are_read_from_pe32_and_pe32_plus() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("engine.dll");
        for plus in [false, true] {
            std::fs::write(&path, pe(&["kernel32.dll", "direct.dll"], &["delayed.dll"], plus)).unwrap();
            assert_eq!(imported_libraries(&path).unwrap(), ["kernel32.dll", "direct.dll", "delayed.dll"]);
        }
    }

    #[test]
    fn malformed_import_addresses_return_errors_instead_of_panicking() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("broken.dll");
        let mut outside_file = pe(&["direct.dll"], &[], true);
        put32(&mut outside_file, 0x20c, 0x17ff);
        outside_file.truncate(0x700);
        let mut overflowing_section = pe(&["direct.dll"], &[], true);
        put32(&mut overflowing_section, 0x188 + 12, 0xfffffff0);
        put32(&mut overflowing_section, 0x98 + 112 + 8, 0xfffffff5);
        let mut overflowing_offset = pe(&["direct.dll"], &[], true);
        put32(&mut overflowing_offset, 0x188 + 20, 0xfffffff0);
        put32(&mut overflowing_offset, 0x98 + 112 + 8, 0x1100);
        for data in [outside_file, overflowing_section, overflowing_offset, vec![0; 12]] {
            std::fs::write(&path, data).unwrap();
            assert!(imported_libraries(&path).is_err());
        }
    }

    #[test]
    fn an_unknown_optional_header_is_reported() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("unsupported.dll");
        let mut data = pe(&["direct.dll"], &[], true);
        data[0x98..0x9a].copy_from_slice(&0x999u16.to_le_bytes());
        std::fs::write(&path, data).unwrap();
        assert!(imported_libraries(&path).is_err());
    }

    #[test]
    fn data_directories_respect_the_declared_count_and_header_size() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("engine.dll");
        let mut data = pe(&["direct.dll"], &["delayed.dll"], true);
        put32(&mut data, 0x98 + 108, 2);
        std::fs::write(&path, &data).unwrap();
        assert_eq!(imported_libraries(&path).unwrap(), ["direct.dll"]);
        data[0x94..0x96].copy_from_slice(&120u16.to_le_bytes());
        std::fs::write(&path, data).unwrap();
        assert!(imported_libraries(&path).is_err());
    }

    #[test]
    fn resolution_walks_cycles_and_prefers_the_entry_directory() {
        let root = tempfile::tempdir().unwrap();
        let entry = write_pe(root.path(), "bin/app.exe", &["shared.dll", "KERNEL32.DLL"], &[]);
        let shared = write_pe(root.path(), "bin/shared.dll", &["leaf.dll"], &[]);
        let leaf = write_pe(root.path(), "search/leaf.dll", &["SHARED.DLL", "missing.dll"], &[]);
        std::fs::write(root.path().join("search/shared.dll"), b"the loader must prefer bin/shared.dll").unwrap();
        let resolution = resolve(&entry, &[root.path().join("search")]).unwrap();
        assert_eq!(resolution.found, [("shared.dll".into(), shared), ("leaf.dll".into(), leaf)]);
        assert_eq!(resolution.missing, ["missing.dll"]);
    }

    #[test]
    fn supplied_files_count_only_in_the_binarys_search_directories() {
        let root = tempfile::tempdir().unwrap();
        write_pe(root.path(), "bin/app.exe", &["Runtime.DLL"], &[]);
        let dirs = [root.path().join("search")];
        let entries = [("engine", "bin/app.exe")];
        for dir in [&root.path().join("bin"), &dirs[0]] {
            let provided = [("runtime.dll".into(), dir.clone())];
            assert!(check_binaries(root.path(), &entries, &dirs, &provided, &mut |_, _| {})
                .unwrap()
                .is_empty());
        }
        let wrong_dir = [("runtime.dll".into(), root.path().join("elsewhere"))];
        let problems = check_binaries(root.path(), &entries, &dirs, &wrong_dir, &mut |_, _| {}).unwrap();
        assert_eq!(
            problems,
            ["bin/app.exe imports Runtime.DLL: it is neither next to it, nor in Windows, nor in the manifest"]
        );
    }

    #[test]
    fn checks_report_missing_entries_and_observe_resolved_files() {
        let root = tempfile::tempdir().unwrap();
        write_pe(root.path(), "bin/app.exe", &["local.dll", "provided.dll"], &[]);
        let library = write_pe(root.path(), "bin/local.dll", &[], &[]);
        let provided = [("provided.dll".into(), root.path().join("bin"))];
        let mut seen = Vec::new();
        let problems = check_binaries(
            root.path(),
            &[("engine", "bin/app.exe"), ("absent", "missing.exe")],
            &[],
            &provided,
            &mut |entry, hit| seen.push((entry.to_string(), hit.clone())),
        )
        .unwrap();
        assert_eq!(problems, ["absent: no missing.exe after installation"]);
        assert_eq!(seen, [("bin/app.exe".into(), ("local.dll".into(), library))]);
    }

    /// Таблица импорта — основа проверки: если она не читается, проверка молча проходит на чём угодно.
    #[test]
    #[cfg(windows)]
    fn imports_are_read_out_of_a_real_binary() {
        let system = Path::new("C:/Windows/System32/notepad.exe");
        if !system.is_file() {
            return;
        }
        let imports: Vec<String> = imported_libraries(system)
            .expect("у notepad есть таблица импорта")
            .iter()
            .map(|n| n.to_ascii_lowercase())
            .collect();
        assert!(imports.iter().any(|n| n == "user32.dll"), "обычный импорт: {imports:?}");
        assert!(imports.iter().any(|n| n == "comdlg32.dll"), "отложенный импорт: {imports:?}");
        assert!(imports.iter().all(|n| n.ends_with(".dll") || n.ends_with(".drv")), "{imports:?}");
    }
}
