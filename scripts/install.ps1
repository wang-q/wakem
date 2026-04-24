# wakem Windows Installer Script
# Follows XDG Base Directory specification (Windows adaptation)
# https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
#
# Installation locations:
# - Program: %LOCALAPPDATA%\Programs\wakem\      (XDG_DATA_HOME equivalent)
# - Config:  %APPDATA%\wakem\                   (XDG_CONFIG_HOME equivalent)

param(
    [switch]$Uninstall,    # Uninstall
    [switch]$AddToPath,    # Add wakem to PATH
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

# GitHub release configuration
$GitHubRepo = "wang-q/wakem"
$Architecture = "x86_64-pc-windows-msvc"

# Configuration files source
$ExamplesDir = "$ProjectDir\examples"
$ConfigFiles = @("minimal.toml", "navigation_layer.toml", "window_manager.toml")
$DefaultConfig = "window_manager.toml"  # Used as the default config.toml

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

Installation Source:
  The installer automatically detects the installation source:
  1. If local build exists (target\release\wakem.exe), uses it
  2. Otherwise, downloads the latest release from GitHub

PATH Configuration:
  Use -AddToPath to add wakem to your user PATH environment variable,
  allowing you to run 'wakem' from any command prompt or PowerShell session.

Usage:
    .\install.ps1 [options]

Options:
    -Uninstall    Uninstall wakem
    -AddToPath    Add wakem to PATH environment variable
    -Help         Show this help message

Examples:
    # Install (auto-detect: local build or download from GitHub)
    .\scripts\install.ps1

    # Install and add to PATH
    .\scripts\install.ps1 -AddToPath

    # Build locally and install
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

function Add-ToUserPath {
    param(
        [string]$Directory
    )
    try {
        $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $pathEntries = $currentPath -split ';' | Where-Object { $_ -ne '' }
        
        # Check if already in PATH
        foreach ($entry in $pathEntries) {
            if ($entry -ieq $Directory) {
                Write-Host "Directory already in PATH: $Directory" -ForegroundColor Yellow
                return $true
            }
        }
        
        # Add to PATH
        $newPath = $currentPath
        if (-not $currentPath.EndsWith(';')) {
            $newPath += ';'
        }
        $newPath += $Directory
        
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "Added to PATH: $Directory" -ForegroundColor Green
        return $true
    } catch {
        Write-Warning "Failed to add to PATH: $_"
        return $false
    }
}

function Remove-FromUserPath {
    param(
        [string]$Directory
    )
    try {
        $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $pathEntries = $currentPath -split ';' | Where-Object { $_ -ne '' }
        
        $newEntries = $pathEntries | Where-Object { $_ -ine $Directory }
        
        if ($newEntries.Count -eq $pathEntries.Count) {
            Write-Host "Directory not found in PATH: $Directory" -ForegroundColor Yellow
            return $false
        }
        
        $newPath = ($newEntries -join ';')
        if ($newPath -ne '' -and -not $newPath.EndsWith(';')) {
            $newPath += ';'
        }
        
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "Removed from PATH: $Directory" -ForegroundColor Green
        return $true
    } catch {
        Write-Warning "Failed to remove from PATH: $_"
        return $false
    }
}

function Get-LatestVersion {
    $apiUrl = "https://api.github.com/repos/$GitHubRepo/releases/latest"
    try {
        $response = Invoke-WebRequest -Uri $apiUrl -UseBasicParsing -ErrorAction Stop
        $release = $response.Content | ConvertFrom-Json
        return $release.tag_name
    } catch {
        Write-Error "Failed to get latest version from GitHub API: $_"
        return $null
    }
}

function Download-Release {
    param(
        [string]$Version,
        [string]$OutputDir
    )
    $zipUrl = "https://github.com/$GitHubRepo/releases/download/$Version/wakem-$Architecture.zip"
    $zipPath = Join-Path $OutputDir "wakem-$Version.zip"
    try {
        Write-Host "Downloading wakem $Version..." -ForegroundColor Cyan
        Write-Host "  URL: $zipUrl" -ForegroundColor Gray
        Invoke-WebRequest -Uri $zipUrl -OutFile $zipPath -UseBasicParsing -ErrorAction Stop
        Write-Host "Downloaded to: $zipPath" -ForegroundColor Green
        return $zipPath
    } catch {
        Write-Error "Failed to download release: $_"
        return $null
    }
}

function Extract-Release {
    param(
        [string]$ZipPath,
        [string]$OutputDir
    )
    try {
        Write-Host "Extracting archive..." -ForegroundColor Cyan
        Expand-Archive -Path $ZipPath -DestinationPath $OutputDir -Force
        $extractedExe = Join-Path $OutputDir "wakem.exe"
        if (Test-Path $extractedExe) {
            Write-Host "Extracted to: $extractedExe" -ForegroundColor Green
            return $extractedExe
        } else {
            Write-Error "wakem.exe not found in extracted archive"
            return $null
        }
    } catch {
        Write-Error "Failed to extract archive: $_"
        return $null
    }
}

function Download-ConfigFile {
    param(
        [string]$FileName,
        [string]$OutputPath
    )
    $configUrl = "https://raw.githubusercontent.com/$GitHubRepo/main/examples/$FileName"
    try {
        Write-Host "  Downloading $FileName..." -ForegroundColor Gray
        Invoke-WebRequest -Uri $configUrl -OutFile $OutputPath -UseBasicParsing -ErrorAction Stop
        return $true
    } catch {
        Write-Warning "Failed to download $FileName from GitHub: $_"
        return $false
    }
}

function Install-ConfigFiles {
    param(
        [bool]$UseLocal
    )
    $installedCount = 0
    $sourceDesc = if ($UseLocal) { "local examples directory" } else { "GitHub repository" }
    Write-Host "Installing configuration files from $sourceDesc..." -ForegroundColor Cyan

    # First, install all example config files
    foreach ($file in $ConfigFiles) {
        $sourcePath = Join-Path $ExamplesDir $file
        $destPath = Join-Path $ConfigDir $file

        if ($UseLocal -and (Test-Path $sourcePath)) {
            # Copy from local examples directory
            Copy-Item $sourcePath $destPath -Force
            Write-Host "  Copied $file" -ForegroundColor Green
            $installedCount++
        } elseif (-not $UseLocal) {
            # Download from GitHub
            if (Download-ConfigFile -FileName $file -OutputPath $destPath) {
                Write-Host "  Downloaded $file" -ForegroundColor Green
                $installedCount++
            }
        }
    }

    # Create default config.toml from minimal.toml if it doesn't exist
    if (-not (Test-Path $ConfigFile)) {
        $defaultSource = Join-Path $ConfigDir $DefaultConfig
        if (Test-Path $defaultSource) {
            Copy-Item $defaultSource $ConfigFile -Force
            Write-Host "  Created default config.toml (from $DefaultConfig)" -ForegroundColor Green
        } elseif ($UseLocal) {
            # Try to copy directly from examples dir if not found in config dir
            $defaultSource = Join-Path $ExamplesDir $DefaultConfig
            if (Test-Path $defaultSource) {
                Copy-Item $defaultSource $ConfigFile -Force
                Write-Host "  Created default config.toml (from $DefaultConfig)" -ForegroundColor Green
            }
        } else {
            # Download directly as config.toml
            if (Download-ConfigFile -FileName $DefaultConfig -OutputPath $ConfigFile) {
                Write-Host "  Created default config.toml (from $DefaultConfig)" -ForegroundColor Green
            }
        }
    } else {
        Write-Host "  Config file already exists: $ConfigFile" -ForegroundColor Yellow
    }

    if ($installedCount -eq 0) {
        Write-Warning "No configuration files were installed"
    } else {
        Write-Host "Installed $installedCount configuration file(s) to: $ConfigDir" -ForegroundColor Green
    }
    return $installedCount
}

function Install-Wakem {
    Write-Host "Installing wakem..."
    Write-Host ""

    # Determine source executable: prefer local build, fallback to GitHub release
    $SourceExe = $null
    $TempDir = $null
    $DownloadedZip = $null

    if (Test-Path $BuildExe) {
        # Use local build
        $SourceExe = $BuildExe
        Write-Host "Using local build: $SourceExe" -ForegroundColor Green
    } else {
        # Download from GitHub release
        Write-Host "Local build not found, downloading from GitHub release..." -ForegroundColor Yellow
        Write-Host ""

        # Get latest version
        $Version = Get-LatestVersion
        if (-not $Version) {
            Write-Error "Failed to get latest version. Please check your internet connection or build locally with: cargo build --release"
            exit 1
        }
        Write-Host "Latest version: $Version" -ForegroundColor Cyan

        # Create temp directory
        $TempDir = Join-Path $env:TEMP "wakem-install-$(Get-Random)"
        New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

        # Download release
        $DownloadedZip = Download-Release -Version $Version -OutputDir $TempDir
        if (-not $DownloadedZip) {
            Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Error "Failed to download release. Please check your internet connection or build locally with: cargo build --release"
            exit 1
        }

        # Extract release
        $SourceExe = Extract-Release -ZipPath $DownloadedZip -OutputDir $TempDir
        if (-not $SourceExe) {
            Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Error "Failed to extract release archive."
            exit 1
        }
        Write-Host ""
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
    Copy-Item $SourceExe $InstalledExe -Force
    Write-Host "Installed executable to: $InstallDir" -ForegroundColor Green

    # Cleanup temp files if we downloaded
    if ($TempDir -and (Test-Path $TempDir)) {
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    # Create configuration directory
    if (-not (Test-Path $ConfigDir)) {
        New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
        Write-Host "Created config directory: $ConfigDir" -ForegroundColor Green
    }

    # Install configuration files
    # Check if local examples directory exists and has config files
    $hasLocalConfigs = $false
    foreach ($file in $ConfigFiles) {
        if (Test-Path (Join-Path $ExamplesDir $file)) {
            $hasLocalConfigs = $true
            break
        }
    }

    Write-Host ""
    Install-ConfigFiles -UseLocal $hasLocalConfigs
    Write-Host ""

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

    # Add to PATH if requested
    if ($AddToPath) {
        Write-Host ""
        Add-ToUserPath -Directory $InstallDir
    }

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

    # Remove from PATH
    Write-Host ""
    Remove-FromUserPath -Directory $InstallDir

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
