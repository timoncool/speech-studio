//! Tauri-оболочка Speech Studio. Ничего тяжёлого сама не делает: поднимает нативный `speech-server`
//! (axum, тот же REST/SSE-контракт, что бэкенд-питон) на 127.0.0.1:8797 (`SPEECH_STUDIO_PORT`) и
//! открывает окно на этот URL. Сервер сам раздаёт SPA (frontend/dist) и API на одном origin — фронт
//! работает с относительными путями без правок, а постоянный порт держит origin, и с ним
//! localStorage окна, одинаковым между запусками. Если на порту уже отвечает Speech Studio, второй
//! сервис не поднимается, а окно открывается на неё; второй запуск релизной сборки на порту по
//! умолчанию отдаёт фокус уже открытому окну.
//!
//! Где лежат ресурсы, данные, профиль WebView2 и временные файлы — решает модуль `layout`.

mod layout;

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use speech_server::service::{self, Claim};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Ждать, пока встроенный сервер ответит на /health, или его поток не сообщит, что остановился.
fn wait_for_own_service(port: u16, stopped: &Receiver<Result<(), String>>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if service::wait_until_serving(port, Duration::from_millis(500)) {
            return Ok(());
        }
        match stopped.try_recv() {
            Ok(Err(e)) => return Err(format!("Встроенный сервер Speech Studio не запустился: {e}")),
            Ok(Ok(())) => return Err("Встроенный сервер Speech Studio остановился сразу после запуска.".into()),
            Err(TryRecvError::Disconnected) => {
                return Err("Поток встроенного сервера Speech Studio аварийно завершился при запуске.".into())
            }
            Err(TryRecvError::Empty) => {}
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Встроенный сервер Speech Studio не ответил на 127.0.0.1:{port} за 30 с."
            ));
        }
    }
}

/// Окна ещё нет, консоль скрыта: единственный способ сказать пользователю, почему студия не
/// открылась, — нативный диалог.
fn fatal(message: &str) {
    eprintln!("[ERROR] {message}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Speech Studio")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn open_in_browser(app: &tauri::AppHandle, url: &tauri::Url) {
    use tauri_plugin_opener::OpenerExt;
    if let Err(e) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("[ERROR] ссылка {url} не открылась в браузере: {e}");
    }
}

/// Live as a widget: a bar with no frame, no shadow and no background, kept over every other window; the one already
/// open is brought forward. Built off the WebView2 callback that asked for it: a window made inside that callback
/// deadlocks WebView2. The bar's close button goes to `?widget&close`, which closes this window alone: a page closing
/// itself (`window.close()`) would end the whole app.
fn open_talk_widget(app: tauri::AppHandle, url: tauri::Url) {
    tauri::async_runtime::spawn(async move {
        if let Some(win) = app.get_webview_window("widget") {
            if let Err(e) = win.show().and_then(|_| win.set_focus()) {
                eprintln!("[ERROR] виджет Live не вышел на передний план: {e}");
            }
            return;
        }
        let for_close = app.clone();
        let built = WebviewWindowBuilder::new(&app, "widget", WebviewUrl::External(url))
            .title(speech_server::i18n::text("talk-widget-title", &[]))
            .inner_size(468.0, 120.0)
            .min_inner_size(360.0, 108.0)
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(true)
            .disable_drag_drop_handler()
            .on_navigation(move |target| {
                if !target.query_pairs().any(|(key, _)| key == "close") {
                    return true;
                }
                let app = for_close.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(win) = app.get_webview_window("widget") {
                        if let Err(e) = win.close() {
                            eprintln!("[ERROR] виджет Live не закрылся: {e}");
                        }
                    }
                });
                false
            })
            .build();
        if let Err(e) = built {
            eprintln!("[ERROR] виджет Live не открылся: {e}");
        }
    });
}

/// tauri-plugin-single-instance называет мьютекс и окно только по identifier, общему у установленной,
/// портативной и дев-сборок: с ним дев-копия рядом с открытой установленной молча выходила бы, подняв
/// старое окно. Замок нужен только релизной сборке на порту по умолчанию; дев-сборка и копия с явным
/// `SPEECH_STUDIO_PORT` живут на своём порту, а на том же — открывают окно на уже работающий сервис.
fn single_instance_wanted() -> bool {
    !cfg!(debug_assertions) && !service::port_is_explicit()
}

/// Поднять встроенный сервис на порту или найти уже работающий. Err — текст для пользователя.
fn start_or_reuse_service(repo_root: &std::path::Path) -> Result<u16, String> {
    let port = service::listen_port()?;
    match service::claim_port(port, Duration::from_secs(20)).map_err(|busy| busy.to_string())? {
        Claim::Bound(listener) => {
            let root = repo_root.to_path_buf();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("speech-server".into())
                .spawn(move || {
                    let result = speech_server::serve_blocking(&root, listener).map_err(|e| format!("{e:#}"));
                    if let Err(e) = &result {
                        eprintln!("[ERROR] встроенный speech-server остановился: {e}");
                    }
                    let _ = tx.send(result);
                })
                .map_err(|e| format!("Не удалось запустить поток сервера Speech Studio: {e}"))?;
            wait_for_own_service(port, &rx)?;
        }
        Claim::AlreadyRunning(running) => {
            eprintln!(
                "на 127.0.0.1:{port} уже работает Speech Studio {} ({}): окно откроется на неё",
                running.version, running.service_executable
            );
        }
    }
    Ok(port)
}

/// Окружение сервера до старта: путь к onnxruntime 1.31.1 (PATH под движок/инструменты добавит
/// augment_path_for_tools внутри serve_blocking).
fn setup_server_env(repo_root: &PathBuf) {
    speech_server::apply_gpu_choice(repo_root);
    speech_server::set_ort_dylib_env(repo_root);
}

/// Проверка обновления на GitHub-релизе и (по согласию юзера) установка. Драйвится из Rust: фронт
/// грузится с внешнего http-URL встроенного сервера, где Tauri JS-IPC ненадёжен, а Rust-апдейтер
/// работает независимо от webview. Тихо выходит при отсутствии апдейта/сети. На лету ставится только
/// копия из NSIS-установщика; портатив (нельзя перезаписать запущенный ~489-МБ каталог), MSI
/// (msiexec не принимает /D=, а Program Files без повышения прав недоступен) и сборка без типа
/// бандла получают предложение открыть страницу релиза.
const RELEASES_URL: &str = "https://github.com/timoncool/speech-studio/releases/latest";
fn spawn_update_check(app: tauri::AppHandle, portable: bool) {
    use tauri::utils::config::BundleType;
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
    use tauri_plugin_updater::UpdaterExt;
    let install_in_place =
        !portable && tauri::utils::platform::bundle_type() == Some(BundleType::Nsis);
    tauri::async_runtime::spawn(async move {
        // Прокси из настроек: GitHub, откуда обновления, — среди сайтов, ради которых прокси и ставят.
        let builder = match speech_server::net::fixed() {
            speech_server::net::Fixed::System => app.updater_builder(),
            speech_server::net::Fixed::Direct => app.updater_builder().no_proxy(),
            speech_server::net::Fixed::Through(proxy) => app.updater_builder().proxy(proxy),
        };
        // Установщик — ребёнок этого процесса, а процесс сидит в своём job с kill-on-close: без
        // освобождения установщик умер бы вместе со студией, ничего не поставив. Свой хук заменяет
        // штатный, поэтому cleanup_before_exit вызывается здесь же.
        let cleanup = app.clone();
        let mut builder = builder.on_before_exit(move || {
            cleanup.cleanup_before_exit();
            speech_server::process_group::terminate_group_members();
            if !speech_server::process_group::release_children() {
                eprintln!("[ERROR] установщик обновления не выведен из job object студии");
            }
        });
        if install_in_place {
            // Без /D= установщик, запущенный из студии, ставит копию в папку по умолчанию, а не в
            // текущую; NSIS требует его последним аргументом и без кавычек.
            builder = builder.installer_arg(format!("/D={}", layout::executable_directory().display()));
        }
        let updater = match builder.build() {
            Ok(u) => u,
            Err(_) => return,
        };
        let update = match updater.check().await {
            Ok(Some(u)) => u,
            _ => return, // нет апдейта или ошибка сети -> тихо
        };
        let ver = update.version.clone();
        // the window tells its language a moment after it loads; the question waits for it
        let _ = tauri::async_runtime::spawn_blocking(|| {
            for _ in 0..100 {
                if speech_server::i18n::reported() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        })
        .await;
        if !install_in_place {
            let open = app
                .dialog()
                .message(speech_server::i18n::text("update-open-page", &[("version", ver.clone())]))
                .title(speech_server::i18n::text("update-title", &[]))
                .kind(MessageDialogKind::Info)
                .buttons(MessageDialogButtons::OkCancelCustom(
                    speech_server::i18n::text("update-button-open", &[]),
                    speech_server::i18n::text("update-button-later", &[]),
                ))
                .blocking_show();
            if open {
                // Браузер, запущенный изнутри job студии, закрылся бы вместе с ней.
                let opened = speech_server::process_group::detach_from_group(
                    &mut speech_server::process_group::host_command(if cfg!(windows) {
                        "explorer"
                    } else if cfg!(target_os = "macos") {
                        "open"
                    } else {
                        "xdg-open"
                    }),
                )
                .arg(RELEASES_URL)
                .spawn();
                if let Err(e) = opened {
                    app.dialog()
                        .message(speech_server::i18n::text("update-open-failed", &[("url", RELEASES_URL.to_string()), ("error", e.to_string())]))
                        .title(speech_server::i18n::text("update-title", &[]))
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                }
            }
            return;
        }
        let yes = app
            .dialog()
            .message(speech_server::i18n::text("update-install-now", &[("version", ver.clone())]))
            .title(speech_server::i18n::text("update-title", &[]))
            .kind(MessageDialogKind::Info)
            .buttons(MessageDialogButtons::OkCancelCustom(
                speech_server::i18n::text("update-button-install", &[]),
                speech_server::i18n::text("update-button-later", &[]),
            ))
            .blocking_show();
        if !yes {
            return;
        }
        match update.download_and_install(|_, _| {}, || {}).await {
            Ok(_) => {
                app.restart(); // Windows: инсталлятор сам закроет приложение; на прочих ОС перезапустим
            }
            Err(e) => {
                app.dialog()
                    .message(speech_server::i18n::text("update-failed", &[("error", e.to_string())]))
                    .title(speech_server::i18n::text("update-title", &[]))
                    .kind(MessageDialogKind::Error)
                    .blocking_show();
            }
        }
    });
}

/// Спрятать ОКНО консоли. Exe — console-subsystem (чтобы дети ffmpeg/llama/roformer наследовали ОДНУ
/// консоль и НЕ плодили своих окон на каждый спавн). Сама консоль остаётся выделенной (дети её наследуют),
/// прячем только её ОКНО -> исчезает чёрное окно и его пустая запись в ALT+TAB/панели задач; в переключателе
/// остаётся лишь GUI-окно (с иконкой, см. .icon() ниже). Дети по-прежнему не открывают окон.
#[cfg(windows)]
fn hide_console_window() {
    extern "system" {
        fn GetConsoleWindow() -> isize;
        fn ShowWindow(hwnd: isize, n_cmd_show: i32) -> i32;
        fn GetConsoleProcessList(lpdw_process_list: *mut u32, dw_process_count: u32) -> u32;
    }
    unsafe {
        // Прячем ТОЛЬКО собственную консоль. Если exe запущен ИЗ существующего терминала, наш процесс
        // делит его консоль (к ней привязано >1 процесса) — это ЧУЖОЕ окно терминала пользователя, трогать
        // нельзя. При двойном клике из Проводника загрузчик создаёт нам отдельную консоль (count==1).
        let mut pids = [0u32; 4];
        let n = GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32);
        if n != 1 {
            return;
        }
        let hwnd = GetConsoleWindow();
        if hwnd != 0 {
            ShowWindow(hwnd, 0); // SW_HIDE
        }
    }
}

/// The port of the studio's own service and whether the close question is on screen.
#[derive(Default)]
struct Shell {
    port: std::sync::atomic::AtomicU16,
    asking: std::sync::atomic::AtomicBool,
}

/// A call to the studio's own service on 127.0.0.1: the shell needs no HTTP client for two small requests. Answers the
/// body of a 2xx reply.
fn local_call(port: u16, method: &str, path: &str, body: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nx-hub-skip: 1\r\nConnection: close\r\n\r\n{body}", body.len());
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).map_err(|e| e.to_string())?;
    let (head, rest) = reply.split_once("\r\n\r\n").unwrap_or((&reply, ""));
    let status = head.split_whitespace().nth(1).unwrap_or_default();
    if !status.starts_with('2') {
        return Err(format!("{method} {path}: {status} {rest}"));
    }
    Ok(rest.to_string())
}

/// Live's hotkey, Ctrl+Shift+Space from any window, as Voicebox and Discord have theirs: a tap turns the conversation's
/// microphone on or off; held longer than a third of a second, the microphone is open while the keys are down.
#[derive(Default)]
struct TalkKey {
    down: std::sync::Mutex<Option<(std::time::Instant, bool)>>,
}

const HOLD: std::time::Duration = std::time::Duration::from_millis(330);

fn talk_key(app: &tauri::AppHandle, pressed: bool) {
    let port = app.state::<Shell>().port.load(std::sync::atomic::Ordering::SeqCst);
    let key = app.state::<TalkKey>();
    let mut down = key.down.lock().unwrap_or_else(|p| p.into_inner());
    let set = |muted: bool| local_call(port, "POST", "/speech/listen/mute", &format!("{{\"muted\":{muted}}}"));
    if pressed {
        if down.is_some() {
            return;
        }
        let state = match local_call(port, "GET", "/listen/record/live", "") {
            Ok(body) => body,
            Err(e) => return eprintln!("[ERROR] клавиша Live: состояние разговора не прочитано: {e}"),
        };
        let parsed: serde_json::Value = serde_json::from_str(&state).unwrap_or_default();
        if parsed["running"].as_bool() != Some(true) {
            return;
        }
        let was_muted = parsed["muted"].as_bool().unwrap_or(false);
        if was_muted {
            if let Err(e) = set(false) {
                return eprintln!("[ERROR] клавиша Live: микрофон не включился: {e}");
            }
        }
        *down = Some((std::time::Instant::now(), was_muted));
    } else if let Some((at, was_muted)) = down.take() {
        let held = at.elapsed() >= HOLD;
        // a tap leaves the microphone the other way from before; a hold gives it back as it was
        let mute_now = if was_muted { held } else { !held };
        if mute_now {
            if let Err(e) = set(true) {
                eprintln!("[ERROR] клавиша Live: микрофон не выключился: {e}");
            }
        }
    }
}

/// Closing the window while tasks run asks first, in the studio's language; with nothing running, or a service that
/// does not answer, the studio closes.
fn confirm_close(app: &tauri::AppHandle) {
    use std::sync::atomic::Ordering;
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
    let shell = app.state::<Shell>();
    if shell.asking.swap(true, Ordering::SeqCst) {
        return;
    }
    let running = service::jobs_in_progress(shell.port.load(Ordering::SeqCst), Duration::from_secs(2)).unwrap_or(0);
    let close = running == 0
        || app
            .dialog()
            .message(speech_server::i18n::text("quit-running", &[("n", running.to_string())]))
            .title(speech_server::i18n::text("quit-title", &[]))
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                speech_server::i18n::text("quit-button-close", &[]),
                speech_server::i18n::text("quit-button-stay", &[]),
            ))
            .blocking_show();
    shell.asking.store(false, Ordering::SeqCst);
    if close {
        app.exit(0);
    }
}

/// The voice gate's launch-on-login setting and whether it is ready to open Live on an autostart launch (on, with a
/// voice enrolled). Read straight from the config file and the owner print beside it, so it needs no running server;
/// their folder is the models root (its env, else `<repo>/models`).
fn gate_config(repo_root: &std::path::Path) -> (bool, bool) {
    let models = std::env::var_os("SPEECH_STUDIO_MODELS_ROOT").map(PathBuf::from).unwrap_or_else(|| repo_root.join("models"));
    let dir = models.join("voice-gate");
    let cfg: serde_json::Value = std::fs::read(dir.join("config.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(serde_json::Value::Null);
    let autostart = cfg.get("autostart").and_then(|v| v.as_bool()).unwrap_or(false);
    let ready = cfg.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) && dir.join("owner.json").is_file();
    (autostart, ready)
}

/// Register or drop launch-on-login to match the voice gate's setting (off by default). Driven from Rust because the
/// server-served frontend has no Tauri IPC; run at startup and at exit, so a change made during a session is in force
/// by the next login.
fn sync_autostart(app: &tauri::AppHandle, repo_root: &std::path::Path) {
    use tauri_plugin_autostart::ManagerExt;
    let want = gate_config(repo_root).0;
    let mgr = app.autolaunch();
    let is = mgr.is_enabled().unwrap_or(false);
    if want && !is {
        if let Err(e) = mgr.enable() {
            eprintln!("[ERROR] автозапуск не включился: {e}");
        }
    } else if !want && is {
        if let Err(e) = mgr.disable() {
            eprintln!("[ERROR] автозапуск не выключился: {e}");
        }
    }
}

pub fn run() {
    // Что бы ни завершило процесс — окно, диспетчер задач, taskkill, падение, — llama-server,
    // bs_roformer-cli, whisper и ffmpeg уходят вместе с ним.
    if !speech_server::process_group::bind_children_to_this_process() {
        eprintln!("[ERROR] процесс не встал в свой job object: сайдкары гасятся только своими деструкторами");
    }
    #[cfg(windows)]
    hide_console_window();
    speech_server::diagnostics::init_logging();
    let placed = match layout::resolve().and_then(|l| layout::apply_environment(&l).map(|_| l)) {
        Ok(l) => l,
        Err(e) => layout::fatal(&e),
    };
    let repo_root = placed.server_root;
    speech_server::diagnostics::log_to_file(&repo_root);
    speech_server::ensure_library_path(&repo_root);
    setup_server_env(&repo_root);
    // Маршрут прокси до проверки обновлений: сервер ставит его в своём потоке, без гарантии, что раньше.
    speech_server::init_proxy_route(&repo_root);
    // the exit handler re-syncs launch-on-login, so a setting changed during a session is in force by the next login
    let repo_for_exit = repo_root.clone();

    let context = tauri::generate_context!();
    service::set_app_version(context.package_info().version.to_string());

    let mut builder = tauri::Builder::default();
    if single_instance_wanted() {
        // Первым: второй запуск должен уйти до остальной настройки, отдав фокус открытому окну.
        // Сервис поднимается только в .setup(), который Tauri зовёт после setup плагинов: второй
        // запуск выходит здесь, не тронув порт, и сервер живёт в процессе, чьё окно останется.
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let shown = win.unminimize().and_then(|_| win.show()).and_then(|_| win.set_focus());
                if let Err(e) = shown {
                    eprintln!("[ERROR] повторный запуск: окно Speech Studio не вышло на передний план: {e}");
                }
            }
        }));
    }
    builder
        // a plugin's link script answers a click through IPC, which a page on http://127.0.0.1 is not given;
        // left to the webview, a click reaches on_new_window
        .plugin(tauri_plugin_opener::Builder::new().open_js_links_on_click(false).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    let pressed = event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed;
                    if let Some(queue) = app.try_state::<std::sync::Mutex<std::sync::mpsc::Sender<bool>>>() {
                        let _ = queue.lock().unwrap_or_else(|p| p.into_inner()).send(pressed);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        // launch-on-login for the voice gate; `--autostarted` lets the app tell it was started that way. Off until the
        // gate's setting turns it on (synced from Rust in setup and at exit).
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostarted"]),
        ))
        .setup(move |app| {
            // axum-бэкенд поднимается В ЭТОМ ЖЕ процессе на фоновом потоке — ОДИН exe, без speech-server.exe-сайдкара.
            // Окно открывается только после ответа /health, чтобы не встать на пустую страницу.
            let port = match start_or_reuse_service(&repo_root) {
                Ok(p) => p,
                Err(message) => {
                    fatal(&message);
                    app.handle().exit(1);
                    return Ok(());
                }
            };
            app.manage(Shell::default());
            app.manage(TalkKey::default());
            let (to_key, keys) = std::sync::mpsc::channel::<bool>();
            app.manage(std::sync::Mutex::new(to_key));
            let for_key = app.handle().clone();
            std::thread::spawn(move || {
                for pressed in keys {
                    talk_key(&for_key, pressed);
                }
            });
            {
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
                let key = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
                if let Err(e) = app.global_shortcut().register(key) {
                    eprintln!("[ERROR] клавиша Live Ctrl+Shift+Space занята другой программой: {e}");
                }
            }
            app.state::<Shell>().port.store(port, std::sync::atomic::Ordering::SeqCst);
            // launch-on-login in force per the voice gate's setting; on an autostart launch with the gate on and a
            // voice enrolled, open Live now so the owner can just start talking
            sync_autostart(&app.handle(), &repo_root);
            if std::env::args().any(|a| a == "--autostarted") && gate_config(&repo_root).1 {
                if let Err(e) = local_call(port, "POST", "/speech/listen", "{}") {
                    eprintln!("[ERROR] автозапуск: Live не открылся: {e}");
                }
            }
            let url = format!("http://127.0.0.1:{port}/");
            // Иконка бандла для GUI-окна: без явной установки окно оставалось пустым в ALT+TAB/панели задач
            // (иконка висела на консольном окне). Ставим её на само GUI-окно.
            let icon = app.default_window_icon().cloned();
            let (for_tabs, for_links, for_widget) = (app.handle().clone(), app.handle().clone(), app.handle().clone());
            let win = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(url.parse().expect("валидный URL")),
            )
            .title(format!("Speech Studio {}", app.package_info().version))
            .inner_size(1600.0, 1040.0)
            .min_inner_size(1280.0, 800.0)
            .resizable(true)
            // Tauri v2 по умолчанию перехватывает OS-drop файлов -> HTML5 onDrop в дропзоне НЕ срабатывает
            // (юзеры жаловались «перетаскивание не работает»). Отключаем перехват -> webview сам ловит drop.
            .disable_drag_drop_handler()
            // WebView2 молча глотает target="_blank" и window.open, если у окна нет обработчика новых окон.
            .on_new_window(move |target, _features| {
                let own = target.host_str() == Some("127.0.0.1") && target.port() == Some(port);
                if own && target.query_pairs().any(|(key, _)| key == "widget") {
                    open_talk_widget(for_widget.clone(), target);
                } else {
                    open_in_browser(&for_tabs, &target);
                }
                tauri::webview::NewWindowResponse::Deny
            })
            .on_navigation(move |target| {
                let own = target.host_str() == Some("127.0.0.1") && target.port() == Some(port);
                if own || !matches!(target.scheme(), "http" | "https") {
                    return true;
                }
                open_in_browser(&for_links, target);
                false
            })
            .build()?;
            // Иконка окна (ALT+TAB/таскбар) — ПОСЛЕ создания: не паникуем, если не выйдет, окно рабочее.
            if let Some(ic) = icon {
                let _ = win.set_icon(ic);
            }
            // авто-обновление: проверка на GitHub-релизе в фоне, установка по согласию (см. spawn_update_check)
            spawn_update_check(app.handle().clone(), layout::is_portable());
            Ok(())
        })
        .build(context)
        .expect("ошибка запуска Tauri")
        .run(move |app, event| match &event {
            tauri::RunEvent::WindowEvent { label, event: tauri::WindowEvent::CloseRequested { api, .. }, .. } if label == "main" => {
                api.prevent_close();
                let app = app.clone();
                std::thread::spawn(move || confirm_close(&app));
            }
            tauri::RunEvent::Exit => {
                sync_autostart(app, &repo_for_exit);
                speech_server::flush_hub_on_exit();
                // the servers and engines the studio started end with it, also where no job object ends them (macOS)
                speech_server::process_group::terminate_group_members();
            }
            _ => {}
        });
}
