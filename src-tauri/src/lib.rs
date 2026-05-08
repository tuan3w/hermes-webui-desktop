use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::time::Duration;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

const SERVER_HOST: &str = "127.0.0.1";

struct ServerState {
    child: Mutex<Option<tauri_plugin_shell::process::CommandChild>>,
}

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

fn find_python() -> String {
    if let Ok(p) = std::env::var("HERMES_WEBUI_PYTHON") {
        if !p.is_empty() {
            return p;
        }
    }
    "python3".to_string()
}

fn show_error(handle: &tauri::AppHandle, msg: &str) {
    let escaped = msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    if let Some(win) = handle.get_webview_window("main") {
        let _ = win.eval(&format!(
            r#"if(window.__hermesShowError){{window.__hermesShowError("{escaped}")}}"#
        ));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(ServerState {
            child: Mutex::new(None),
        })
        .setup(|app| {
            // Show splash immediately.
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("/".into()),
            )
            .title("Hermes")
            .inner_size(1400.0, 900.0)
            .min_inner_size(900.0, 600.0)
            .build()?;

            // Grant microphone (and all other media) permission requests automatically.
            #[cfg(target_os = "linux")]
            {
                use webkit2gtk::{PermissionRequestExt, WebViewExt};
                let _ = window.with_webview(|webview| {
                    webview.inner().connect_permission_request(|_, request| {
                        request.allow();
                        true
                    });
                });
            }

            let handle = app.handle().clone();
            let resource_dir = app
                .path()
                .resource_dir()
                .expect("could not resolve resource dir");

            tauri::async_runtime::spawn(async move {
                let port = free_port();
                let port_str = port.to_string();
                let server_py = resource_dir.join("server.py");
                let python = find_python();

                eprintln!("[hermes] resource_dir = {}", resource_dir.display());
                eprintln!("[hermes] server_py    = {}", server_py.display());
                eprintln!("[hermes] python       = {python}");
                eprintln!("[hermes] port         = {port}");

                if !server_py.exists() {
                    let msg = format!(
                        "server.py not found:\n{}",
                        server_py.display()
                    );
                    eprintln!("[hermes] ERROR: {msg}");
                    std::thread::sleep(Duration::from_millis(800));
                    show_error(&handle, &msg);
                    return;
                }

                // The AppImage runtime injects PYTHONHOME/PYTHONPATH pointing at its
                // own (incomplete) bundled Python, which breaks the system python3.
                // Use a bash wrapper to unset them before exec-ing python3.
                let bash_cmd = format!(
                    "unset PYTHONHOME PYTHONPATH; exec '{}' '{}'",
                    python,
                    server_py.display()
                );
                let spawn_result = handle
                    .shell()
                    .command("bash")
                    .args(["--noprofile", "--norc", "-c", &bash_cmd])
                    .env("HERMES_WEBUI_HOST", SERVER_HOST)
                    .env("HERMES_WEBUI_PORT", &port_str)
                    .env("PYTHONDONTWRITEBYTECODE", "1")
                    .current_dir(&resource_dir)
                    .spawn();

                let (mut rx, child) = match spawn_result {
                    Ok(v) => v,
                    Err(e) => {
                        let msg = format!("Could not launch {python}: {e}");
                        eprintln!("[hermes] ERROR: {msg}");
                        std::thread::sleep(Duration::from_millis(800));
                        show_error(&handle, &msg);
                        return;
                    }
                };

                *handle.state::<ServerState>().child.lock().unwrap() = Some(child);

                // Drain server output concurrently so errors appear in real time.
                tauri::async_runtime::spawn(async move {
                    use tauri_plugin_shell::process::CommandEvent;
                    while let Some(event) = rx.recv().await {
                        match event {
                            CommandEvent::Stdout(b) | CommandEvent::Stderr(b) => {
                                if let Ok(line) = std::str::from_utf8(&b) {
                                    eprint!("[server] {}", line);
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

                // Wait for the server to accept connections (blocking, in thread pool).
                let ready = tauri::async_runtime::spawn_blocking(move || wait_for_server(port))
                    .await
                    .unwrap_or(false);

                if ready {
                    eprintln!("[hermes] server ready on port {port}");
                    if let Some(win) = handle.get_webview_window("main") {
                        let url = format!("http://{}:{}", SERVER_HOST, port);
                        let _ = win.eval(&format!("window.location='{url}'"));
                    }
                } else {
                    let msg = format!(
                        "Server did not respond on :{port} within 60 s.\n\
                         Run from terminal to see full error output.\n\
                         Make sure python3 has pyyaml:\n\
                         python3 -m pip install pyyaml"
                    );
                    eprintln!("[hermes] ERROR: {msg}");
                    show_error(&handle, &msg);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let handle = window.app_handle().clone();
                let child = {
                    let state = handle.state::<ServerState>();
                    let c = state.child.lock().unwrap().take();
                    c
                };
                if let Some(c) = child {
                    let _ = c.kill();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
