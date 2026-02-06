#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Install or update the Redis Windows Service (redis-service.exe wrapper).
.DESCRIPTION
    Copies the Rust redis-service binary, Redis binaries, and config to
    C:\Program Files\Redis, registers as a Windows Service, starts it,
    and verifies Redis responds on port 6380.
.PARAMETER ServiceName
    Windows service name (default: Redis)
.PARAMETER DisplayName
    Display name shown in services.msc (default: Redis for Windows Agent)
.PARAMETER Uninstall
    Stop and remove the service, leaving files in place
.PARAMETER SkipVerify
    Skip the post-install verification steps
.EXAMPLE
    .\install-redis-service.ps1
    .\install-redis-service.ps1 -Uninstall
#>
param(
    [string]$ServiceName = "Redis",
    [string]$DisplayName = "Redis for Windows Agent",
    [switch]$Uninstall,
    [switch]$SkipVerify
)

$ErrorActionPreference = "Stop"
$ServiceDir = "C:\Program Files\Redis"
$ProjectRoot = $PSScriptRoot

# --- Uninstall path ---
if ($Uninstall) {
    Write-Host "Stopping and removing service '$ServiceName'..." -ForegroundColor Yellow
    Stop-Service $ServiceName -ErrorAction SilentlyContinue
    & sc.exe delete $ServiceName 2>&1 | Out-Null
    Write-Host "Service removed." -ForegroundColor Green
    exit 0
}

# --- Locate the release binary ---
$ExeCandidates = @(
    "T:\RustCache\cargo-target\x86_64-pc-windows-msvc\release\redis-service.exe",
    (Join-Path $ProjectRoot "agent-sync\target\x86_64-pc-windows-msvc\release\redis-service.exe"),
    (Join-Path $ProjectRoot "agent-sync\target\release\redis-service.exe")
)
$ExeSource = $ExeCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $ExeSource) {
    Write-Error "redis-service.exe not found. Build with: cd agent-sync && cargo build --features service --release"
    exit 1
}
Write-Host "Source binary: $ExeSource" -ForegroundColor Cyan

# --- Ensure target directory exists ---
if (-not (Test-Path $ServiceDir)) {
    New-Item -ItemType Directory -Path $ServiceDir -Force | Out-Null
}

# --- Stop existing service before replacing binary ---
$svc = Get-Service $ServiceName -ErrorAction SilentlyContinue
if ($svc -and $svc.Status -ne 'Stopped') {
    Write-Host "Stopping existing service..." -ForegroundColor Yellow
    Stop-Service $ServiceName -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

# --- Copy files ---
Write-Host "Deploying files to $ServiceDir..." -ForegroundColor Cyan

Copy-Item $ExeSource (Join-Path $ServiceDir "redis-service.exe") -Force
Write-Host "  redis-service.exe  ($('{0:N1} MB' -f ((Get-Item $ExeSource).Length / 1MB)))"

$requiredFiles = @("redis-server.exe", "redis-cli.exe", "redis-agent.conf")
$optionalFiles = @("redis-benchmark.exe", "redis-check-aof.exe", "redis-check-rdb.exe", "redis-sentinel.exe")
# MSYS2 runtime DLLs required by Redis 8.x MSYS2 builds
$msysDlls = Get-ChildItem (Join-Path $ProjectRoot "msys-*.dll") -ErrorAction SilentlyContinue

foreach ($file in ($requiredFiles + $optionalFiles)) {
    $src = Join-Path $ProjectRoot $file
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $ServiceDir $file) -Force
        Write-Host "  $file"
    } else {
        $existing = Join-Path $ServiceDir $file
        if ($requiredFiles -contains $file) {
            if (Test-Path $existing) {
                Write-Host "  $file  (keeping existing)" -ForegroundColor DarkGray
            } else {
                Write-Warning "  $file not found in project root or service dir"
            }
        }
    }
}

# Copy MSYS2 DLLs alongside redis-server.exe
foreach ($dll in $msysDlls) {
    Copy-Item $dll.FullName (Join-Path $ServiceDir $dll.Name) -Force
    Write-Host "  $($dll.Name)" -ForegroundColor DarkGray
}

# --- Remove stale .NET symlink if present ---
$staleLink = Join-Path $ServiceDir "RedisService.exe"
if (Test-Path $staleLink) {
    Remove-Item $staleLink -Force -ErrorAction SilentlyContinue
    Write-Host "  Removed stale RedisService.exe symlink" -ForegroundColor DarkGray
}

# --- Register and start service ---
$binPath = "`"$ServiceDir\redis-service.exe`" -c `"$ServiceDir\redis-agent.conf`""

# Check if service already exists
$existingSvc = Get-Service $ServiceName -ErrorAction SilentlyContinue
if ($existingSvc) {
    # Update the binary path in case it changed
    & sc.exe config $ServiceName binPath= $binPath start= auto | Out-Null
    Write-Host "Updated service binary path." -ForegroundColor Cyan
} else {
    & sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= $DisplayName
    Write-Host "Created service '$ServiceName'." -ForegroundColor Green
}

Write-Host "Starting service..." -ForegroundColor Cyan
& net start $ServiceName
if ($LASTEXITCODE -ne 0) {
    Write-Error "Service failed to start. Check Event Log: Get-EventLog -LogName Application -Source 'Redis*' -Newest 10"
    exit 1
}

# --- Verification ---
if (-not $SkipVerify) {
    Write-Host "`n--- Verification ---" -ForegroundColor Cyan

    # 1. PING test
    Start-Sleep -Seconds 1
    $cliPath = Join-Path $ServiceDir "redis-cli.exe"
    $ping = & $cliPath -p 6380 ping 2>&1
    if ($ping -eq "PONG") {
        Write-Host "[PASS] redis-cli PING -> PONG" -ForegroundColor Green
    } else {
        Write-Warning "[FAIL] redis-cli PING -> $ping"
    }

    # 2. Service state
    $svcState = (Get-Service $ServiceName).Status
    if ($svcState -eq 'Running') {
        Write-Host "[PASS] Service state: Running" -ForegroundColor Green
    } else {
        Write-Warning "[FAIL] Service state: $svcState"
    }

    # 3. Event Log (last 5 entries)
    Write-Host "`n--- Recent Event Log ---" -ForegroundColor Cyan
    try {
        Get-EventLog -LogName Application -Source "Redis*" -Newest 5 -ErrorAction SilentlyContinue |
            Format-Table TimeGenerated, EntryType, Message -AutoSize -Wrap
    } catch {
        Write-Host "(no Redis event log entries found)" -ForegroundColor DarkGray
    }

    # 4. INFO check
    $info = & $cliPath -p 6380 info server 2>&1 | Select-String "redis_version"
    if ($info) {
        Write-Host "[PASS] $($info.ToString().Trim())" -ForegroundColor Green
    }
}

Write-Host "`nRedis service installed and running on port 6380." -ForegroundColor Green
