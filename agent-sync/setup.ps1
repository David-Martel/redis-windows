# PowerShell script to setup the high-performance build environment
# Run this script to install all required tools

$ErrorActionPreference = "Stop"

Write-Host "==================================" -ForegroundColor Cyan
Write-Host "agent-sync Build System Setup" -ForegroundColor Cyan
Write-Host "==================================" -ForegroundColor Cyan

# Function to check if a command exists
function Test-CommandExists {
    param($command)
    $null = Get-Command $command -ErrorAction SilentlyContinue
    return $?
}

# Install Rust if not present
if (-not (Test-CommandExists "cargo")) {
    Write-Host "`nInstalling Rust..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri "https://win.rustup.rs" -OutFile "rustup-init.exe"
    .\rustup-init.exe -y
    Remove-Item "rustup-init.exe"
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
}

Write-Host "`nInstalling build tools..." -ForegroundColor Yellow

# Install cargo-make
if (-not (Test-CommandExists "cargo-make")) {
    Write-Host "Installing cargo-make..." -ForegroundColor Green
    cargo install cargo-make
}

# Install just
if (-not (Test-CommandExists "just")) {
    Write-Host "Installing just..." -ForegroundColor Green
    cargo install just
}

# Install cargo-nextest
if (-not (Test-CommandExists "cargo-nextest")) {
    Write-Host "Installing cargo-nextest..." -ForegroundColor Green
    cargo install cargo-nextest
}

# Install sccache
if (-not (Test-CommandExists "sccache")) {
    Write-Host "Installing sccache..." -ForegroundColor Green
    cargo install sccache
}

# Install additional tools
Write-Host "`nInstalling additional development tools..." -ForegroundColor Yellow
cargo install cargo-watch --quiet 2>$null
cargo install cargo-edit --quiet 2>$null
cargo install cargo-outdated --quiet 2>$null
cargo install cargo-audit --quiet 2>$null
cargo install cargo-deny --quiet 2>$null

# Setup sccache
Write-Host "`nConfiguring sccache..." -ForegroundColor Yellow
& ".\scripts\setup-sccache.ps1"

# Verify installation
Write-Host "`n==================================" -ForegroundColor Green
Write-Host "Installation Complete!" -ForegroundColor Green
Write-Host "==================================" -ForegroundColor Green

Write-Host "`nInstalled tools:" -ForegroundColor Cyan
Write-Host "  ✓ cargo-make - Task automation" -ForegroundColor White
Write-Host "  ✓ just - Command runner" -ForegroundColor White
Write-Host "  ✓ cargo-nextest - Fast test runner" -ForegroundColor White
Write-Host "  ✓ sccache - Build caching" -ForegroundColor White

Write-Host "`nQuick start commands:" -ForegroundColor Cyan
Write-Host "  cargo make dev     - Development build cycle" -ForegroundColor White
Write-Host "  cargo make test    - Run tests" -ForegroundColor White
Write-Host "  cargo make ci      - Run CI pipeline" -ForegroundColor White
Write-Host "  just build        - Build project" -ForegroundColor White
Write-Host "  just test         - Run tests with nextest" -ForegroundColor White

Write-Host "`nFor more commands, run:" -ForegroundColor Yellow
Write-Host "  cargo make --list-all-steps" -ForegroundColor White
Write-Host "  just --list" -ForegroundColor White

Write-Host "`nRefer to BUILD.md for detailed documentation." -ForegroundColor Green
