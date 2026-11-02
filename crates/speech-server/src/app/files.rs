use crate::platform::process_group;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::path::Path;

#[cfg(test)]
mod tests;

/// Открыть каталог в проводнике (без выделения файла).
pub(crate) fn open_folder(dir: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let cmd = "explorer.exe";
    #[cfg(target_os = "macos")]
    let cmd = "open";
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let cmd = "xdg-open";
    dub_core::proc::host_command(cmd).arg(dir).spawn().map(|_| ())
}

/// POST /pick-folder — нативный диалог выбора папки (rfd). Возвращает {dir} или {dir:null} при отмене.
/// Для batch-экспорта: юзер выбирает ОДНУ папку назначения, дальше save-output кладёт туда все файлы.
pub(super) async fn pick_folder() -> Json<Value> {
    let dir = tokio::task::spawn_blocking(|| rfd::FileDialog::new().set_title(t!("export-pick-folder")).pick_folder())
        .await
        .ok()
        .flatten();
    Json(json!({ "dir": dir.map(|d| d.to_string_lossy().into_owned()) }))
}

/// Открыть проводник/файловый менеджер с ВЫДЕЛЕННЫМ файлом (не плеер). Windows: explorer /select.
pub(crate) fn reveal_in_explorer(path: String) {
    tokio::task::spawn_blocking(move || {
        #[cfg(windows)]
        {
            // explorer /select,"<path>" — выделяет файл в открытом каталоге. Один аргумент.
            let _ = process_group::detach_from_group(&mut std::process::Command::new("explorer"))
                .arg(format!("/select,{path}"))
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = dub_core::proc::host_command("open").arg("-R").arg(&path).spawn();
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            // прочие ОС: открыть родительский каталог (выделение файла непортабельно).
            let parent = std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let _ = dub_core::proc::host_command("xdg-open").arg(parent).spawn();
        }
    });
}

/// Отдать файл с поддержкой Range (для <video> seek). Используем tower-http ServeFile — он
/// корректно обрабатывает Range/If-Range/Content-Range. dl -> Content-Disposition attachment.
pub(crate) async fn serve_file_range(path: &Path, req: axum::http::Request<axum::body::Body>, download_name: Option<String>) -> Response {
    use tower::ServiceExt;
    use tower_http::services::ServeFile;
    let svc = ServeFile::new(path);
    match svc.oneshot(req).await {
        Ok(mut resp) => {
            if let Some(name) = download_name {
                // RFC 6266: filename — ASCII для старых клиентов, filename* — настоящее имя в UTF-8.
                let ascii: String = name
                    .chars()
                    .map(|c| {
                        if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let utf8 = percent_encoding::utf8_percent_encode(&name, percent_encoding::NON_ALPHANUMERIC);
                if let Ok(v) = axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{utf8}")) {
                    resp.headers_mut().insert(axum::http::header::CONTENT_DISPOSITION, v);
                }
            }
            resp.map(axum::body::Body::new)
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
