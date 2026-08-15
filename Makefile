.DEFAULT_GOAL := run

.PHONY: install dev run build build-dmg

# Node deps and PATH-forwarding sidecar stubs so Tauri can start.
install:
	./scripts/fetch-sidecars.sh
	npm install

# Hot-reload the desktop app (Vite frontend + Rust/Tauri).
dev run:
	npm run tauri dev

# Production .app (no disk image). Sidecars must already be in src-tauri/binaries/.
build:
	npm run tauri build -- --bundles app

# Production .dmg (builds the .app as part of packaging).
build-dmg:
	npm run tauri build -- --bundles dmg
