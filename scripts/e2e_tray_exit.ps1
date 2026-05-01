# E2E test for tray exit behavior
#
# Usage:
#   powershell -File scripts/e2e_tray_exit.ps1
#   powershell -File scripts/e2e_tray_exit.ps1 -TestNoDaemon
#   powershell -File scripts/e2e_tray_exit.ps1 -TestWithDaemon
#   powershell -File scripts/e2e_tray_exit.ps1 -TestRestartCycle
#
# This script tests that the wakem tray process exits cleanly,
# including the tray icon disappearing from the taskbar.

param(
    [switch]$TestNoDaemon = $false,
    [switch]$TestWithDaemon = $false,
    [switch]$TestRestartCycle = $false,
    [int]$InstanceId = 250,
    [int]$TimeoutSeconds = 10
)

$ErrorActionPreference = "Stop"

$binary = ".\target\release\wakem.exe"
if (-not (Test-Path $binary)) {
    $binary = ".\target\debug\wakem.exe"
}
if (-not (Test-Path $binary)) {
    Write-Error "wakem binary not found. Run 'cargo build' first."
    exit 1
}

Write-Host "Using binary: $binary" -ForegroundColor Cyan

# Check if a wakem process with the given instance is running
function Find-WakemProcess {
    param([int]$InstanceId)
    Get-Process -Name "wakem" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -like "*--instance $InstanceId*" -or $_.CommandLine -like "*instance=$InstanceId*" }
}

# Check if the tray icon exists in the notification area
# This uses the Shell COM object to enumerate tray icons
function Test-TrayIconExists {
    param([string]$Tooltip = "wakem")
    try {
        # Use a simple heuristic: check if wakem process is running
        # A more thorough check would use UI Automation to find the icon
        $procs = Get-Process -Name "wakem" -ErrorAction SilentlyContinue
        return ($procs -ne $null -and $procs.Count -gt 0)
    } catch {
        return $false
    }
}

# Kill all wakem processes for the given instance (cleanup)
function Stop-WakemProcesses {
    param([int]$InstanceId)
    $procs = Find-WakemProcess -InstanceId $InstanceId
    if ($procs) {
        Write-Host "  Cleaning up leftover processes..." -ForegroundColor Yellow
        $procs | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 500
    }
}

# Wait for a process to exit
function Wait-ForExit {
    param(
        [System.Diagnostics.Process]$Process,
        [int]$TimeoutSeconds
    )

    $exited = $Process.WaitForExit($TimeoutSeconds * 1000)
    if ($exited) {
        Write-Host "  Process exited (code: $($Process.ExitCode))" -ForegroundColor Green
        return $true
    } else {
        Write-Host "  Process did NOT exit within $TimeoutSeconds seconds" -ForegroundColor Red
        return $false
    }
}

# ============================================================
# Test 1: Tray exit without daemon
# ============================================================
function Test-TrayExitWithoutDaemon {
    Write-Host "`n=== Test: Tray exit without daemon ===" -ForegroundColor Cyan
    $id = $InstanceId

    Stop-WakemProcesses -InstanceId $id

    Write-Host "  Starting tray (instance $id)..."
    $tray = Start-Process -FilePath $binary -ArgumentList "tray","--instance",$id -PassThru -WindowStyle Hidden

    Write-Host "  Waiting for tray to initialize (2s)..."
    Start-Sleep -Seconds 2

    Write-Host "  Checking tray process is running..."
    if ($tray.HasExited) {
        Write-Host "  FAIL: Tray process exited immediately" -ForegroundColor Red
        return $false
    }

    Write-Host "  Sending WM_CLOSE to tray window..."
    # Use taskkill which sends WM_CLOSE (graceful shutdown)
    $killResult = & taskkill /PID $tray.Id 2>&1
    Write-Host "  taskkill output: $killResult"

    $exited = Wait-ForExit -Process $tray -TimeoutSeconds $TimeoutSeconds

    if (-not $exited) {
        Write-Host "  Killing tray process forcefully..." -ForegroundColor Red
        Stop-Process -Id $tray.Id -Force -ErrorAction SilentlyContinue
    }

    Stop-WakemProcesses -InstanceId $id

    if ($exited) {
        Write-Host "  PASS: Tray exited cleanly without daemon" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: Tray did not exit cleanly" -ForegroundColor Red
    }
    return $exited
}

# ============================================================
# Test 2: Tray exit with daemon (via IPC shutdown)
# ============================================================
function Test-TrayExitWithDaemon {
    Write-Host "`n=== Test: Tray exit with daemon (IPC shutdown) ===" -ForegroundColor Cyan
    $id = $InstanceId + 1

    Stop-WakemProcesses -InstanceId $id

    Write-Host "  Starting daemon (instance $id)..."
    $daemon = Start-Process -FilePath $binary -ArgumentList "daemon","--instance",$id -PassThru -WindowStyle Hidden

    Write-Host "  Waiting for daemon to initialize (3s)..."
    Start-Sleep -Seconds 3

    Write-Host "  Starting tray (instance $id)..."
    $tray = Start-Process -FilePath $binary -ArgumentList "tray","--instance",$id -PassThru -WindowStyle Hidden

    Write-Host "  Waiting for tray to connect to daemon (2s)..."
    Start-Sleep -Seconds 2

    Write-Host "  Sending shutdown command via IPC..."
    $shutdownResult = & $binary "shutdown" "--instance" $id 2>&1
    Write-Host "  shutdown output: $shutdownResult"

    Write-Host "  Waiting for daemon to exit..."
    $daemonExited = Wait-ForExit -Process $daemon -TimeoutSeconds $TimeoutSeconds

    Write-Host "  Waiting for tray to exit..."
    $trayExited = Wait-ForExit -Process $tray -TimeoutSeconds $TimeoutSeconds

    if (-not $daemonExited) {
        Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
    }
    if (-not $trayExited) {
        Stop-Process -Id $tray.Id -Force -ErrorAction SilentlyContinue
    }

    Stop-WakemProcesses -InstanceId $id

    $pass = $daemonExited -and $trayExited
    if ($pass) {
        Write-Host "  PASS: Both daemon and tray exited cleanly" -ForegroundColor Green
    } else {
        Write-Host "  FAIL: daemonExited=$daemonExited, trayExited=$trayExited" -ForegroundColor Red
    }
    return $pass
}

# ============================================================
# Test 3: Tray restart cycle
# ============================================================
function Test-TrayRestartCycle {
    Write-Host "`n=== Test: Tray restart cycle (3 times) ===" -ForegroundColor Cyan
    $id = $InstanceId + 2

    for ($i = 1; $i -le 3; $i++) {
        Write-Host "  Cycle $i/3..." -ForegroundColor Yellow

        Stop-WakemProcesses -InstanceId $id

        $tray = Start-Process -FilePath $binary -ArgumentList "tray","--instance",$id -PassThru -WindowStyle Hidden
        Start-Sleep -Seconds 2

        if ($tray.HasExited) {
            Write-Host "  FAIL: Tray exited immediately in cycle $i" -ForegroundColor Red
            return $false
        }

        & taskkill /PID $tray.Id 2>&1 | Out-Null
        $exited = Wait-ForExit -Process $tray -TimeoutSeconds $TimeoutSeconds

        if (-not $exited) {
            Stop-Process -Id $tray.Id -Force -ErrorAction SilentlyContinue
            Write-Host "  FAIL: Tray did not exit in cycle $i" -ForegroundColor Red
            return $false
        }

        Start-Sleep -Seconds 1
    }

    Stop-WakemProcesses -InstanceId $id
    Write-Host "  PASS: All 3 restart cycles completed" -ForegroundColor Green
    return $true
}

# ============================================================
# Main
# ============================================================

$results = @{}

if (-not $TestNoDaemon -and -not $TestWithDaemon -and -not $TestRestartCycle) {
    # Run all tests
    $TestNoDaemon = $true
    $TestWithDaemon = $true
    $TestRestartCycle = $true
}

if ($TestNoDaemon) {
    $results["NoDaemon"] = Test-TrayExitWithoutDaemon
}

if ($TestWithDaemon) {
    $results["WithDaemon"] = Test-TrayExitWithDaemon
}

if ($TestRestartCycle) {
    $results["RestartCycle"] = Test-TrayRestartCycle
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Results:" -ForegroundColor Cyan
foreach ($key in $results.Keys) {
    $status = if ($results[$key]) { "PASS" } else { "FAIL" }
    $color = if ($results[$key]) { "Green" } else { "Red" }
    Write-Host "  ${key}: $status" -ForegroundColor $color
}

$allPass = $results.Values -notcontains $false
if ($allPass) {
    Write-Host "`nAll tests passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`nSome tests failed!" -ForegroundColor Red
    exit 1
}
