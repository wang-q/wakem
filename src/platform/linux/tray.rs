//! Linux tray lifecycle (placeholder)

use crate::platform::traits::AppCommand;
use anyhow::Result;

pub fn run_tray_message_loop(_callback: Box<dyn Fn(AppCommand) + Send>) -> Result<()> {
    Err(anyhow::anyhow!(
        "Linux tray not yet implemented. DBus/AppIndicator required."
    ))
}

pub fn stop_tray() {
    // No-op
}
