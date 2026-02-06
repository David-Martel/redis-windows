@echo off
:: Redis Windows Service Installer - batch wrapper
:: Runs install-redis-service.ps1 with admin elevation
::
:: Usage:
::   install-redis-service.bat              Install/update service
::   install-redis-service.bat -Uninstall   Remove service

net session >nul 2>&1 || (
    echo ERROR: Administrator privileges required.
    echo Right-click and "Run as administrator", or use an elevated terminal.
    exit /b 1
)

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-redis-service.ps1" %*
