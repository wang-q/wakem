# wakem Windows 安装脚本
# 遵循 XDG Base Directory 规范（Windows 适配版）
# https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html
#
# 安装位置：
# - 程序: %LOCALAPPDATA%\Programs\wakem\      (XDG_DATA_HOME 等价)
# - 配置: %APPDATA%\wakem\                   (XDG_CONFIG_HOME 等价)

param(
    [switch]$Service,      # 安装为 Windows 服务（需要管理员权限）
    [switch]$Uninstall,    # 卸载
    [switch]$Help          # 显示帮助
)

$AppName = "wakem"
$AppDisplayName = "wakem - Window/Keyboard/Mouse Enhancer"

# XDG 风格的目录（Windows 适配）
$InstallDir = "$env:LOCALAPPDATA\Programs\$AppName"     # 程序安装目录
$ConfigDir = "$env:APPDATA\$AppName"                    # 配置目录
$ConfigFile = "$ConfigDir\config.toml"                  # 主配置文件
$DataDir = "$env:LOCALAPPDATA\$AppName"                 # 数据目录（日志等）

# 启动项目录
$StartupDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"
$ShortcutPath = "$StartupDir\$AppName.lnk"

# 安装后的可执行文件路径
$InstalledExe = "$InstallDir\wakem.exe"

# 构建目录（源代码位置）
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
$ProjectDir = Split-Path -Parent $ScriptDir
$BuildExe = "$ProjectDir\target\release\wakem.exe"

function Show-Help {
    Write-Host @"
wakem Windows 安装脚本

遵循 XDG Base Directory 规范安装：
  - 程序: %LOCALAPPDATA%\Programs\wakem\
  - 配置: %APPDATA%\wakem\

用法:
    .\install.ps1 [选项]

选项:
    -Service      安装为 Windows 服务（需要管理员权限）
    -Uninstall    卸载 wakem
    -Help         显示此帮助信息

示例:
    # 构建并安装
    cargo build --release
    .\scripts\install.ps1

    # 安装为 Windows 服务
    .\scripts\install.ps1 -Service

    # 卸载
    .\scripts\install.ps1 -Uninstall
"@
}

function Test-Admin {
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Install-Wakem {
    Write-Host "Installing wakem..."
    Write-Host ""

    # 检查构建产物
    if (-not (Test-Path $BuildExe)) {
        Write-Error "Executable not found: $BuildExe"
        Write-Host "Please build the project first: cargo build --release"
        exit 1
    }

    # 创建安装目录
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        Write-Host "Created install directory: $InstallDir" -ForegroundColor Green
    }

    # 复制可执行文件
    Copy-Item $BuildExe $InstalledExe -Force
    Write-Host "Copied executable to: $InstallDir" -ForegroundColor Green

    # 创建配置目录
    if (-not (Test-Path $ConfigDir)) {
        New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
        Write-Host "Created config directory: $ConfigDir" -ForegroundColor Green
    }

    # 创建默认配置文件
    if (-not (Test-Path $ConfigFile)) {
        Write-Host "Creating default configuration file..."
        @"
# wakem 配置文件
# 位置: %APPDATA%\wakem\config.toml

[keyboard]
# 键位重映射
remap = [
    { from = "CapsLock", to = "Ctrl+Alt+Win" },
]

# 导航层
[[keyboard.layers]]
name = "navigation"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
    { from = "K", to = "Up" },
    { from = "L", to = "Right" },
]

[window]
enabled = true

[[window.mappings]]
key = "Alt+Grave"
action = "SwitchToNextWindow"
"@ | Out-File -FilePath $ConfigFile -Encoding UTF8
        Write-Host "Created default config: $ConfigFile" -ForegroundColor Green
    } else {
        Write-Host "Config file already exists: $ConfigFile"
    }

    # 创建数据目录
    if (-not (Test-Path $DataDir)) {
        New-Item -ItemType Directory -Path $DataDir -Force | Out-Null
    }

    # 创建启动项快捷方式
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
    Write-Host "安装位置:"
    Write-Host "  程序: $InstallDir"
    Write-Host "  配置: $ConfigDir"
    Write-Host "  数据: $DataDir"
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  1. Edit config: $ConfigFile"
    Write-Host "  2. Start daemon (as Admin): wakem daemon"
    Write-Host "  3. Client will start automatically on next login"
    Write-Host ""
    Write-Host "Or start now:"
    Write-Host "  - Start client: $InstalledExe"
    Write-Host "  - Start daemon (Admin): wakem daemon"
}

function Install-Service {
    if (-not (Test-Admin)) {
        Write-Error "Administrator privileges required to install as service."
        Write-Host "Please run PowerShell as Administrator and try again."
        exit 1
    }

    # 确保已安装
    if (-not (Test-Path $InstalledExe)) {
        Write-Host "wakem not installed. Installing first..."
        Install-Wakem
    }

    Write-Host "Installing wakem as Windows service..."

    $serviceName = "wakemd"

    # 检查服务是否已存在
    $existingService = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    if ($existingService) {
        Write-Host "Service already exists. Stopping and removing..."
        Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
        sc.exe delete $serviceName | Out-Null
        Start-Sleep -Seconds 2
    }

    # 创建服务
    $binPath = "`"$InstalledExe`" daemon"
    sc.exe create $serviceName binPath= $binPath start= auto displayname= "wakem Daemon" | Out-Null

    if ($LASTEXITCODE -eq 0) {
        Write-Host "Service created successfully" -ForegroundColor Green
        Start-Service -Name $serviceName
        Write-Host "Service started" -ForegroundColor Green
    } else {
        Write-Error "Failed to create service. You may need to use a tool like nssm."
        Write-Host "Download nssm from: https://nssm.cc/"
    }
}

function Uninstall-Wakem {
    Write-Host "Uninstalling wakem..."
    Write-Host ""

    # 移除启动项快捷方式
    if (Test-Path $ShortcutPath) {
        Remove-Item $ShortcutPath -Force
        Write-Host "Removed startup shortcut" -ForegroundColor Green
    }

    # 停止并移除服务
    $serviceName = "wakemd"
    $service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    if ($service) {
        if (Test-Admin) {
            Stop-Service -Name $serviceName -Force -ErrorAction SilentlyContinue
            sc.exe delete $serviceName | Out-Null
            Write-Host "Removed service" -ForegroundColor Green
        } else {
            Write-Warning "Service exists but requires Administrator privileges to remove"
        }
    }

    # 停止运行中的进程
    $processes = Get-Process -Name "wakem" -ErrorAction SilentlyContinue
    if ($processes) {
        $processes | Stop-Process -Force
        Write-Host "Stopped running processes" -ForegroundColor Green
    }

    # 移除安装目录
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

# 主逻辑
if ($Help) {
    Show-Help
    exit 0
}

if ($Uninstall) {
    Uninstall-Wakem
    exit 0
}

if ($Service) {
    Install-Service
} else {
    Install-Wakem
}
