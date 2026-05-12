use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

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
