// Tray Exit Logic Tests

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use wakem::platform::types::AppCommand;
use wakem::tray::connect_and_handle_tray_commands;

#[tokio::test]
async fn test_exit_during_connection_phase() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let exit_called = Arc::new(AtomicBool::new(false));
    let exit_called_clone = exit_called.clone();

    let on_exit = move || {
        exit_called_clone.store(true, Ordering::SeqCst);
    };

    let open_config = |_instance_id: u32| -> anyhow::Result<()> { Ok(()) };

    let handle = tokio::spawn(async move {
        connect_and_handle_tray_commands(cmd_rx, 999, on_exit, open_config).await;
    });

    cmd_tx.send(AppCommand::Exit).await.unwrap();

    tokio::time::timeout(tokio::time::Duration::from_secs(2), handle)
        .await
        .expect("Handler should exit within timeout")
        .expect("Handler task should not panic");

    assert!(
        exit_called.load(Ordering::SeqCst),
        "on_exit should have been called"
    );
}

#[tokio::test]
async fn test_exit_during_connection_phase_with_delay() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let exit_called = Arc::new(AtomicBool::new(false));
    let exit_called_clone = exit_called.clone();

    let on_exit = move || {
        exit_called_clone.store(true, Ordering::SeqCst);
    };

    let open_config = |_instance_id: u32| -> anyhow::Result<()> { Ok(()) };

    let handle = tokio::spawn(async move {
        connect_and_handle_tray_commands(cmd_rx, 999, on_exit, open_config).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    cmd_tx.send(AppCommand::Exit).await.unwrap();

    tokio::time::timeout(tokio::time::Duration::from_secs(2), handle)
        .await
        .expect("Handler should exit within timeout")
        .expect("Handler task should not panic");

    assert!(
        exit_called.load(Ordering::SeqCst),
        "on_exit should have been called even during connection retries"
    );
}

#[tokio::test]
async fn test_other_commands_ignored_during_connection() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let exit_called = Arc::new(AtomicBool::new(false));
    let exit_called_clone = exit_called.clone();

    let on_exit = move || {
        exit_called_clone.store(true, Ordering::SeqCst);
    };

    let open_config = |_instance_id: u32| -> anyhow::Result<()> { Ok(()) };

    let handle = tokio::spawn(async move {
        connect_and_handle_tray_commands(cmd_rx, 999, on_exit, open_config).await;
    });

    cmd_tx.send(AppCommand::ReloadConfig).await.unwrap();
    cmd_tx.send(AppCommand::ToggleActive).await.unwrap();
    cmd_tx.send(AppCommand::OpenConfigFolder).await.unwrap();
    cmd_tx.send(AppCommand::Exit).await.unwrap();

    tokio::time::timeout(tokio::time::Duration::from_secs(2), handle)
        .await
        .expect("Handler should exit within timeout")
        .expect("Handler task should not panic");

    assert!(
        exit_called.load(Ordering::SeqCst),
        "on_exit should have been called after Exit command"
    );
}

#[tokio::test]
async fn test_channel_close_exits_handler() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>(16);
    let exit_called = Arc::new(AtomicBool::new(false));
    let exit_called_clone = exit_called.clone();

    let on_exit = move || {
        exit_called_clone.store(true, Ordering::SeqCst);
    };

    let open_config = |_instance_id: u32| -> anyhow::Result<()> { Ok(()) };

    let handle = tokio::spawn(async move {
        connect_and_handle_tray_commands(cmd_rx, 999, on_exit, open_config).await;
    });

    drop(cmd_tx);

    tokio::time::timeout(tokio::time::Duration::from_secs(2), handle)
        .await
        .expect("Handler should exit within timeout")
        .expect("Handler task should not panic");

    assert!(
        !exit_called.load(Ordering::SeqCst),
        "on_exit should NOT have been called on channel close"
    );
}

#[cfg(target_os = "windows")]
mod windows_stop_tray_tests {
    use wakem::platform::windows::tray::stop_tray;

    #[test]
    fn test_stop_tray_callable_from_any_thread() {
        let handle = std::thread::spawn(|| {
            stop_tray();
        });
        handle
            .join()
            .expect("stop_tray should not panic from another thread");
    }

    #[test]
    fn test_stop_tray_callable_without_init() {
        stop_tray();
    }
}
