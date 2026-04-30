//! Context-aware mapping conditions and wildcard matching

use serde::{Deserialize, Serialize};

/// Context condition for mapping rules
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextCondition {
    /// Window class name matching (supports wildcards)
    #[serde(default)]
    pub window_class: Option<String>,
    /// Process name matching (supports wildcards)
    #[serde(default)]
    pub process_name: Option<String>,
    /// Window title matching (supports wildcards)
    #[serde(default)]
    pub window_title: Option<String>,
    /// Executable path matching (supports wildcards)
    #[serde(default)]
    pub executable_path: Option<String>,
}

impl ContextCondition {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn with_window_class(mut self, class: impl Into<String>) -> Self {
        self.window_class = Some(class.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_process_name(mut self, name: impl Into<String>) -> Self {
        self.process_name = Some(name.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = Some(title.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_executable_path(mut self, path: impl Into<String>) -> Self {
        self.executable_path = Some(path.into());
        self
    }

    /// Check if current context matches
    pub fn matches(
        &self,
        process_name: &str,
        window_class: &str,
        window_title: &str,
        executable_path: Option<&str>,
    ) -> bool {
        if let Some(ref pattern) = self.process_name {
            if !wildcard_match(process_name, pattern) {
                return false;
            }
        }

        if let Some(ref pattern) = self.window_class {
            if !wildcard_match(window_class, pattern) {
                return false;
            }
        }

        if let Some(ref pattern) = self.window_title {
            if !wildcard_match(window_title, pattern) {
                return false;
            }
        }

        if let Some(ref pattern) = self.executable_path {
            let path = executable_path.unwrap_or("");
            if !wildcard_match(path, pattern) {
                return false;
            }
        }

        true
    }
}

/// Context information (current active window, etc.)
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ContextInfo {
    pub window_class: String,
    pub process_name: String,
    pub process_path: String,
    pub window_title: String,
    pub window_handle: isize,
}

/// Wildcard matching (supports * and ?)
///
/// Performance optimizations:
/// - Fast path for exact matches and simple patterns (no allocation)
/// - Uses dynamic programming (DP) for complex patterns
/// - Time complexity: O(m*n) worst case, O(1) best case
pub fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') && !pattern.contains('?') {
        return text.eq_ignore_ascii_case(pattern);
    }

    if pattern.starts_with('*') && !pattern[1..].contains('*') && !pattern.contains('?')
    {
        let suffix = &pattern[1..];
        return text.len() >= suffix.len()
            && text[text.len() - suffix.len()..].eq_ignore_ascii_case(suffix);
    }

    if pattern.ends_with('*')
        && !pattern[..pattern.len() - 1].contains('*')
        && !pattern.contains('?')
    {
        let prefix = &pattern[..pattern.len() - 1];
        return text.len() >= prefix.len()
            && text[..prefix.len()].eq_ignore_ascii_case(prefix);
    }

    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    wildcard_match_dp(&text_lower, &pattern_lower)
}

fn wildcard_match_dp(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let m = text_chars.len();
    let n = pattern_chars.len();

    if n == 0 {
        return m == 0;
    }

    const WILDCARD_MAX_INPUT_SIZE: usize = 1024;
    if m > WILDCARD_MAX_INPUT_SIZE || n > WILDCARD_MAX_INPUT_SIZE {
        return false;
    }

    let mut prev = vec![false; n + 1];
    let mut curr = vec![false; n + 1];

    prev[0] = true;

    for j in 1..=n {
        if pattern_chars[j - 1] == '*' {
            prev[j] = prev[j - 1];
        } else {
            break;
        }
    }

    for i in 1..=m {
        curr[0] = false;
        for j in 1..=n {
            match pattern_chars[j - 1] {
                '*' => {
                    curr[j] = curr[j - 1] || prev[j];
                }
                '?' => {
                    curr[j] = prev[j - 1];
                }
                _ => {
                    curr[j] = prev[j - 1] && (text_chars[i - 1] == pattern_chars[j - 1]);
                }
            }
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.iter_mut().for_each(|v| *v = false);
    }

    prev[n]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_condition_empty() {
        let cond = ContextCondition::new();
        assert!(cond.matches("notepad", "Notepad", "Untitled", None));
    }

    #[test]
    fn test_context_condition_creation() {
        let cond = ContextCondition::new()
            .with_process_name("chrome")
            .with_window_class("Chrome_WidgetWin_1");
        assert!(cond.matches("chrome", "Chrome_WidgetWin_1", "New Tab", None));
        assert!(!cond.matches("firefox", "MozillaWindowClass", "New Tab", None));
    }

    #[test]
    fn test_context_condition_matching() {
        let cond = ContextCondition::new().with_process_name("notepad*");
        assert!(cond.matches("notepad.exe", "Notepad", "Untitled", None));
        assert!(!cond.matches("chrome.exe", "Chrome_WidgetWin_1", "Tab", None));
    }

    #[test]
    fn test_context_condition_empty_matches_all() {
        let cond = ContextCondition::default();
        assert!(cond.matches("", "", "", None));
        assert!(cond.matches("anything", "goes", "here", Some("/path/to/exe")));
    }

    #[test]
    fn test_context_condition_process_match() {
        let cond = ContextCondition::new().with_process_name("code");
        assert!(cond.matches("code", "Window", "Title", None));
        assert!(!cond.matches("other", "Window", "Title", None));
    }

    #[test]
    fn test_complex_context_condition() {
        let cond = ContextCondition::new()
            .with_process_name("chrome*")
            .with_window_title("*GitHub*");
        assert!(cond.matches("chrome.exe", "Chrome_WidgetWin_1", "GitHub - Repo", None));
        assert!(!cond.matches("chrome.exe", "Chrome_WidgetWin_1", "Google", None));
        assert!(!cond.matches("firefox", "Chrome_WidgetWin_1", "GitHub - Repo", None));
    }

    #[test]
    fn test_context_info_default() {
        let info = ContextInfo::default();
        assert!(info.window_class.is_empty());
        assert!(info.process_name.is_empty());
        assert!(info.process_path.is_empty());
        assert!(info.window_title.is_empty());
        assert_eq!(info.window_handle, 0);
    }

    #[test]
    fn test_wildcard_matching() {
        assert!(wildcard_match("notepad.exe", "notepad*"));
        assert!(wildcard_match("notepad.exe", "*.exe"));
        assert!(wildcard_match("test", "test"));
        assert!(wildcard_match("Test", "test"));
        assert!(!wildcard_match("test", "other"));
        assert!(wildcard_match("anything", "*"));
    }
}
