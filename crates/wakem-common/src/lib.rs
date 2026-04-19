pub mod config;
pub mod ipc;
pub mod types;

use std::path::PathBuf;

/// 获取配置文件的默认路径
/// 参考 keymapper 的实现，按优先级搜索多个位置
pub fn resolve_config_file_path(filename: Option<&str>) -> Option<PathBuf> {
    let filename = filename.unwrap_or("wakem.conf");

    // 如果文件已存在，直接返回
    let path = PathBuf::from(filename);
    if path.exists() {
        return Some(path.canonicalize().unwrap_or(path));
    }

    // 搜索路径列表
    let search_paths = vec![
        // XDG_CONFIG_HOME
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(|p| PathBuf::from(p)),
        // HOME/.config
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".config")),
        // Windows: %USERPROFILE%
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| PathBuf::from(p)),
        // Windows: %LOCALAPPDATA%
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p)),
        // Windows: %APPDATA%
        std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p)),
    ];

    for base_path in search_paths.into_iter().flatten() {
        // 直接查找
        let path = base_path.join(filename);
        if path.exists() {
            return Some(path);
        }
        // 在 wakem 子目录中查找
        let path = base_path.join("wakem").join(filename);
        if path.exists() {
            return Some(path);
        }
    }

    // 默认在用户目录创建
    if let Some(home) = std::env::var("USERPROFILE")
        .ok()
        .map(|p| PathBuf::from(p))
    {
        return Some(home.join(filename));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_config_path() {
        // 测试能返回路径（不检查具体值，因为环境不同）
        let _ = resolve_config_file_path(None);
    }
}
