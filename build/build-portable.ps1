#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build Voice-typeless portable edition (zip archive, no installer).

.DESCRIPTION
    1. Build frontend (npm)
    2. Build Tauri binary
    3. Assemble portable folder: exe + portable.txt + optional models
    4. Package as Voice-typeless-v{VERSION}-portable.zip

    The resulting zip can be extracted anywhere and run immediately —
    no installation, no registry, no %APPDATA% footprint.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ── Paths ──────────────────────────────────────────────────────────────────────
$Root    = Resolve-Path "$PSScriptRoot\.."
$Frontend = "$Root\frontend"
$TauriSrc = "$Root\src-tauri"
$ReleaseBin = "$TauriSrc\target\release\voice-typeless.exe"
$ModelsSrc  = "$Root\models"

# Read version from tauri.conf.json
$TauriConf = Get-Content "$TauriSrc\tauri.conf.json" -Raw | ConvertFrom-Json
$Version   = $TauriConf.version

$PortableDir  = "$Root\dist\Voice-typeless-v$Version-portable"
$PortableZip  = "$Root\dist\Voice-typeless-v$Version-portable.zip"
$MarkerFile   = "$PortableDir\portable.txt"
$ModelsTarget = "$PortableDir\models\sensevoice-small"

# ── Step 1 — Build frontend ─────────────────────────────────────────────────────
Write-Host "`n[1/4] Building frontend…" -ForegroundColor Cyan
Push-Location $Frontend
npm run build
if ($LASTEXITCODE -ne 0) { throw "Frontend build failed" }
Pop-Location

# ── Step 2 — Build Tauri binary ────────────────────────────────────────────────
Write-Host "`n[2/4] Building Tauri binary…" -ForegroundColor Cyan
Push-Location $TauriSrc
# `cargo tauri build` bakes the frontend dist into the binary.
# (MSI/NSIS installers will also be created in target/release/bundle/
#  but we only need the standalone EXE from target/release/.)
cargo tauri build *>&1
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed" }
Pop-Location

# Verify binary exists
if (-not (Test-Path $ReleaseBin)) {
    throw "Release binary not found at $ReleaseBin"
}
$BinSize = (Get-Item $ReleaseBin).Length
Write-Host "       Binary: $([math]::Round($BinSize / 1KB)) KB"

# ── Step 3 — Assemble portable folder ──────────────────────────────────────────
Write-Host "`n[3/4] Assembling portable folder…" -ForegroundColor Cyan

# Create portable directory (fresh)
if (Test-Path $PortableDir) {
    Remove-Item -Recurse -Force $PortableDir
}
$null = New-Item -ItemType Directory -Path $PortableDir -Force

# Copy executable
Copy-Item $ReleaseBin "$PortableDir\voice-typeless.exe"

# Create portable.txt marker (empty file)
$null = New-Item -ItemType File -Path $MarkerFile -Force

# Optionally bundle model files (if they exist locally)
if (Test-Path "$ModelsSrc\sensevoice-small\model.int8.onnx") {
    Write-Host "       Bundling model files (sensevoice-small)…"
    $null = New-Item -ItemType Directory -Path $ModelsTarget -Force
    Copy-Item "$ModelsSrc\sensevoice-small\model.int8.onnx" $ModelsTarget
    Copy-Item "$ModelsSrc\sensevoice-small\tokens.txt"      $ModelsTarget

    $ModelSize = (Get-Item "$ModelsTarget\model.int8.onnx").Length
    Write-Host "         model.int8.onnx : $([math]::Round($ModelSize / 1MB)) MB"
}

# ── Step 4 — Package as zip ─────────────────────────────────────────────────────
Write-Host "`n[4/4] Packaging as ZIP…" -ForegroundColor Cyan

# Remove any previous zip
if (Test-Path $PortableZip) { Remove-Item -Force $PortableZip }

Compress-Archive -Path "$PortableDir\*" -DestinationPath $PortableZip

$ZipSize = (Get-Item $PortableZip).Length
Write-Host "       Archive: $PortableZip"
Write-Host "       Size:    $([math]::Round($ZipSize / 1MB)) MB"

# ── Done ────────────────────────────────────────────────────────────────────────
Write-Host "`n✅ Portable build complete!" -ForegroundColor Green
Write-Host "   Folder: $PortableDir"
Write-Host "   ZIP:    $PortableZip"
