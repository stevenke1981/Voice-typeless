.PHONY: dev build test test-core test-frontend clean setup install

# Install all dependencies (run once after cloning)
install:
	cd frontend && npm install

# Start Tauri dev server (frontend + Rust shell)
dev:
	powershell -File scripts/dev.ps1

# Check prerequisites without installing
setup:
	powershell -File scripts/setup.ps1

# Run the reusable Rust core tests
test-core:
	cd core-rs && cargo test

# Run frontend type-check
test-frontend:
	cd frontend && npm run check

test: test-core test-frontend

# Build production bundle
build:
	powershell -Command ". scripts/env-msvc.ps1 -Quiet; Set-Location frontend; npm run build"
	powershell -Command ". scripts/env-msvc.ps1 -Quiet; Set-Location src-tauri; cargo build --release"

# Clean build artifacts (Windows-safe)
clean:
	powershell -Command "if (Test-Path frontend/dist) { Remove-Item -Recurse -Force frontend/dist }; if (Test-Path frontend/node_modules) { Remove-Item -Recurse -Force frontend/node_modules }"
	cd core-rs && cargo clean
	cd src-tauri && cargo clean
