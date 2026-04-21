#!/usr/bin/env pwsh
# Voice-typeless — prerequisite installer & checker
Set-StrictMode -Version Latest
$ErrorActionPreference = "Continue"
$ok = $true

function Check($name, $cmd) {
    try {
        $ver = Invoke-Expression $cmd 2>&1 | Select-Object -First 1
        Write-Host "  [OK] $name — $ver" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "  [MISSING] $name" -ForegroundColor Red
        return $false
    }
}

function TryInstall($name, $wingetId, $chocoId, $manualUrl) {
    Write-Host "  → Installing $name..." -ForegroundColor Yellow
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        winget install $wingetId -e --silent --accept-package-agreements --accept-source-agreements 2>&1
        if ($LASTEXITCODE -eq 0) { Write-Host "  [INSTALLED] $name via winget" -ForegroundColor Green; return }
    }
    if (Get-Command choco -ErrorAction SilentlyContinue) {
        choco install $chocoId -y 2>&1
        if ($LASTEXITCODE -eq 0) { Write-Host "  [INSTALLED] $name via chocolatey" -ForegroundColor Green; return }
    }
    Write-Host "  [MANUAL] Install $name from: $manualUrl" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "  Voice-typeless — Environment Setup" -ForegroundColor Cyan
Write-Host "  =====================================" -ForegroundColor Cyan
Write-Host ""

# ── MSVC environment (fixes VS2025 incomplete toolset) ───────────────────────
Write-Host ""
Write-Host "  Configuring MSVC build environment..." -ForegroundColor Cyan
. "$PSScriptRoot\env-msvc.ps1"

# ── Go ──────────────────────────────────────────────────────────────────────
$hasGo = Check "Go 1.23+" "go version"
if (-not $hasGo) {
    TryInstall "Go" "GoLang.Go" "golang" "https://go.dev/dl/"
    # Refresh PATH after install
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")
    $hasGo = Check "Go (after install)" "go version"
}

# ── Rust/Cargo ──────────────────────────────────────────────────────────────
$hasRust = Check "Rust/Cargo" "cargo --version"
if (-not $hasRust) {
    Write-Host "  → Install Rust from https://rustup.rs" -ForegroundColor Cyan
    $ok = $false
}

# ── Node.js ──────────────────────────────────────────────────────────────────
$hasNode = Check "Node.js" "node --version"
if (-not $hasNode) {
    TryInstall "Node.js" "OpenJS.NodeJS.LTS" "nodejs-lts" "https://nodejs.org/"
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")
    $hasNode = Check "Node.js (after install)" "node --version"
}

# ── npm ──────────────────────────────────────────────────────────────────────
Check "npm" "npm --version" | Out-Null

# ── WebView2 ─────────────────────────────────────────────────────────────────
$wv2 = Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if ($wv2) {
    Write-Host "  [OK] WebView2 Runtime — $($wv2.pv)" -ForegroundColor Green
} else {
    Write-Host "  [MISSING] WebView2 Runtime — install from https://developer.microsoft.com/microsoft-edge/webview2/" -ForegroundColor Red
    $ok = $false
}

# ── Frontend deps ─────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  Installing frontend dependencies..." -ForegroundColor Cyan
Push-Location "$PSScriptRoot\..\frontend"
npm install
if ($LASTEXITCODE -ne 0) {
    Write-Host "  [ERROR] npm install failed" -ForegroundColor Red
    $ok = $false
} else {
    Write-Host "  [OK] Frontend dependencies installed" -ForegroundColor Green
}
Pop-Location

Write-Host ""
if (-not $ok) {
    Write-Host "  Some prerequisites are missing. Fix them and re-run setup." -ForegroundColor Red
    exit 1
}
Write-Host "  All prerequisites satisfied!" -ForegroundColor Green
Write-Host "  Run 'make dev' or 'scripts/dev.ps1' to start development." -ForegroundColor Cyan
Write-Host "  NOTE: Go is required to build core/. Install from https://go.dev/dl/ if missing." -ForegroundColor Yellow
Write-Host ""
