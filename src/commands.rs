use anyhow::Result;

use crate::config;

use crate::platform::CurrentPlatform;
use crate::runtime_util::{run_async, run_with_client};

/// Get server status
pub fn cmd_status_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let (active, loaded) = client.get_status().await?;
        println!("wakemd instance {}:", instance_id);
        println!("  Active: {}", if active { "yes" } else { "no" });
        println!("  Config loaded: {}", if loaded { "yes" } else { "no" });
        Ok(())
    })
}

/// Reload configuration
pub fn cmd_reload_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.reload_config().await?;
        println!("Configuration reloaded successfully");
        Ok(())
    })
}

/// Save configuration
pub fn cmd_save_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.save_config().await?;
        println!("Configuration saved successfully");
        Ok(())
    })
}

/// Enable mapping
pub fn cmd_enable_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.set_active(true).await?;
        println!("wakem enabled");
        Ok(())
    })
}

/// Disable mapping
pub fn cmd_disable_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.set_active(false).await?;
        println!("wakem disabled");
        Ok(())
    })
}

/// Open config folder
pub fn cmd_config_sync(instance_id: u32) -> Result<()> {
    let config_path = config::resolve_config_file_path(None, instance_id)
        .ok_or_else(|| anyhow::anyhow!("Could not resolve config file path"))?;

    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Could not get config directory"))?;

    CurrentPlatform::open_folder(config_dir)?;

    println!("Config folder opened: {}", config_dir.display());
    Ok(())
}

/// List running instances
pub fn cmd_instances_sync() -> Result<()> {
    run_async(|| async {
        let instances = crate::ipc::discover_instances().await;

        println!("Discovered instances:");
        let mut found = false;
        for info in &instances {
            found = true;
            let state = if info.active { "active" } else { "disabled" };
            println!("  Instance {}: {} ({})", info.id, info.address, state);
        }

        if !found {
            println!("  No instances found");
        }

        Ok(())
    })
}

/// Record macro
pub fn cmd_record_sync(instance_id: u32, name: &str) -> Result<()> {
    // Validate macro name: must be non-empty and contain only alphanumeric characters, underscores, and hyphens
    if name.is_empty() {
        return Err(anyhow::anyhow!("Macro name cannot be empty"));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(anyhow::anyhow!(
            "Macro name can only contain alphanumeric characters, underscores, and hyphens"
        ));
    }

    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.start_macro_recording(&name_owned).await?;
        println!("Recording macro '{}'...", name_owned);
        println!("Press Ctrl+Shift+Esc to stop recording");
        Ok(())
    })
}

/// Stop recording macro
pub fn cmd_stop_record_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let (name, count) = client.stop_macro_recording().await?;
        println!("Macro '{}' saved with {} actions", name, count);
        Ok(())
    })
}

/// Play macro
pub fn cmd_play_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.play_macro(&name_owned).await?;
        println!("Playing macro '{}'", name_owned);
        Ok(())
    })
}

/// List all macros
pub fn cmd_macros_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let macros = client.get_macros().await?;
        if macros.is_empty() {
            println!("No macros recorded");
        } else {
            println!("Available macros:");
            for name in macros {
                println!("  - {}", name);
            }
        }
        Ok(())
    })
}

/// Bind macro to trigger key
pub fn cmd_bind_macro_sync(
    instance_id: u32,
    macro_name: &str,
    trigger: &str,
) -> Result<()> {
    let macro_name_owned = macro_name.to_string();
    let trigger_owned = trigger.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.bind_macro(&macro_name_owned, &trigger_owned).await?;
        println!("Macro '{}' bound to '{}'", macro_name_owned, trigger_owned);
        Ok(())
    })
}

/// Delete macro
pub fn cmd_delete_macro_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.delete_macro(&name_owned).await?;
        println!("Macro '{}' deleted", name_owned);
        Ok(())
    })
}
