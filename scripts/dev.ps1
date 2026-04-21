#!/usr/bin/env pwsh
# Start Voice-typeless dev environment (Tauri CLI via npm)
Set-StrictMode -Version Latest

Write-Host ""
Write-Host "  Starting Voice-typeless dev environment..." -ForegroundColor Cyan
Write-Host "  Frontend WebView: http://localhost:1420" -ForegroundColor Gray
Write-Host ""

# Configure MSVC env for cargo (handles VS2025's incomplete toolset)
. "$PSScriptRoot\env-msvc.ps1"

Push-Location "$PSScriptRoot\.."

# Prefer npm-installed Tauri CLI (faster, no cargo compile needed)
if (Test-Path "frontend/node_modules/.bin/tauri") {
    Set-Location frontend
    npx tauri dev
} elseif (Get-Command "cargo" -ErrorAction SilentlyContinue) {
    cargo tauri dev
} else {
    Write-Error "Tauri CLI not found. Run 'make install' first."
    exit 1
}

Pop-Location
