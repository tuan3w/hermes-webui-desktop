# Pinned Links Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a 36px icon strip to the left of the Hermes window where users can pin local URLs and open them in native app windows (no browser chrome).

**Architecture:** A second `Webview` ("strip") is added to the main `Window` alongside the Hermes webview ("hermes") using Tauri v2's multi-webview API. Pin data is stored in `[app_data]/pins.json`. Clicking a pin opens a new `WebviewWindowBuilder` window for that URL; if one is already open it is focused instead.

**Tech Stack:** Tauri v2, Rust, vanilla HTML/JS (strip.html), serde_json (already a dependency)

---

## File Map

| Action | File |
|--------|------|
| Create | `desktop/strip.html` |
| Create | `src-tauri/src/pins.rs` |
| Modify | `src-tauri/src/lib.rs` |
| Modify | `src-tauri/capabilities/default.json` |
| Create | `src-tauri/capabilities/strip.json` |

---

## Task 1: Create `desktop/strip.html`

The pin strip UI. Always-visible 36px column on the left. Shows pin icons (first letter of label) and a "+" button at the bottom. Clicking "+" opens an inline popover to add URL + label. Right-clicking a pin shows "Remove". Communicates with Rust via `window.__TAURI__.core.invoke()`.

**Files:**
- Create: `desktop/strip.html`

- [ ] **Step 1: Write the HTML file**

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }

  body {
    width: 36px;
    height: 100vh;
    background: #111;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px 0;
    gap: 6px;
    overflow: hidden;
    border-right: 1px solid #222;
    user-select: none;
  }

  .pin {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: #2a2a3a;
    color: #8ab4f8;
    font-size: 11px;
    font-weight: 700;
    font-family: system-ui, sans-serif;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background 0.1s;
  }
  .pin:hover { background: #3a3a5a; border-color: #4a4aaa; }

  .spacer { flex: 1; }

  .add-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: transparent;
    border: 1px dashed #444;
    color: #555;
    font-size: 18px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: border-color 0.1s, color 0.1s;
  }
  .add-btn:hover { border-color: #8ab4f8; color: #8ab4f8; }

  #ctx-menu {
    position: fixed;
    background: #1e1e2e;
    border: 1px solid #444;
    border-radius: 6px;
    padding: 4px 0;
    font-family: system-ui, sans-serif;
    font-size: 12px;
    display: none;
    z-index: 1000;
    min-width: 100px;
  }
  #ctx-menu .item { padding: 6px 12px; cursor: pointer; color: #ccc; }
  #ctx-menu .item:hover { background: #333; }
  #ctx-menu .item.danger { color: #f87171; }

  #popover {
    position: fixed;
    left: 40px;
    bottom: 12px;
    background: #1e1e2e;
    border: 1px solid #444;
    border-radius: 8px;
    padding: 12px;
    display: none;
    z-index: 1000;
    width: 220px;
    font-family: system-ui, sans-serif;
  }
  #popover label { font-size: 11px; color: #888; display: block; margin-bottom: 2px; }
  #popover input {
    width: 100%;
    background: #111;
    border: 1px solid #444;
    border-radius: 4px;
    color: #eee;
    font-size: 12px;
    padding: 5px 8px;
    margin-bottom: 8px;
    outline: none;
  }
  #popover input:focus { border-color: #4a7aff; }
  #popover .row { display: flex; gap: 6px; }
  #popover button {
    flex: 1; padding: 6px; border-radius: 4px;
    border: none; font-size: 12px; cursor: pointer;
  }
  #popover .confirm { background: #4a7aff; color: #fff; }
  #popover .cancel { background: #333; color: #aaa; }
</style>
</head>
<body>

<div id="pins"></div>
<div class="spacer"></div>
<div class="add-btn" id="add-btn" title="Add pin">+</div>

<div id="ctx-menu">
  <div class="item danger" id="ctx-remove">Remove</div>
</div>

<div id="popover">
  <label>URL</label>
  <input id="pop-url" type="url" placeholder="http://localhost:1111">
  <label>Label</label>
  <input id="pop-label" type="text" placeholder="Hub">
  <div class="row">
    <button class="cancel" id="pop-cancel">Cancel</button>
    <button class="confirm" id="pop-confirm">Add</button>
  </div>
</div>

<script>
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let ctxTarget = null;

async function loadPins() {
  const pins = await invoke('list_pins');
  const container = document.getElementById('pins');
  container.innerHTML = '';
  for (const pin of pins) {
    const el = document.createElement('div');
    el.className = 'pin';
    el.textContent = pin.label.charAt(0).toUpperCase();
    el.title = pin.label + '\n' + pin.url;
    el.addEventListener('click', () => invoke('open_pin', { url: pin.url, label: pin.label }));
    el.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      ctxTarget = pin.url;
      const menu = document.getElementById('ctx-menu');
      menu.style.display = 'block';
      menu.style.left = '40px';
      menu.style.top = Math.min(e.clientY, window.innerHeight - 60) + 'px';
    });
    container.appendChild(el);
  }
}

document.addEventListener('click', () => {
  document.getElementById('ctx-menu').style.display = 'none';
});

document.getElementById('ctx-remove').addEventListener('click', async () => {
  if (!ctxTarget) return;
  await invoke('remove_pin', { url: ctxTarget });
  ctxTarget = null;
});

document.getElementById('add-btn').addEventListener('click', (e) => {
  e.stopPropagation();
  const pop = document.getElementById('popover');
  pop.style.display = pop.style.display === 'block' ? 'none' : 'block';
  if (pop.style.display === 'block') document.getElementById('pop-url').focus();
});

document.getElementById('pop-cancel').addEventListener('click', () => {
  document.getElementById('popover').style.display = 'none';
  document.getElementById('pop-url').value = '';
  document.getElementById('pop-label').value = '';
});

document.getElementById('pop-confirm').addEventListener('click', async () => {
  const url = document.getElementById('pop-url').value.trim();
  const rawLabel = document.getElementById('pop-label').value.trim();
  if (!url) return;
  const label = rawLabel || (() => { try { return new URL(url).hostname; } catch { return url; } })();
  await invoke('add_pin', { url, label });
  document.getElementById('popover').style.display = 'none';
  document.getElementById('pop-url').value = '';
  document.getElementById('pop-label').value = '';
});

document.getElementById('pop-url').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') document.getElementById('pop-label').focus();
});
document.getElementById('pop-label').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') document.getElementById('pop-confirm').click();
});

listen('pins-updated', loadPins);
loadPins();
</script>
</body>
</html>
```

- [ ] **Step 2: Verify strip.html in browser**

Open `desktop/strip.html` directly in a browser. Confirm:
- Dark 36px-wide column renders
- "+" button visible at bottom
- Clicking "+" shows the popover (URL + label form)
- (Invoke calls will fail — that's expected in browser, not in Tauri)

- [ ] **Step 3: Commit**

```bash
git add desktop/strip.html
git commit -m "feat(strip): add pin strip HTML/JS"
```

---

## Task 2: Create `src-tauri/src/pins.rs` with data layer and tests

**Files:**
- Create: `src-tauri/src/pins.rs`
- Modify: `src-tauri/Cargo.toml` (add `tempfile` dev-dep)
- Modify: `src-tauri/src/lib.rs` (add `mod pins;`)

- [ ] **Step 1: Write `pins.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub url: String,
    pub label: String,
}

pub fn read_pins(pins_path: &Path) -> Vec<Pin> {
    match std::fs::read_to_string(pins_path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("[pins] corrupt pins.json, resetting: {e}");
            Vec::new()
        }),
        Err(_) => Vec::new(),
    }
}

pub fn write_pins(pins_path: &Path, pins: &[Pin]) -> Result<(), String> {
    if let Some(parent) = pins_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(pins).map_err(|e| e.to_string())?;
    std::fs::write(pins_path, json).map_err(|e| e.to_string())
}

pub fn add_pin_to_list(pins: &mut Vec<Pin>, url: String, label: String) {
    if pins.iter().any(|p| p.url == url) {
        return;
    }
    pins.push(Pin { url, label });
}

pub fn remove_pin_from_list(pins: &mut Vec<Pin>, url: &str) {
    pins.retain(|p| p.url != url);
}

// ── State & Commands ──────────────────────────────────────────────────────────

pub struct PinsState {
    pub path: PathBuf,
    pub pins: Mutex<Vec<Pin>>,
}

impl PinsState {
    pub fn load(path: PathBuf) -> Self {
        let pins = read_pins(&path);
        Self { path, pins: Mutex::new(pins) }
    }
}

fn sanitize_label(url: &str) -> String {
    url.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(32)
        .collect::<String>()
        .to_lowercase()
}

#[tauri::command]
pub fn list_pins(state: tauri::State<'_, PinsState>) -> Vec<Pin> {
    state.pins.lock().unwrap().clone()
}

#[tauri::command]
pub fn add_pin(
    state: tauri::State<'_, PinsState>,
    handle: tauri::AppHandle,
    url: String,
    label: String,
) -> Result<(), String> {
    let mut pins = state.pins.lock().unwrap();
    add_pin_to_list(&mut pins, url, label);
    write_pins(&state.path, &pins)?;
    drop(pins);
    let _ = handle.emit("pins-updated", ());
    Ok(())
}

#[tauri::command]
pub fn remove_pin(
    state: tauri::State<'_, PinsState>,
    handle: tauri::AppHandle,
    url: String,
) -> Result<(), String> {
    let mut pins = state.pins.lock().unwrap();
    remove_pin_from_list(&mut pins, &url);
    write_pins(&state.path, &pins)?;
    drop(pins);
    let _ = handle.emit("pins-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn open_pin(
    handle: tauri::AppHandle,
    url: String,
    label: String,
) -> Result<(), String> {
    let win_label = sanitize_label(&url);
    if let Some(existing) = handle.get_webview_window(&win_label) {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    let parsed = url
        .parse::<tauri::Url>()
        .map_err(|e| format!("Invalid URL: {e}"))?;
    tauri::WebviewWindowBuilder::new(
        &handle,
        win_label,
        tauri::WebviewUrl::External(parsed),
    )
    .title(label)
    .inner_size(1200.0, 800.0)
    .resizable(true)
    .build()
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pins.json");
        (dir, path)
    }

    #[test]
    fn read_missing_file_returns_empty() {
        let (_dir, path) = tmp();
        assert!(read_pins(&path).is_empty());
    }

    #[test]
    fn write_then_read_roundtrip() {
        let (_dir, path) = tmp();
        let pins = vec![Pin { url: "http://localhost:1111".into(), label: "Hub".into() }];
        write_pins(&path, &pins).unwrap();
        assert_eq!(read_pins(&path), pins);
    }

    #[test]
    fn add_pin_appends() {
        let mut pins = vec![];
        add_pin_to_list(&mut pins, "http://localhost:1111".into(), "Hub".into());
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].label, "Hub");
    }

    #[test]
    fn add_pin_deduplicates_by_url() {
        let mut pins = vec![];
        add_pin_to_list(&mut pins, "http://localhost:1111".into(), "Hub".into());
        add_pin_to_list(&mut pins, "http://localhost:1111".into(), "Hub2".into());
        assert_eq!(pins.len(), 1);
    }

    #[test]
    fn remove_pin_removes_by_url() {
        let mut pins = vec![
            Pin { url: "http://localhost:1111".into(), label: "Hub".into() },
            Pin { url: "http://localhost:7373".into(), label: "Proto".into() },
        ];
        remove_pin_from_list(&mut pins, "http://localhost:1111");
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].url, "http://localhost:7373");
    }

    #[test]
    fn read_corrupt_json_returns_empty() {
        let (_dir, path) = tmp();
        fs::write(&path, b"not json").unwrap();
        assert!(read_pins(&path).is_empty());
    }
}
```

- [ ] **Step 2: Add `tempfile` dev-dependency**

In `src-tauri/Cargo.toml`, add after `[dependencies]` block:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Declare the module in `lib.rs`**

Add `mod pins;` near the top of `src-tauri/src/lib.rs`, after the existing `use` lines:

```rust
mod pins;
```

- [ ] **Step 4: Run unit tests**

```bash
cd src-tauri && cargo test pins
```

Expected:
```
test pins::tests::add_pin_appends ... ok
test pins::tests::add_pin_deduplicates_by_url ... ok
test pins::tests::read_corrupt_json_returns_empty ... ok
test pins::tests::read_missing_file_returns_empty ... ok
test pins::tests::remove_pin_removes_by_url ... ok
test pins::tests::write_then_read_roundtrip ... ok

test result: ok. 6 passed; 0 failed
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/pins.rs src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "feat(pins): add pin data layer with unit tests"
```

---

## Task 3: Refactor `lib.rs` — multi-webview window + register pins commands

Switch from `WebviewWindowBuilder` to `WindowBuilder + add_child()` so strip and Hermes share one window. Update all helper functions to use the correct window/webview handles. Register pins commands.

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add imports at top of `lib.rs`**

After the existing `use tauri::Manager;` line, add:

```rust
use tauri::window::WindowBuilder;
use tauri::webview::WebviewBuilder;
```

- [ ] **Step 2: Update `show_window` — use `Window` instead of `WebviewWindow`**

Replace:
```rust
fn show_window(handle: &tauri::AppHandle) {
    if let Some(win) = handle.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.unminimize();
    }
}
```

With:
```rust
fn show_window(handle: &tauri::AppHandle) {
    if let Some(win) = handle.get_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.unminimize();
    }
}
```

- [ ] **Step 3: Update `set_status` and `show_error` — use `get_webview("hermes")`**

Replace both helper functions:

```rust
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
```

- [ ] **Step 4: Update `open_devtools` command — use `AppHandle`**

Replace:
```rust
#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) {
    window.open_devtools();
}
```

With:
```rust
#[tauri::command]
fn open_devtools(handle: tauri::AppHandle) {
    if let Some(wv) = handle.get_webview("hermes") {
        wv.open_devtools();
    }
}
```

- [ ] **Step 5: Update `background_check_update` — use `get_webview("hermes")`**

Inside `background_check_update`, replace:
```rust
if let Some(win) = handle.get_webview_window("main") {
    let v = version.replace('"', "\\\"");
    let _ = win.eval(&format!(
        r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
    ));
}
```

With:
```rust
if let Some(wv) = handle.get_webview("hermes") {
    let v = version.replace('"', "\\\"");
    let _ = wv.eval(&format!(
        r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
    ));
}
```

- [ ] **Step 6: Update `check_update` tray handler — use `get_webview("hermes")`**

Inside `build_tray`, in the `"check_update"` match arm, replace:
```rust
if let Some(win) = h.get_webview_window("main") {
    show_window(&h);
    let v = version.replace('"', "\\\"");
    let _ = win.eval(&format!(
        r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
    ));
}
```

With:
```rust
if let Some(wv) = h.get_webview("hermes") {
    show_window(&h);
    let v = version.replace('"', "\\\"");
    let _ = wv.eval(&format!(
        r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
    ));
}
```

- [ ] **Step 7: Replace window creation in `setup` closure**

Replace the entire `WebviewWindowBuilder` block and the `#[cfg(target_os = "linux")]` block that follows it:

```rust
// OLD — remove this entire block:
let window = tauri::WebviewWindowBuilder::new(
    app,
    "main",
    tauri::WebviewUrl::App("/".into()),
)
.title("Hermes")
.inner_size(1400.0, 900.0)
.min_inner_size(900.0, 600.0)
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
```

With:

```rust
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
```

- [ ] **Step 8: Register `PinsState` and pins commands**

After `let app_data_dir = app.path().app_data_dir().expect("app data dir");`, add:

```rust
let pins_path = app_data_dir.join("pins.json");
app.manage(pins::PinsState::load(pins_path));
```

In `.invoke_handler(...)`, replace:
```rust
.invoke_handler(tauri::generate_handler![open_devtools, check_update, install_update])
```

With:
```rust
.invoke_handler(tauri::generate_handler![
    open_devtools,
    check_update,
    install_update,
    pins::list_pins,
    pins::add_pin,
    pins::remove_pin,
    pins::open_pin,
])
```

- [ ] **Step 9: Update server-ready navigation — use `get_webview("hermes")`**

Find:
```rust
if let Some(win) = handle.get_webview_window("main") {
    let _ = win.eval(&format!("window.location='http://{SERVER_HOST}:{port}'"));
}
```

Replace with:
```rust
if let Some(wv) = handle.get_webview("hermes") {
    let _ = wv.eval(&format!("window.location='http://{SERVER_HOST}:{port}'"));
}
```

Also find the log-streaming eval:
```rust
if let Some(win) = handle_log.get_webview_window("main") {
    let _ = win.eval(&format!(
        r#"window.__hermesAppendLog && window.__hermesAppendLog("{esc}", false)"#
    ));
}
```

Replace with:
```rust
if let Some(wv) = handle_log.get_webview("hermes") {
    let _ = wv.eval(&format!(
        r#"window.__hermesAppendLog && window.__hermesAppendLog("{esc}", false)"#
    ));
}
```

- [ ] **Step 10: Add resize handler in `on_window_event`**

In the `.on_window_event(|window, event| match event { ... })` block, add a `Resized` arm between `CloseRequested` and `Destroyed`:

```rust
tauri::WindowEvent::Resized(physical_size) => {
    let h = window.app_handle();
    let phy_w = physical_size.width;
    let phy_h = physical_size.height;
    if let Some(strip) = h.get_webview("strip") {
        let _ = strip.set_bounds(tauri::Rect {
            position: tauri::Position::Physical(tauri::PhysicalPosition::new(0, 0)),
            size: tauri::Size::Physical(tauri::PhysicalSize::new(36, phy_h)),
        });
    }
    if let Some(hermes_wv) = h.get_webview("hermes") {
        let _ = hermes_wv.set_bounds(tauri::Rect {
            position: tauri::Position::Physical(tauri::PhysicalPosition::new(36, 0)),
            size: tauri::Size::Physical(tauri::PhysicalSize::new(
                phy_w.saturating_sub(36),
                phy_h,
            )),
        });
    }
}
```

- [ ] **Step 11: Build**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error"
```

Expected: no output (zero compile errors).

- [ ] **Step 12: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(strip): multi-webview layout + register pins commands"
```

---

## Task 4: Add capabilities for strip and hermes webviews

**Files:**
- Create: `src-tauri/capabilities/strip.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Create `strip.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "strip",
  "description": "Capabilities for the pin strip webview",
  "local": true,
  "windows": ["strip"],
  "permissions": [
    "core:default"
  ]
}
```

- [ ] **Step 2: Update `default.json` — cover `hermes` instead of `main`**

In `src-tauri/capabilities/default.json`, change `"windows": ["main"]` to `"windows": ["hermes"]`.

- [ ] **Step 3: Run the app and smoke-test**

```bash
cd src-tauri && cargo tauri dev
```

Verify:
1. Window opens with a 36px dark strip on the left
2. Splash screen loads on the right (Hermes startup logs appear)
3. After Hermes starts, the right pane shows the Hermes UI
4. "+" button is visible at the bottom of the strip
5. Clicking "+" opens the add-pin popover
6. Add `http://localhost:1111` with label `Hub` → pin icon "H" appears
7. Click "H" → new window opens for `http://localhost:1111` (or shows error if not running — that's fine)
8. Click "H" again → existing window is focused, not a second window
9. Right-click "H" → "Remove" option → removes the pin
10. Window resize → strip stays 36px, Hermes fills the rest

- [ ] **Step 4: Commit**

```bash
git add src-tauri/capabilities/strip.json src-tauri/capabilities/default.json
git commit -m "feat(strip): add capabilities for strip and hermes webviews"
```
