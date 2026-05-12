.PHONY: build build-deb dev icons sync-upstream clean help

TAURI_CLI_VERSION := ^2.0
SRC_TAURI        := src-tauri
ICON_SRC         := hermes-webui/static/favicon-512.png
ICON_DIR         := $(SRC_TAURI)/icons

help:
	@echo "Hermes Desktop — available targets:"
	@echo "  make build          Build the Tauri app for the current platform"
	@echo "  make build-deb      Build .deb package only (Linux)"
	@echo "  make dev            Run in development mode (hot-reload webview)"
	@echo "  make icons          Regenerate icons from hermes-webui/static/favicon-512.png"
	@echo "  make sync-upstream  Pull latest hermes-webui and commit the submodule bump"
	@echo "  make clean          Remove build artifacts"

# ── Build ─────────────────────────────────────────────────────────────────────
build: icons _ensure-tauri
	cd $(SRC_TAURI) && cargo tauri build
	@echo ""
	@echo "Artifacts:"
	@find $(SRC_TAURI)/target/release/bundle \( -name "*.deb" -o -name "*.AppImage" -o -name "*.dmg" -o -name "*.exe" \) 2>/dev/null | sed 's/^/  /'

build-deb: icons _ensure-tauri
	cd $(SRC_TAURI) && cargo tauri build --bundles deb
	@echo ""
	@echo "Artifact:"
	@find $(SRC_TAURI)/target/release/bundle/deb -name "*.deb" 2>/dev/null | sed 's/^/  /'

# ── Dev mode ──────────────────────────────────────────────────────────────────
dev: _ensure-tauri
	cd $(SRC_TAURI) && cargo tauri dev

# ── Icons ─────────────────────────────────────────────────────────────────────
icons: $(ICON_DIR)/32x32.png

$(ICON_DIR)/32x32.png: $(ICON_SRC)
	@mkdir -p $(ICON_DIR)
	@if command -v convert >/dev/null 2>&1; then \
	  convert $(ICON_SRC) -resize 32x32   $(ICON_DIR)/32x32.png; \
	  convert $(ICON_SRC) -resize 128x128 $(ICON_DIR)/128x128.png; \
	  convert $(ICON_SRC) -resize 256x256 $(ICON_DIR)/256x256.png; \
	elif command -v sips >/dev/null 2>&1; then \
	  sips -z 32  32  $(ICON_SRC) --out $(ICON_DIR)/32x32.png; \
	  sips -z 128 128 $(ICON_SRC) --out $(ICON_DIR)/128x128.png; \
	  sips -z 256 256 $(ICON_SRC) --out $(ICON_DIR)/256x256.png; \
	elif command -v ffmpeg >/dev/null 2>&1; then \
	  ffmpeg -i $(ICON_SRC) -vf scale=32:32   $(ICON_DIR)/32x32.png   -y -frames:v 1 -q:v 1 2>/dev/null; \
	  ffmpeg -i $(ICON_SRC) -vf scale=128:128 $(ICON_DIR)/128x128.png -y -frames:v 1 -q:v 1 2>/dev/null; \
	  ffmpeg -i $(ICON_SRC) -vf scale=256:256 $(ICON_DIR)/256x256.png -y -frames:v 1 -q:v 1 2>/dev/null; \
	else \
	  cp $(ICON_SRC) $(ICON_DIR)/32x32.png; \
	  cp $(ICON_SRC) $(ICON_DIR)/128x128.png; \
	  cp $(ICON_SRC) $(ICON_DIR)/256x256.png; \
	  echo "Warning: ImageMagick/sips/ffmpeg not found — all icon sizes are identical"; \
	fi
	@cp $(ICON_SRC) $(ICON_DIR)/512x512.png
	@cp $(ICON_SRC) $(ICON_DIR)/icon.png
	@echo "Icons generated in $(ICON_DIR)/"

# ── Upstream sync ─────────────────────────────────────────────────────────────
sync-upstream:
	git submodule update --remote --merge hermes-webui
	@if git diff --quiet hermes-webui; then \
	  echo "Already up to date."; \
	else \
	  NEW_SHA=$$(git -C hermes-webui rev-parse --short HEAD); \
	  git add hermes-webui; \
	  git commit -m "chore: bump hermes-webui to $${NEW_SHA}"; \
	  echo "Bumped hermes-webui to $${NEW_SHA}"; \
	fi

# ── Clean ─────────────────────────────────────────────────────────────────────
clean:
	cd $(SRC_TAURI) && cargo clean

# ── Internal ──────────────────────────────────────────────────────────────────
_ensure-tauri:
	@if ! cargo tauri --version >/dev/null 2>&1; then \
	  echo "[tauri] Installing tauri-cli $(TAURI_CLI_VERSION)..."; \
	  cargo install tauri-cli --version "$(TAURI_CLI_VERSION)" --locked; \
	fi
