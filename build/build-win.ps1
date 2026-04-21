#!/usr/bin/env pwsh
# Build Voice-typeless for Windows 10/11
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "Building Voice-typeless (Windows 10/11)..." -ForegroundColor Cyan

# Build frontend
Push-Location "$PSScriptRoot\..\frontend"
npm run build
Pop-Location

# Build Tauri app
Push-Location "$PSScriptRoot\..\src-tauri"
cargo tauri build
Pop-Location

Write-Host "Build complete!" -ForegroundColor Green
