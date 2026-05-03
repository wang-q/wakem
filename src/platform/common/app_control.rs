//! Cross-platform application control utilities

/// Generate the daemon identifier string for the given instance
///
/// Used for window title matching and process identification.
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
