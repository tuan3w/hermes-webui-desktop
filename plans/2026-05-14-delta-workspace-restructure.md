# Delta Workspace Restructure — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hermes-webui wrapper with a proper Delta workspace UI — React+Vite frontend, Tauri commands as API gateway, hermes-webui fork as headless agent backend.

**Architecture:** Tauri shell spawns the Python server as before, but instead of navigating the webview to hermes-webui's chat UI, it emits a `server:ready` event to the React app. The React app handles its own loading state and calls `http://127.0.0.1:{port}/api/*` for all agent operations. Tauri commands handle filesystem I/O and (later) OS keychain.

**Tech Stack:** Tauri v2, Rust, React 18, Vite 5, TypeScript, Zustand, `@tauri-apps/api` v2

**Spec:** `specs/2026-05-14-delta-workspace-restructure-design.md`

---

## Phase 1 — Foundation

Replaces all `win.eval` JS injections with Tauri events. Updates Tauri config to point at the React app. After this phase the app boots, shows a React loading screen, and transitions to a placeholder workspace shell.

---

### Task 1: Fork hermes-webui

**Files:**
- Modify: `.gitmodules`
- Modify: `Makefile` (update `update` target)

- [ ] **Step 1: Fork the repo**

  Go to `https://github.com/nesquena/hermes-webui` → Fork → create `<your-org>/hermes-webui`.

  *(This is a manual step in the GitHub UI. Record the new URL before continuing.)*

- [ ] **Step 2: Update the submodule to point at the fork**

  ```bash
  git submodule set-url hermes-webui https://github.com/<your-org>/hermes-webui.git
  git submodule sync
  git submodule update --init
  ```

  Expected: no errors, `hermes-webui/` directory unchanged.

- [ ] **Step 3: Update Makefile — remove upstream sync, point at fork**

  Change the `update sync-upstream` target in `Makefile` from:
  ```makefile
  update sync-upstream:
  	git submodule update --remote --merge hermes-webui
  	@if git diff --quiet hermes-webui; then \
  	  echo "Already up to date."; \
  	else \
  	  NEW_SHA=$$(git -C hermes-webui rev-parse --short HEAD); \
  	  git add hermes-webui; \
  	  git commit -m "chore: bump hermes-webui to $${NEW_SHA}"; \
  	  echo "Bumped hermes-webui to $${NEW_SHA}"; \
  	fi
  ```
  To:
  ```makefile
  update:
  	git submodule update --remote --merge hermes-webui
  	@if git diff --quiet hermes-webui; then \
  	  echo "Already up to date."; \
  	else \
  	  NEW_SHA=$$(git -C hermes-webui rev-parse --short HEAD); \
  	  git add hermes-webui; \
  	  git commit -m "chore: bump hermes-webui to $${NEW_SHA}"; \
  	  echo "Bumped hermes-webui to $${NEW_SHA}"; \
  	fi
  ```

  Also remove `sync-upstream` from the `.PHONY` line.

- [ ] **Step 4: Verify**

  ```bash
  git submodule status
  ```
  Expected: shows commit hash for `hermes-webui` pointing at your fork.

- [ ] **Step 5: Commit**

  ```bash
  git add .gitmodules Makefile
  git commit -m "chore: fork hermes-webui — drop upstream auto-sync"
  ```

---

### Task 2: Scaffold React + Vite app

**Files:**
- Create: `ui/package.json`
- Create: `ui/vite.config.ts`
- Create: `ui/tsconfig.json`
- Create: `ui/index.html`
- Create: `ui/src/main.tsx`
- Create: `ui/src/App.tsx`
- Create: `ui/src/app.css`

- [ ] **Step 1: Create `ui/package.json`**

  ```json
  {
    "name": "delta-ui",
    "version": "0.1.0",
    "private": true,
    "scripts": {
      "dev": "vite",
      "build": "tsc && vite build",
      "preview": "vite preview",
      "test": "vitest run",
      "test:watch": "vitest"
    },
    "dependencies": {
      "@tauri-apps/api": "^2.0.0",
      "react": "^18.3.1",
      "react-dom": "^18.3.1",
      "react-markdown": "^9.0.0",
      "zustand": "^5.0.0"
    },
    "devDependencies": {
      "@testing-library/jest-dom": "^6.4.0",
      "@testing-library/react": "^16.0.0",
      "@types/react": "^18.3.1",
      "@types/react-dom": "^18.3.1",
      "@vitejs/plugin-react": "^4.3.0",
      "jsdom": "^24.0.0",
      "typescript": "^5.5.0",
      "vite": "^5.4.0",
      "vitest": "^2.0.0"
    }
  }
  ```

- [ ] **Step 2: Create `ui/vite.config.ts`**

  ```ts
  import { defineConfig } from 'vite'
  import react from '@vitejs/plugin-react'

  export default defineConfig({
    plugins: [react()],
    clearScreen: false,
    server: {
      port: 5173,
      strictPort: true,
    },
    envPrefix: ['VITE_', 'TAURI_'],
    build: {
      outDir: 'dist',
      target: 'chrome105',
      minify: false,
    },
    test: {
      environment: 'jsdom',
      setupFiles: ['./src/test-setup.ts'],
    },
  })
  ```

- [ ] **Step 3: Create `ui/tsconfig.json`**

  ```json
  {
    "compilerOptions": {
      "target": "ES2020",
      "useDefineForClassFields": true,
      "lib": ["ES2020", "DOM", "DOM.Iterable"],
      "module": "ESNext",
      "skipLibCheck": true,
      "moduleResolution": "bundler",
      "allowImportingTsExtensions": true,
      "resolveJsonModule": true,
      "isolatedModules": true,
      "noEmit": true,
      "jsx": "react-jsx",
      "strict": true,
      "noUnusedLocals": true,
      "noUnusedParameters": true,
      "noFallthroughCasesInSwitch": true
    },
    "include": ["src"]
  }
  ```

- [ ] **Step 4: Create `ui/index.html`**

  ```html
  <!DOCTYPE html>
  <html lang="en">
    <head>
      <meta charset="UTF-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1.0" />
      <title>Delta</title>
    </head>
    <body>
      <div id="root"></div>
      <script type="module" src="/src/main.tsx"></script>
    </body>
  </html>
  ```

- [ ] **Step 5: Create `ui/src/main.tsx`**

  ```tsx
  import React from 'react'
  import ReactDOM from 'react-dom/client'
  import App from './App'
  import './app.css'

  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  )
  ```

- [ ] **Step 6: Create `ui/src/App.tsx`** (placeholder — replaced in Task 5)

  ```tsx
  export default function App() {
    return (
      <div style={{ color: '#e8e8ed', background: '#0f1011', height: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: 'system-ui' }}>
        <p>Delta — scaffold OK</p>
      </div>
    )
  }
  ```

- [ ] **Step 7: Create `ui/src/app.css`**

  ```css
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  html, body, #root { height: 100%; }
  body {
    background: #0f1011;
    color: #e8e8ed;
    font-family: -apple-system, BlinkMacSystemFont, "Inter", "Segoe UI", sans-serif;
    font-size: 13px;
    -webkit-font-smoothing: antialiased;
    overflow: hidden;
  }
  ```

- [ ] **Step 8: Create `ui/src/test-setup.ts`**

  ```ts
  import '@testing-library/jest-dom'
  ```

- [ ] **Step 9: Install dependencies**

  ```bash
  cd ui && npm install
  ```

  Expected: `node_modules/` created, no errors.

- [ ] **Step 10: Verify dev server starts**

  ```bash
  cd ui && npm run dev
  ```

  Expected: `http://localhost:5173` serves the placeholder page.  
  Stop with Ctrl+C.

- [ ] **Step 11: Commit**

  ```bash
  git add ui/
  git commit -m "feat(ui): scaffold React+Vite workspace app"
  ```

---

### Task 3: Update Tauri config and Makefile for React app

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `Makefile`

- [ ] **Step 1: Update `tauri.conf.json`**

  Change the `build` section from:
  ```json
  "build": {
    "frontendDist": "../desktop",
    "devUrl": "http://localhost:1421"
  }
  ```
  To:
  ```json
  "build": {
    "frontendDist": "../ui/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "cd ../ui && npm run dev",
    "beforeBuildCommand": "cd ../ui && npm run build"
  }
  ```

- [ ] **Step 2: Update Makefile dev targets**

  Replace the `dev` and `dev-ui` targets:
  ```makefile
  dev: _ensure-tauri
  	cd $(SRC_TAURI) && cargo tauri dev

  dev-ui:
  	@echo "Serving React workspace at http://localhost:5173"
  	cd ui && npm run dev
  ```

  Remove the old `python3 -m http.server 1421` line entirely.

- [ ] **Step 3: Verify tauri dev starts without error**

  ```bash
  make dev
  ```

  Expected: Vite starts on :5173, Tauri window opens showing "Delta — scaffold OK".  
  *(The Python server will fail at this point — that's fine. We're just checking that Tauri loads the React app.)*  
  Stop with Ctrl+C.

- [ ] **Step 4: Commit**

  ```bash
  git add src-tauri/tauri.conf.json Makefile
  git commit -m "feat(tauri): point frontendDist at React app, update dev commands"
  ```

---

### Task 4: Replace win.eval calls with Tauri events in lib.rs

The current `lib.rs` injects JS via `win.eval()` for status updates, errors, log lines, and navigation. These are all replaced with `app_handle.emit()` calls that the React app listens to.

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Events emitted (payload type):**
| Event | Payload | Replaces |
|-------|---------|----------|
| `boot:status` | `String` | `window.__hermesSetStatus(msg)` |
| `boot:error` | `String` | `window.__hermesShowError(msg)` |
| `boot:log` | `String` | `window.__hermesAppendLog(line)` |
| `server:ready` | `u16` (port) | `window.location = 'http://...'` |
| `updater:available` | `String` (version) | `window.__hermesShowUpdate(v)` |

- [ ] **Step 1: Add `tauri::Emitter` import**

  At the top of `src-tauri/src/lib.rs`, change:
  ```rust
  use tauri::Manager;
  ```
  To:
  ```rust
  use tauri::{Emitter, Manager};
  ```

- [ ] **Step 2: Replace `set_status` helper**

  Replace the entire `set_status` function:
  ```rust
  fn set_status(handle: &tauri::AppHandle, msg: &str) {
      let esc = msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
      if let Some(win) = handle.get_webview_window("main") {
          let _ = win.eval(&format!(
              r#"window.__hermesSetStatus && window.__hermesSetStatus("{esc}")"#
          ));
      }
  }
  ```
  With:
  ```rust
  fn set_status(handle: &tauri::AppHandle, msg: &str) {
      let _ = handle.emit("boot:status", msg.to_string());
  }
  ```

- [ ] **Step 3: Replace `show_error` helper**

  Replace the entire `show_error` function:
  ```rust
  fn show_error(handle: &tauri::AppHandle, msg: &str) {
      eprintln!("[hermes] ERROR: {msg}");
      let esc = msg.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
      if let Some(win) = handle.get_webview_window("main") {
          let _ = win.eval(&format!(
              r#"window.__hermesShowError && window.__hermesShowError("{esc}")"#
          ));
      }
  }
  ```
  With:
  ```rust
  fn show_error(handle: &tauri::AppHandle, msg: &str) {
      eprintln!("[delta] ERROR: {msg}");
      let _ = handle.emit("boot:error", msg.to_string());
  }
  ```

- [ ] **Step 4: Replace log emission inside the server output loop**

  Find this block in the `tauri::async_runtime::spawn` that collects server output (around line 443):
  ```rust
  if let Some(win) = handle_log.get_webview_window("main") {
      let _ = win.eval(&format!(
          r#"window.__hermesAppendLog && window.__hermesAppendLog("{esc}", false)"#
      ));
  }
  ```
  Replace with:
  ```rust
  let _ = handle_log.emit("boot:log", trimmed.clone());
  ```

  Also remove the `esc` variable that was defined just above it (it's no longer needed in that block).

- [ ] **Step 5: Replace the server-ready navigation**

  Find this block (around line 466):
  ```rust
  if ready {
      eprintln!("[hermes] server ready on :{port}");
      if let Some(win) = handle.get_webview_window("main") {
          let _ = win.eval(&format!("window.location='http://{SERVER_HOST}:{port}'"));
      }
  ```
  Replace with:
  ```rust
  if ready {
      eprintln!("[delta] server ready on :{port}");
      let _ = handle.emit("server:ready", port);
  ```

- [ ] **Step 6: Replace updater show_update eval**

  In `background_check_update`, find:
  ```rust
  if let Some(win) = handle.get_webview_window("main") {
      let v = version.replace('"', "\\\"");
      let _ = win.eval(&format!(
          r#"window.__hermesShowUpdate && window.__hermesShowUpdate("{v}")"#
      ));
  }
  ```
  Replace with:
  ```rust
  let _ = handle.emit("updater:available", version.clone());
  ```

  In the `check_update` tray handler, find the same pattern and replace it the same way:
  ```rust
  let _ = h.emit("updater:available", version.clone());
  ```
  (Remove the `show_window(&h)` call too — the React app will handle showing the notification without stealing focus.)

- [ ] **Step 7: Verify Rust compiles**

  ```bash
  cd src-tauri && cargo build 2>&1 | tail -5
  ```

  Expected: `Compiling delta...` then `Finished`. No errors.

- [ ] **Step 8: Commit**

  ```bash
  git add src-tauri/src/lib.rs
  git commit -m "feat(tauri): replace win.eval injections with typed Tauri events"
  ```

---

### Task 5: React app — boot event handling + loading/error states

**Files:**
- Create: `ui/src/store/app.ts`
- Create: `ui/src/components/BootScreen.tsx`
- Modify: `ui/src/App.tsx`
- Create: `ui/src/App.test.tsx`

- [ ] **Step 1: Write failing test**

  Create `ui/src/App.test.tsx`:
  ```tsx
  import { render, screen } from '@testing-library/react'
  import { describe, it, expect, vi, beforeEach } from 'vitest'

  // Mock Tauri event API
  vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn(() => Promise.resolve(() => {})),
  }))

  import App from './App'

  describe('App boot states', () => {
    beforeEach(() => { vi.clearAllMocks() })

    it('shows loading screen on initial render', () => {
      render(<App />)
      expect(screen.getByText(/starting delta/i)).toBeInTheDocument()
    })
  })
  ```

- [ ] **Step 2: Run test — expect it to fail**

  ```bash
  cd ui && npm test
  ```

  Expected: FAIL — "Cannot find 'Starting Delta'" (App currently shows scaffold text).

- [ ] **Step 3: Create `ui/src/store/app.ts`**

  ```ts
  import { create } from 'zustand'

  export interface AppState {
    serverPort: number | null
    serverReady: boolean
    workspacePath: string
    setServerReady: (port: number) => void
    setWorkspacePath: (path: string) => void
  }

  export const useAppStore = create<AppState>((set) => ({
    serverPort: null,
    serverReady: false,
    workspacePath: '',
    setServerReady: (port) => set({ serverPort: port, serverReady: true }),
    setWorkspacePath: (path) => set({ workspacePath: path }),
  }))
  ```

- [ ] **Step 4: Create `ui/src/components/BootScreen.tsx`**

  ```tsx
  interface BootScreenProps {
    status: string
    logs: string[]
    error?: string
  }

  export function BootScreen({ status, logs, error }: BootScreenProps) {
    const hasError = !!error
    return (
      <div style={{
        height: '100vh', display: 'flex', flexDirection: 'column',
        alignItems: 'center', justifyContent: 'center', gap: 12,
        background: '#0f1011', color: '#e8e8ed',
      }}>
        <div style={{ fontSize: 22, fontWeight: 700, letterSpacing: '-0.03em' }}>Delta</div>
        {!hasError && (
          <div style={{ fontSize: 12, color: '#8e8e9a' }}>{status}</div>
        )}
        {hasError && (
          <>
            <div style={{ fontSize: 12, color: '#f87171', maxWidth: 480, textAlign: 'center' }}>{error}</div>
            <pre style={{
              fontSize: 11, color: '#52525b', background: '#161618',
              border: '1px solid #2a2a2e', borderRadius: 6, padding: '10px 14px',
              maxWidth: 560, maxHeight: 200, overflow: 'auto', whiteSpace: 'pre-wrap',
            }}>
              {logs.slice(-20).join('\n')}
            </pre>
          </>
        )}
        {!hasError && logs.length > 0 && (
          <pre style={{
            fontSize: 10, color: '#52525b', maxWidth: 480,
            maxHeight: 100, overflow: 'auto', whiteSpace: 'pre-wrap',
          }}>
            {logs.slice(-5).join('\n')}
          </pre>
        )}
      </div>
    )
  }
  ```

- [ ] **Step 5: Replace `ui/src/App.tsx`**

  ```tsx
  import { useEffect, useState } from 'react'
  import { listen } from '@tauri-apps/api/event'
  import { useAppStore } from './store/app'
  import { BootScreen } from './components/BootScreen'

  type BootPhase = 'booting' | 'ready' | 'error'

  export default function App() {
    const [phase, setPhase] = useState<BootPhase>('booting')
    const [status, setStatus] = useState('Starting Delta…')
    const [logs, setLogs] = useState<string[]>([])
    const [errorMsg, setErrorMsg] = useState('')
    const setServerReady = useAppStore(s => s.setServerReady)

    useEffect(() => {
      const unlisteners = [
        listen<string>('boot:status', e => setStatus(e.payload)),
        listen<string>('boot:log', e => setLogs(prev => [...prev, e.payload])),
        listen<string>('boot:error', e => {
          setErrorMsg(e.payload)
          setPhase('error')
        }),
        listen<number>('server:ready', e => {
          setServerReady(e.payload)
          setPhase('ready')
        }),
      ]
      return () => { unlisteners.forEach(p => p.then(fn => fn())) }
    }, [setServerReady])

    if (phase === 'booting') return <BootScreen status={status} logs={logs} />
    if (phase === 'error') return <BootScreen status={status} logs={logs} error={errorMsg} />

    // Phase 3 will replace this placeholder with <Shell />
    return (
      <div style={{ height: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#e8e8ed' }}>
        <p>Workspace — server on :{useAppStore.getState().serverPort}</p>
      </div>
    )
  }
  ```

- [ ] **Step 6: Run tests — expect them to pass**

  ```bash
  cd ui && npm test
  ```

  Expected: PASS.

- [ ] **Step 7: Manual smoke test**

  ```bash
  make dev
  ```

  Expected: Tauri window shows "Delta — Starting Delta…" while Python server boots, then transitions to "Workspace — server on :{port}".

- [ ] **Step 8: Commit**

  ```bash
  git add ui/src/
  git commit -m "feat(ui): boot screen with status/error/log events from Tauri"
  ```

---

## Phase 2 — Workspace Shell

Adds the work item data model, Tauri filesystem commands, and the full app shell (rail + list panel + main area routing). After this phase the app reads real work items from `~/Documents/Delta/` and navigates between them.

---

### Task 6: Work item types + Zustand store

**Files:**
- Create: `ui/src/types/workItem.ts`
- Modify: `ui/src/store/app.ts`
- Create: `ui/src/store/app.test.ts`

- [ ] **Step 1: Write failing test**

  Create `ui/src/store/app.test.ts`:
  ```ts
  import { describe, it, expect, beforeEach } from 'vitest'
  import { useAppStore } from './app'

  describe('app store', () => {
    beforeEach(() => {
      useAppStore.setState({
        serverPort: null, serverReady: false, workspacePath: '',
        workItems: [], selectedItem: null,
      })
    })

    it('selectItem updates selectedItem', () => {
      const item = { id: 'order-management', type: 'spec' as const, path: '/tmp/specs/order-management', title: 'Order Management' }
      useAppStore.getState().selectItem(item)
      expect(useAppStore.getState().selectedItem?.id).toBe('order-management')
    })

    it('setWorkItems replaces the list', () => {
      const items = [
        { id: 'a', type: 'spec' as const, path: '/p/a', title: 'A' },
        { id: 'b', type: 'research' as const, path: '/p/b', title: 'B' },
      ]
      useAppStore.getState().setWorkItems(items)
      expect(useAppStore.getState().workItems).toHaveLength(2)
    })
  })
  ```

- [ ] **Step 2: Run test — expect fail**

  ```bash
  cd ui && npm test
  ```

  Expected: FAIL — `workItems` and `selectItem` not defined on store.

- [ ] **Step 3: Create `ui/src/types/workItem.ts`**

  ```ts
  export type WorkItemType = 'spec' | 'research' | 'pr'

  export interface WorkItem {
    id: string          // directory name, e.g. "order-management"
    type: WorkItemType
    path: string        // absolute path to the item directory
    title: string       // humanised from id
  }

  export interface AgentMessage {
    role: 'user' | 'assistant'
    content: string
    timestamp: number
  }

  export interface AgentThread {
    messages: AgentMessage[]
  }
  ```

- [ ] **Step 4: Update `ui/src/store/app.ts`**

  ```ts
  import { create } from 'zustand'
  import type { WorkItem } from '../types/workItem'

  export interface AppState {
    serverPort: number | null
    serverReady: boolean
    workspacePath: string
    workItems: WorkItem[]
    selectedItem: WorkItem | null
    setServerReady: (port: number) => void
    setWorkspacePath: (path: string) => void
    setWorkItems: (items: WorkItem[]) => void
    selectItem: (item: WorkItem | null) => void
  }

  export const useAppStore = create<AppState>((set) => ({
    serverPort: null,
    serverReady: false,
    workspacePath: '',
    workItems: [],
    selectedItem: null,
    setServerReady: (port) => set({ serverPort: port, serverReady: true }),
    setWorkspacePath: (path) => set({ workspacePath: path }),
    setWorkItems: (items) => set({ workItems: items }),
    selectItem: (item) => set({ selectedItem: item }),
  }))
  ```

- [ ] **Step 5: Run tests — expect pass**

  ```bash
  cd ui && npm test
  ```

  Expected: PASS.

- [ ] **Step 6: Commit**

  ```bash
  git add ui/src/types/ ui/src/store/
  git commit -m "feat(ui): work item types and app store"
  ```

---

### Task 7: Rust filesystem commands

**Files:**
- Create: `src-tauri/src/commands/fs.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add `tempfile` dev-dependency to `src-tauri/Cargo.toml`**

  In `src-tauri/Cargo.toml`, find the `[dev-dependencies]` section (or add it) and add:
  ```toml
  [dev-dependencies]
  tempfile = "3"
  ```

- [ ] **Step 2: Write failing tests**

  Create `src-tauri/src/commands/fs.rs` with tests only first:
  ```rust
  use serde::Serialize;
  use std::path::Path;

  #[derive(Debug, Serialize, Clone)]
  pub struct WorkItem {
      pub id: String,
      pub item_type: String,
      pub path: String,
      pub title: String,
  }

  #[tauri::command]
  pub fn list_work_items(workspace_path: String) -> Result<Vec<WorkItem>, String> {
      todo!()
  }

  #[tauri::command]
  pub fn read_file(path: String) -> Result<String, String> {
      todo!()
  }

  #[tauri::command]
  pub fn write_file(path: String, content: String) -> Result<(), String> {
      todo!()
  }

  #[cfg(test)]
  mod tests {
      use super::*;
      use std::fs;
      use tempfile::TempDir;

      #[test]
      fn list_empty_workspace_returns_empty() {
          let tmp = TempDir::new().unwrap();
          let result = list_work_items(tmp.path().to_str().unwrap().to_string()).unwrap();
          assert!(result.is_empty());
      }

      #[test]
      fn list_finds_spec_directories() {
          let tmp = TempDir::new().unwrap();
          fs::create_dir_all(tmp.path().join("specs/order-management")).unwrap();
          let result = list_work_items(tmp.path().to_str().unwrap().to_string()).unwrap();
          assert_eq!(result.len(), 1);
          assert_eq!(result[0].item_type, "spec");
          assert_eq!(result[0].id, "order-management");
          assert_eq!(result[0].title, "order management");
      }

      #[test]
      fn list_finds_all_types() {
          let tmp = TempDir::new().unwrap();
          fs::create_dir_all(tmp.path().join("specs/spec-a")).unwrap();
          fs::create_dir_all(tmp.path().join("research/research-b")).unwrap();
          fs::create_dir_all(tmp.path().join("prs/PR-123")).unwrap();
          let result = list_work_items(tmp.path().to_str().unwrap().to_string()).unwrap();
          assert_eq!(result.len(), 3);
      }

      #[test]
      fn read_file_returns_contents() {
          let tmp = TempDir::new().unwrap();
          let file = tmp.path().join("story.md");
          fs::write(&file, "# Hello").unwrap();
          let content = read_file(file.to_str().unwrap().to_string()).unwrap();
          assert_eq!(content, "# Hello");
      }

      #[test]
      fn write_file_creates_file() {
          let tmp = TempDir::new().unwrap();
          let file = tmp.path().join("out.md");
          write_file(file.to_str().unwrap().to_string(), "content".into()).unwrap();
          assert_eq!(fs::read_to_string(&file).unwrap(), "content");
      }
  }
  ```

- [ ] **Step 3: Run tests — expect fail (todo!)**

  ```bash
  cd src-tauri && cargo test commands::fs 2>&1 | tail -10
  ```

  Expected: FAIL with "not yet implemented".

- [ ] **Step 4: Implement the commands**

  Replace the `todo!()` bodies in `src-tauri/src/commands/fs.rs`:

  ```rust
  #[tauri::command]
  pub fn list_work_items(workspace_path: String) -> Result<Vec<WorkItem>, String> {
      let base = Path::new(&workspace_path);
      let mut items = Vec::new();

      for (folder, type_name) in [("specs", "spec"), ("research", "research"), ("prs", "pr")] {
          let dir = base.join(folder);
          if !dir.exists() {
              continue;
          }
          let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
          for entry in entries.flatten() {
              let path = entry.path();
              if path.is_dir() {
                  let id = path
                      .file_name()
                      .and_then(|n| n.to_str())
                      .unwrap_or("")
                      .to_string();
                  let title = id.replace('-', " ").replace('_', " ");
                  items.push(WorkItem {
                      id,
                      item_type: type_name.to_string(),
                      path: path.to_str().unwrap_or("").to_string(),
                      title,
                  });
              }
          }
      }
      Ok(items)
  }

  #[tauri::command]
  pub fn read_file(path: String) -> Result<String, String> {
      std::fs::read_to_string(&path).map_err(|e| format!("read_file: {e}"))
  }

  #[tauri::command]
  pub fn write_file(path: String, content: String) -> Result<(), String> {
      std::fs::write(&path, content).map_err(|e| format!("write_file: {e}"))
  }
  ```

- [ ] **Step 5: Run tests — expect pass**

  ```bash
  cd src-tauri && cargo test commands::fs 2>&1 | tail -10
  ```

  Expected: all 5 tests pass.

- [ ] **Step 6: Register module and commands in `lib.rs`**

  At the top of `src-tauri/src/lib.rs`, after the existing `use` statements, add:
  ```rust
  mod commands;
  ```

  Create `src-tauri/src/commands/mod.rs`:
  ```rust
  pub mod fs;
  ```

  In `run()`, update the `invoke_handler` line from:
  ```rust
  .invoke_handler(tauri::generate_handler![open_devtools, check_update, install_update])
  ```
  To:
  ```rust
  .invoke_handler(tauri::generate_handler![
      open_devtools,
      check_update,
      install_update,
      commands::fs::list_work_items,
      commands::fs::read_file,
      commands::fs::write_file,
  ])
  ```

- [ ] **Step 7: Verify build**

  ```bash
  cd src-tauri && cargo build 2>&1 | tail -5
  ```

  Expected: `Finished`.

- [ ] **Step 8: Commit**

  ```bash
  git add src-tauri/src/commands/ src-tauri/src/lib.rs src-tauri/Cargo.toml
  git commit -m "feat(tauri): add filesystem Tauri commands with tests"
  ```

---

### Task 8: useWorkItems hook

**Files:**
- Create: `ui/src/hooks/useWorkItems.ts`
- Create: `ui/src/hooks/useFile.ts`

- [ ] **Step 1: Write failing tests**

  Create `ui/src/hooks/useWorkItems.test.ts`:
  ```ts
  import { describe, it, expect, vi, beforeEach } from 'vitest'
  import { renderHook, act } from '@testing-library/react'

  const mockInvoke = vi.fn()
  vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }))

  import { useWorkItems } from './useWorkItems'
  import { useAppStore } from '../store/app'

  describe('useWorkItems', () => {
    beforeEach(() => {
      vi.clearAllMocks()
      useAppStore.setState({ workItems: [], workspacePath: '/workspace' })
    })

    it('calls list_work_items and populates store', async () => {
      mockInvoke.mockResolvedValue([
        { id: 'spec-a', item_type: 'spec', path: '/workspace/specs/spec-a', title: 'spec a' },
      ])
      const { result } = renderHook(() => useWorkItems())
      await act(async () => { await result.current.refresh() })
      expect(mockInvoke).toHaveBeenCalledWith('list_work_items', { workspacePath: '/workspace' })
      expect(useAppStore.getState().workItems).toHaveLength(1)
    })
  })
  ```

- [ ] **Step 2: Run test — expect fail**

  ```bash
  cd ui && npm test
  ```

  Expected: FAIL — `useWorkItems` not found.

- [ ] **Step 3: Create `ui/src/hooks/useWorkItems.ts`**

  ```ts
  import { invoke } from '@tauri-apps/api/core'
  import { useAppStore } from '../store/app'
  import type { WorkItem } from '../types/workItem'

  interface RawWorkItem {
    id: string
    item_type: string
    path: string
    title: string
  }

  export function useWorkItems() {
    const { workspacePath, setWorkItems } = useAppStore()

    async function refresh() {
      const raw = await invoke<RawWorkItem[]>('list_work_items', { workspacePath })
      const items: WorkItem[] = raw.map(r => ({
        id: r.id,
        type: r.item_type as WorkItem['type'],
        path: r.path,
        title: r.title,
      }))
      setWorkItems(items)
    }

    return { refresh }
  }
  ```

- [ ] **Step 4: Create `ui/src/hooks/useFile.ts`**

  ```ts
  import { invoke } from '@tauri-apps/api/core'

  export function useFile() {
    async function readFile(path: string): Promise<string> {
      return invoke<string>('read_file', { path })
    }

    async function writeFile(path: string, content: string): Promise<void> {
      return invoke<void>('write_file', { path, content })
    }

    return { readFile, writeFile }
  }
  ```

- [ ] **Step 5: Run tests — expect pass**

  ```bash
  cd ui && npm test
  ```

  Expected: PASS.

- [ ] **Step 6: Commit**

  ```bash
  git add ui/src/hooks/
  git commit -m "feat(ui): useWorkItems and useFile hooks"
  ```

---

### Task 9: App shell (Rail + ListPanel + routing)

**Files:**
- Create: `ui/src/components/Rail.tsx`
- Create: `ui/src/components/ListPanel.tsx`
- Create: `ui/src/components/Shell.tsx`
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: Create `ui/src/components/Rail.tsx`**

  ```tsx
  import { useAppStore } from '../store/app'
  import type { WorkItemType } from '../types/workItem'

  type ViewMode = 'mine' | 'all'

  interface RailProps {
    view: ViewMode
    onViewChange: (v: ViewMode) => void
  }

  const ICON: Record<WorkItemType, string> = { spec: '📋', research: '🔬', pr: '🔀' }

  export function Rail({ view, onViewChange }: RailProps) {
    return (
      <div style={{
        width: 46, background: '#0f1011', borderRight: '1px solid #2a2a2e',
        display: 'flex', flexDirection: 'column', alignItems: 'center',
        padding: '8px 0', gap: 2, flexShrink: 0,
      }}>
        <RailBtn active={view === 'mine'} onClick={() => onViewChange('mine')} title="My Work">◎</RailBtn>
        <RailBtn active={view === 'all'} onClick={() => onViewChange('all')} title="All Work">◫</RailBtn>
        <div style={{ flex: 1 }} />
        <RailBtn active={false} onClick={() => {}} title="Settings">⚙</RailBtn>
      </div>
    )
  }

  function RailBtn({ active, onClick, title, children }: {
    active: boolean
    onClick: () => void
    title: string
    children: React.ReactNode
  }) {
    return (
      <button onClick={onClick} title={title} style={{
        width: 32, height: 32, borderRadius: 7, border: 'none', cursor: 'pointer',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: 15, position: 'relative',
        background: active ? '#1c1c1e' : 'transparent',
        color: active ? '#e8e8ed' : '#52525b',
      }}>
        {active && (
          <span style={{
            position: 'absolute', left: 0, top: '50%', transform: 'translateY(-50%)',
            width: 2, height: 13, background: '#5e6ad2', borderRadius: '0 2px 2px 0',
          }} />
        )}
        {children}
      </button>
    )
  }
  ```

- [ ] **Step 2: Create `ui/src/components/ListPanel.tsx`**

  ```tsx
  import { useEffect } from 'react'
  import { useAppStore } from '../store/app'
  import { useWorkItems } from '../hooks/useWorkItems'
  import type { WorkItem, WorkItemType } from '../types/workItem'

  const TYPE_COLOR: Record<WorkItemType, string> = {
    spec: '#4ade80',
    research: '#a78bfa',
    pr: '#fb923c',
  }

  export function ListPanel() {
    const { workItems, selectedItem, selectItem } = useAppStore()
    const { refresh } = useWorkItems()

    useEffect(() => { refresh() }, [])

    const grouped = workItems.reduce<Record<WorkItemType, WorkItem[]>>(
      (acc, item) => { acc[item.type].push(item); return acc },
      { spec: [], research: [], pr: [] },
    )

    return (
      <div style={{
        width: 260, background: '#161618', borderRight: '1px solid #2a2a2e',
        display: 'flex', flexDirection: 'column', flexShrink: 0,
      }}>
        <div style={{
          padding: '10px 12px 8px', borderBottom: '1px solid #2a2a2e',
          fontSize: 11, fontWeight: 600, color: '#52525b', textTransform: 'uppercase', letterSpacing: '0.05em',
        }}>
          Work Items
        </div>
        <div style={{ flex: 1, overflowY: 'auto' }}>
          {(['spec', 'research', 'pr'] as WorkItemType[]).map(type => (
            grouped[type].length > 0 && (
              <section key={type}>
                <div style={{
                  padding: '8px 12px 4px', fontSize: 10, fontWeight: 600,
                  color: '#52525b', textTransform: 'uppercase', letterSpacing: '0.04em',
                }}>
                  {type === 'pr' ? 'Pull Requests' : type === 'spec' ? 'Specs' : 'Research'}
                </div>
                {grouped[type].map(item => (
                  <div key={item.id} onClick={() => selectItem(item)} style={{
                    display: 'flex', alignItems: 'center', gap: 8,
                    padding: '6px 12px', cursor: 'pointer', borderRadius: 5, margin: '0 4px',
                    background: selectedItem?.id === item.id ? 'rgba(94,106,210,.12)' : 'transparent',
                  }}>
                    <span style={{ width: 7, height: 7, borderRadius: '50%', background: TYPE_COLOR[item.type], flexShrink: 0 }} />
                    <span style={{ fontSize: 12, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {item.title}
                    </span>
                  </div>
                ))}
              </section>
            )
          ))}
          {workItems.length === 0 && (
            <div style={{ padding: 20, fontSize: 11, color: '#52525b', textAlign: 'center' }}>
              No work items found in workspace.
            </div>
          )}
        </div>
      </div>
    )
  }
  ```

- [ ] **Step 3: Create `ui/src/components/Shell.tsx`**

  ```tsx
  import { useState } from 'react'
  import { useAppStore } from '../store/app'
  import { Rail } from './Rail'
  import { ListPanel } from './ListPanel'

  export function Shell() {
    const [view, setView] = useState<'mine' | 'all'>('mine')
    const selectedItem = useAppStore(s => s.selectedItem)

    return (
      <div style={{ height: '100vh', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        {/* Title bar */}
        <div style={{
          height: 37, background: '#161618', borderBottom: '1px solid #2a2a2e',
          display: 'flex', alignItems: 'center', padding: '0 14px',
          WebkitAppRegion: 'drag' as React.CSSProperties['WebkitAppRegion'],
          flexShrink: 0,
        }}>
          <span style={{ fontSize: 12, fontWeight: 600, color: '#8e8e9a' }}>Delta</span>
          {selectedItem && (
            <>
              <span style={{ color: '#52525b', margin: '0 6px', fontSize: 11 }}>/</span>
              <span style={{ fontSize: 12, color: '#52525b' }}>{selectedItem.title}</span>
            </>
          )}
        </div>

        {/* Main layout */}
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          <Rail view={view} onViewChange={setView} />
          <ListPanel />
          <main style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', minWidth: 0 }}>
            {!selectedItem ? (
              <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#52525b', flexDirection: 'column', gap: 8 }}>
                <div style={{ fontSize: 28, opacity: 0.3 }}>◻</div>
                <div style={{ fontSize: 13 }}>Select a work item</div>
              </div>
            ) : (
              <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#52525b' }}>
                {selectedItem.type} workspace — coming in Phase 3
              </div>
            )}
          </main>
        </div>
      </div>
    )
  }
  ```

- [ ] **Step 4: Wire Shell into App.tsx**

  In `ui/src/App.tsx`, add the import and replace the placeholder ready state:
  ```tsx
  import { Shell } from './components/Shell'
  ```

  Replace:
  ```tsx
  // Phase 3 will replace this placeholder with <Shell />
  return (
    <div style={{ height: '100vh', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#e8e8ed' }}>
      <p>Workspace — server on :{useAppStore.getState().serverPort}</p>
    </div>
  )
  ```
  With:
  ```tsx
  return <Shell />
  ```

- [ ] **Step 5: Also need to set workspacePath on ready**

  In `App.tsx`, update the `server:ready` listener to also set the workspace path.

  `@tauri-apps/api/path` is included in `@tauri-apps/api` v2 — no new dependency needed.

  Update the `server:ready` listener block in `App.tsx`:
  ```tsx
  listen<number>('server:ready', async (e) => {
    const { documentDir } = await import('@tauri-apps/api/path')
    const docDir = await documentDir()
    // docDir includes trailing separator on macOS/Linux
    useAppStore.getState().setWorkspacePath(`${docDir}Delta`)
    setServerReady(e.payload)
    setPhase('ready')
  }),
  ```

- [ ] **Step 6: Manual smoke test**

  Create a test workspace:
  ```bash
  mkdir -p ~/Documents/Delta/specs/order-management
  mkdir -p ~/Documents/Delta/research/camera-occlusion
  mkdir -p ~/Documents/Delta/prs/PR-4523
  ```

  Run:
  ```bash
  make dev
  ```

  Expected: workspace shell shows with Rail, ListPanel listing the 3 items, clicking an item selects it in the title bar.

- [ ] **Step 7: Commit**

  ```bash
  git add ui/src/components/
  git commit -m "feat(ui): workspace shell — rail, list panel, routing"
  ```

---

## Phase 3 — Workspace Pages + Agent

Adds the three workspace pages (Spec, Research, PR) with real file rendering, the shared AgentPanel, and the hermes-webui fork endpoints that power the agent.

---

### Task 10: Spec page

**Files:**
- Create: `ui/src/pages/Spec/SpecPage.tsx`
- Create: `ui/src/pages/Spec/AcList.tsx`

- [ ] **Step 1: Write failing test for AcList**

  Create `ui/src/pages/Spec/AcList.test.tsx`:
  ```tsx
  import { render, screen } from '@testing-library/react'
  import { describe, it, expect } from 'vitest'
  import { AcList } from './AcList'

  describe('AcList', () => {
    it('renders acceptance criteria parsed from markdown', () => {
      const md = `## Acceptance Criteria\n- AC-01: User can create order\n- AC-02: Order validates stock`
      render(<AcList markdown={md} />)
      expect(screen.getByText('AC-01: User can create order')).toBeInTheDocument()
      expect(screen.getByText('AC-02: Order validates stock')).toBeInTheDocument()
    })

    it('renders empty state when no AC section found', () => {
      render(<AcList markdown="# Story\nSome text" />)
      expect(screen.getByText(/no acceptance criteria/i)).toBeInTheDocument()
    })
  })
  ```

- [ ] **Step 2: Run test — expect fail**

  ```bash
  cd ui && npm test
  ```

  Expected: FAIL — `AcList` not found.

- [ ] **Step 3: Create `ui/src/pages/Spec/AcList.tsx`**

  ```tsx
  interface AcListProps {
    markdown: string
  }

  export function AcList({ markdown }: AcListProps) {
    const acSection = markdown.match(/## Acceptance Criteria\n([\s\S]*?)(?=\n##|$)/)
    const items = acSection
      ? acSection[1].split('\n').filter(l => l.trim().startsWith('- ')).map(l => l.replace(/^- /, ''))
      : []

    if (items.length === 0) {
      return <div style={{ color: '#52525b', fontSize: 12, padding: '12px 0' }}>No acceptance criteria found.</div>
    }

    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        {items.map((item, i) => (
          <div key={i} style={{
            display: 'flex', alignItems: 'flex-start', gap: 8,
            border: '1px solid #2a2a2e', borderRadius: 6, padding: '8px 10px',
          }}>
            <span style={{ fontSize: 10, color: '#52525b', fontFamily: 'monospace', marginTop: 1, minWidth: 30 }}>
              {`AC-${String(i + 1).padStart(2, '0')}`}
            </span>
            <span style={{ fontSize: 12, lineHeight: 1.6, flex: 1 }}>{item.replace(/^AC-\d+:\s*/, '')}</span>
          </div>
        ))}
      </div>
    )
  }
  ```

- [ ] **Step 4: Run tests — expect pass**

  ```bash
  cd ui && npm test
  ```

  Expected: PASS.

- [ ] **Step 5: Create `ui/src/pages/Spec/SpecPage.tsx`**

  ```tsx
  import { useEffect, useState } from 'react'
  import ReactMarkdown from 'react-markdown'
  import { useFile } from '../../hooks/useFile'
  import { AcList } from './AcList'
  import type { WorkItem } from '../../types/workItem'

  type Tab = 'story' | 'tech' | 'ac'

  interface SpecPageProps {
    item: WorkItem
  }

  export function SpecPage({ item }: SpecPageProps) {
    const [tab, setTab] = useState<Tab>('story')
    const [storyMd, setStoryMd] = useState('')
    const [techMd, setTechMd] = useState('')
    const { readFile } = useFile()

    useEffect(() => {
      readFile(`${item.path}/story.md`).then(setStoryMd).catch(() => setStoryMd(''))
      readFile(`${item.path}/tech.md`).then(setTechMd).catch(() => setTechMd(''))
    }, [item.path])

    const tabs: { id: Tab; label: string }[] = [
      { id: 'story', label: 'story.md' },
      { id: 'tech', label: 'tech.md' },
      { id: 'ac', label: 'AC Checklist' },
    ]

    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
        {/* Tabs */}
        <div style={{ height: 36, borderBottom: '1px solid #2a2a2e', padding: '0 16px', display: 'flex', alignItems: 'flex-end', flexShrink: 0 }}>
          {tabs.map(t => (
            <button key={t.id} onClick={() => setTab(t.id)} style={{
              padding: '7px 12px', fontSize: 12, cursor: 'pointer', border: 'none',
              background: 'transparent', color: tab === t.id ? '#e8e8ed' : '#52525b',
              borderBottom: tab === t.id ? '2px solid #5e6ad2' : '2px solid transparent',
              marginBottom: -1,
            }}>
              {t.label}
            </button>
          ))}
        </div>

        {/* Content */}
        <div style={{ flex: 1, overflow: 'auto', padding: '24px 28px' }}>
          {tab === 'story' && (
            <div className="doc">
              <ReactMarkdown>{storyMd || '*No story.md found*'}</ReactMarkdown>
            </div>
          )}
          {tab === 'tech' && (
            <div className="doc">
              <ReactMarkdown>{techMd || '*No tech.md found*'}</ReactMarkdown>
            </div>
          )}
          {tab === 'ac' && <AcList markdown={storyMd} />}
        </div>
      </div>
    )
  }
  ```

- [ ] **Step 6: Commit**

  ```bash
  git add ui/src/pages/Spec/
  git commit -m "feat(ui): spec workspace page — story/tech/AC tabs"
  ```

---

### Task 11: Research page

**Files:**
- Create: `ui/src/pages/Research/ResearchPage.tsx`
- Create: `ui/src/pages/Research/Pipeline.tsx`

- [ ] **Step 1: Write failing test**

  Create `ui/src/pages/Research/Pipeline.test.tsx`:
  ```tsx
  import { render, screen } from '@testing-library/react'
  import { describe, it, expect } from 'vitest'
  import { Pipeline } from './Pipeline'

  const STAGES = ['Problem', 'Survey', 'Eval', 'Challenge', 'Report']

  describe('Pipeline', () => {
    it('renders all 5 stage columns', () => {
      render(<Pipeline currentStage={1} onStageClick={() => {}} />)
      STAGES.forEach(stage => {
        expect(screen.getByText(stage)).toBeInTheDocument()
      })
    })

    it('marks completed stages', () => {
      render(<Pipeline currentStage={3} onStageClick={() => {}} />)
      // Stages 0-2 are done, 3 is active
      expect(screen.getByText('Problem').closest('[data-status]')).toHaveAttribute('data-status', 'done')
      expect(screen.getByText('Eval').closest('[data-status]')).toHaveAttribute('data-status', 'active')
    })
  })
  ```

- [ ] **Step 2: Run test — expect fail**

  ```bash
  cd ui && npm test
  ```

  Expected: FAIL.

- [ ] **Step 3: Create `ui/src/pages/Research/Pipeline.tsx`**

  ```tsx
  const STAGE_NAMES = ['Problem', 'Survey', 'Eval', 'Challenge', 'Report'] as const

  interface PipelineProps {
    currentStage: number   // 0-indexed
    onStageClick: (stage: number) => void
  }

  type StageStatus = 'done' | 'active' | 'pending'

  function stageStatus(index: number, current: number): StageStatus {
    if (index < current) return 'done'
    if (index === current) return 'active'
    return 'pending'
  }

  const STATUS_COLOR: Record<StageStatus, string> = {
    done: '#4ade80',
    active: '#5e6ad2',
    pending: '#52525b',
  }

  export function Pipeline({ currentStage, onStageClick }: PipelineProps) {
    return (
      <div style={{ display: 'flex', flex: 1, overflow: 'auto', minHeight: 0 }}>
        {STAGE_NAMES.map((name, i) => {
          const status = stageStatus(i, currentStage)
          return (
            <div
              key={name}
              data-status={status}
              onClick={() => onStageClick(i)}
              style={{
                minWidth: 160, flex: 1, borderRight: i < 4 ? '1px solid #2a2a2e' : 'none',
                display: 'flex', flexDirection: 'column', overflow: 'hidden', cursor: 'pointer',
              }}
            >
              <div style={{
                padding: '9px 12px 8px', borderBottom: '1px solid #2a2a2e', flexShrink: 0,
                display: 'flex', alignItems: 'center', gap: 6,
              }}>
                <span style={{
                  fontSize: 9, fontWeight: 700, background: '#1c1c1e', border: '1px solid #2a2a2e',
                  padding: '1px 5px', borderRadius: 3, color: '#52525b',
                }}>
                  {String(i + 1).padStart(2, '0')}
                </span>
                <span style={{ fontSize: 11, fontWeight: 600, color: STATUS_COLOR[status] }}>{name}</span>
              </div>
              <div style={{ flex: 1, padding: 10 }}>
                {status === 'active' && (
                  <div style={{
                    border: '1px dashed #2a2a2e', borderRadius: 5, padding: 11,
                    textAlign: 'center', fontSize: 11, color: '#5e6ad2', cursor: 'pointer',
                  }}>
                    + Add note
                  </div>
                )}
              </div>
            </div>
          )
        })}
      </div>
    )
  }
  ```

- [ ] **Step 4: Run tests — expect pass**

  ```bash
  cd ui && npm test
  ```

  Expected: PASS.

- [ ] **Step 5: Create `ui/src/pages/Research/ResearchPage.tsx`**

  ```tsx
  import { useEffect, useState } from 'react'
  import { useFile } from '../../hooks/useFile'
  import { Pipeline } from './Pipeline'
  import type { WorkItem } from '../../types/workItem'

  interface StagesJson {
    currentStage: number
  }

  interface ResearchPageProps {
    item: WorkItem
  }

  export function ResearchPage({ item }: ResearchPageProps) {
    const [currentStage, setCurrentStage] = useState(0)
    const { readFile, writeFile } = useFile()

    useEffect(() => {
      readFile(`${item.path}/stages.json`)
        .then(raw => {
          const data = JSON.parse(raw) as StagesJson
          setCurrentStage(data.currentStage ?? 0)
        })
        .catch(() => setCurrentStage(0))
    }, [item.path])

    async function handleStageClick(stage: number) {
      setCurrentStage(stage)
      await writeFile(`${item.path}/stages.json`, JSON.stringify({ currentStage: stage }))
    }

    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
        <div style={{
          height: 44, borderBottom: '1px solid #2a2a2e', padding: '0 20px',
          display: 'flex', alignItems: 'center', gap: 10, flexShrink: 0,
        }}>
          <span style={{ fontSize: 14 }}>🔬</span>
          <span style={{ fontSize: 14, fontWeight: 600 }}>{item.title}</span>
        </div>
        <Pipeline currentStage={currentStage} onStageClick={handleStageClick} />
      </div>
    )
  }
  ```

- [ ] **Step 6: Commit**

  ```bash
  git add ui/src/pages/Research/
  git commit -m "feat(ui): research workspace page — 5-stage pipeline"
  ```

---

### Task 12: PR page

**Files:**
- Create: `ui/src/pages/PR/PRPage.tsx`
- Create: `ui/src/pages/PR/DiffView.tsx`

- [ ] **Step 1: Write failing test**

  Create `ui/src/pages/PR/DiffView.test.tsx`:
  ```tsx
  import { render, screen } from '@testing-library/react'
  import { describe, it, expect } from 'vitest'
  import { DiffView } from './DiffView'

  describe('DiffView', () => {
    it('renders added lines in green', () => {
      const lines = [{ type: 'add' as const, content: '+ new line' }]
      render(<DiffView lines={lines} />)
      const el = screen.getByText('+ new line')
      expect(el).toHaveStyle({ color: '#4ade80' })
    })

    it('renders removed lines in red', () => {
      const lines = [{ type: 'remove' as const, content: '- old line' }]
      render(<DiffView lines={lines} />)
      const el = screen.getByText('- old line')
      expect(el).toHaveStyle({ color: '#f87171' })
    })
  })
  ```

- [ ] **Step 2: Run test — expect fail**

  ```bash
  cd ui && npm test
  ```

  Expected: FAIL.

- [ ] **Step 3: Create `ui/src/pages/PR/DiffView.tsx`**

  ```tsx
  export interface DiffLine {
    type: 'add' | 'remove' | 'context' | 'hunk'
    content: string
  }

  interface DiffViewProps {
    lines: DiffLine[]
  }

  const LINE_STYLE: Record<DiffLine['type'], React.CSSProperties> = {
    add:     { background: 'rgba(74,222,128,.06)', color: '#4ade80' },
    remove:  { background: 'rgba(248,113,113,.06)', color: '#f87171' },
    context: { color: '#52525b' },
    hunk:    { color: '#5e6ad2' },
  }

  export function DiffView({ lines }: DiffViewProps) {
    return (
      <div style={{ fontFamily: '"SF Mono","Fira Code",monospace', fontSize: 11, lineHeight: 1.7 }}>
        {lines.map((line, i) => (
          <div key={i} style={{ padding: '1px 16px', ...LINE_STYLE[line.type] }}>
            {line.content}
          </div>
        ))}
      </div>
    )
  }
  ```

- [ ] **Step 4: Run tests — expect pass**

  ```bash
  cd ui && npm test
  ```

  Expected: PASS.

- [ ] **Step 5: Create `ui/src/pages/PR/PRPage.tsx`**

  ```tsx
  import { useEffect, useState } from 'react'
  import { useFile } from '../../hooks/useFile'
  import { DiffView, DiffLine } from './DiffView'
  import type { WorkItem } from '../../types/workItem'

  interface PRMeta {
    title: string
    branch: string
    adoUrl?: string
    linkedSpec?: string
  }

  interface PRPageProps {
    item: WorkItem
  }

  // Parse unified diff text into typed lines
  function parseDiff(raw: string): DiffLine[] {
    return raw.split('\n').map(line => {
      if (line.startsWith('@@')) return { type: 'hunk' as const, content: line }
      if (line.startsWith('+') && !line.startsWith('+++')) return { type: 'add' as const, content: line }
      if (line.startsWith('-') && !line.startsWith('---')) return { type: 'remove' as const, content: line }
      return { type: 'context' as const, content: line }
    })
  }

  export function PRPage({ item }: PRPageProps) {
    const [meta, setMeta] = useState<PRMeta | null>(null)
    const [diffLines, setDiffLines] = useState<DiffLine[]>([])
    const { readFile } = useFile()

    useEffect(() => {
      readFile(`${item.path}/meta.json`)
        .then(raw => setMeta(JSON.parse(raw) as PRMeta))
        .catch(() => setMeta({ title: item.title, branch: 'unknown' }))

      readFile(`${item.path}/diff.patch`)
        .then(raw => setDiffLines(parseDiff(raw)))
        .catch(() => setDiffLines([{ type: 'context', content: '(no diff available)' }]))
    }, [item.path])

    return (
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' }}>
        <div style={{
          height: 44, borderBottom: '1px solid #2a2a2e', padding: '0 20px',
          display: 'flex', alignItems: 'center', gap: 10, flexShrink: 0,
        }}>
          <span style={{ fontSize: 14 }}>🔀</span>
          <span style={{ fontSize: 14, fontWeight: 600 }}>{meta?.title ?? item.title}</span>
          {meta?.branch && (
            <span style={{ fontSize: 11, color: '#52525b', background: '#1c1c1e', padding: '2px 8px', borderRadius: 4 }}>
              {meta.branch}
            </span>
          )}
        </div>
        <div style={{ flex: 1, overflow: 'auto', padding: '16px 0' }}>
          <DiffView lines={diffLines} />
        </div>
      </div>
    )
  }
  ```

- [ ] **Step 6: Commit**

  ```bash
  git add ui/src/pages/PR/
  git commit -m "feat(ui): PR workspace page — diff view"
  ```

---

### Task 13: Wire pages into Shell

**Files:**
- Modify: `ui/src/components/Shell.tsx`

- [ ] **Step 1: Update Shell to render the correct page based on selected item type**

  In `ui/src/components/Shell.tsx`, add imports at the top:
  ```tsx
  import { SpecPage } from '../pages/Spec/SpecPage'
  import { ResearchPage } from '../pages/Research/ResearchPage'
  import { PRPage } from '../pages/PR/PRPage'
  ```

  Replace the `{selectedItem ? ... : ...}` block in `<main>` with:
  ```tsx
  {!selectedItem ? (
    <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#52525b', flexDirection: 'column', gap: 8 }}>
      <div style={{ fontSize: 28, opacity: 0.3 }}>◻</div>
      <div style={{ fontSize: 13 }}>Select a work item</div>
    </div>
  ) : selectedItem.type === 'spec' ? (
    <SpecPage item={selectedItem} />
  ) : selectedItem.type === 'research' ? (
    <ResearchPage item={selectedItem} />
  ) : (
    <PRPage item={selectedItem} />
  )}
  ```

- [ ] **Step 2: Manual smoke test**

  ```bash
  make dev
  ```

  Expected: clicking a spec shows tabs (story.md / tech.md / AC Checklist), clicking a research item shows the 5-stage pipeline, clicking a PR item shows the diff view.

- [ ] **Step 3: Commit**

  ```bash
  git add ui/src/components/Shell.tsx
  git commit -m "feat(ui): wire spec/research/PR pages into shell"
  ```

---

### Task 14: AgentPanel + useAgentStream

**Files:**
- Create: `ui/src/components/AgentPanel.tsx`
- Create: `ui/src/hooks/useAgentStream.ts`

- [ ] **Step 1: Write failing test for useAgentStream**

  Create `ui/src/hooks/useAgentStream.test.ts`:
  ```ts
  import { describe, it, expect, vi, beforeEach } from 'vitest'
  import { renderHook, act } from '@testing-library/react'
  import { useAgentStream } from './useAgentStream'
  import { useAppStore } from '../store/app'

  describe('useAgentStream', () => {
    beforeEach(() => {
      useAppStore.setState({ serverPort: 8080, serverReady: true })
    })

    it('returns empty messages initially', () => {
      const { result } = renderHook(() => useAgentStream())
      expect(result.current.messages).toEqual([])
    })

    it('adds user message when send is called', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        body: {
          getReader: () => ({
            read: vi.fn()
              .mockResolvedValueOnce({ done: false, value: new TextEncoder().encode('data: {"delta":"Hello"}\n\n') })
              .mockResolvedValueOnce({ done: true, value: undefined }),
          }),
        },
      })
      vi.stubGlobal('fetch', mockFetch)

      const { result } = renderHook(() => useAgentStream())
      await act(async () => {
        await result.current.send('test message', [])
      })

      expect(result.current.messages[0]).toMatchObject({ role: 'user', content: 'test message' })
      vi.unstubAllGlobals()
    })
  })
  ```

- [ ] **Step 2: Run test — expect fail**

  ```bash
  cd ui && npm test
  ```

  Expected: FAIL.

- [ ] **Step 3: Create `ui/src/hooks/useAgentStream.ts`**

  ```ts
  import { useState, useCallback } from 'react'
  import { useAppStore } from '../store/app'
  import type { AgentMessage, AgentThread } from '../types/workItem'

  interface StreamChunk {
    delta?: string
    done?: boolean
  }

  export function useAgentStream(initialThread?: AgentThread) {
    const serverPort = useAppStore(s => s.serverPort)
    const [messages, setMessages] = useState<AgentMessage[]>(initialThread?.messages ?? [])
    const [streaming, setStreaming] = useState(false)

    const send = useCallback(async (content: string, contextFiles: string[]) => {
      const userMsg: AgentMessage = { role: 'user', content, timestamp: Date.now() }
      setMessages(prev => [...prev, userMsg])
      setStreaming(true)

      const assistantMsg: AgentMessage = { role: 'assistant', content: '', timestamp: Date.now() }
      setMessages(prev => [...prev, assistantMsg])

      try {
        const response = await fetch(`http://127.0.0.1:${serverPort}/api/chat`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            messages: [...messages, userMsg].map(m => ({ role: m.role, content: m.content })),
            context_files: contextFiles,
          }),
        })

        if (!response.ok || !response.body) throw new Error(`HTTP ${response.status}`)

        const reader = response.body.getReader()
        const decoder = new TextDecoder()
        let accumulated = ''

        while (true) {
          const { done, value } = await reader.read()
          if (done) break

          const text = decoder.decode(value, { stream: true })
          for (const line of text.split('\n')) {
            if (!line.startsWith('data: ')) continue
            try {
              const chunk = JSON.parse(line.slice(6)) as StreamChunk
              if (chunk.delta) {
                accumulated += chunk.delta
                setMessages(prev => {
                  const next = [...prev]
                  next[next.length - 1] = { ...next[next.length - 1], content: accumulated }
                  return next
                })
              }
            } catch { /* ignore parse errors */ }
          }
        }
      } catch (e) {
        setMessages(prev => {
          const next = [...prev]
          next[next.length - 1] = { ...next[next.length - 1], content: `Error: ${String(e)}` }
          return next
        })
      } finally {
        setStreaming(false)
      }
    }, [messages, serverPort])

    return { messages, streaming, send, setMessages }
  }
  ```

- [ ] **Step 4: Run tests — expect pass**

  ```bash
  cd ui && npm test
  ```

  Expected: PASS.

- [ ] **Step 5: Create `ui/src/components/AgentPanel.tsx`**

  ```tsx
  import { useRef, useEffect, useState } from 'react'
  import ReactMarkdown from 'react-markdown'
  import { useAgentStream } from '../hooks/useAgentStream'
  import type { AgentThread, WorkItem } from '../types/workItem'

  interface AgentPanelProps {
    item: WorkItem
    contextFiles: string[]
    initialThread?: AgentThread
    onThreadUpdate?: (thread: AgentThread) => void
  }

  const CHIPS: Record<WorkItem['type'], string[]> = {
    spec: ['Review AC', 'Generate tech.md', 'Find edge cases'],
    research: ['Next stage', 'Summarise findings', 'Suggest eval criteria'],
    pr: ['Review against AC', 'Check edge cases', 'Summarise changes'],
  }

  export function AgentPanel({ item, contextFiles, initialThread, onThreadUpdate }: AgentPanelProps) {
    const { messages, streaming, send } = useAgentStream(initialThread)
    const [input, setInput] = useState('')
    const bottomRef = useRef<HTMLDivElement>(null)

    useEffect(() => {
      bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
    }, [messages])

    useEffect(() => {
      onThreadUpdate?.({ messages })
    }, [messages, onThreadUpdate])

    async function handleSend() {
      const text = input.trim()
      if (!text || streaming) return
      setInput('')
      await send(text, contextFiles)
    }

    return (
      <div style={{
        width: 320, background: '#161618', borderLeft: '1px solid #2a2a2e',
        display: 'flex', flexDirection: 'column', flexShrink: 0,
      }}>
        {/* Header */}
        <div style={{
          height: 44, borderBottom: '1px solid #2a2a2e', padding: '0 12px',
          display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0,
        }}>
          <span style={{ fontSize: 12, fontWeight: 600 }}>Agent</span>
          <span style={{
            fontSize: 9, color: '#52525b', background: '#1c1c1e',
            padding: '2px 6px', borderRadius: 4, border: '1px solid #2a2a2e',
          }}>
            claude-3-5-sonnet
          </span>
        </div>

        {/* Messages */}
        <div style={{ flex: 1, overflow: 'auto', padding: 12, display: 'flex', flexDirection: 'column', gap: 10 }}>
          {messages.length === 0 && (
            <div style={{ fontSize: 11, color: '#52525b', textAlign: 'center', marginTop: 20 }}>
              Context loaded: {item.title}
            </div>
          )}
          {messages.map((msg, i) => (
            <div key={i} style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
              <span style={{
                fontSize: 9, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.06em',
                color: msg.role === 'user' ? '#52525b' : '#5e6ad2',
              }}>
                {msg.role}
              </span>
              <div style={{ fontSize: 12, lineHeight: 1.65 }}>
                <ReactMarkdown>{msg.content}</ReactMarkdown>
              </div>
            </div>
          ))}
          <div ref={bottomRef} />
        </div>

        {/* Quick chips */}
        <div style={{ padding: 8, display: 'flex', flexWrap: 'wrap', gap: 5, borderTop: '1px solid #2a2a2e', flexShrink: 0 }}>
          {CHIPS[item.type].map(chip => (
            <button key={chip} onClick={() => send(chip, contextFiles)} disabled={streaming} style={{
              padding: '4px 8px', borderRadius: 5, fontSize: 11, cursor: 'pointer',
              background: '#1c1c1e', border: '1px solid #2a2a2e', color: '#8e8e9a',
            }}>
              {chip}
            </button>
          ))}
        </div>

        {/* Input */}
        <div style={{ padding: 8, borderTop: '1px solid #2a2a2e', display: 'flex', gap: 6, alignItems: 'flex-end', flexShrink: 0 }}>
          <textarea
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSend() } }}
            placeholder="Ask the agent…"
            rows={2}
            style={{
              flex: 1, background: '#1c1c1e', border: '1px solid #2a2a2e', borderRadius: 7,
              color: '#e8e8ed', fontSize: 12, padding: '7px 10px', resize: 'none',
              fontFamily: 'inherit', lineHeight: 1.5, outline: 'none',
            }}
          />
          <button onClick={handleSend} disabled={streaming || !input.trim()} style={{
            width: 30, height: 30, borderRadius: 7, background: '#5e6ad2',
            border: 'none', cursor: 'pointer', color: '#fff', fontSize: 14,
            opacity: streaming || !input.trim() ? 0.5 : 1,
          }}>
            ↑
          </button>
        </div>
      </div>
    )
  }
  ```

- [ ] **Step 6: Commit**

  ```bash
  git add ui/src/components/AgentPanel.tsx ui/src/hooks/useAgentStream.ts
  git commit -m "feat(ui): AgentPanel with streaming + quick action chips"
  ```

---

### Task 15: Wire AgentPanel into Shell + thread persistence

**Files:**
- Modify: `ui/src/components/Shell.tsx`
- Modify: `ui/src/pages/Spec/SpecPage.tsx`
- Modify: `ui/src/pages/Research/ResearchPage.tsx`
- Modify: `ui/src/pages/PR/PRPage.tsx`

- [ ] **Step 1: Load and save .thread.json in each page**

  Add thread loading to `SpecPage.tsx`. Update the `useEffect` that reads files:
  ```tsx
  const [thread, setThread] = useState<AgentThread>({ messages: [] })
  const { readFile, writeFile } = useFile()

  useEffect(() => {
    readFile(`${item.path}/story.md`).then(setStoryMd).catch(() => setStoryMd(''))
    readFile(`${item.path}/tech.md`).then(setTechMd).catch(() => setTechMd(''))
    readFile(`${item.path}/.thread.json`)
      .then(raw => setThread(JSON.parse(raw) as AgentThread))
      .catch(() => setThread({ messages: [] }))
  }, [item.path])

  async function handleThreadUpdate(updated: AgentThread) {
    setThread(updated)
    await writeFile(`${item.path}/.thread.json`, JSON.stringify(updated, null, 2))
  }
  ```

  Apply the same pattern to `ResearchPage.tsx` and `PRPage.tsx`.

- [ ] **Step 2: Add AgentPanel to SpecPage layout**

  In `SpecPage.tsx`, import and add `AgentPanel`:
  ```tsx
  import { AgentPanel } from '../../components/AgentPanel'
  import type { AgentThread } from '../../types/workItem'
  ```

  Wrap the existing content in a flex row and add the panel:
  ```tsx
  return (
    <div style={{ display: 'flex', height: '100%', overflow: 'hidden' }}>
      {/* Left: artifact */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', borderRight: '1px solid #2a2a2e' }}>
        {/* existing tabs + content */}
      </div>
      {/* Right: agent */}
      <AgentPanel
        item={item}
        contextFiles={[`${item.path}/story.md`, `${item.path}/tech.md`]}
        initialThread={thread}
        onThreadUpdate={handleThreadUpdate}
      />
    </div>
  )
  ```

  Apply the same layout pattern to `ResearchPage.tsx` (contextFiles: `notes.md`, `stages.json`) and `PRPage.tsx` (contextFiles: `meta.json`, `diff.patch`).

- [ ] **Step 3: Manual end-to-end test**

  ```bash
  make dev
  ```

  1. Click a spec item → agent panel appears on the right with "Context loaded" message
  2. Type a message → verify it appears and streams a response from hermes-webui
  3. Click a quick chip (e.g. "Review AC") → verify it sends and streams
  4. Switch to another item and back → verify thread is restored from `.thread.json`

- [ ] **Step 4: Commit**

  ```bash
  git add ui/src/
  git commit -m "feat(ui): wire agent panel to pages with thread persistence"
  ```

---

### Task 16: hermes-webui fork — workspace agent endpoints

**Files (in `hermes-webui/` fork):**
- Create: `hermes-webui/api/workspace.py`
- Modify: `hermes-webui/server.py` (or `api/__init__.py` — check existing routing)

- [ ] **Step 1: Understand existing route registration**

  ```bash
  grep -n "router\|app.include\|APIRouter" hermes-webui/server.py hermes-webui/api/*.py 2>/dev/null | head -20
  ```

  Note the pattern used to register routers. Use the same pattern below.

- [ ] **Step 2: Create `hermes-webui/api/workspace.py`**

  ```python
  from fastapi import APIRouter
  from pydantic import BaseModel
  from typing import Optional
  import json

  router = APIRouter(prefix="/api/workspace", tags=["workspace"])


  class WorkspaceRequest(BaseModel):
      messages: list[dict]
      context_files: list[dict]  # [{"path": str, "content": str}]


  class WorkspaceResponse(BaseModel):
      role: str = "assistant"
      content: str


  def _build_context(context_files: list[dict]) -> str:
      parts = []
      for f in context_files:
          parts.append(f"### {f.get('path', 'file')}\n```\n{f.get('content', '')}\n```")
      return "\n\n".join(parts)


  @router.post("/spec/review")
  async def spec_review(req: WorkspaceRequest):
      """Review story.md for missing AC, edge cases, and completeness."""
      context = _build_context(req.context_files)
      system = (
          "You are a BA agent reviewing a software specification. "
          "Identify missing acceptance criteria, edge cases, and ambiguities. "
          "Be specific and concise. Use markdown.\n\n"
          f"Context:\n{context}"
      )
      # Re-use existing chat completion logic from hermes-webui
      from api.chat import complete_stream  # import the existing stream helper
      return await complete_stream(system=system, messages=req.messages)


  @router.post("/research/next-stage")
  async def research_next_stage(req: WorkspaceRequest):
      """Provide guidance for advancing to the next research pipeline stage."""
      context = _build_context(req.context_files)
      system = (
          "You are an AI research advisor. Based on the current stage notes, "
          "summarise what has been accomplished and what is needed to advance to the next stage. "
          "Be concrete and actionable.\n\n"
          f"Context:\n{context}"
      )
      from api.chat import complete_stream
      return await complete_stream(system=system, messages=req.messages)


  @router.post("/pr/review")
  async def pr_review(req: WorkspaceRequest):
      """Check a PR diff against the linked spec's acceptance criteria."""
      context = _build_context(req.context_files)
      system = (
          "You are a code reviewer checking a pull request against its spec. "
          "For each acceptance criterion, state whether the diff addresses it (PASS/FAIL/PARTIAL). "
          "List any issues found.\n\n"
          f"Context:\n{context}"
      )
      from api.chat import complete_stream
      return await complete_stream(system=system, messages=req.messages)
  ```

  > **Note:** The exact import path for `complete_stream` depends on hermes-webui's internal structure. Run `grep -r "def.*stream\|async.*stream" hermes-webui/api/` to find the right function and adjust the import.

- [ ] **Step 3: Register the router**

  Find where other routers are registered in hermes-webui (likely `server.py` or `api/__init__.py`):
  ```bash
  grep -n "include_router\|app.include" hermes-webui/server.py hermes-webui/api/*.py 2>/dev/null
  ```

  Add to the same file, following the existing pattern:
  ```python
  from api.workspace import router as workspace_router
  app.include_router(workspace_router)
  ```

- [ ] **Step 4: Test the endpoints manually**

  Start hermes-webui manually:
  ```bash
  cd hermes-webui && python server.py
  ```

  In another terminal:
  ```bash
  curl -s -X POST http://127.0.0.1:8000/api/workspace/spec/review \
    -H "Content-Type: application/json" \
    -d '{"messages":[{"role":"user","content":"Review this spec"}],"context_files":[{"path":"story.md","content":"# Story\n- AC-01: User can login"}]}'
  ```

  Expected: streaming response with review comments.

- [ ] **Step 5: Update quick-action chips in AgentPanel to use workspace endpoints**

  In `ui/src/components/AgentPanel.tsx`, the `send` function in `useAgentStream` currently calls `/api/chat`. Update `useAgentStream.ts` to accept an optional `endpoint` parameter, and pass the appropriate workspace endpoint based on the chip clicked.

  Update `useAgentStream.ts` signature:
  ```ts
  const send = useCallback(async (content: string, contextFiles: string[], endpoint?: string) => {
    // ...
    const url = endpoint
      ? `http://127.0.0.1:${serverPort}${endpoint}`
      : `http://127.0.0.1:${serverPort}/api/chat`
    const response = await fetch(url, { ... })
  ```

  In `AgentPanel.tsx`, update the chip `onClick`:
  ```tsx
  const CHIP_ENDPOINTS: Record<WorkItem['type'], Record<string, string>> = {
    spec:     { 'Review AC': '/api/workspace/spec/review', 'Generate tech.md': '/api/chat', 'Find edge cases': '/api/chat' },
    research: { 'Next stage': '/api/workspace/research/next-stage', 'Summarise findings': '/api/chat', 'Suggest eval criteria': '/api/chat' },
    pr:       { 'Review against AC': '/api/workspace/pr/review', 'Check edge cases': '/api/chat', 'Summarise changes': '/api/chat' },
  }

  // In chip onClick:
  onClick={() => send(chip, contextFiles, CHIP_ENDPOINTS[item.type][chip])}
  ```

- [ ] **Step 6: Commit hermes-webui fork changes**

  ```bash
  cd hermes-webui
  git add api/workspace.py server.py   # or whichever file was modified
  git commit -m "feat(workspace): add spec/research/pr agent endpoints"
  cd ..
  git add hermes-webui
  git commit -m "chore: bump hermes-webui to include workspace endpoints"
  ```

- [ ] **Step 7: Final smoke test**

  ```bash
  make dev
  ```

  1. Open a spec → click "Review AC" chip → verify workspace-specific agent response
  2. Open a research item → click "Next stage" → verify guidance response
  3. Open a PR → click "Review against AC" → verify review response

---

## Done

After Task 16, the Delta workspace is fully restructured:
- Tauri shell emits typed events instead of injecting JS
- React+Vite workspace app with three page types
- Shared AgentPanel with streaming and thread persistence
- hermes-webui fork with workspace-specific agent endpoints

**Not included in this plan (next iterations):**
- Jira / Confluence / Azure DevOps live integrations
- Project overview and portfolio pages
- Workspace setup wizard (workspace path currently hardcoded to `~/Documents/Delta`)
- OS keychain for API key storage (`tauri-plugin-stronghold`)
- `tauri-plugin-fs` migration (currently using custom Rust commands)
