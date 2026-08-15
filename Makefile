.DEFAULT_GOAL := run

.PHONY: install dev run

# Node deps and PATH-forwarding sidecar stubs so Tauri can start.
install:
	./scripts/fetch-sidecars.sh
	npm install

# Hot-reload the desktop app (Vite frontend + Rust/Tauri).
dev run:
	npm run tauri dev
