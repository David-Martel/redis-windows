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
            Write-Status "?? Stopping existing $Name service..." "Yellow"
            & net stop $Name
        }
        
        Write-Status "??? Removing existing $Name service..." "Yellow"
        & sc.exe delete $Name
        Start-Sleep 2
    }
}

function New-ServiceDirectory {
    param($Path)
    
    if (-not (Test-Path $Path)) {
        Write-Status "?? Creating service directory: $Path" "Yellow"
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
    Write-Status "? Service directory configured with proper permissions" "Green"
}

function Install-RedisExecutables {
    param($TargetDirectory, $Method)
    
    $sourceFiles = @{
        "redis-service.exe" = "T:\RustCache\cargo-target\x86_64-pc-windows-msvc\release\redis-service.exe"
        "redis-server.exe" = "redis-server.exe"
        "redis-cli.exe" = "redis-cli.exe"
    }
    
    $missingFiles = @()
    foreach ($entry in $sourceFiles.GetEnumerator()) {
        if (-not (Test-Path $entry.Value)) {
            # Fallback to local target
            if ($entry.Key -eq "redis-service.exe") {
                $localPath = "agent-sync\target\release\redis-service.exe"
                if (Test-Path $localPath) {
                    $sourceFiles["redis-service.exe"] = $localPath
                    continue
                }
            }
            $missingFiles += $entry.Value
        }
    }
    
    if ($missingFiles.Count -gt 0) {
        Write-Status "? Missing required files:" "Red"
        foreach ($file in $missingFiles) {
            Write-Status "   - $file" "Red"
        }
        return $false
    }
    
    Write-Status "?? Installing Redis executables to: $TargetDirectory" "Yellow"
    
    foreach ($entry in $sourceFiles.GetEnumerator()) {
        $targetFile = $entry.Name
        $sourceFile = $entry.Value
        $targetPath = Join-Path $TargetDirectory $targetFile
        
        try {
            if (Test-Path $targetPath) {
                Remove-Item $targetPath -Force
            }
            
            Copy-Item $sourceFile $targetPath -Force
            Write-Status "? File copied: $targetFile" "Green"
        } catch {
            Write-Status "? Failed to copy: $targetFile - $($_.Exception.Message)" "Red"
            return $false
        }
    }
    
    return $true
}

function Install-RedisConfiguration {
    param($TargetDirectory, $ConfigFileName)
    
    $sourceConfig = $ConfigFileName
    $targetConfig = Join-Path $TargetDirectory $ConfigFileName
    
    if (-not (Test-Path $sourceConfig)) {
        Write-Status "? Configuration file not found: $sourceConfig" "Red"
        return $false
    }
    
    Write-Status "?? Installing configuration: $ConfigFileName" "Yellow"
    Copy-Item $sourceConfig $targetConfig -Force
    Write-Status "? Configuration installed" "Green"
    
    return $true
}

function New-RedisWindowsService {
    param($Name, $Display, $ServiceDir, $ConfigFile)
    
    $servicePath = Join-Path $ServiceDir "redis-service.exe"
    $configPath = Join-Path $ServiceDir $ConfigFile
    
    Write-Status "?? Creating Windows service: $Name" "Yellow"
    
    $binPath = "`"$servicePath`" -c `"$configPath`""
    
    # sc.exe create [name] binPath= "[path]" start= auto DisplayName= "[display]"
    $scCommand = "sc.exe create $Name binPath= `"$binPath`" start= auto DisplayName= `"$Display`""
    Write-Status "Executing: $scCommand" "Cyan"
    
    & cmd.exe /c $scCommand
    
    if ($LASTEXITCODE -eq 0) {
        Write-Status "? Windows service created successfully" "Green"
        return $true
    } else {
        Write-Status "? Failed to create Windows service (Exit code $LASTEXITCODE)" "Red"
        return $false
    }
}

function Add-RedisFirewallRules {
    Write-Status "?? Adding Windows Firewall rules..." "Yellow"
    try {
        & netsh advfirewall firewall delete rule name="Redis Server (Port 6379)" | Out-Null
        & netsh advfirewall firewall delete rule name="Redis Server (Port 6380)" | Out-Null
        & netsh advfirewall firewall add rule name="Redis Server (Port 6379)" dir=in action=allow protocol=TCP localport=6379 | Out-Null
        & netsh advfirewall firewall add rule name="Redis Server (Port 6380)" dir=in action=allow protocol=TCP localport=6380 | Out-Null
        Write-Status "? Firewall rules added for ports 6379 and 6380" "Green"
    } catch {
        Write-Status "?? Failed to add firewall rules: $($_.Exception.Message)" "Yellow"
    }
}

function Test-RedisService {
    param($Name)
    Write-Status "?? Testing Redis service..." "Yellow"
    try {
        & net start $Name
        Start-Sleep 3
        $service = Get-Service -Name $Name -ErrorAction SilentlyContinue
        if ($service -and $service.Status -eq "Running") {
            Write-Status "? Redis service started successfully" "Green"
            if (Test-Path "redis-cli.exe") {
                $testResult = & ".\redis-cli.exe" -p 6380 ping 2>$null
                if ($testResult -eq "PONG") {
                    Write-Status "? Redis is responding to connections" "Green"
                    return $true
                }
            }
            return $true
        }
        return $false
    } catch {
        return $false
    }
}

function Uninstall-RedisService {
    param($Name, $ServiceDir)
    Write-Status "??? Uninstalling Redis service..." "Yellow"
    Stop-ExistingService -Name $Name
    if (Test-Path $ServiceDir) {
        Write-Status "?? Removing service directory: $ServiceDir" "Yellow"
        Remove-Item $ServiceDir -Recurse -Force
        Write-Status "? Service directory removed" "Green"
    }
}

# Main execution
if (-not (Test-Administrator)) {
    Write-Status "? This script requires Administrator privileges" "Red"
    exit 1
}

if ($Uninstall) {
    Uninstall-RedisService -Name $ServiceName -ServiceDir $ServiceDirectory
    exit 0
}

Stop-ExistingService -Name $ServiceName
New-ServiceDirectory -Path $ServiceDirectory
if (Install-RedisExecutables -TargetDirectory $ServiceDirectory -Method "copy") {
    if (Install-RedisConfiguration -TargetDirectory $ServiceDirectory -ConfigFileName $ConfigFile) {
        if (New-RedisWindowsService -Name $ServiceName -Display $DisplayName -ServiceDir $ServiceDirectory -ConfigFile $ConfigFile) {
            Add-RedisFirewallRules
            Test-RedisService -Name $ServiceName
        }
    }
}
