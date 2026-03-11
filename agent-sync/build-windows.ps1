#!/usr/bin/env powershell
# Cross-platform build script for agent-sync Rust project on Windows

param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$Release,
    [switch]$Test,
    [switch]$Clean,
    [switch]$Verbose,
    [switch]$Service,
    [switch]$Clippy,
    [switch]$Deny
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

# Load CargoTools from standard module path (auto-discovered by PowerShell)
Import-Module CargoTools -ErrorAction SilentlyContinue
$hasCargoTools = $null -ne (Get-Module CargoTools)
if ($hasCargoTools) {
    $toolchainInfo = Get-CargoEnvironment
    if (-not (Test-RustToolchain)) {
        Write-Host "Error: Rust toolchain validation failed" -ForegroundColor Red
        exit 1
    }
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

# sccache health check
$sccacheOk = $false
try {
    $null = & sccache --show-stats 2>&1
    if ($LASTEXITCODE -eq 0) {
        $sccacheOk = $true
        Write-Verbose "sccache is available and responding"
    }
} catch {
    Write-Verbose "sccache not available; building without cache acceleration"
}

# Clean if requested
if ($Clean) {
    Write-Step "Cleaning build artifacts"
    if (Test-Path "target") { Remove-Item -Recurse -Force "target" }
    & cargo clean
}

# cargo deny check if requested
if ($Deny) {
    Write-Step "Running cargo deny check"
    & cargo deny check
    if ($LASTEXITCODE -ne 0) {
        Write-Host "cargo deny check failed" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    Write-Success "cargo deny check passed"
}

# Clippy if requested
if ($Clippy) {
    Write-Step "Running clippy"
    $clippyArgs = @("clippy", "--target", $Target, "--", "-D", "warnings")
    if ($Verbose) { $clippyArgs += "--verbose" }
    & cargo @clippyArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Clippy check failed" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    Write-Success "Clippy passed"
}

# Build library
Write-Step "Building library for $Target"
$buildArgs = @("build", "--target", $Target)
if ($Release) { $buildArgs += "--release" }
if ($Verbose) { $buildArgs += "--verbose" }

if ($hasCargoTools) {
    $additionalArgs = @("--target", $Target)
    if ($Release) { $additionalArgs += "--release" }
    if ($Verbose) { $additionalArgs += "--verbose" }
    Invoke-CargoWrapper -Command build -AdditionalArgs $additionalArgs
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
        $additionalArgs = @("--bin", "redis-service", "--features", "service", "--target", $Target)
        if ($Release) { $additionalArgs += "--release" }
        if ($Verbose) { $additionalArgs += "--verbose" }
        Invoke-CargoWrapper -Command build -AdditionalArgs $additionalArgs
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

    # Prefer cargo-nextest when available
    $hasNextest = $null -ne (Get-Command "cargo-nextest" -ErrorAction SilentlyContinue)
    if (-not $hasNextest) {
        # Also check via cargo subcommand discovery
        $nextestCheck = & cargo nextest --version 2>&1
        $hasNextest = $LASTEXITCODE -eq 0
    }

    if ($hasNextest) {
        Write-Verbose "Using cargo-nextest for test execution"
        $nextestArgs = @("nextest", "run", "--target", $Target)
        if ($Release) { $nextestArgs += "--release" }
        & cargo @nextestArgs
    } elseif ($hasCargoTools) {
        $additionalArgs = @("--target", $Target)
        if ($Release) { $additionalArgs += "--release" }
        Invoke-CargoWrapper -Command test -AdditionalArgs $additionalArgs
    } else {
        & cargo @testArgs
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Tests failed" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    Write-Success "All tests passed"
}

Write-Success "Build completed successfully"
