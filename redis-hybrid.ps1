# Redis Hybrid Configuration Manager
# This script helps manage Redis configurations between WSL and Windows

param(
    [Parameter(Mandatory = $false)]
    [ValidateSet("wsl", "windows", "switch-to-wsl", "switch-to-windows", "status", "test")]
    [string]$Action = "status",
    
    [Parameter(Mandatory = $false)]
    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Write-ColorOutput {
    param([string]$Message, [string]$Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Test-WSLRedis {
    try {
        $result = wsl -e redis-cli ping 2>&1
        return $result -eq "PONG"
    }
    catch {
        return $false
    }
}

function Test-WindowsRedis {
    try {
        $service = Get-Service -Name "Redis" -ErrorAction SilentlyContinue
        return $service.Status -eq "Running"
    }
    catch {
        return $false
    }
}

function Get-RedisStatus {
    Write-ColorOutput "🔍 Redis Status Check" "Cyan"
    Write-ColorOutput "===================" "Cyan"
    
    # Check WSL Redis
    $wslActive = Test-WSLRedis
    if ($wslActive) {
        Write-ColorOutput "✅ WSL Redis: RUNNING (localhost:6379)" "Green"
        try {
            $wslInfo = wsl -e redis-cli info server | Select-String "redis_version"
            Write-ColorOutput "   Version: $($wslInfo.ToString().Split(':')[1].Trim())" "Gray"
        }
        catch {}
    }
    else {
        Write-ColorOutput "⚪ WSL Redis: STOPPED" "Yellow"
    }
    
    # Check Windows Redis
    $windowsActive = Test-WindowsRedis
    if ($windowsActive) {
        Write-ColorOutput "✅ Windows Redis: RUNNING (localhost:6379)" "Green"
        try {
            $service = Get-Service -Name "Redis"
            Write-ColorOutput "   Status: $($service.Status)" "Gray"
        }
        catch {}
    }
    else {
        Write-ColorOutput "⚪ Windows Redis: STOPPED" "Yellow"
    }
    
    # Check for port conflicts
    if ($wslActive -and $windowsActive) {
        Write-ColorOutput "⚠️  WARNING: Both Redis instances may be attempting to use port 6379!" "Red"
    }
    
    # Check agent-memory-server connection
    Write-ColorOutput "" 
    Write-ColorOutput "🔌 Agent Memory Server Test:" "Cyan"
    try {
        Push-Location "C:\Users\david\agent-memory-server"
        $testResult = & .\.venv\Scripts\python.exe -c "import redis; r = redis.Redis(host='localhost', port=6379); print('✅ Connected:', r.ping())" 2>&1
        Write-ColorOutput "   $testResult" "Green"
    }
    catch {
        Write-ColorOutput "   ❌ Cannot connect to Redis on localhost:6379" "Red"
        Write-ColorOutput "   Error: $($_.Exception.Message)" "Red"
    }
    finally {
        Pop-Location
    }
}

function Switch-ToWSL {
    Write-ColorOutput "🔄 Switching to WSL Redis..." "Cyan"
    
    # Stop Windows Redis if running
    $windowsService = Get-Service -Name "Redis" -ErrorAction SilentlyContinue
    if ($windowsService -and $windowsService.Status -eq "Running") {
        Write-ColorOutput "⏹️  Stopping Windows Redis service..." "Yellow"
        Stop-Service -Name "Redis" -Force
        Write-ColorOutput "✅ Windows Redis stopped" "Green"
    }
    
    # Start WSL Redis
    Write-ColorOutput "▶️  Starting WSL Redis..." "Yellow"
    wsl -e sudo systemctl start redis-server
    
    Start-Sleep -Seconds 3
    
    if (Test-WSLRedis) {
        Write-ColorOutput "✅ WSL Redis started successfully" "Green"
        
        # Test agent-memory-server connection
        Write-ColorOutput "🧪 Testing agent-memory-server connection..." "Yellow"
        try {
            Push-Location "C:\Users\david\agent-memory-server"
            & .\.venv\Scripts\python.exe -c "import redis; r = redis.Redis(); print('✅ Agent Memory Server can connect to Redis')"
            Write-ColorOutput "✅ Agent Memory Server connection successful" "Green"
        }
        catch {
            Write-ColorOutput "❌ Agent Memory Server connection failed" "Red"
        }
        finally {
            Pop-Location
        }
    }
    else {
        Write-ColorOutput "❌ Failed to start WSL Redis" "Red"
    }
}

function Switch-ToWindows {
    Write-ColorOutput "🔄 Switching to Windows Redis..." "Cyan"
    
    # Stop WSL Redis if running
    if (Test-WSLRedis) {
        Write-ColorOutput "⏹️  Stopping WSL Redis..." "Yellow"
        wsl -e sudo systemctl stop redis-server
        Write-ColorOutput "✅ WSL Redis stopped" "Green"
    }
    
    # Start Windows Redis
    Write-ColorOutput "▶️  Starting Windows Redis service..." "Yellow"
    try {
        Start-Service -Name "Redis"
        Write-ColorOutput "✅ Windows Redis started successfully" "Green"
        
        # Test connection
        Write-ColorOutput "🧪 Testing connection..." "Yellow"
        Start-Sleep -Seconds 2
        
        if (Test-WindowsRedis) {
            Write-ColorOutput "✅ Windows Redis is responding" "Green"
        }
        else {
            Write-ColorOutput "❌ Windows Redis service started but not responding" "Red"
        }
    }
    catch {
        Write-ColorOutput "❌ Failed to start Windows Redis: $($_.Exception.Message)" "Red"
        Write-ColorOutput "💡 Make sure to run as Administrator" "Yellow"
    }
}

function Optimize-WSLRedis {
    Write-ColorOutput "🔧 Optimizing WSL Redis for agent workloads..." "Cyan"
    
    if (-not (Test-Path ".\optimize-wsl-redis.sh")) {
        Write-ColorOutput "❌ optimize-wsl-redis.sh not found in current directory" "Red"
        return
    }
    
    # Copy script to WSL and run it
    wsl -e bash -c "chmod +x /mnt/t/projects/redis-windows/optimize-wsl-redis.sh && /mnt/t/projects/redis-windows/optimize-wsl-redis.sh"
}

function Test-Configuration {
    Write-ColorOutput "🧪 Testing Redis Configuration..." "Cyan"
    
    if (Test-WSLRedis) {
        Write-ColorOutput "📊 WSL Redis Configuration:" "Cyan"
        wsl -e redis-cli config get maxmemory
        wsl -e redis-cli config get maxmemory-policy
        wsl -e redis-cli info memory | Select-String "used_memory_human"
    }
    
    if (Test-WindowsRedis) {
        Write-ColorOutput "📊 Windows Redis Configuration:" "Cyan"
        # Windows Redis CLI commands would go here
        Write-ColorOutput "   Windows Redis is running but CLI testing not implemented yet" "Yellow"
    }
}

# Main script logic
switch ($Action) {
    "status" { Get-RedisStatus }
    "wsl" { Optimize-WSLRedis }
    "windows" { Write-ColorOutput "Windows optimization not implemented yet" "Yellow" }
    "switch-to-wsl" { Switch-ToWSL }
    "switch-to-windows" { Switch-ToWindows }
    "test" { Test-Configuration }
}

Write-ColorOutput ""
Write-ColorOutput "📋 Available Commands:" "Cyan"
Write-ColorOutput "   .\redis-hybrid.ps1 status           - Show current Redis status" "Gray"
Write-ColorOutput "   .\redis-hybrid.ps1 switch-to-wsl    - Switch to WSL Redis" "Gray"
Write-ColorOutput "   .\redis-hybrid.ps1 switch-to-windows - Switch to Windows Redis" "Gray"
Write-ColorOutput "   .\redis-hybrid.ps1 wsl              - Optimize WSL Redis config" "Gray"
Write-ColorOutput "   .\redis-hybrid.ps1 test             - Test current configuration" "Gray"
