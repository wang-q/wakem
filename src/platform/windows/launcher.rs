use anyhow::Result;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};
use crate::types::LaunchAction;

/// 程序启动器
pub struct Launcher;

impl Launcher {
    pub fn new() -> Self {
        Self
    }

    /// 执行启动动作
    pub fn launch(&self, action: &LaunchAction) -> Result<()> {
        info!("Launching program: {}", action.program);
        debug!("Args: {:?}", action.args);
        debug!("Working dir: {:?}", action.working_dir);

        // 检查程序路径
        let program = if Path::new(&action.program).exists() {
            action.program.clone()
        } else {
            // 尝试在 PATH 中查找
            action.program.clone()
        };

        // 使用 std::process::Command 启动程序
        let mut cmd = Command::new(&program);
        cmd.args(&action.args);
        
        if let Some(ref dir) = action.working_dir {
            cmd.current_dir(dir);
        }

        // 异步启动，不等待
        match cmd.spawn() {
            Ok(child) => {
                info!("Program launched successfully: {} (pid: {:?})", action.program, child.id());
                Ok(())
            }
            Err(e) => {
                Err(anyhow::anyhow!(
                    "Failed to launch program {}: {}",
                    action.program,
                    e
                ))
            }
        }
    }

    /// 从字符串创建简单的启动动作
    pub fn create_action(program: impl Into<String>) -> LaunchAction {
        LaunchAction {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env_vars: Vec::new(),
        }
    }

    /// 从命令行字符串解析（如 "notepad.exe file.txt"）
    pub fn parse_command(command: &str) -> LaunchAction {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return LaunchAction {
                program: String::new(),
                args: Vec::new(),
                working_dir: None,
                env_vars: Vec::new(),
            };
        }

        LaunchAction {
            program: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
            working_dir: None,
            env_vars: Vec::new(),
        }
    }
}

impl Default for Launcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let action = Launcher::parse_command("notepad.exe file.txt");
        assert_eq!(action.program, "notepad.exe");
        assert_eq!(action.args, vec!["file.txt"]);
    }

    #[test]
    fn test_parse_command_no_args() {
        let action = Launcher::parse_command("calc.exe");
        assert_eq!(action.program, "calc.exe");
        assert!(action.args.is_empty());
    }
}
