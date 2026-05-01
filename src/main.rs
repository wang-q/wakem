use anyhow::Result;
use clap::Parser;
use tracing::{debug, info};

use wakem::cli::{Cli, Commands};
use wakem::commands;
use wakem::config;
use wakem::daemon;
use wakem::tray;

fn init_logging(cli: &Cli) -> (Option<config::Config>, Option<std::path::PathBuf>) {
    let config_path = cli
        .config
        .clone()
        .or_else(|| config::resolve_config_file_path(None, cli.instance));

    let (log_level, config_result) = if let Some(ref path) = config_path {
        match config::Config::from_file(path) {
            Ok(cfg) => (cfg.log_level.clone(), Some(Ok(cfg))),
            Err(e) => {
                eprintln!("Failed to load config for log level: {}", e);
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
        Some(Commands::Status) => commands::cmd_status_sync(cli.instance),
        Some(Commands::Reload) => commands::cmd_reload_sync(cli.instance),
        Some(Commands::Save) => commands::cmd_save_sync(cli.instance),
        Some(Commands::Enable) => commands::cmd_enable_sync(cli.instance),
        Some(Commands::Disable) => commands::cmd_disable_sync(cli.instance),
        Some(Commands::Config) => commands::cmd_config_sync(cli.instance),
        Some(Commands::Instances) => commands::cmd_instances_sync(),
        Some(Commands::Record { name }) => {
            commands::cmd_record_sync(cli.instance, &name)
        }
        Some(Commands::StopRecord) => commands::cmd_stop_record_sync(cli.instance),
        Some(Commands::Play { name }) => commands::cmd_play_sync(cli.instance, &name),
        Some(Commands::Macros) => commands::cmd_macros_sync(cli.instance),
        Some(Commands::BindMacro {
            macro_name,
            trigger,
        }) => commands::cmd_bind_macro_sync(cli.instance, &macro_name, &trigger),
        Some(Commands::DeleteMacro { name }) => {
            commands::cmd_delete_macro_sync(cli.instance, &name)
        }
        Some(Commands::Tray) => tray::run_tray_sync(cli.instance, false, false),
        None => tray::run_tray_sync(cli.instance, true, true),
    }
}

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
