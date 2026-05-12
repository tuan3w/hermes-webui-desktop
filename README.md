# Hermes Desktop

Native desktop app for [Hermes Web UI](https://github.com/nesquena/hermes-webui) — a lightweight, dark-themed interface for [Hermes Agent](https://hermes-agent.nousresearch.com/).

Built with [Tauri v2](https://tauri.app/). Ships as a single installer that bundles the Hermes Web UI Python server — no separate install required.

## Platforms

| Platform | Format |
|----------|--------|
| Linux    | `.deb`, `.AppImage` |
| macOS    | `.dmg` (Intel + Apple Silicon) |
| Windows  | `.exe` (NSIS installer) |

## How it works

On first launch the app uses the bundled [uv](https://github.com/astral-sh/uv) binary to:
1. Create a Python 3.11 virtual environment in the app data directory
2. Install the Python dependencies from `requirements.txt`

Subsequent launches skip setup and start the server immediately (~1–2 s).
No system Python is required.

## Building

**Prerequisites:** Rust stable, `curl`

```bash
# Clone with submodule
git clone --recurse-submodules https://github.com/tuan3w/hermes-webui-desktop
cd hermes-webui-desktop

# Build (downloads uv automatically, then builds the Tauri app)
bash scripts/build-desktop.sh
```

Artifacts land in `src-tauri/target/release/bundle/`.

## macOS — first launch warning

The app is not signed with an Apple Developer certificate, so macOS Gatekeeper will block it on first launch.

**One-time fix:**

1. Right-click (or Control-click) the `.dmg` → **Open**
2. Click **Open** in the dialog that appears

Or run this in Terminal after installing:

```bash
xattr -dr com.apple.quarantine /Applications/hermes-webui-desktop.app
```

You only need to do this once.

## Staying up to date with upstream

The Hermes Web UI source lives in `hermes-webui/` as a git submodule pointing at [nesquena/hermes-webui](https://github.com/nesquena/hermes-webui).

A [scheduled workflow](.github/workflows/sync-upstream.yml) bumps the submodule pointer daily and pushes a commit. A [build workflow](.github/workflows/build-release.yml) produces release artifacts whenever a `v*` tag is pushed.

To manually pull the latest upstream:

```bash
git submodule update --remote --merge hermes-webui
git add hermes-webui
git commit -m "chore: bump hermes-webui"
```

## Repository structure

```
hermes-webui-desktop/
├── src-tauri/                  # Tauri wrapper (Rust)
│   ├── src/lib.rs              # App logic: venv setup, server spawn
│   ├── build.rs                # Downloads uv binary at build time
│   └── tauri.conf.json
├── desktop/                    # Splash screen shown during startup
├── scripts/build-desktop.sh    # Local build helper
├── .github/workflows/
│   ├── sync-upstream.yml       # Daily submodule bump
│   └── build-release.yml      # Multi-platform build + GitHub Release
└── hermes-webui/               # git submodule → nesquena/hermes-webui
```
