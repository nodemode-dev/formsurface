#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_updater::UpdaterExt;

mod engine_start;
mod reveal;
mod text_edit;
mod traffic_lights;
#[cfg(windows)]
mod win_job;

struct Backend(Mutex<Option<CommandChild>>);
struct PageListening(AtomicBool);
struct ExitGate(AtomicBool);
struct EngineDone(AtomicBool);
#[cfg(windows)]
struct Service(Mutex<Option<(u16, String)>>);

const ENGINE_PORT: &str = "8766";
const ENGINE_GONE: &str = "The FormSurface engine stopped while the app was running, so the window can no longer save or export. FormSurface will close now; start it again to continue from the last saved project.";
const FALLBACK_LOGICAL: (f64, f64) = (1440.0, 900.0);
const MIN_LOGICAL: (f64, f64) = (1080.0, 680.0);
const SCREEN_FRACTION: f64 = 0.9;

fn kill_backend(handle: &tauri::AppHandle) {
    handle.state::<EngineDone>().0.store(true, Ordering::SeqCst);
    let child = handle.state::<Backend>().0.lock().unwrap().take();
    if let Some(child) = child {
        #[cfg(unix)]
        {
            let pid = child.pid().to_string();
            let signal = |sig: &str| {
                std::process::Command::new("kill")
                    .args([sig, pid.as_str()])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success())
            };
            if signal("-TERM") {
                for _ in 0..30 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if !signal("-0") {
                        return;
                    }
                }
            }
        }
        let _ = child.kill();
    }
    #[cfg(windows)]
    win_job::end();
}

#[cfg(windows)]
fn ask_to_close(port: u16, key: &str) -> bool {
    use std::io::{Read, Write};
    let wait = std::time::Duration::from_secs(2);
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut stream) = std::net::TcpStream::connect_timeout(&address, wait) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(wait));
    let _ = stream.set_write_timeout(Some(wait));
    let request = format!("POST /api/shutdown HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Formsurface-Window-Key: {key}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut reply = [0u8; 12];
    stream.read_exact(&mut reply).is_ok() && reply.starts_with(b"HTTP/1.") && reply.ends_with(b" 200")
}

#[cfg(windows)]
fn wait_exit(pid: u32, millis: u32) {
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
        fn WaitForSingleObject(handle: *mut std::ffi::c_void, millis: u32) -> u32;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    }
    let handle = unsafe { OpenProcess(0x0010_0000, 0, pid) };
    if !handle.is_null() {
        unsafe {
            WaitForSingleObject(handle, millis);
            CloseHandle(handle);
        }
    }
}

fn close_backend(handle: &tauri::AppHandle) {
    handle.state::<EngineDone>().0.store(true, Ordering::SeqCst);
    #[cfg(windows)]
    {
        let service = handle.state::<Service>().0.lock().ok().and_then(|slot| slot.clone());
        let pid = handle.state::<Backend>().0.lock().ok().and_then(|slot| slot.as_ref().map(|child| child.pid()));
        if let (Some((port, key)), Some(pid)) = (service, pid) {
            if ask_to_close(port, &key) {
                wait_exit(pid, 4000);
            }
        }
    }
    kill_backend(handle);
}

fn close_splash(handle: &tauri::AppHandle) {
    if let Some(splash) = handle.get_webview_window("splash") {
        let _ = splash.close();
    }
}

fn open_exit_gate(handle: &tauri::AppHandle) {
    handle.state::<ExitGate>().0.store(true, Ordering::SeqCst);
}

fn limit(wait: std::time::Duration) -> tauri::async_runtime::Receiver<()> {
    let (tick, timer) = tauri::async_runtime::channel::<()>(1);
    std::thread::spawn(move || {
        std::thread::sleep(wait);
        let _ = tick.blocking_send(());
    });
    timer
}

async fn read_port(
    rx: &mut tauri::async_runtime::Receiver<CommandEvent>,
    key: &mut Option<String>,
    tail: &mut engine_start::Tail,
) -> Result<u16, engine_start::Failure> {
    let pattern = regex::Regex::new(r"http://127\.0\.0\.1:(\d+)").unwrap();
    let key_pattern = regex::Regex::new(r"formsurface-window-key: ([A-Za-z0-9_-]{32,})").unwrap();
    let mut timer = limit(engine_start::ADDRESS_WAIT);
    let mut ended: Option<Option<i32>> = None;
    loop {
        let event = std::future::poll_fn(|cx| match timer.poll_recv(cx) {
            std::task::Poll::Ready(_) => std::task::Poll::Ready(Err(())),
            std::task::Poll::Pending => rx.poll_recv(cx).map(Ok),
        })
        .await;
        let text = match event {
            Err(()) => {
                return Err(match ended {
                    Some(code) => engine_start::Failure::Stopped(code),
                    None => engine_start::Failure::TimedOut,
                })
            }
            Ok(None) => return Err(engine_start::Failure::Stopped(ended.flatten())),
            Ok(Some(CommandEvent::Terminated(end))) => {
                ended = Some(end.code);
                timer = limit(engine_start::EXIT_WAIT);
                continue;
            }
            Ok(Some(CommandEvent::Stdout(bytes) | CommandEvent::Stderr(bytes))) => String::from_utf8_lossy(&bytes).into_owned(),
            Ok(Some(_)) => continue,
        };
        for line in text.lines() {
            tail.push(line);
        }
        if key.is_none() {
            *key = key_pattern.captures(&text).map(|found| found[1].to_string());
            if key.is_some() {
                continue;
            }
        }
        if ended.is_none() {
            if let Some(port) = pattern.captures(&text).and_then(|found| found[1].parse::<u16>().ok()) {
                return Ok(port);
            }
        }
    }
}

fn start_failed(handle: &tauri::AppHandle, failure: engine_start::Failure, tail: &engine_start::Tail) {
    let text = engine_start::message(&failure, &tail.lines(), engine_start::engine_log().as_deref());
    eprintln!("formsurface: {text}");
    engine_failed(handle, &text);
}

fn engine_failed(handle: &tauri::AppHandle, message: &str) {
    if handle.state::<EngineDone>().0.swap(true, Ordering::SeqCst) {
        return;
    }
    handle.dialog().message(message).title(engine_start::APP).kind(MessageDialogKind::Error).blocking_show();
    close_splash(handle);
    open_exit_gate(handle);
    kill_backend(handle);
    handle.exit(1);
}

fn ask_page_to_quit(handle: &tauri::AppHandle) -> bool {
    if handle.state::<ExitGate>().0.load(Ordering::SeqCst) {
        return false;
    }
    let ready = handle.state::<PageListening>().0.load(Ordering::SeqCst);
    if !ready || handle.get_webview_window("main").is_none() {
        return false;
    }
    handle.emit("formsurface://exit-requested", ()).is_ok()
}

fn show_main_window(handle: &tauri::AppHandle) {
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn listen_for_exit(listening: tauri::State<PageListening>) {
    listening.0.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn confirm_exit(app: tauri::AppHandle, cancel: Option<bool>) {
    if cancel == Some(true) {
        return;
    }
    open_exit_gate(&app);
    app.exit(0);
}

fn single_instance() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_single_instance::Builder::<tauri::Wry>::new()
        .callback(|app, _args, _cwd| {
            show_main_window(app);
        })
        .build()
}

fn check_for_updates(handle: tauri::AppHandle) {
    let configured = handle
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|updater| updater.get("pubkey"))
        .and_then(|key| key.as_str())
        .map_or(false, |key| !key.is_empty());
    if !configured {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let Ok(updater) = handle.updater() else { return };
        let Ok(Some(update)) = updater.check().await else { return };
        let install = handle
            .dialog()
            .message(format!("FormSurface {} is available. Install it and restart?", update.version))
            .title("Update available")
            .buttons(MessageDialogButtons::OkCancel)
            .blocking_show();
        if install && update.download_and_install(|_, _| {}, || {}).await.is_ok() {
            open_exit_gate(&handle);
            close_backend(&handle);
            handle.restart();
        }
    });
}

fn main() {
    reveal::mark_launch();
    let app = tauri::Builder::default()
        .plugin(single_instance())
        .manage(PageListening(AtomicBool::new(false)))
        .manage(ExitGate(AtomicBool::new(false)))
        .manage(EngineDone(AtomicBool::new(false)))
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, WindowEvent::Resized(_) | WindowEvent::Focused(_) | WindowEvent::ThemeChanged(_) | WindowEvent::ScaleFactorChanged { .. })
            {
                traffic_lights::place(window);
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && ask_page_to_quit(window.app_handle()) {
                    api.prevent_close();
                }
            }
        })
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Backend(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            listen_for_exit,
            confirm_exit,
            traffic_lights::set_window_title,
            reveal::app_ready,
            text_edit::text_edit
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            reveal::round_splash(&handle);
            #[cfg(windows)]
            app.manage(Service(Mutex::new(None)));
            let started = app.shell().sidecar("formsurface-engine").map_err(|_| engine_start::Failure::Missing).and_then(|engine| {
                engine
                    .env("FORMSURFACE_PORT", ENGINE_PORT)
                    .env(engine_start::SHELL_ENV, std::process::id().to_string())
                    .env("FORMSURFACE_WINDOW_KEY_STDOUT", "1")
                    .spawn()
                    .map_err(|err| engine_start::Failure::NotStarted(err.to_string()))
            });
            let (mut rx, child) = match started {
                Ok(pair) => pair,
                Err(failure) => {
                    std::thread::spawn(move || start_failed(&handle, failure, &engine_start::Tail::default()));
                    return Ok(());
                }
            };
            #[cfg(windows)]
            win_job::hold_pid(child.pid());
            app.state::<Backend>().0.lock().unwrap().replace(child);

            tauri::async_runtime::spawn(async move {
                let mut key: Option<String> = None;
                let mut tail = engine_start::Tail::default();
                let port = read_port(&mut rx, &mut key, &mut tail).await;
                if port.is_ok() && handle.state::<EngineDone>().0.load(Ordering::SeqCst) {
                    return;
                }
                match port {
                    Ok(found) => {
                        #[cfg(windows)]
                        {
                            if let Ok(mut slot) = handle.state::<Service>().0.lock() {
                                *slot = Some((found, key.clone().unwrap_or_default()));
                            }
                        }
                        let origin = format!("http://127.0.0.1:{found}");
                        let url: tauri::Url = origin.parse().expect("loopback url");
                        let (width, height) = handle
                            .primary_monitor()
                            .ok()
                            .flatten()
                            .map(|monitor| {
                                let size = monitor.size();
                                let scale = monitor.scale_factor();
                                (
                                    (size.width as f64 / scale) * SCREEN_FRACTION,
                                    (size.height as f64 / scale) * SCREEN_FRACTION,
                                )
                            })
                            .unwrap_or(FALLBACK_LOGICAL);
                        let builder = WebviewWindowBuilder::new(&handle, "main", WebviewUrl::External(url))
                            .title("FormSurface")
                            .visible(reveal::BUILD_VISIBLE)
                            .background_color(reveal::BACKGROUND)
                            .inner_size(width, height)
                            .min_inner_size(MIN_LOGICAL.0, MIN_LOGICAL.1)
                            .center();
                        let builder = match key.as_deref() {
                            Some(value) => builder.initialization_script(format!(
                                "if (window.location.origin === {origin:?}) {{ window.__FORMSURFACE_WINDOW_KEY = {value:?}; }}"
                            )),
                            None => builder,
                        };
                        #[cfg(target_os = "macos")]
                        let builder = builder.title_bar_style(tauri::TitleBarStyle::Overlay).hidden_title(true);
                        let builder = builder.on_page_load(|window, payload| {
                            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                                traffic_lights::place(&window.as_ref().window());
                            }
                        });
                        match builder.build() {
                            Ok(window) => {
                                reveal::conceal(&window);
                                traffic_lights::place(&window.as_ref().window());
                            }
                            Err(_) => return start_failed(&handle, engine_start::Failure::Window, &engine_start::Tail::default()),
                        }
                        check_for_updates(handle.clone());
                        while let Some(event) = rx.recv().await {
                            if matches!(event, CommandEvent::Terminated(_)) {
                                engine_failed(&handle, ENGINE_GONE);
                                break;
                            }
                        }
                    }
                    Err(failure) => start_failed(&handle, failure, &tail),
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("tauri app should build");

    app.run(|handle, event| match event {
        RunEvent::Exit => close_backend(handle),
        RunEvent::ExitRequested { api, .. } => {
            if ask_page_to_quit(handle) {
                api.prevent_exit();
            }
        }
        _ => {}
    });
}
