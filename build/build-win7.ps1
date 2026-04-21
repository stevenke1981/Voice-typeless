#!/usr/bin/env pwsh
# Build Voice-typeless slim edition for Windows 7
# CPU-only inference; no DirectML
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "Building Voice-typeless (Windows 7 slim)..." -ForegroundColor Yellow

Push-Location "$PSScriptRoot\..\core"
$env:GOFLAGS = '-tags=win7'
go build ./...
Pop-Location

Write-Host "Win7 slim build complete!" -ForegroundColor Green
