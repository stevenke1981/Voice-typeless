#!/usr/bin/env pwsh
# Build Voice-typeless slim edition for Windows 7
# CPU-only inference; no DirectML
# NOTE: Rust core replaces Go; uses Tauri + core-rs build
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "Building Voice-typeless (Windows 7 slim)..." -ForegroundColor Yellow

# Build frontend
Push-Location "$PSScriptRoot\..\frontend"
npm run build
Pop-Location

# Build Tauri app (core-rs included as workspace dep)
Push-Location "$PSScriptRoot\..\src-tauri"
cargo tauri build
Pop-Location

Write-Host "Win7 slim build complete!" -ForegroundColor Green
