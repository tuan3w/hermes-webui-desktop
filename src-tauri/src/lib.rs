use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Manager;
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

/// Strip the `\\?\` extended-length prefix that Windows APIs sometimes produce.
/// Tools like `uv` and Python subprocess calls can fail when given such paths.
#[cfg(windows)]
fn strip_unc_prefix(path: &Path) -> std::borrow::Cow<'_, Path> {
    let s = path.to_str().unwrap_or("");
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        std::borrow::Cow::Owned(PathBuf::from(stripped))
    } else {
        std::borrow::Cow::Borrowed(path)
    }
}

#[cfg(not(windows))]
fn strip_unc_prefix(path: &Path) -> std::borrow::Cow<'_, Path> {
    std::borrow::Cow::Borrowed(path)
}

fn show_window(handle: &tauri::AppHandle) {
    if let Some(win) = handle.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.unminimize();
    }
}

fn set_status(handle: &tauri::AppHandle, msg: &str) {
    let esc = msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    if let Some(win) = handle.get_webview_window("main") {
        let _ = win.eval(&format!(
            r#"window.__hermesSetStatus && window.__hermesSetStatus("{esc}")"#
        ));
    }
}

fn show_error(handle: &tauri::AppHandle, msg: &str) {
    eprintln!("[hermes] ERROR: {msg}");
    let esc = msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    if let Some(win) = handle.get_webview_window("main") {
        let _ = win.eval(&format!(
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

    let venv_dir_norm = strip_unc_prefix(&venv_dir);
    run_uv(handle, &["venv", "--python", UV_PYTHON, venv_dir_norm.to_str().unwrap()])
        .await
        .map_err(|e| format!("Could not create Python environment.\n\n{e}"))?;

    let req = resource_dir.join("requirements.txt");
    set_status(handle, "Installing Python dependencies…");

    let python_norm = strip_unc_prefix(&python);
    run_uv(
        handle,
        &["pip", "install", "-r", req.to_str().unwrap(), "--python", python_norm.to_str().unwrap()],
    )
    .await
    .map_err(|e| format!("Could not install Python dependencies.\n\n{e}"))?;

    Ok(python)
}

// ── Auto-updater ──────────────────────────────────────────────────────────────

/// Injected into the Python server page after DOMContentLoaded.
/// Functions are defined immediately (no DOM access needed); DOM nodes are
/// created only once the body exists to avoid crashes during early injection.
const INJECT_BANNER_JS: &str = r#"(function(){
  // Safe to define immediately — no DOM access.
  window.__hermesUpdateLater=function(){
    var b=document.getElementById('__hu-banner');
    if(b)b.classList.remove('hu-on');
  };
  window.__hermesInstallUpdate=async function(){
    var btn=document.getElementById('__hu-install');
    var sub=document.getElementById('__hu-sub');
    if(btn){btn.disabled=true;btn.textContent='Installing…';}
    try{
      await window.__TAURI__.core.invoke('install_update');
    }catch(e){
      if(btn){btn.disabled=false;btn.textContent='Update';}
      if(sub)sub.textContent='Error: '+e;
    }
  };
  window.__hermesShowUpdate=function(v){
    var b=document.getElementById('__hu-banner');
    if(b){
      document.getElementById('__hu-title').textContent='Update available — v'+v;
      b.classList.add('hu-on');
    }else{
      // DOM not ready yet; remember and show once it is.
      window.__hermesPendingUpdate=v;
    }
  };
  function _injectDOM(){
    if(document.getElementById('__hu-banner'))return;
    var s=document.createElement('style');
    s.textContent=[
      '#__hu-banner{position:fixed;bottom:20px;left:20px;background:#0f1011;border:1px solid #23252a;',
      'border-radius:12px;padding:14px 16px;width:272px;box-shadow:0 4px 24px rgba(0,0,0,.7);',
      'font-family:system-ui,-apple-system,sans-serif;z-index:2147483647;display:none;}',
      '#__hu-banner.hu-on{display:block;}',
      '#__hu-title{font-size:13px;font-weight:600;color:#f7f8f8;letter-spacing:-.02em;margin-bottom:3px;}',
      '#__hu-sub{font-size:12px;color:#8a8f98;margin-bottom:12px;}',
      '#__hu-actions{display:flex;gap:8px;justify-content:flex-end;}',
      '.__hu-btn{padding:5px 12px;border-radius:8px;font-size:12px;font-weight:500;cursor:pointer;',
      'border:none;font-family:inherit;transition:opacity .15s;}',
      '.__hu-btn:hover{opacity:.8;}.__hu-btn:disabled{opacity:.45;cursor:default;}',
      '.__hu-primary{background:#5e6ad2;color:#fff;}',
      '.__hu-ghost{background:transparent;color:#8a8f98;border:1px solid #23252a;}'
    ].join('');
    document.head.appendChild(s);
    var d=document.createElement('div');
    d.id='__hu-banner';
    d.innerHTML='<div id="__hu-title">Update available</div>'
      +'<div id="__hu-sub">A new version is ready to install.</div>'
      +'<div id="__hu-actions">'
      +'<button class="__hu-btn __hu-ghost" onclick="window.__hermesUpdateLater()">Later</button>'
      +'<button class="__hu-btn __hu-primary" id="__hu-install" onclick="window.__hermesInstallUpdate()">Update</button>'
      +'</div>';
    document.body.appendChild(d);
    if(window.__hermesPendingUpdate){
      window.__hermesShowUpdate(window.__hermesPendingUpdate);
      delete window.__hermesPendingUpdate;
    }
  }
  if(document.body)_injectDOM();
  else document.addEventListener('DOMContentLoaded',_injectDOM);
})();"#;

fn inject_and_show_update(win: &tauri::WebviewWindow<tauri::Wry>, version: &str) {
    let _ = win.eval(INJECT_BANNER_JS);
    let v = version.replace('"', "\\\"");
    let _ = win.eval(&format!(
        r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
    ));
}

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
            if let Some(win) = handle.get_webview_window("main") {
                inject_and_show_update(&win, &version);
            }
        }
        Ok(None) => eprintln!("[updater] up to date"),
        Err(e) => eprintln!("[updater] check failed: {e}"),
    }
}

#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) {
    window.open_devtools();
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
                                    if let Some(win) = h.get_webview_window("main") {
                                        show_window(&h);
                                        inject_and_show_update(&win, &version);
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
        .invoke_handler(tauri::generate_handler![open_devtools, check_update, install_update])
        .setup(|app| {
            build_tray(app)?;

            let page_load_handle = app.handle().clone();
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("/".into()),
            )
            .title("Hermes")
            .inner_size(1400.0, 900.0)
            .min_inner_size(900.0, 600.0)
            .on_page_load(move |win, payload| {
                // Only inject into the Python server pages, not the splash screen.
                // Wait for Finished so document.body exists when the script runs.
                if !matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) { return; }
                let url = payload.url().as_str();
                if url.starts_with("http://127.0.0.1") {
                    let _ = win.eval(INJECT_BANNER_JS);
                    // If an update was already found (race: check completed before nav),
                    // show the banner immediately in this new page context.
                    if let Some(version) = page_load_handle
                        .state::<UpdateState>()
                        .pending
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|u| u.version.clone())
                    {
                        let v = version.replace('"', "\\\"");
                        let _ = win.eval(&format!(
                            r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
                        ));
                    }
                }
            })
            .build()?;

            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::{PermissionRequestExt, WebViewExt};
                let _ = window.with_webview(|wv| {
                    wv.inner().connect_permission_request(|_, req| {
                        req.allow();
                        true
                    });
                });
            }

            let handle = app.handle().clone();
            let resource_dir = app.path().resource_dir().expect("resource dir");
            let app_data_dir = app.path().app_data_dir().expect("app data dir");

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

                // bootstrap.py discovers or installs hermes-agent, then execs
                // into server.py with the correct env set up.
                //
                // Augment PATH so shutil.which("hermes") and shutil.which("uv")
                // find binaries installed by uv tool / cargo / pip even when
                // the desktop app is launched outside a login shell.
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_default();
                let existing_path = std::env::var("PATH").unwrap_or_default();
                let augmented_path = format!(
                    "{home}/.local/bin:{home}/.hermes/bin:{home}/.cargo/bin:{existing_path}"
                );

                // Prefer the hermes-agent's own Python (uv tool install or venv) over
                // the webui venv — it already has run_agent + all agent deps installed.
                // Read the shebang of the `hermes` CLI to find it; fall back to the
                // webui venv Python if the hermes binary isn't found.
                let hermes_python: Option<String> = (|| {
                    let candidates = [
                        format!("{home}/.local/bin/hermes"),
                        format!("{home}/.hermes/bin/hermes"),
                    ];
                    for path in &candidates {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            if let Some(first) = content.lines().next() {
                                if let Some(interp) = first.strip_prefix("#!") {
                                    let interp = interp.trim().split_whitespace().next().unwrap_or("").to_string();
                                    if !interp.is_empty() && std::path::Path::new(&interp).exists() {
                                        return Some(interp);
                                    }
                                }
                            }
                        }
                    }
                    None
                })();
                // Strip \\?\ prefix from the resolved Python path so bootstrap.py
                // and subprocess calls receive a plain Win32 path, not a UNC path.
                let webui_python = {
                    let raw = hermes_python
                        .as_deref()
                        .unwrap_or_else(|| python.to_str().unwrap())
                        .to_string();
                    if raw.starts_with(r"\\?\") { raw[4..].to_string() } else { raw }
                };

                let spawn_result = if cfg!(windows) {
                    handle
                        .shell()
                        .command(&webui_python)
                        .args([
                            bootstrap_py.to_str().unwrap(),
                            "--foreground",
                            "--skip-agent-install",
                        ])
                        .env("HERMES_WEBUI_HOST", SERVER_HOST)
                        .env("HERMES_WEBUI_PORT", port.to_string())
                        .env("HERMES_WEBUI_PYTHON", &webui_python)
                        .env("PATH", &augmented_path)
                        .env("PYTHONDONTWRITEBYTECODE", "1")
                        .env("HERMES_DESKTOP", "1")
                        .current_dir(&resource_dir)
                        .spawn()
                } else {
                    handle
                        .shell()
                        .command("sh")
                        .arg("-c")
                        .arg(format!(
                            "unset PYTHONHOME PYTHONPATH; exec '{}' '{}' --foreground",
                            &webui_python,
                            bootstrap_py.display()
                        ))
                        .env("HERMES_WEBUI_HOST", SERVER_HOST)
                        .env("HERMES_WEBUI_PORT", port.to_string())
                        .env("HERMES_WEBUI_PYTHON", &webui_python)
                        .env("PATH", &augmented_path)
                        .env("PYTHONDONTWRITEBYTECODE", "1")
                        .env("HERMES_DESKTOP", "1")
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
                                        if let Some(win) = handle_log.get_webview_window("main") {
                                            let _ = win.eval(&format!(
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
                    if let Some(win) = handle.get_webview_window("main") {
                        let _ = win.eval(&format!("window.location='http://{SERVER_HOST}:{port}'"));
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
