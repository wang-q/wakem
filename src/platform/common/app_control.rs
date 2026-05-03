//! Cross-platform application control utilities

/// Generate the daemon identifier string for the given instance
///
/// Used for window title matching on Windows and process identification on Unix.
/// On Windows, the actual process name is `wakemd.exe` but window titles
/// do not include the `.exe` suffix.
pub fn daemon_process_name(instance_id: u32) -> String {
    if instance_id == 0 {
        "wakemd".to_string()
    } else {
        format!("wakemd-instance{}", instance_id)
    }
}

pub fn open_folder_with_opener(
    path: &std::path::Path,
    opener: &str,
) -> anyhow::Result<()> {
    std::process::Command::new(opener).arg(path).spawn()?;
    Ok(())
}

/// Open a folder using the platform-appropriate file manager
#[cfg(target_os = "windows")]
pub fn open_folder(path: &std::path::Path) -> anyhow::Result<()> {
    open_folder_with_opener(path, "explorer")
}

/// Open a folder using the platform-appropriate file manager
#[cfg(target_os = "macos")]
pub fn open_folder(path: &std::path::Path) -> anyhow::Result<()> {
    open_folder_with_opener(path, "open")
}
