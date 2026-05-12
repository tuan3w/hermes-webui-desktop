mod pins;

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;
use tauri::webview::WebviewBuilder;
use tauri::window::WindowBuilder;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_updater::UpdaterExt;

const SERVER_HOST: &str = "127.0.0.1";
const UV_PYTHON: &str = "3.11";

struct ServerState {
    child: Mutex<Option<tauri_plugin_shell::process::CommandChild>>,
}

struct UpdateState {
    pending: Mutex<Option<tauri_plugin_updater::Update>>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn free_port() -> u16 {
    TcpListener::bind((SERVER_HOST, 0))
        .expect("could not bind to find a free port")
        .local_addr()
        .unwrap()
        .port()
}

fn wait_for_server(port: u16) -> bool {
    for _ in 0..120 {
        if TcpStream::connect((SERVER_HOST, port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
}

fn venv_python(venv_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        venv_dir.join("Scripts").join("python.exe")
    } else {
        venv_dir.join("bin").join("python")
    }
}

fn show_window(handle: &tauri::AppHandle) {
    if let Some(win) = handle.get_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.unminimize();
    }
}

fn set_status(handle: &tauri::AppHandle, msg: &str) {
    let esc = msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    if let Some(wv) = handle.get_webview("hermes") {
        let _ = wv.eval(&format!(
            r#"window.__hermesSetStatus && window.__hermesSetStatus("{esc}")"#
        ));
    }
}

fn show_error(handle: &tauri::AppHandle, msg: &str) {
    eprintln!("[hermes] ERROR: {msg}");
    let esc = msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    if let Some(wv) = handle.get_webview("hermes") {
        let _ = wv.eval(&format!(
            r#"window.__hermesShowError && window.__hermesShowError("{esc}")"#
        ));
    }
}

// ── uv / venv ─────────────────────────────────────────────────────────────────

async fn run_uv(handle: &tauri::AppHandle, args: &[&str]) -> Result<(), String> {
    let (mut rx, _child) = handle
        .shell()
        .sidecar("uv")
        .map_err(|e| format!("uv sidecar unavailable: {e}\nRun `cargo build` to download it."))?
        .args(args)
        .spawn()
        .map_err(|e| format!("Failed to spawn uv: {e}"))?;

    let mut output = String::new();
    loop {
        match rx.recv().await {
            Some(CommandEvent::Stdout(b)) | Some(CommandEvent::Stderr(b)) => {
                if let Ok(s) = std::str::from_utf8(&b) {
                    eprint!("[uv] {s}");
                    output.push_str(s);
                    if let Some(line) = s.lines().filter(|l| !l.trim().is_empty()).last() {
                        set_status(handle, line.trim());
                    }
                }
            }
            Some(CommandEvent::Terminated(status)) => {
                return if status.code == Some(0) {
                    Ok(())
                } else {
                    Err(format!(
                        "uv {} failed (exit {:?})\n{}",
                        args.first().unwrap_or(&""),
                        status.code,
                        output.trim()
                    ))
                };
            }
            None => return Ok(()),
            _ => {}
        }
    }
}

async fn ensure_venv(
    handle: &tauri::AppHandle,
    resource_dir: &Path,
    app_data_dir: &Path,
) -> Result<PathBuf, String> {
    let venv_dir = app_data_dir.join("venv");
    let python = venv_python(&venv_dir);

    if python.exists() {
        eprintln!("[hermes] venv OK: {}", venv_dir.display());
        return Ok(python);
    }

    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| format!("Cannot create app data dir: {e}"))?;

    set_status(handle, "Setting up Python environment (first launch)…");
    eprintln!("[hermes] Creating venv at {}", venv_dir.display());

    run_uv(handle, &["venv", "--python", UV_PYTHON, venv_dir.to_str().unwrap()])
        .await
        .map_err(|e| format!("Could not create Python environment.\n\n{e}"))?;

    let req = resource_dir.join("requirements.txt");
    set_status(handle, "Installing Python dependencies…");

    run_uv(
        handle,
        &["pip", "install", "-r", req.to_str().unwrap(), "--python", python.to_str().unwrap()],
    )
    .await
    .map_err(|e| format!("Could not install Python dependencies.\n\n{e}"))?;

    Ok(python)
}

// ── Auto-updater ──────────────────────────────────────────────────────────────

async fn background_check_update(handle: tauri::AppHandle) {
    let updater = match handle.updater() {
        Ok(u) => u,
        Err(e) => { eprintln!("[updater] disabled: {e}"); return; }
    };
    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            eprintln!("[updater] new version: {version}");
            *handle.state::<UpdateState>().pending.lock().unwrap() = Some(update);
            if let Some(wv) = handle.get_webview("hermes") {
                let v = version.replace('"', "\\\"");
                let _ = wv.eval(&format!(
                    r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
                ));
            }
        }
        Ok(None) => eprintln!("[updater] up to date"),
        Err(e) => eprintln!("[updater] check failed: {e}"),
    }
}

#[tauri::command]
fn open_devtools(handle: tauri::AppHandle) {
    if let Some(wv) = handle.get_webview("hermes") {
        wv.open_devtools();
    }
}

#[tauri::command]
async fn check_update(
    handle: tauri::AppHandle,
    state: tauri::State<'_, UpdateState>,
) -> Result<Option<String>, String> {
    let updater = handle.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            *state.pending.lock().unwrap() = Some(update);
            Ok(Some(version))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn install_update(
    handle: tauri::AppHandle,
    state: tauri::State<'_, UpdateState>,
) -> Result<(), String> {
    let update = state
        .pending
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| "No pending update".to_string())?;

    update
        .download_and_install(
            |chunk, total| { if let Some(t) = total { eprintln!("[updater] {chunk}/{t}"); } },
            || eprintln!("[updater] installing…"),
        )
        .await
        .map_err(|e| e.to_string())?;

    handle.restart();
}

// ── System tray ───────────────────────────────────────────────────────────────

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};

    let open = MenuItemBuilder::with_id("open", "Open Hermes").build(app)?;
    let check_update_item = MenuItemBuilder::with_id("check_update", "Check for Updates…").build(app)?;
    let about = PredefinedMenuItem::about(
        app,
        Some("About Hermes"),
        Some(tauri::menu::AboutMetadata {
            name: Some("Hermes WebUI Desktop".to_string()),
            version: Some(app.package_info().version.to_string()),
            copyright: Some("© 2026 Nguyen Duc Tuan (tuan3w)".to_string()),
            website: Some("https://github.com/tuan3w/hermes-webui-desktop".to_string()),
            website_label: Some("GitHub".to_string()),
            ..Default::default()
        }),
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&open, &sep, &check_update_item, &about, &sep2, &quit])
        .build()?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Hermes WebUI Desktop")
        .on_menu_event({
            let handle = app.handle().clone();
            move |_tray, event| {
                let h = handle.clone();
                match event.id().as_ref() {
                    "open" => show_window(&h),
                    "quit" => {
                        // Kill Python server before exit
                        if let Some(c) = h.state::<ServerState>().child.lock().unwrap().take() {
                            let _ = c.kill();
                        }
                        h.exit(0);
                    }
                    "check_update" => {
                        tauri::async_runtime::spawn(async move {
                            let state = h.state::<UpdateState>();
                            match check_update(h.clone(), state).await {
                                Ok(Some(version)) => {
                                    if let Some(wv) = h.get_webview("hermes") {
                                        show_window(&h);
                                        let v = version.replace('"', "\\\"");
                                        let _ = wv.eval(&format!(
                                            r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
                                        ));
                                    }
                                }
                                Ok(None) => eprintln!("[updater] up to date"),
                                Err(e) => eprintln!("[updater] check error: {e}"),
                            }
                        });
                    }
                    _ => {}
                }
            }
        })
        .on_tray_icon_event({
            let handle = app.handle().clone();
            move |_tray, event| {
                // Left-click / double-click → show window (works on macOS & Windows)
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    show_window(&handle);
                }
            }
        })
        .build(app)?;

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single instance must be first
        .plugin(
            tauri_plugin_single_instance::Builder::new()
                .callback(|handle, _args, _cwd| {
                    // Second launch attempt → focus existing window
                    show_window(handle);
                })
                .build(),
        )
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(ServerState { child: Mutex::new(None) })
        .manage(UpdateState { pending: Mutex::new(None) })
        .invoke_handler(tauri::generate_handler![
            open_devtools,
            check_update,
            install_update,
            pins::list_pins,
            pins::add_pin,
            pins::remove_pin,
            pins::open_pin,
        ])
        .setup(|app| {
            build_tray(app)?;

            let window = WindowBuilder::new(app, "main")
                .title("Hermes")
                .inner_size(1436.0, 900.0)
                .min_inner_size(936.0, 600.0)
                .build()?;

            // Strip webview — left 36px, always visible
            window.add_child(
                WebviewBuilder::new("strip", tauri::WebviewUrl::App("strip.html".into())),
                tauri::Position::Logical(tauri::LogicalPosition::new(0.0, 0.0)),
                tauri::Size::Logical(tauri::LogicalSize::new(36.0, 900.0)),
            )?;

            // Hermes webview — fills rest, starts with splash screen
            let hermes_wv = window.add_child(
                WebviewBuilder::new("hermes", tauri::WebviewUrl::App("/".into())),
                tauri::Position::Logical(tauri::LogicalPosition::new(36.0, 0.0)),
                tauri::Size::Logical(tauri::LogicalSize::new(1400.0, 900.0)),
            )?;

            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::{PermissionRequestExt, WebViewExt};
                let _ = hermes_wv.with_webview(|wv| {
                    wv.inner().connect_permission_request(|_, req| {
                        req.allow();
                        true
                    });
                });
            }

            let handle = app.handle().clone();
            let resource_dir = app.path().resource_dir().expect("resource dir");
            let app_data_dir = app.path().app_data_dir().expect("app data dir");

            let pins_path = app_data_dir.join("pins.json");
            app.manage(pins::PinsState::load(pins_path));

            // Update check runs in parallel with server startup
            let update_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                background_check_update(update_handle).await;
            });

            tauri::async_runtime::spawn(async move {
                set_status(&handle, "Checking Python environment…");

                let python = match ensure_venv(&handle, &resource_dir, &app_data_dir).await {
                    Ok(p) => p,
                    Err(e) => { show_error(&handle, &e); return; }
                };

                let port = free_port();
                let bootstrap_py = resource_dir.join("bootstrap.py");

                if !bootstrap_py.exists() {
                    show_error(&handle, &format!("bootstrap.py not found:\n{}", bootstrap_py.display()));
                    return;
                }

                set_status(&handle, "Starting server…");

                // Launch bootstrap.py --foreground --skip-agent-install.
                // bootstrap.py discovers HERMES_WEBUI_AGENT_DIR from env vars
                // and common paths (~/.hermes/hermes-agent, ~/hermes-agent, etc.),
                // then execs into server.py with the correct env set up.
                let spawn_result = if cfg!(windows) {
                    handle
                        .shell()
                        .command(python.to_str().unwrap())
                        .args([
                            bootstrap_py.to_str().unwrap(),
                            "--foreground",
                            "--skip-agent-install",
                        ])
                        .env("HERMES_WEBUI_HOST", SERVER_HOST)
                        .env("HERMES_WEBUI_PORT", port.to_string())
                        .env("PYTHONDONTWRITEBYTECODE", "1")
                        .current_dir(&resource_dir)
                        .spawn()
                } else {
                    handle
                        .shell()
                        .command("sh")
                        .arg("-c")
                        .arg(format!(
                            "unset PYTHONHOME PYTHONPATH; exec '{}' '{}' --foreground --skip-agent-install",
                            python.display(),
                            bootstrap_py.display()
                        ))
                        .env("HERMES_WEBUI_HOST", SERVER_HOST)
                        .env("HERMES_WEBUI_PORT", port.to_string())
                        .env("PYTHONDONTWRITEBYTECODE", "1")
                        .current_dir(&resource_dir)
                        .spawn()
                };

                let (mut rx, child) = match spawn_result {
                    Ok(v) => v,
                    Err(e) => { show_error(&handle, &format!("Could not launch server:\n{e}")); return; }
                };

                *handle.state::<ServerState>().child.lock().unwrap() = Some(child);

                // Collect server output: show each line on the splash screen and
                // keep the last 20 lines so we can include them in any error message.
                let log_buf: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
                let log_buf_writer = log_buf.clone();
                let handle_log = handle.clone();

                tauri::async_runtime::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        match event {
                            CommandEvent::Stdout(b) | CommandEvent::Stderr(b) => {
                                if let Ok(text) = std::str::from_utf8(&b) {
                                    eprint!("[server] {text}");
                                    for line in text.lines() {
                                        let trimmed = line.trim().to_string();
                                        if trimmed.is_empty() { continue; }
                                        // Stream each line to the log panel in the splash screen
                                        let esc = trimmed
                                            .replace('\\', "\\\\")
                                            .replace('"', "\\\"")
                                            .replace('\n', "\\n");
                                        if let Some(wv) = handle_log.get_webview("hermes") {
                                            let _ = wv.eval(&format!(
                                                r#"window.__hermesAppendLog && window.__hermesAppendLog("{esc}", false)"#
                                            ));
                                        }
                                        log_buf_writer.lock().unwrap().push(trimmed);
                                    }
                                }
                            }
                            CommandEvent::Terminated(s) => {
                                eprintln!("[hermes] server exited: {:?}", s.code);
                                break;
                            }
                            _ => {}
                        }
                    }
                });

                let ready = tauri::async_runtime::spawn_blocking(move || wait_for_server(port))
                    .await
                    .unwrap_or(false);

                if ready {
                    eprintln!("[hermes] server ready on :{port}");
                    if let Some(wv) = handle.get_webview("hermes") {
                        let _ = wv.eval(&format!("window.location='http://{SERVER_HOST}:{port}'"));
                    }
                } else {
                    let tail = log_buf.lock().unwrap().join("\n");
                    let detail = if tail.is_empty() {
                        String::new()
                    } else {
                        format!("\n\nLast output:\n{tail}")
                    };
                    show_error(
                        &handle,
                        &format!(
                            "Server did not respond on :{port} within 60 s.{detail}"
                        ),
                    );
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Intercept close — hide to tray instead of quitting
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Resized(physical_size) => {
                let phy_w = physical_size.width;
                let phy_h = physical_size.height;
                if phy_w == 0 {
                    return; // minimized — skip layout
                }
                let scale = window.scale_factor().unwrap_or(1.0);
                let strip_phys = (36.0 * scale).round() as u32;
                let h = window.app_handle();
                if let Some(strip) = h.get_webview("strip") {
                    let _ = strip.set_bounds(tauri::Rect {
                        position: tauri::Position::Physical(tauri::PhysicalPosition::new(0, 0)),
                        size: tauri::Size::Physical(tauri::PhysicalSize::new(strip_phys, phy_h)),
                    });
                }
                if let Some(hermes_wv) = h.get_webview("hermes") {
                    let _ = hermes_wv.set_bounds(tauri::Rect {
                        position: tauri::Position::Physical(tauri::PhysicalPosition::new(strip_phys as i32, 0)),
                        size: tauri::Size::Physical(tauri::PhysicalSize::new(
                            phy_w.saturating_sub(strip_phys),
                            phy_h,
                        )),
                    });
                }
            }
            tauri::WindowEvent::Destroyed => {
                if let Some(c) = window
                    .app_handle()
                    .state::<ServerState>()
                    .child
                    .lock()
                    .unwrap()
                    .take()
                {
                    let _ = c.kill();
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
