# Dual Redis Management Script
# ============================
# Manages both WSL Redis (port 6379) and Windows Redis (port 6380)

param(
    [Parameter(Mandatory = $false)]
    [ValidateSet("start", "stop", "restart", "status", "test", "optimize", "help")]
    [string]$Action = "help",
    
    [Parameter(Mandatory = $false)]
    [ValidateSet("wsl", "windows", "both")]
    [string]$Target = "both"
)

function Write-Status {
    param($Message, $Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Test-RedisConnection {
    param($Port, $Name)
    
    try {
        $result = redis-cli -p $Port ping 2>$null
        if ($result -eq "PONG") {
            Write-Status "✅ $Name Redis (port $Port): Connected" "Green"
            return $true
        }
        else {
            Write-Status "❌ $Name Redis (port $Port): Not responding" "Red"
            return $false
        }
    }
    catch {
        Write-Status "❌ $Name Redis (port $Port): Connection failed" "Red"
        return $false
    }
}

function Start-WSLRedis {
    Write-Status "🚀 Starting WSL Redis (port 6379)..." "Yellow"
    wsl -e sudo systemctl start redis-server
    Start-Sleep 2
    
    if (Test-RedisConnection -Port 6379 -Name "WSL") {
        Write-Status "✅ WSL Redis started successfully" "Green"
    }
    else {
        Write-Status "❌ Failed to start WSL Redis" "Red"
        wsl -e sudo systemctl status redis-server
    }
}

function Stop-WSLRedis {
    Write-Status "🛑 Stopping WSL Redis..." "Yellow"
    wsl -e sudo systemctl stop redis-server
    Write-Status "✅ WSL Redis stopped" "Green"
}

function Start-WindowsRedis {
    Write-Status "🚀 Starting Windows Redis Service (port 6380)..." "Yellow"
    
    try {
        net start Redis 2>$null
        Start-Sleep 2
        
        if (Test-RedisConnection -Port 6380 -Name "Windows") {
            Write-Status "✅ Windows Redis Service started successfully" "Green"
        }
        else {
            Write-Status "❌ Windows Redis Service not responding" "Red"
        }
    }
    catch {
        Write-Status "❌ Failed to start Windows Redis Service" "Red"
        Write-Status "💡 Try: sc.exe create Redis binpath=`"$PWD\publish\RedisService.exe -c $PWD\redis-agent.conf`" start=auto" "Cyan"
    }
}

function Stop-WindowsRedis {
    Write-Status "🛑 Stopping Windows Redis Service..." "Yellow"
    
    try {
        net stop Redis 2>$null
        Write-Status "✅ Windows Redis Service stopped" "Green"
    }
    catch {
        Write-Status "❌ Failed to stop Windows Redis Service (may not be running)" "Yellow"
    }
}

function Get-RedisStatus {
    Write-Status "`n📊 Redis Status Report:" "Cyan"
    Write-Status "========================" "Cyan"
    
    # WSL Redis Status
    Write-Status "`n🐧 WSL Redis (Development - Port 6379):" "Yellow"
    $wslActive = wsl -e systemctl is-active redis-server 2>$null
    if ($wslActive -eq "active") {
        Write-Status "   Status: Running" "Green"
        Test-RedisConnection -Port 6379 -Name "WSL"
        
        # Get WSL Redis info
        $wslMemory = wsl -e redis-cli info memory | grep used_memory_human 2>$null
        $wslClients = wsl -e redis-cli info clients | grep connected_clients 2>$null
        Write-Status "   Memory: $wslMemory" "White"
        Write-Status "   Clients: $wslClients" "White"
    }
    else {
        Write-Status "   Status: Stopped" "Red"
    }
    
    # Windows Redis Status
    Write-Status "`n🪟 Windows Redis Service (Production - Port 6380):" "Yellow"
    $windowsService = Get-Service -Name "Redis" -ErrorAction SilentlyContinue
    if ($windowsService) {
        Write-Status "   Service Status: $($windowsService.Status)" $(if ($windowsService.Status -eq "Running") { "Green" }else { "Red" })
        if ($windowsService.Status -eq "Running") {
            Test-RedisConnection -Port 6380 -Name "Windows"
        }
    }
    else {
        Write-Status "   Service Status: Not Installed" "Red"
        Write-Status "   💡 Run: .\manage-dual-redis.ps1 -Action optimize to install" "Cyan"
    }
    
    # Port usage
    Write-Status "`n🌐 Port Usage:" "Yellow"
    $port6379 = netstat -an | findstr ":6379.*LISTENING"
    $port6380 = netstat -an | findstr ":6380.*LISTENING"
    
    if ($port6379) {
        Write-Status "   Port 6379 (WSL): In use" "Green"
    }
    else {
        Write-Status "   Port 6379 (WSL): Available" "Yellow"
    }
    
    if ($port6380) {
        Write-Status "   Port 6380 (Windows): In use" "Green"
    }
    else {
        Write-Status "   Port 6380 (Windows): Available" "Yellow"
    }
}

function Optimize-RedisSetup {
    Write-Status "`n🔧 Optimizing Dual Redis Setup..." "Cyan"
    
    # Optimize WSL Redis
    Write-Status "`n1️⃣ Optimizing WSL Redis Configuration..." "Yellow"
    if (Test-Path ".\optimize-wsl-redis.sh") {
        wsl -e chmod +x ./optimize-wsl-redis.sh
        wsl -e ./optimize-wsl-redis.sh
    }
    else {
        Write-Status "❌ optimize-wsl-redis.sh not found" "Red"
    }
    
    # Setup Windows Redis Service
    Write-Status "`n2️⃣ Setting up Windows Redis Service..." "Yellow"
    $servicePath = "$PWD\publish\RedisService.exe"
    $configPath = "$PWD\redis-agent.conf"
    
    if (Test-Path $servicePath) {
        # Stop existing service if running
        try { net stop Redis 2>$null } catch {}
        
        # Remove existing service
        try { sc.exe delete Redis 2>$null } catch {}
        
        # Create new service with port 6380 config
        $createCmd = "sc.exe create Redis binpath=`"$servicePath -c $configPath`" start=auto"
        Write-Status "   Creating service: $createCmd" "Cyan"
        Invoke-Expression $createCmd
        
        Write-Status "✅ Windows Redis Service configured for port 6380" "Green"
    }
    else {
        Write-Status "❌ RedisService.exe not found. Run: dotnet publish first" "Red"
    }
    
    # Configure agent-memory-server for dual Redis
    Write-Status "`n3️⃣ Configuring Agent Memory Server..." "Yellow"
    $envFile = "C:\Users\david\agent-memory-server\.env"
    if (Test-Path $envFile) {
        Write-Status "   Current .env uses: $(Get-Content $envFile | Select-String 'REDIS_URL')" "White"
        Write-Status "   💡 You can switch between:" "Cyan"
        Write-Status "     - WSL Redis (dev): REDIS_URL=redis://localhost:6379" "Cyan"
        Write-Status "     - Windows Redis (prod): REDIS_URL=redis://localhost:6380" "Cyan"
    }
}

function Test-DualRedis {
    Write-Status "`n🧪 Testing Dual Redis Configuration..." "Cyan"
    
    # Test both Redis instances
    $wslOk = Test-RedisConnection -Port 6379 -Name "WSL"
    $windowsOk = Test-RedisConnection -Port 6380 -Name "Windows"
    
    if ($wslOk -and $windowsOk) {
        Write-Status "`n✅ Both Redis instances are running successfully!" "Green"
        Write-Status "🎯 Configuration:" "Yellow"
        Write-Status "   - WSL Redis (Development): localhost:6379" "White"
        Write-Status "   - Windows Redis (Production): localhost:6380" "White"
        
        # Test with sample data
        Write-Status "`n🔍 Testing data operations..." "Yellow"
        redis-cli -p 6379 set "wsl-test" "WSL Redis Working" >$null
        redis-cli -p 6380 set "windows-test" "Windows Redis Working" >$null
        
        $wslData = redis-cli -p 6379 get "wsl-test"
        $windowsData = redis-cli -p 6380 get "windows-test"
        
        Write-Status "   WSL Redis data: $wslData" "Green"
        Write-Status "   Windows Redis data: $windowsData" "Green"
        
        # Cleanup test data
        redis-cli -p 6379 del "wsl-test" >$null
        redis-cli -p 6380 del "windows-test" >$null
        
    }
    elseif ($wslOk) {
        Write-Status "`n⚠️ Only WSL Redis is running" "Yellow"
    }
    elseif ($windowsOk) {
        Write-Status "`n⚠️ Only Windows Redis is running" "Yellow"
    }
    else {
        Write-Status "`n❌ Neither Redis instance is running" "Red"
    }
}

function Show-Help {
    Write-Status "`n🔧 Dual Redis Management Script" "Cyan"
    Write-Status "================================" "Cyan"
    Write-Status ""
    Write-Status "This script manages both WSL Redis (port 6379) and Windows Redis (port 6380)" "White"
    Write-Status "allowing them to run simultaneously for different purposes:" "White"
    Write-Status ""
    Write-Status "📊 Redis Configuration:" "Yellow"
    Write-Status "  - WSL Redis (Development): localhost:6379" "White"
    Write-Status "  - Windows Redis (Production): localhost:6380" "White"
    Write-Status ""
    Write-Status "🚀 Usage:" "Yellow"
    Write-Status "  .\manage-dual-redis.ps1 -Action <action> [-Target <target>]" "White"
    Write-Status ""
    Write-Status "📋 Actions:" "Yellow"
    Write-Status "  start     - Start Redis instance(s)" "White"
    Write-Status "  stop      - Stop Redis instance(s)" "White"
    Write-Status "  restart   - Restart Redis instance(s)" "White"
    Write-Status "  status    - Show status of both instances" "White"
    Write-Status "  test      - Test connections and functionality" "White"
    Write-Status "  optimize  - Optimize configurations and setup services" "White"
    Write-Status "  help      - Show this help message" "White"
    Write-Status ""
    Write-Status "🎯 Targets:" "Yellow"
    Write-Status "  wsl       - Target only WSL Redis (port 6379)" "White"
    Write-Status "  windows   - Target only Windows Redis (port 6380)" "White"
    Write-Status "  both      - Target both instances (default)" "White"
    Write-Status ""
    Write-Status "💡 Examples:" "Yellow"
    Write-Status "  .\manage-dual-redis.ps1 -Action status" "Cyan"
    Write-Status "  .\manage-dual-redis.ps1 -Action start -Target wsl" "Cyan"
    Write-Status "  .\manage-dual-redis.ps1 -Action optimize" "Cyan"
    Write-Status "  .\manage-dual-redis.ps1 -Action test" "Cyan"
}

# Main execution logic
switch ($Action) {
    "start" {
        if ($Target -eq "wsl" -or $Target -eq "both") { Start-WSLRedis }
        if ($Target -eq "windows" -or $Target -eq "both") { Start-WindowsRedis }
    }
    "stop" {
        if ($Target -eq "wsl" -or $Target -eq "both") { Stop-WSLRedis }
        if ($Target -eq "windows" -or $Target -eq "both") { Stop-WindowsRedis }
    }
    "restart" {
        if ($Target -eq "wsl" -or $Target -eq "both") { 
            Stop-WSLRedis
            Start-WSLRedis 
        }
        if ($Target -eq "windows" -or $Target -eq "both") { 
            Stop-WindowsRedis
            Start-WindowsRedis 
        }
    }
    "status" { Get-RedisStatus }
    "test" { Test-DualRedis }
    "optimize" { Optimize-RedisSetup }
    "help" { Show-Help }
    default { Show-Help }
}

Write-Status ""
