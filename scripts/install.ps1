# wakem Windows Installer Script
# Follows XDG Base Directory specification (Windows adaptation)
# https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
#
# Installation locations:
# - Program: %LOCALAPPDATA%\Programs\wakem\      (XDG_DATA_HOME equivalent)
# - Config:  %APPDATA%\wakem\                   (XDG_CONFIG_HOME equivalent)

param(
    [switch]$Uninstall,    # Uninstall
    [switch]$Help          # Show help
)

$AppName = "wakem"
$AppDisplayName = "wakem - Window/Keyboard/Mouse Enhancer"

# XDG-style directories (Windows adapted)
$InstallDir = "$env:LOCALAPPDATA\Programs\$AppName"     # Program installation directory
$ConfigDir = "$env:APPDATA\$AppName"                    # Configuration directory
$ConfigFile = "$ConfigDir\config.toml"                  # Main configuration file
$DataDir = "$env:LOCALAPPDATA\$AppName"                 # Data directory (logs etc.)

# Startup directory
$StartupDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"
$ShortcutPath = "$StartupDir\$AppName.lnk"

# Installed executable path
$InstalledExe = "$InstallDir\wakem.exe"

# Build directory (source location)
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
$ProjectDir = Split-Path -Parent $ScriptDir
$BuildExe = "$ProjectDir\target\release\wakem.exe"

function Show-Help {
    Write-Host @"
wakem Windows Installer

Installs following XDG Base Directory convention:
  - Program: %LOCALAPPDATA%\Programs\wakem\
  - Config:  %APPDATA%\wakem\

Note: wakem runs as a user-mode application with system tray.
It does NOT support running as a Windows Service because core
features (keyboard/mouse capture, window management, SendInput,
window hooks) require an interactive user session.

Usage:
    .\install.ps1 [options]

Options:
    -Uninstall    Uninstall wakem
    -Help         Show this help message

Examples:
    # Build and install
    cargo build --release
    .\scripts\install.ps1

    # Uninstall
    .\scripts\install.ps1 -Uninstall
"@
}

function Test-Admin {
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-ProcessRunning {
    param(
        [string]$ExePath
    )
    $running = Get-Process | Where-Object { $_.Path -eq $ExePath }
    return $running
}

function Wait-ProcessClosed {
    param(
        [string]$ExePath,
        [string]$ProcessName
    )
    while ((Test-ProcessRunning -ExePath $ExePath)) {
        Write-Warning "$ProcessName is currently running at '$ExePath'"
        Write-Host "Please close $ProcessName and press Enter to continue..." -ForegroundColor Yellow
        Read-Host | Out-Null
    }
}

function Install-Wakem {
    Write-Host "Installing wakem..."
    Write-Host ""

    # Check build artifact
    if (-not (Test-Path $BuildExe)) {
        Write-Error "Executable not found: $BuildExe"
        Write-Host "Please build the project first: cargo build --release"
        exit 1
    }

    # If a previous installation is running, wait for it to close before overwriting
    if (Test-Path $InstalledExe) {
        Wait-ProcessClosed -ExePath $InstalledExe -ProcessName $AppName
    }

    # Create installation directory
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        Write-Host "Created install directory: $InstallDir" -ForegroundColor Green
    }

    # Copy executable
    Copy-Item $BuildExe $InstalledExe -Force
    Write-Host "Copied executable to: $InstallDir" -ForegroundColor Green

    # Create configuration directory
    if (-not (Test-Path $ConfigDir)) {
        New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
        Write-Host "Created config directory: $ConfigDir" -ForegroundColor Green
    }

    # Create default config file (format matches Config struct definition)
    if (-not (Test-Path $ConfigFile)) {
        Write-Host "Creating default configuration file..."
        @"
# wakem configuration file
# Location: %APPDATA%\wakem\config.toml
# See docs/config.md for full reference

# Log level: trace, debug, info, warn, error
log_level = "info"

# Show system tray icon
tray_icon = true

# Auto-reload configuration changes
auto_reload = true

[keyboard]
# Key remapping: map source key to target action
# Target can be a key name or window action string
[keyboard.remap]
CapsLock = "Backspace"

# Navigation layer: HJKL as arrow keys while holding CapsLock
[keyboard.layers.navigation]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.navigation.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
U = "Home"
O = "End"
Y = "PageUp"
N = "PageDown"

# Window management shortcuts
[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"Ctrl+Alt+Left" = "HalfScreen(Left)"
"Ctrl+Alt+Right" = "HalfScreen(Right)"
"Alt+Grave" = "SwitchToNextWindow"

# Quick launch programs (shortcut = command)
[launch]
"Ctrl+Alt+T" = "wt.exe"
"@ | Out-File -FilePath $ConfigFile -Encoding UTF8
        Write-Host "Created default config: $ConfigFile" -ForegroundColor Green
    } else {
        Write-Host "Config file already exists: $ConfigFile"
    }

    # Create data directory
    if (-not (Test-Path $DataDir)) {
        New-Item -ItemType Directory -Path $DataDir -Force | Out-Null
    }

    # Create startup shortcut (auto-start on login, runs in user session)
    $WshShell = New-Object -comObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = $InstalledExe
    $Shortcut.WorkingDirectory = $ConfigDir
    $Shortcut.Description = $AppDisplayName
    $Shortcut.Save()
    Write-Host "Created startup shortcut: $ShortcutPath" -ForegroundColor Green

    Write-Host ""
    Write-Host "Installation complete!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Installation locations:"
    Write-Host "  Program: $InstallDir"
    Write-Host "  Config:  $ConfigDir"
    Write-Host "  Data:    $DataDir"
    Write-Host ""
    Write-Host "Startup shortcut created at:"
    Write-Host "  $ShortcutPath"
    Write-Host "  -> wakem will auto-start on next login (user session)"
    Write-Host ""

    # Offer to launch after installation
    if ($Host.UI.RawUI.KeyAvailable) {
        $ans = Read-Host -Prompt "Start wakem now? (y/n)"
        if ($ans -eq "y") {
            Start-Process -FilePath $InstalledExe -WorkingDirectory $ConfigDir
            Write-Host "Started wakem" -ForegroundColor Green
            exit 0
        }
    }
    Write-Host "To start manually: & $InstalledExe"
}

function Uninstall-Wakem {
    Write-Host "Uninstalling wakem..."
    Write-Host ""

    # Remove startup shortcut
    if (Test-Path $ShortcutPath) {
        Remove-Item $ShortcutPath -Force
        Write-Host "Removed startup shortcut" -ForegroundColor Green
    }

    # Best-effort: clean up legacy service if it exists (from older installs)
    $serviceName = "wakemd"
    $service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    if ($service) {
        if (Test-Admin) {
            Write-Host "Found legacy service '$serviceName', removing..."
            Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
            sc.exe delete $serviceName | Out-Null
            Write-Host "Removed legacy service" -ForegroundColor Green
        } else {
            Write-Warning "Legacy service '$serviceName' exists but requires admin to remove"
            Write-Host "  Run: sc.exe delete $serviceName (as Administrator)"
        }
    }

    # If wakem is running from install dir, wait for user to close it
    if (Test-Path $InstalledExe) {
        Wait-ProcessClosed -ExePath $InstalledExe -ProcessName $AppName
    }

    # Stop any remaining processes (e.g., launched from other paths)
    $processes = Get-Process -Name $AppName -ErrorAction SilentlyContinue
    if ($processes) {
        $processes | Stop-Process -Force
        Write-Host "Stopped remaining processes" -ForegroundColor Green
    }

    # Remove installation directory
    if (Test-Path $InstallDir) {
        Remove-Item $InstallDir -Recurse -Force
        Write-Host "Removed program directory: $InstallDir" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "Uninstallation complete!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Note: The following were NOT removed:"
    Write-Host "  - Config: $ConfigDir"
    Write-Host "  - Data:   $DataDir"
    Write-Host ""
    Write-Host "To remove them manually:"
    Write-Host "  Remove-Item '$ConfigDir' -Recurse -Force"
    Write-Host "  Remove-Item '$DataDir' -Recurse -Force"
}

# Main logic
if ($Help) {
    Show-Help
    exit 0
}

if ($Uninstall) {
    Uninstall-Wakem
    exit 0
}

Install-Wakem
