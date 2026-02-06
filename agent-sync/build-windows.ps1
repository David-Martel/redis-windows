#!/usr/bin/env powershell
# Cross-platform build script for agent-sync Rust project on Windows

param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$Release,
    [switch]$Test,
    [switch]$Clean,
    [switch]$Verbose,
    [switch]$Service
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

# Check for CargoTools module
$cargoToolsPath = "C:\Users\david\wezterm\tools\CargoTools\CargoTools.psm1"
$hasCargoTools = Test-Path $cargoToolsPath
if ($hasCargoTools) {
    Import-Module $cargoToolsPath -Force
    Initialize-CargoEnv
    Write-Step "Using CargoTools for build orchestration (sccache enabled)"
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

# Build library
Write-Step "Building library for $Target"
$buildArgs = @("build", "--target", $Target)
if ($Release) { $buildArgs += "--release" }
if ($Verbose) { $buildArgs += "--verbose" }

if ($hasCargoTools) {
    Invoke-CargoWrapper build --target $Target
} else {
    & cargo @buildArgs
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "Library build failed" -ForegroundColor Red
    exit $LASTEXITCODE
}
Write-Success "Library build completed"

# Build service binary if requested
if ($Service) {
    Write-Step "Building redis-service binary for $Target"
    $serviceArgs = @("build", "--bin", "redis-service", "--features", "service", "--target", $Target)
    if ($Release) { $serviceArgs += "--release" }
    if ($Verbose) { $serviceArgs += "--verbose" }

    if ($hasCargoTools) {
        $cargoArgs = @("--bin", "redis-service", "--features", "service", "--target", $Target)
        if ($Release) { $cargoArgs += "--release" }
        Invoke-CargoWrapper build @cargoArgs
    } else {
        & cargo @serviceArgs
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Service binary build failed" -ForegroundColor Red
        exit $LASTEXITCODE
    }

    $profile = if ($Release) { "release" } else { "debug" }
    $binaryPath = Join-Path $PSScriptRoot "target\$Target\$profile\redis-service.exe"
    if (Test-Path $binaryPath) {
        $size = [math]::Round((Get-Item $binaryPath).Length / 1MB, 2)
        Write-Success "Service binary: $binaryPath ($size MB)"
    }
}

# Test if requested
if ($Test) {
    Write-Step "Running tests"
    $testArgs = @("test", "--target", $Target)
    if ($Release) { $testArgs += "--release" }

    if ($hasCargoTools) {
        Invoke-CargoWrapper test --target $Target
    } else {
        & cargo @testArgs
    }
}

Write-Success "Build completed successfully"


