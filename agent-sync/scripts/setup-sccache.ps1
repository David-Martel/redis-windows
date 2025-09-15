# PowerShell script to setup sccache for Windows
# This script installs and configures sccache for faster Rust compilation

$ErrorActionPreference = "Stop"

Write-Host "Setting up sccache for faster Rust builds..." -ForegroundColor Green

# Check if sccache is already installed
$sccacheInstalled = Get-Command sccache -ErrorAction SilentlyContinue

if (-not $sccacheInstalled) {
    Write-Host "Installing sccache..." -ForegroundColor Yellow
    cargo install sccache

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to install sccache"
        exit 1
    }
} else {
    Write-Host "sccache is already installed" -ForegroundColor Green
}

# Configure sccache environment variables
Write-Host "Configuring sccache..." -ForegroundColor Yellow

# Set cache directory
$cacheDir = Join-Path $env:LOCALAPPDATA "sccache"
if (-not (Test-Path $cacheDir)) {
    New-Item -ItemType Directory -Path $cacheDir -Force | Out-Null
}

# Set environment variables for current session
$env:SCCACHE_DIR = $cacheDir
$env:RUSTC_WRAPPER = "sccache"
$env:SCCACHE_CACHE_SIZE = "10G"
$env:SCCACHE_IDLE_TIMEOUT = "0"

# Set environment variables permanently for user
[System.Environment]::SetEnvironmentVariable("SCCACHE_DIR", $cacheDir, [System.EnvironmentVariableTarget]::User)
[System.Environment]::SetEnvironmentVariable("RUSTC_WRAPPER", "sccache", [System.EnvironmentVariableTarget]::User)
[System.Environment]::SetEnvironmentVariable("SCCACHE_CACHE_SIZE", "10G", [System.EnvironmentVariableTarget]::User)
[System.Environment]::SetEnvironmentVariable("SCCACHE_IDLE_TIMEOUT", "0", [System.EnvironmentVariableTarget]::User)

Write-Host "Environment variables set:" -ForegroundColor Green
Write-Host "  SCCACHE_DIR: $cacheDir"
Write-Host "  RUSTC_WRAPPER: sccache"
Write-Host "  SCCACHE_CACHE_SIZE: 10G"
Write-Host "  SCCACHE_IDLE_TIMEOUT: 0"

# Start sccache server
Write-Host "`nStarting sccache server..." -ForegroundColor Yellow
sccache --start-server

if ($LASTEXITCODE -eq 0) {
    Write-Host "sccache server started successfully" -ForegroundColor Green
} else {
    Write-Host "sccache server may already be running" -ForegroundColor Yellow
}

# Show sccache statistics
Write-Host "`nsccache statistics:" -ForegroundColor Yellow
sccache --show-stats

Write-Host "`n✅ sccache setup complete!" -ForegroundColor Green
Write-Host "Note: You may need to restart your terminal for environment variables to take effect." -ForegroundColor Yellow
