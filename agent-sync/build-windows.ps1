#!/usr/bin/env powershell
# Cross-platform build script for agent-sync Rust project on Windows

param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$Release,
    [switch]$Test,
    [switch]$Clean,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Step)
    Write-Host "==> $Step" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "Success: $Message" -ForegroundColor Green
}

# Verify environment
Write-Step "Verifying build environment"
try {
    $rustVersion = & rustc --version
    Write-Success "Rust: $rustVersion"
} catch {
    Write-Host "Error: Rust not found" -ForegroundColor Red
    exit 1
}

# Clean if requested
if ($Clean) {
    Write-Step "Cleaning build artifacts"
    if (Test-Path "target") { Remove-Item -Recurse -Force "target" }
    & cargo clean
}

# Build
Write-Step "Building for $Target"
$buildArgs = @("build", "--target", $Target)
if ($Release) { $buildArgs += "--release" }
if ($Verbose) { $buildArgs += "--verbose" }

& cargo @buildArgs
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed" -ForegroundColor Red
    exit $LASTEXITCODE
}

# Test if requested
if ($Test) {
    Write-Step "Running tests"
    $testArgs = @("test", "--target", $Target)
    if ($Release) { $testArgs += "--release" }
    & cargo @testArgs
}

Write-Success "Build completed successfully"
