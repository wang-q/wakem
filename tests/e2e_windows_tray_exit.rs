//! E2E test for tray exit behavior
//!
//! Tests that the tray process exits cleanly when:
//! 1. No daemon is running (tray-only mode)
//! 2. A daemon is running (normal mode, exit via IPC shutdown)
//!
//! # Running
//!
//! This test requires a Windows desktop session and is NOT included in
//! `cargo test` by default. Run it explicitly:
//!
//! ```powershell
//! cargo test --test e2e_windows_tray_exit -- --ignored
//! ```
//!
//! Or use the helper script:
//!
//! ```powershell
//! powershell -File scripts/e2e_tray_exit.ps1
//! ```

use std::process::{Child, Command};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

const TRAY_EXIT_TIMEOUT: Duration = Duration::from_secs(10);
const DAEMON_STARTUP_WAIT: Duration = Duration::from_secs(3);
const TRAY_STARTUP_WAIT: Duration = Duration::from_secs(2);

fn wakem_binary() -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!("target/{}/wakem.exe", profile)
}

fn start_tray(instance_id: u32) -> std::io::Result<Child> {
    info!("Starting wakem --instance {} tray", instance_id);
    Command::new(wakem_binary())
        .args(["--instance", &instance_id.to_string(), "tray"])
        .spawn()
}

fn start_daemon(instance_id: u32) -> std::io::Result<Child> {
    info!("Starting wakem --instance {} daemon", instance_id);
    Command::new(wakem_binary())
        .args(["--instance", &instance_id.to_string(), "daemon"])
        .spawn()
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                info!("Process exited with status: {}", status);
                return true;
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    error!("Process did not exit within {:?}", timeout);
                    return false;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                error!("Error checking process status: {}", e);
                return false;
            }
        }
    }
}

fn cleanup_process(child: &mut Child, name: &str) {
    match child.try_wait() {
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            error!("{} process was still running, killed it", name);
        }
        Ok(Some(_)) => {
            debug!("{} process already exited", name);
        }
        Err(e) => {
            error!("Error checking {} process: {}", name, e);
        }
    }
}

/// Send IPC shutdown command to the daemon, which triggers the full
/// exit flow: daemon shuts down → tray detects disconnect → tray exits.
async fn send_ipc_shutdown(instance_id: u32) -> bool {
    use wakem::client::DaemonClient;
    use wakem::ipc::get_instance_address;

    let mut client = DaemonClient::new();
    let address = get_instance_address(instance_id);

    match tokio::time::timeout(Duration::from_secs(5), client.connect(&address, None))
        .await
    {
        Ok(Ok(())) => {
            info!("Connected to daemon, sending shutdown...");
            match client.shutdown().await {
                Ok(()) => {
                    info!("Shutdown command sent successfully");
                    true
                }
                Err(e) => {
                    error!("Failed to send shutdown: {}", e);
                    false
                }
            }
        }
        Ok(Err(e)) => {
            error!("Failed to connect to daemon: {}", e);
            false
        }
        Err(_) => {
            error!("Connection to daemon timed out");
            false
        }
    }
}

/// Test that tray exits when there is no daemon running.
///
/// The tray should still respond to the Exit command during the
/// connection retry phase. We kill the process as a fallback since
/// we cannot simulate the right-click menu click from a test.
///
/// The key assertion: the process should exit within a reasonable
/// timeout after being signaled.
#[test]
#[ignore = "requires Windows desktop session; run with --ignored"]
fn test_tray_exit_without_daemon() {
    let instance_id: u32 = 250;

    let mut tray = start_tray(instance_id).expect("Failed to start tray process");

    info!("Waiting for tray to initialize...");
    std::thread::sleep(TRAY_STARTUP_WAIT);

    info!("Sending WM_CLOSE via taskkill (simulates right-click Exit)");
    let kill_result = Command::new("taskkill")
        .args(["/PID", &tray.id().to_string()])
        .output();

    match kill_result {
        Ok(output) => {
            if output.status.success() {
                info!("taskkill succeeded");
            } else {
                debug!(
                    "taskkill output: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
        Err(e) => {
            error!("Failed to run taskkill: {}", e);
        }
    }

    let exited = wait_for_exit(&mut tray, TRAY_EXIT_TIMEOUT);
    assert!(exited, "Tray process should exit within timeout");

    cleanup_process(&mut tray, "tray");
}

/// Test that tray exits cleanly when a daemon is running.
///
/// This tests the full exit flow via IPC:
/// 1. Start daemon
/// 2. Start tray (connects to daemon)
/// 3. Send IPC Shutdown command
/// 4. Daemon shuts down, tray detects disconnect
/// 5. Verify both processes exit
#[test]
#[ignore = "requires Windows desktop session; run with --ignored"]
fn test_tray_exit_with_daemon_via_ipc() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let instance_id: u32 = 251;

    let mut daemon = start_daemon(instance_id).expect("Failed to start daemon process");
    info!("Waiting for daemon to initialize...");
    std::thread::sleep(DAEMON_STARTUP_WAIT);

    let mut tray = start_tray(instance_id).expect("Failed to start tray process");
    info!("Waiting for tray to initialize and connect to daemon...");
    std::thread::sleep(TRAY_STARTUP_WAIT);

    info!("Sending IPC shutdown to daemon...");
    let shutdown_ok = rt.block_on(send_ipc_shutdown(instance_id));
    assert!(shutdown_ok, "IPC shutdown should succeed");

    info!("Waiting for daemon to exit...");
    let daemon_exited = wait_for_exit(&mut daemon, TRAY_EXIT_TIMEOUT);
    assert!(daemon_exited, "Daemon process should exit after shutdown");

    info!("Waiting for tray to exit...");
    let tray_exited = wait_for_exit(&mut tray, TRAY_EXIT_TIMEOUT);
    assert!(
        tray_exited,
        "Tray process should exit after daemon shuts down"
    );

    cleanup_process(&mut daemon, "daemon");
    cleanup_process(&mut tray, "tray");
}

/// Test that the tray process can be started and stopped multiple times
/// without leaving zombie processes.
#[test]
#[ignore = "requires Windows desktop session; run with --ignored"]
fn test_tray_restart_cycle() {
    let instance_id: u32 = 252;

    for i in 0..3 {
        info!("Tray restart cycle {}/3", i + 1);

        let mut tray = start_tray(instance_id).expect("Failed to start tray process");
        std::thread::sleep(TRAY_STARTUP_WAIT);

        let _ = Command::new("taskkill")
            .args(["/PID", &tray.id().to_string()])
            .output();

        let exited = wait_for_exit(&mut tray, TRAY_EXIT_TIMEOUT);
        assert!(exited, "Tray process should exit in cycle {}", i + 1);

        cleanup_process(&mut tray, "tray");
        std::thread::sleep(Duration::from_secs(1));
    }
}
