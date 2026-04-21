#!/usr/bin/env pwsh
# Detect and configure MSVC environment for Rust/Cargo builds on Windows.
# Dot-source this script before any cargo command:  . "$PSScriptRoot\env-msvc.ps1"
#
# Problem: VS 2025 (MSVC 14.50+) ships without C++ headers on this machine.
# Solution: fall back to VS 2019 BuildTools (MSVC 14.29) which has the full toolset.

param([switch]$Quiet)

function Find-MsvcToolset {
    param([string]$vsRoot, [string]$versionPattern)
    $msvcBase = Join-Path $vsRoot "VC\Tools\MSVC"
    if (-not (Test-Path $msvcBase)) { return $null }
    Get-ChildItem $msvcBase -Directory |
        Where-Object { $_.Name -match $versionPattern } |
        Where-Object { Test-Path (Join-Path $_.FullName "include\excpt.h") } |
        Sort-Object Name -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}

$sdk = "C:\Program Files (x86)\Windows Kits\10"
$sdkVer = Get-ChildItem "$sdk\Lib" -Directory -ErrorAction SilentlyContinue |
    Sort-Object Name -Descending | Select-Object -First 1 -ExpandProperty Name

# Try VS2019 BuildTools (preferred — known-complete toolset)
$msvcDir = Find-MsvcToolset "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools" "14\."

# Fallback: VS2022
if (-not $msvcDir) {
    $msvcDir = Find-MsvcToolset "C:\Program Files\Microsoft Visual Studio\17\Community" "14\."
}

# Fallback: VS2022 BuildTools
if (-not $msvcDir) {
    $msvcDir = Find-MsvcToolset "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools" "14\."
}

if (-not $msvcDir -or -not $sdkVer) {
    Write-Warning "env-msvc.ps1: Could not find a complete MSVC toolset with headers."
    Write-Warning "Install 'Desktop development with C++' workload via Visual Studio Installer."
    return
}

$env:INCLUDE = "$msvcDir\include;" +
               "$sdk\Include\$sdkVer\ucrt;" +
               "$sdk\Include\$sdkVer\um;" +
               "$sdk\Include\$sdkVer\shared"

$env:LIB     = "$msvcDir\lib\x64;" +
               "$sdk\Lib\$sdkVer\ucrt\x64;" +
               "$sdk\Lib\$sdkVer\um\x64"

$env:PATH    = "$msvcDir\bin\HostX64\x64;$env:PATH"

if (-not $Quiet) {
    Write-Host "  [MSVC] Using: $msvcDir" -ForegroundColor Gray
    Write-Host "  [MSVC] SDK:  $sdkVer" -ForegroundColor Gray
}

## Changelog
# | Version | Date       | Content                                         | Scope       |
# |---------|------------|-------------------------------------------------|-------------|
# | v1.0    | 2026-04-21 | Initial: auto-detect VS2019/2022 MSVC toolset   | scripts/    |
