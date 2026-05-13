# Delta — Workspace Restructure Design

**Date:** 2026-05-14  
**Status:** Approved  
**Scope:** Replace hermes-webui wrapper with Delta workspace UI. v1 delivers app shell + Spec, Research, and PR workspaces with a shared agent panel.

---

## Problem

Delta is currently a Tauri shell that boots a Python server and navigates a webview to hermes-webui's chat UI. This is a wrapper, not a product. The brainstorm established the target: a work-item-centric workspace where every artifact (spec, research topic, PR) carries a persistent agent thread. The agent knows what you're looking at — no re-explaining every session.

---

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Frontend | React + Vite + TypeScript | Component model needed for multiple page types |
| Agent backend | hermes-webui fork (owned, no upstream sync) | Already has Claude integration, streaming, session management. Build on it, don't replace it. |
| External API calls | Tauri commands (Rust) | Keys in OS keychain, scales cleanly to multiple APIs (Jira, ADO, Confluence) |
| Agent calls | React → hermes-webui HTTP API directly | Streaming via fetch + ReadableStream; no Rust proxy needed for this path |
| Work item storage | Files on disk | `story.md`, `tech.md`, `.thread.json` per item. Files > database. |
| hermes-webui upstream | Dropped | Fork owned by team. Cherry-pick upstream changes manually if ever needed. |

---

## Architecture

```
Tauri shell (Rust)
├── Spawns hermes-webui Python server on boot
│   └── emits "server:ready" Tauri event with port when TCP is accepting
├── React + Vite workspace UI  ← new main frontend
│   ├── Shows loading state until "server:ready" event
│   ├── Calls http://127.0.0.1:{port}/api/* for all agent operations
│   └── invoke() for filesystem, keychain, and native integrations
└── Rust Tauri commands
    ├── fs: list_work_items, read_file, write_file
    ├── keychain: set_api_key, get_api_key (via tauri-plugin-stronghold)
    └── integrations: jira_query, confluence_publish, ado_get_pr (stubbed in v1)
```

hermes-webui Python server is **headless** — its own chat UI is not used. The React app calls its API for agent operations. New workspace-specific agent endpoints are added to the fork alongside existing routes.

---

## Repo Structure Changes

```
delta/
├── src/                        ← NEW: React + Vite app
│   ├── main.tsx
│   ├── App.tsx                 ← shell: rail + list panel + main area
│   ├── components/
│   │   ├── Rail.tsx
│   │   ├── ListPanel.tsx
│   │   ├── AgentPanel.tsx      ← shared across all page types
│   │   └── WorkspaceHeader.tsx
│   ├── pages/
│   │   ├── Spec/               ← story.md + tech.md + AC list + agent
│   │   ├── Research/           ← 5-stage pipeline + agent
│   │   └── PR/                 ← diff view + agent
│   ├── hooks/
│   │   ├── useWorkItems.ts     ← reads from Tauri fs commands
│   │   ├── useAgentStream.ts   ← fetch stream from hermes-webui API
│   │   └── useFile.ts
│   └── types/
│       └── workItem.ts
├── src-tauri/
│   └── src/
│       ├── lib.rs              ← simplified: remove uv/venv/poll logic
│       │                          add: register commands, emit server:ready
│       └── commands/
│           ├── fs.rs
│           ├── keychain.rs
│           └── integrations.rs ← stubs in v1
├── desktop/
│   └── index.html              ← splash screen (kept, shown during Python boot)
└── hermes-webui/               ← submodule pointing to team fork
    └── api/                    ← new workspace endpoints added here
        ├── spec.py             ← POST /api/spec/review, /api/spec/gen-tech-md
        └── research.py         ← POST /api/research/next-stage, /api/research/eval
```

**Removed from lib.rs:** The `win.eval("window.location = ...")` navigation call — the React app is now always loaded from `frontendDist`, not navigated to after Python boots.

**Kept in lib.rs:** uv sidecar spawn, venv creation, `wait_for_server` TCP poll — all unchanged. Replace the final navigation call with a `window.emit("server:ready", port)` event instead.

**Changed:** `tauri.conf.json` → `frontendDist: "../src/dist"`, `devUrl: "http://localhost:5173"`

**`desktop/index.html`:** No longer used as `frontendDist`. The React app handles its own loading state. The file stays in the repo but is retired as the boot screen.

---

## Boot Sequence

1. Tauri launches → loads React app (`src/dist/index.html` in prod, `http://localhost:5173` in dev)
2. React shows loading state: "Starting Delta…"
3. Rust: uv creates/updates venv, spawns Python server, polls TCP (existing logic, unchanged)
4. Rust: server accepts connections → emits `server:ready` Tauri event with `{ port }`
5. React: receives event, stores port in app state, loading state clears, agent features enabled
6. React: calls hermes-webui API normally from this point

Startup drops from ~10s to ~3s once venv is warm (first run still slow for venv creation).

---

## Work Item Data Model

```
~/Documents/Delta/              ← workspace root (set on first launch)
├── specs/
│   └── order-management/
│       ├── story.md            ← BA spec
│       ├── tech.md             ← SA/Dev tech spec
│       └── .thread.json        ← agent conversation history
├── research/
│   └── camera-occlusion/
│       ├── notes.md
│       ├── stages.json         ← pipeline stage state (1–5)
│       └── .thread.json
└── prs/
    └── PR-4523/
        ├── meta.json           ← title, branch, ADO link, linked spec path
        └── .thread.json
```

Work item type inferred from parent directory. `.thread.json` is loaded automatically when a work item is opened — agent resumes in context, no re-explaining.

---

## Work Item Types → Page Renderings

| Type | Left panel (artifact) | Agent context loaded |
|------|-----------------------|----------------------|
| Spec | story.md + tech.md tabs, AC checklist | Full spec + .thread.json |
| Research | 5-stage pipeline (Problem→Survey→Eval→Challenge→Report) | notes.md + stages.json + .thread.json |
| PR | Diff view, inline comments | meta.json + linked story.md + .thread.json |

---

## Agent Panel (shared component)

`AgentPanel` renders on every page. The work item in context determines what gets sent to the agent.

`useAgentStream` hook:
1. Calls `POST http://127.0.0.1:{port}/api/chat` (hermes-webui endpoint) with messages + context files
2. Reads response as `ReadableStream` (SSE/chunked)
3. Appends tokens to displayed message in real time
4. On work item switch or unmount: saves updated `.thread.json` via `invoke('write_file', ...)`

Quick-action chips (contextual per page type):
- Spec: "Review AC", "Generate tech.md", "Find edge cases"
- Research: "Next stage", "Summarize findings", "Suggest eval criteria"
- PR: "Review against AC", "Check edge cases", "Summarise changes"

---

## hermes-webui Fork — New Endpoints

Added to the fork alongside existing routes. Existing `/api/chat` and session endpoints unchanged.

```
POST /api/spec/review           ← BA agent: review story.md, return AC gaps
POST /api/spec/gen-tech-md      ← generate tech.md from story.md
POST /api/research/next-stage   ← advance pipeline stage with agent guidance
POST /api/research/eval         ← run evaluation criteria check
POST /api/pr/review             ← check PR diff against linked spec AC
```

All endpoints accept `{ messages, context_files: [{ path, content }] }` — same shape as the existing chat endpoint. The context files are loaded by the React app via Tauri `read_file` commands before the request is made.

---

## Tauri Commands (v1)

```rust
// fs.rs
list_work_items(workspace_path: String) → Vec<WorkItem>
read_file(path: String) → String
write_file(path: String, content: String) → ()

// keychain.rs
set_api_key(service: String, key: String) → ()
get_api_key(service: String) → Option<String>

// integrations.rs (stubbed — return mock data in v1)
jira_query(project: String, filter: String) → Vec<JiraIssue>
confluence_publish(page_id: String, content: String) → ()
ado_get_pr(pr_id: u32) → PRMeta
```

---

## Key Commands (updated)

```bash
make dev          # Vite dev server (:5173) + Tauri hot-reload + Python server
make dev-ui       # Vite dev server only (no Tauri, no Python)
make build        # Release build: vite build + cargo tauri build
make update       # Pull latest from hermes-webui fork
make icons        # Regenerate icons
```

---

## What Is NOT in v1

- Jira / Confluence / ADO live integrations (commands stubbed, return mock data)
- Project overview page (portfolio / sprint health)
- Workspace setup wizard (workspace root hardcoded to `~/Documents/Delta/` for now)
- PR diff fetched from ADO (PR page shows mock diff in v1)
- `tauri-plugin-stronghold` keychain (API keys in a plain config file in v1, migrate later)

---

## Open Questions (resolved at implementation time)

- hermes-webui fork: which GitHub org hosts it? (team decision before implementation)
- Vite + Tauri dev setup: use `tauri dev` with `beforeDevCommand: "npm run dev"` or run separately?
- `.thread.json` format: match hermes-webui's existing session JSON shape to enable future compatibility
