use anyhow::Result;
use clap::Parser;
use tracing::{debug, info};

mod cli;
mod client;
mod config;
mod constants;
mod daemon;
mod ipc;
mod platform;
mod runtime;
mod shutdown;
mod tray;
mod types;

use cli::{Cli, Commands};
use client::DaemonClient;

/// Initialize logging system with support for reading log level from config file
/// Returns the parsed Config if successfully loaded, so it can be reused by the daemon
fn init_logging(cli: &Cli) -> (Option<config::Config>, Option<std::path::PathBuf>) {
    let config_path = cli
        .config
        .clone()
        .or_else(|| config::resolve_config_file_path(None, cli.instance));

    let (log_level, config_result) = if let Some(ref path) = config_path {
        match config::Config::from_file(path) {
            Ok(cfg) => (cfg.log_level.clone(), Some(Ok(cfg))),
            Err(e) => {
                eprintln!("Debug: Failed to load config for log level: {}", e);
                ("info".to_string(), Some(Err(e)))
            }
        }
    } else {
        ("info".to_string(), None)
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_level));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    if let Some(result) = config_result {
        match result {
            Ok(cfg) => {
                info!("Logging initialized with level: {}", log_level);
                return (Some(cfg), config_path);
            }
            Err(err) => {
                debug!("Failed to load config for log level: {}", err);
            }
        }
    }

    info!("Logging initialized with level: {}", log_level);
    (None, config_path)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (preloaded_config, config_path) = init_logging(&cli);

    match cli.command {
        Some(Commands::Daemon) => {
            run_daemon(cli.instance, preloaded_config, config_path)
        }
        Some(Commands::Status) => cmd_status_sync(cli.instance),
        Some(Commands::Reload) => cmd_reload_sync(cli.instance),
        Some(Commands::Save) => cmd_save_sync(cli.instance),
        Some(Commands::Enable) => cmd_enable_sync(cli.instance),
        Some(Commands::Disable) => cmd_disable_sync(cli.instance),
        Some(Commands::Config) => cmd_config_sync(cli.instance),
        Some(Commands::Instances) => cmd_instances_sync(),
        Some(Commands::Record { name }) => cmd_record_sync(cli.instance, &name),
        Some(Commands::StopRecord) => cmd_stop_record_sync(cli.instance),
        Some(Commands::Play { name }) => cmd_play_sync(cli.instance, &name),
        Some(Commands::Macros) => cmd_macros_sync(cli.instance),
        Some(Commands::BindMacro {
            macro_name,
            trigger,
        }) => cmd_bind_macro_sync(cli.instance, &macro_name, &trigger),
        Some(Commands::DeleteMacro { name }) => {
            cmd_delete_macro_sync(cli.instance, &name)
        }
        Some(Commands::Tray) => tray::run_tray_sync(cli.instance, false, false),
        None => tray::run_tray_sync(cli.instance, true, true),
    }
}

/// Start the daemon with multi-thread tokio runtime
fn run_daemon(
    instance_id: u32,
    preloaded_config: Option<config::Config>,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        info!("Starting wakemd (instance {})...", instance_id);
        daemon::run_server_with_config(instance_id, preloaded_config, config_path).await
    })
}

/// Open config folder - sync version (used by cmd_config_sync)
fn open_config_folder_sync(instance_id: u32) -> Result<()> {
    use std::process::Command;

    let config_path = config::resolve_config_file_path(None, instance_id)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| {
            std::env::var("USERPROFILE")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
        });

    Command::new("explorer").arg(config_path).spawn()?;

    Ok(())
}

/// Execute an async operation with a daemon client connection
fn run_with_client<F, Fut>(instance_id: u32, op: F) -> Result<()>
where
    F: FnOnce(DaemonClient) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await?;
        op(client).await
    })
}

fn cmd_status_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let (active, loaded) = client.get_status().await?;
        println!("wakemd instance {}:", instance_id);
        println!("  Active: {}", if active { "yes" } else { "no" });
        println!("  Config loaded: {}", if loaded { "yes" } else { "no" });
        Ok(())
    })
}

fn cmd_reload_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.reload_config().await?;
        println!("Configuration reloaded successfully");
        Ok(())
    })
}

fn cmd_save_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.save_config().await?;
        println!("Configuration saved successfully");
        Ok(())
    })
}

fn cmd_enable_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.set_active(true).await?;
        println!("wakem enabled");
        Ok(())
    })
}

fn cmd_disable_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        client.set_active(false).await?;
        println!("wakem disabled");
        Ok(())
    })
}

fn cmd_config_sync(instance_id: u32) -> Result<()> {
    open_config_folder_sync(instance_id)?;
    println!("Config folder opened");
    Ok(())
}

fn cmd_instances_sync() -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let instances = ipc::discover_instances().await;

        println!("Running instances:");
        let mut found = false;
        for info in instances {
            if info.active {
                found = true;
                println!("  Instance {}: {} (active)", info.id, info.address);
            }
        }

        if !found {
            println!("  No running instances found");
        }

        Ok(())
    })
}

fn cmd_record_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.start_macro_recording(&name_owned).await?;
        println!("Recording macro '{}'...", name_owned);
        println!("Press Ctrl+Shift+Esc to stop recording");
        Ok(())
    })
}

fn cmd_stop_record_sync(instance_id: u32) -> Result<()> {
    run_with_client(instance_id, |mut client| async move {
        let (name, count) = client.stop_macro_recording().await?;
        println!("Macro '{}' saved with {} actions", name, count);
        Ok(())
    })
}

fn cmd_play_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.play_macro(&name_owned).await?;
        println!("Playing macro '{}'", name_owned);
        Ok(())
    })
}

fn cmd_macros_sync(instance_id: u32) -> Result<()> {
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

fn cmd_bind_macro_sync(instance_id: u32, macro_name: &str, trigger: &str) -> Result<()> {
    let macro_name_owned = macro_name.to_string();
    let trigger_owned = trigger.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.bind_macro(&macro_name_owned, &trigger_owned).await?;
        println!("Macro '{}' bound to '{}'", macro_name_owned, trigger_owned);
        Ok(())
    })
}

fn cmd_delete_macro_sync(instance_id: u32, name: &str) -> Result<()> {
    let name_owned = name.to_string();
    run_with_client(instance_id, move |mut client| async move {
        client.delete_macro(&name_owned).await?;
        println!("Macro '{}' deleted", name_owned);
        Ok(())
    })
}
