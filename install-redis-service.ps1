# Redis Windows Service Installer
# ==============================
# Installs Redis as a Windows Service with proper executable placement

param(
    [Parameter(Mandatory=$false)]
    [string]$ServiceName = "Redis",
    
    [Parameter(Mandatory=$false)]
    [string]$DisplayName = "Redis for Windows Agent",
    
    [Parameter(Mandatory=$false)]
    [string]$ConfigFile = "redis-agent.conf",
    
    [Parameter(Mandatory=$false)]
    [ValidateSet("symlink", "copy", "auto")]
    [string]$InstallMethod = "auto",
    
    [Parameter(Mandatory=$false)]
    [string]$ServiceDirectory = "C:\Program Files\Redis",
    
    [Parameter(Mandatory=$false)]
    [switch]$Uninstall
)

# Check if running as Administrator
function Test-Administrator {
    $currentUser = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($currentUser)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Write-Status {
    param($Message, $Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Stop-ExistingService {
    param($Name)
    
    $service = Get-Service -Name $Name -ErrorAction SilentlyContinue
    if ($service) {
        if ($service.Status -eq "Running") {
            Write-Status "🛑 Stopping existing $Name service..." "Yellow"
            net stop $Name
        }
        
        Write-Status "🗑️ Removing existing $Name service..." "Yellow"
        sc.exe delete $Name
        Start-Sleep 2
    }
}

function New-ServiceDirectory {
    param($Path)
    
    if (-not (Test-Path $Path)) {
        Write-Status "📁 Creating service directory: $Path" "Yellow"
        New-Item -ItemType Directory -Path $Path -Force | Out-Null
    }
    
    # Set proper permissions for service directory
    $acl = Get-Acl $Path
    $accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule(
        "NT AUTHORITY\SYSTEM", "FullControl", "ContainerInherit,ObjectInherit", "None", "Allow"
    )
    $acl.SetAccessRule($accessRule)
    
    $accessRule = New-Object System.Security.AccessControl.FileSystemAccessRule(
        "BUILTIN\Administrators", "FullControl", "ContainerInherit,ObjectInherit", "None", "Allow"
    )
    $acl.SetAccessRule($accessRule)
    
    Set-Acl -Path $Path -AclObject $acl
    Write-Status "✅ Service directory configured with proper permissions" "Green"
}

function Install-RedisExecutables {
    param($TargetDirectory, $Method)
    
    $sourceFiles = @{
        "RedisService.exe" = "publish\RedisService.exe"
        "redis-server.exe" = "redis-server.exe"
        "redis-cli.exe" = "redis-cli.exe"
    }
    
    $missingFiles = @()
    foreach ($file in $sourceFiles.Values) {
        if (-not (Test-Path $file)) {
            $missingFiles += $file
        }
    }
    
    if ($missingFiles.Count -gt 0) {
        Write-Status "❌ Missing required files:" "Red"
        foreach ($file in $missingFiles) {
            Write-Status "   - $file" "Red"
        }
        
        if ($missingFiles -contains "publish\RedisService.exe") {
            Write-Status "💡 Run: dotnet publish first to build RedisService.exe" "Cyan"
        }
        
        return $false
    }
    
    Write-Status "📦 Installing Redis executables to: $TargetDirectory" "Yellow"
    
    foreach ($targetFile in $sourceFiles.Keys) {
        $sourceFile = $sourceFiles[$targetFile]
        $targetPath = Join-Path $TargetDirectory $targetFile
        
        try {
            if ($Method -eq "symlink") {
                # Try creating symbolic link first
                Write-Status "🔗 Creating symbolic link: $targetFile" "Cyan"
                
                # Remove existing file/link if it exists
                if (Test-Path $targetPath) {
                    Remove-Item $targetPath -Force
                }
                
                # Create symbolic link
                $sourceFullPath = Resolve-Path $sourceFile
                New-Item -ItemType SymbolicLink -Path $targetPath -Target $sourceFullPath -Force | Out-Null
                Write-Status "✅ Symbolic link created: $targetFile" "Green"
            } else {
                # Copy file method
                Write-Status "📄 Copying file: $targetFile" "Cyan"
                Copy-Item $sourceFile $targetPath -Force
                Write-Status "✅ File copied: $targetFile" "Green"
            }
        } catch {
            if ($Method -eq "symlink") {
                Write-Status "⚠️ Symbolic link failed, falling back to copy: $targetFile" "Yellow"
                try {
                    Copy-Item $sourceFile $targetPath -Force
                    Write-Status "✅ File copied (fallback): $targetFile" "Green"
                } catch {
                    Write-Status "❌ Failed to install: $targetFile - $($_.Exception.Message)" "Red"
                    return $false
                }
            } else {
                Write-Status "❌ Failed to copy: $targetFile - $($_.Exception.Message)" "Red"
                return $false
            }
        }
    }
    
    return $true
}

function Install-RedisConfiguration {
    param($TargetDirectory, $ConfigFileName)
    
    $sourceConfig = $ConfigFileName
    $targetConfig = Join-Path $TargetDirectory $ConfigFileName
    
    if (-not (Test-Path $sourceConfig)) {
        Write-Status "❌ Configuration file not found: $sourceConfig" "Red"
        return $false
    }
    
    Write-Status "⚙️ Installing configuration: $ConfigFileName" "Yellow"
    Copy-Item $sourceConfig $targetConfig -Force
    Write-Status "✅ Configuration installed" "Green"
    
    return $true
}

function New-RedisWindowsService {
    param($Name, $Display, $ServiceDir, $ConfigFile)
    
    $servicePath = Join-Path $ServiceDir "RedisService.exe"
    $configPath = Join-Path $ServiceDir $ConfigFile
    
    Write-Status "🔧 Creating Windows service: $Name" "Yellow"
    
    $binPath = "`"$servicePath`" -c `"$configPath`""
    
    try {
        $result = sc.exe create $Name binPath=$binPath start=auto DisplayName=$Display
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✅ Windows service created successfully" "Green"
            return $true
        } else {
            Write-Status "❌ Failed to create Windows service: $result" "Red"
            return $false
        }
    } catch {
        Write-Status "❌ Failed to create Windows service: $($_.Exception.Message)" "Red"
        return $false
    }
}

function Add-RedisFirewallRules {
    # Add firewall rules for both Redis ports
    Write-Status "🔥 Adding Windows Firewall rules..." "Yellow"
    
    try {
        # Remove existing rules first
        Remove-NetFirewallRule -DisplayName "Redis Server*" -ErrorAction SilentlyContinue
        
        # Add rules for both ports
        New-NetFirewallRule -DisplayName "Redis Server (Port 6379)" -Direction Inbound -Protocol TCP -LocalPort 6379 -Action Allow | Out-Null
        New-NetFirewallRule -DisplayName "Redis Server (Port 6380)" -Direction Inbound -Protocol TCP -LocalPort 6380 -Action Allow | Out-Null
        
        Write-Status "✅ Firewall rules added for ports 6379 and 6380" "Green"
    } catch {
        Write-Status "⚠️ Failed to add firewall rules: $($_.Exception.Message)" "Yellow"
        Write-Status "💡 You may need to manually configure Windows Firewall" "Cyan"
    }
}

function Test-RedisService {
    param($Name)
    
    Write-Status "🧪 Testing Redis service..." "Yellow"
    
    try {
        net start $Name
        Start-Sleep 3
        
        $service = Get-Service -Name $Name
        if ($service.Status -eq "Running") {
            Write-Status "✅ Redis service started successfully" "Green"
            
            # Test connection
            $testResult = & ".\redis-cli.exe" -p 6380 ping 2>$null
            if ($testResult -eq "PONG") {
                Write-Status "✅ Redis is responding to connections" "Green"
                return $true
            } else {
                Write-Status "⚠️ Redis service is running but not responding" "Yellow"
                return $false
            }
        } else {
            Write-Status "❌ Redis service failed to start" "Red"
            return $false
        }
    } catch {
        Write-Status "❌ Failed to test Redis service: $($_.Exception.Message)" "Red"
        return $false
    }
}

function Uninstall-RedisService {
    param($Name, $ServiceDir)
    
    Write-Status "🗑️ Uninstalling Redis service..." "Yellow"
    
    # Stop and remove service
    Stop-ExistingService -Name $Name
    
    # Remove firewall rules
    Write-Status "🔥 Removing firewall rules..." "Yellow"
    Remove-NetFirewallRule -DisplayName "Redis Server*" -ErrorAction SilentlyContinue
    
    # Remove service directory (with confirmation)
    if (Test-Path $ServiceDir) {
        $confirm = Read-Host "Remove service directory '$ServiceDir'? (y/N)"
        if ($confirm -eq 'y' -or $confirm -eq 'Y') {
            Write-Status "📁 Removing service directory..." "Yellow"
            Remove-Item $ServiceDir -Recurse -Force
            Write-Status "✅ Service directory removed" "Green"
        }
    }
    
    Write-Status "✅ Redis service uninstalled" "Green"
}

# Main execution
Write-Status "`n🔧 Redis Windows Service Installer" "Cyan"
Write-Status "===================================" "Cyan"

# Check administrator privileges
if (-not (Test-Administrator)) {
    Write-Status "❌ This script requires Administrator privileges" "Red"
    Write-Status "💡 Please run PowerShell as Administrator and try again" "Cyan"
    exit 1
}

# Handle uninstall
if ($Uninstall) {
    Uninstall-RedisService -Name $ServiceName -ServiceDir $ServiceDirectory
    exit 0
}

# Determine installation method
if ($InstallMethod -eq "auto") {
    # Try symlink first, fall back to copy
    $InstallMethod = "symlink"
}

Write-Status "`n📋 Installation Configuration:" "Yellow"
Write-Status "  Service Name: $ServiceName" "White"
Write-Status "  Display Name: $DisplayName" "White"
Write-Status "  Config File: $ConfigFile" "White"
Write-Status "  Service Directory: $ServiceDirectory" "White"
Write-Status "  Install Method: $InstallMethod" "White"

# Stop existing service
Stop-ExistingService -Name $ServiceName

# Create service directory
New-ServiceDirectory -Path $ServiceDirectory

# Install executables
$executablesInstalled = Install-RedisExecutables -TargetDirectory $ServiceDirectory -Method $InstallMethod
if (-not $executablesInstalled) {
    Write-Status "❌ Failed to install Redis executables" "Red"
    exit 1
}

# Install configuration
$configInstalled = Install-RedisConfiguration -TargetDirectory $ServiceDirectory -ConfigFileName $ConfigFile
if (-not $configInstalled) {
    Write-Status "❌ Failed to install Redis configuration" "Red"
    exit 1
}

# Create Windows service
$serviceCreated = New-RedisWindowsService -Name $ServiceName -Display $DisplayName -ServiceDir $ServiceDirectory -ConfigFile $ConfigFile
if (-not $serviceCreated) {
    Write-Status "❌ Failed to create Windows service" "Red"
    exit 1
}

# Add firewall rules
Add-RedisFirewallRules

# Test the service
$serviceWorks = Test-RedisService -Name $ServiceName

Write-Status "`n🎯 Installation Summary:" "Cyan"
if ($serviceWorks) {
    Write-Status "✅ Redis Windows Service installed and running successfully!" "Green"
    Write-Status "`n📊 Service Information:" "Yellow"
    Write-Status "  Service Directory: $ServiceDirectory" "White"
    Write-Status "  Configuration: $ConfigFile" "White"
    Write-Status "  Port: 6380" "White"
    Write-Status "`n🚀 Next Steps:" "Yellow"
    Write-Status "  1. Configure agent-memory-server: .\configure-agent-redis.ps1 -Target windows" "Cyan"
    Write-Status "  2. Test connection: redis-cli -p 6380 ping" "Cyan"
    Write-Status "  3. Monitor service: Get-Service $ServiceName" "Cyan"
} else {
    Write-Status "⚠️ Redis service installed but may have issues" "Yellow"
    Write-Status "💡 Check Windows Event Viewer for error details" "Cyan"
    Write-Status "💡 Run: Get-EventLog -LogName Application -Source 'Redis*' -Newest 5" "Cyan"
}

Write-Status "`n🛠️ Management Commands:" "Yellow"
Write-Status "  Start:     net start $ServiceName" "Cyan"
Write-Status "  Stop:      net stop $ServiceName" "Cyan"
Write-Status "  Status:    Get-Service $ServiceName" "Cyan"
Write-Status "  Uninstall: .\install-redis-service.ps1 -Uninstall" "Cyan"

Write-Status ""
