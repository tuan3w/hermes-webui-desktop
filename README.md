# Delta

AI-native workspace for Scrum teams. Every work item — spec, research topic, PR, bug — carries its own persistent agent thread. No re-explaining context. No switching tools.

Built with [Tauri v2](https://tauri.app/) + [Hermes Web UI](https://github.com/nesquena/hermes-webui).

## What it is

Most teams bolt AI on top of existing tools — a chatbot next to Jira, a Copilot next to the editor. Delta takes the opposite approach: the agent is **inside** the work item, not alongside it.

- Open a spec → your BA agent loads with the story context already there
- Open a PR → your dev agent already knows the acceptance criteria it maps to
- Open a research pipeline → your AI research agent resumes exactly where it left off
- Files are the source of truth — no database, no sync lock-in

## Roles

| Role | Primary work items |
|------|--------------------|
| PO / BA | Story specs (story.md), prototype review |
| PM | Project portfolio, sprint health, stakeholder reports |
| SA | Tech specs (tech.md), architecture decisions |
| AI Engineers | Research pipeline (5 stages), dataset, eval/benchmark |
| Fullstack Engineers | PR review, tech spec, debug session |
| QA / QC | Spec verification, bug reports, test automation |

## Architecture

```
delta/
├── src-tauri/          # Tauri v2 shell (Rust)
│   ├── src/lib.rs      # App startup: venv setup, server spawn, updater
│   └── tauri.conf.json
├── desktop/            # Splash screen (shown during Python server boot)
├── hermes-webui/       # Git submodule — Python FastAPI + React chat UI
└── scripts/
    └── build-desktop.sh
```

On launch, Delta boots a local Python server (via bundled [uv](https://github.com/astral-sh/uv)) and loads the UI at `127.0.0.1:{port}`. No system Python required.

## Building

**Prerequisites:** Rust stable, `curl`

```bash
git clone --recurse-submodules https://github.com/tuan3w/delta
cd delta
bash scripts/build-desktop.sh
```

Artifacts land in `src-tauri/target/release/bundle/`.

## Download

Latest release: [Releases page](https://github.com/tuan3w/delta/releases/latest)

| Platform | File |
|----------|------|
| macOS Apple Silicon | `delta_*_aarch64.dmg` |
| macOS Intel | `delta_*_x64.dmg` |
| Linux (Debian/Ubuntu) | `delta_*_amd64.deb` |
| Windows | `delta_*_x64-setup.exe` |

## macOS — first launch

The app is unsigned. On first open: right-click → **Open** → **Open**.

Or from terminal:

```bash
xattr -dr com.apple.quarantine /Applications/Delta.app
```

## Status

Early development. Core shell is working — Python server boot, auto-update, single-instance window. The work-item workspace UI is in active design.
