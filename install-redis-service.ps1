# Redis Windows Service Installer
param([string]$ServiceName = "Redis", [string]$DisplayName = "Redis for Windows Agent", [switch]$Uninstall)
function Test-Administrator { ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) }
if (-not (Test-Administrator)) { Write-Error "Admin required"; exit 1 }
if ($Uninstall) { Stop-Service $ServiceName -ErrorAction SilentlyContinue; & sc.exe delete $ServiceName; exit 0 }
$ServiceDir = "C:\Program Files\Redis"
if (-not (Test-Path $ServiceDir)) { New-Item -ItemType Directory -Path $ServiceDir -Force }
$ExeSource = "T:\RustCache\cargo-target\x86_64-pc-windows-msvc\release\redis-service.exe"
if (-not (Test-Path $ExeSource)) { $ExeSource = "agent-sync\target\release\redis-service.exe" }
Copy-Item $ExeSource (Join-Path $ServiceDir "redis-service.exe") -Force
Copy-Item "redis-server.exe" (Join-Path $ServiceDir "redis-server.exe") -Force
Copy-Item "redis-cli.exe" (Join-Path $ServiceDir "redis-cli.exe") -Force
Copy-Item "redis-agent.conf" (Join-Path $ServiceDir "redis-agent.conf") -Force
$binPath = "\`"$ServiceDir\redis-service.exe\`" -c \`"$ServiceDir\redis-agent.conf\`""
& sc.exe delete $ServiceName 2>&1 | Out-Null
& sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= $DisplayName
& net start $ServiceName
& "C:\Program Files\Redis\redis-cli.exe" -p 6380 ping
