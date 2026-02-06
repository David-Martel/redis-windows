# Agent Memory Server Redis Configuration Manager
# ==============================================
# Easily switch between WSL Redis (dev) and Windows Redis (prod)

param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("wsl", "windows", "status")]
    [string]$Target
)

$envFile = "C:\Users\david\agent-memory-server\.env"
$envBackup = "C:\Users\david\agent-memory-server\.env.backup"

function Write-Status {
    param($Message, $Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Backup-EnvFile {
    if (Test-Path $envFile) {
        Copy-Item $envFile $envBackup -Force
        Write-Status "✅ Backed up current .env file" "Green"
    }
}

function Update-RedisConfig {
    param($RedisUrl, $Description)
    
    if (Test-Path $envFile) {
        # Read current content
        $content = Get-Content $envFile
        
        # Update Redis URL
        $newContent = $content | ForEach-Object {
            if ($_ -match "^REDIS_URL=") {
                "REDIS_URL=$RedisUrl"
            }
            else {
                $_
            }
        }
        
        # Write updated content
        $newContent | Set-Content $envFile
        Write-Status "✅ Updated agent-memory-server to use $Description" "Green"
        Write-Status "   REDIS_URL=$RedisUrl" "Cyan"
    }
    else {
        Write-Status "❌ .env file not found at $envFile" "Red"
    }
}

function Test-RedisConnection {
    param($Port, $Name)
    
    try {
        $result = redis-cli -p $Port ping 2>$null
        if ($result -eq "PONG") {
            Write-Status "✅ $Name Redis (port $Port): Connected and responding" "Green"
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

function Show-CurrentStatus {
    Write-Status "`n📊 Agent Memory Server Redis Configuration:" "Cyan"
    Write-Status "=============================================" "Cyan"
    
    if (Test-Path $envFile) {
        $currentUrl = Get-Content $envFile | Select-String "REDIS_URL=" | ForEach-Object { $_.ToString().Split("=")[1] }
        Write-Status "`nCurrent Configuration:" "Yellow"
        Write-Status "  REDIS_URL=$currentUrl" "White"
        
        # Determine which Redis this points to
        if ($currentUrl -like "*:6379*") {
            Write-Status "  → Points to WSL Redis (Development)" "Cyan"
            Test-RedisConnection -Port 6379 -Name "WSL"
        }
        elseif ($currentUrl -like "*:6380*") {
            Write-Status "  → Points to Windows Redis (Production)" "Cyan"
            Test-RedisConnection -Port 6380 -Name "Windows"
        }
        else {
            Write-Status "  → Unknown Redis configuration" "Yellow"
        }
    }
    else {
        Write-Status "❌ No .env file found" "Red"
    }
    
    Write-Status "`nAvailable Redis Instances:" "Yellow"
    Test-RedisConnection -Port 6379 -Name "WSL"
    Test-RedisConnection -Port 6380 -Name "Windows"
    
    Write-Status "`n💡 Usage:" "Yellow"
    Write-Status "  .\configure-agent-redis.ps1 -Target wsl      # Switch to WSL Redis (dev)" "Cyan"
    Write-Status "  .\configure-agent-redis.ps1 -Target windows  # Switch to Windows Redis (prod)" "Cyan"
    Write-Status "  .\configure-agent-redis.ps1 -Target status   # Show this status" "Cyan"
}

# Main execution
switch ($Target) {
    "wsl" {
        Write-Status "`n🔧 Configuring agent-memory-server for WSL Redis..." "Yellow"
        Backup-EnvFile
        Update-RedisConfig -RedisUrl "redis://localhost:6379" -Description "WSL Redis (Development)"
        
        Write-Status "`n🧪 Testing connection..." "Yellow"
        if (Test-RedisConnection -Port 6379 -Name "WSL") {
            Write-Status "`n✅ Agent Memory Server configured for WSL Redis (Development)" "Green"
            Write-Status "🚀 You can now start the agent server with:" "Cyan"
            Write-Status "   cd C:\Users\david\agent-memory-server" "White"
            Write-Status "   .venv\Scripts\activate" "White"
            Write-Status "   python -m agent_memory_server.cli api --port 8000" "White"
        }
        else {
            Write-Status "`n⚠️ WSL Redis is not running. Start it with:" "Yellow"
            Write-Status "   .\manage-dual-redis.ps1 -Action start -Target wsl" "Cyan"
        }
    }
    
    "windows" {
        Write-Status "`n🔧 Configuring agent-memory-server for Windows Redis..." "Yellow"
        Backup-EnvFile
        Update-RedisConfig -RedisUrl "redis://localhost:6380" -Description "Windows Redis (Production)"
        
        Write-Status "`n🧪 Testing connection..." "Yellow"
        if (Test-RedisConnection -Port 6380 -Name "Windows") {
            Write-Status "`n✅ Agent Memory Server configured for Windows Redis (Production)" "Green"
            Write-Status "🚀 You can now start the agent server with:" "Cyan"
            Write-Status "   cd C:\Users\david\agent-memory-server" "White"
            Write-Status "   .venv\Scripts\activate" "White"
            Write-Status "   python -m agent_memory_server.cli api --port 8000" "White"
        }
        else {
            Write-Status "`n⚠️ Windows Redis is not running. Start it with:" "Yellow"
            Write-Status "   .\manage-dual-redis.ps1 -Action start -Target windows" "Cyan"
        }
    }
    
    "status" {
        Show-CurrentStatus
    }
}

Write-Status ""
