# Pinned Links — Design Spec

**Date:** 2026-05-12  
**Status:** Approved

## Problem

The suneye team embeds additional local web tools (Hub, prototypes) alongside Hermes. Standard Hermes users should not feel confused by these extras. The solution must be opt-in, discoverable, and non-intrusive.

## Feature: Pinned Links Strip

A slim (36px) icon strip on the left edge of the main window. Users pin any local URL to it. Clicking a pin opens that URL in a new native app window (no browser chrome). Standard users who never add pins see only a "+" button — minimal, self-explanatory.

## Architecture

The main Tauri window hosts two webviews side by side inside a single `Window`:

```
┌──────────────────────────────────────┐
│ strip-webview (36px) │ hermes-webview │
│   desktop/strip.html │  localhost:PORT │
└──────────────────────────────────────┘
```

- **`strip-webview`** — loads `desktop/strip.html`, fully owned by this repo
- **`hermes-webview`** — loads Hermes as today, zero changes to its DOM

When a pin is clicked, the strip calls `invoke('open_pin', { url, label })`. Rust opens a `WebviewWindowBuilder` for that URL. If a window with that URL is already open, it is focused instead of creating a duplicate.

## Components

| File | Role |
|------|------|
| `desktop/strip.html` | Pin strip UI: icon buttons, "+" button, add-pin popover |
| `src-tauri/src/pins.rs` | Commands: `list_pins`, `add_pin`, `remove_pin`, `open_pin` |
| `src-tauri/src/lib.rs` | Window init: create two webviews, load pins on startup |
| `[app_data]/pins.json` | Persisted pin list (Tauri app data dir, never committed) |

## Data Model

```json
[
  { "id": "550e8400-...", "label": "Hub", "url": "http://localhost:1111" },
  { "id": "6ba7b810-...", "label": "Prototypes", "url": "http://localhost:7373" }
]
```

Stored via `tauri::api::path::app_data_dir()`. Survives app restarts.

## User Flows

### Add a pin
1. Click "+" in strip → inline popover appears (no new window)
2. Enter URL + label → confirm
3. Rust appends to `pins.json`, emits `pins-updated` event to strip webview
4. Strip re-renders with new icon

### Remove a pin
Right-click a pin icon → context menu → "Remove"

### Open a pin
Click icon → `invoke('open_pin', { url, label })` → Rust checks open windows → focus existing or open new `WebviewWindowBuilder`

### Empty state
Strip always visible. With no pins: shows only "+" button. Zero visual noise for standard users.

## Error Handling

- URL unreachable when opened: new window shows the URL's own error page (no special handling needed — it's a webview)
- `pins.json` corrupt/missing on startup: treat as empty list, log warning, do not crash

## Out of Scope

- Icon customization per pin (use first letter of label as fallback)
- Pin reordering (drag-and-drop) — add later if needed
- Cloud sync of pins
