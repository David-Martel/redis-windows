# Integrated Build Script for Redis for Windows and Rust Coordination
# ====================================================================

param(
    [switch]$BuildService,      # Build .NET Redis service
    [switch]$BuildRust,         # Build agent-sync and rag-redis
    [switch]$Test,              # Run all tests
    [switch]$Lint,              # Run clippy and fmt checks
    [switch]$InstallService,    # Install Redis as Windows service
    [switch]$Help,
    [string]$Config = "Release"
)

$script:CargoToolsPath = "C:\Users\david\wezterm\tools\CargoTools\CargoTools.psm1"

function Import-CargoTools {
    if (Test-Path $script:CargoToolsPath) {
        Import-Module $script:CargoToolsPath -Force
        Initialize-CargoEnv
        return $true
    }
    Write-Error "CargoTools module not found at $script:CargoToolsPath"
    return $false
}

function Build-DotNetService {
    Write-Host "--- Building .NET Redis Service ---" -ForegroundColor Cyan
    dotnet build RedisService.csproj -c $Config
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to build .NET service"
        return $false
    }
    dotnet publish RedisService.csproj -c $Config -r win-x64 --self-contained -o publish
    if (Test-Path ".\publish\RedisService.exe") {
        Copy-Item ".\publish\RedisService.exe" ".\RedisService.exe" -Force
        Write-Host "Success: RedisService.exe updated" -ForegroundColor Green
        return $true
    }
    return $false
}

function Build-RustProjects {
    if (-not (Import-CargoTools)) { return $false }
    
    $projects = @(
        "T:\projectsedis-windows\agent-sync",
        "W:\dropbox-localag-redis"
    )

    foreach ($project in $projects) {
        if (Test-Path $project) {
            Write-Host "--- Building Rust Project: $project ---" -ForegroundColor Cyan
            Push-Location $project
            try {
                Invoke-CargoWrapper build --release
            } finally {
                Pop-Location
            }
        }
    }
    return $true
}

function Run-Lint {
    if (-not (Import-CargoTools)) { return $false }
    
    $projects = @(
        "T:\projectsedis-windows\agent-sync",
        "W:\dropbox-localag-redis"
    )

    foreach ($project in $projects) {
        if (Test-Path $project) {
            Write-Host "--- Linting Rust Project: $project ---" -ForegroundColor Cyan
            Push-Location $project
            try {
                Write-Host "Running clippy..." -ForegroundColor Gray
                Invoke-CargoWrapper clippy --all-targets --all-features -- -D warnings
                
                Write-Host "Running fmt check..." -ForegroundColor Gray
                Invoke-CargoWrapper fmt --all -- --check
            } finally {
                Pop-Location
            }
        }
    }
    return $true
}

function Run-Tests {
    if (-not (Import-CargoTools)) { return $false }
    
    $projects = @(
        "T:\projectsedis-windows\agent-sync",
        "W:\dropbox-localag-redis"
    )

    foreach ($project in $projects) {
        if (Test-Path $project) {
            Write-Host "--- Testing Rust Project: $project ---" -ForegroundColor Cyan
            Push-Location $project
            try {
                Invoke-CargoWrapper test --workspace --all-features
            } finally {
                Pop-Location
            }
        }
    }
    return $true
}

if ($Help) {
    Get-Help $MyInvocation.MyCommand.Path
    exit 0
}

if ($BuildService) { Build-DotNetService }
if ($BuildRust) { Build-RustProjects }
if ($Lint) { Run-Lint }
if ($Test) { Run-Tests }

if ($InstallService) {
    if (-not (Test-Path ".\RedisService.exe")) {
        Build-DotNetService
    }
    .\install-redis-service.ps1
}

if (-not ($BuildService -or $BuildRust -or $Test -or $Lint -or $InstallService)) {
    Write-Host "No actions specified. Use -Help for usage." -ForegroundColor Yellow
}

