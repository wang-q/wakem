//! Context-aware condition matching
//!
//! Provides [`ContextCondition`] for matching window context
//! (process name, window class, title, executable path) and
//! the [`wildcard_match`] function for pattern matching.

use serde::{Deserialize, Serialize};

/// Context condition for matching current window state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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

    // Fast path: no wildcards - case-insensitive comparison without allocation
    if !pattern.contains('*') && !pattern.contains('?') {
        return text.eq_ignore_ascii_case(pattern);
    }

    // Fast path: simple suffix match (e.g., "*.exe")
    if pattern.starts_with('*') && !pattern[1..].contains('*') && !pattern.contains('?')
    {
        let suffix = &pattern[1..];
        return text.len() >= suffix.len()
            && text[text.len() - suffix.len()..].eq_ignore_ascii_case(suffix);
    }

    // Fast path: simple prefix match (e.g., "chrome*")
    if pattern.ends_with('*')
        && !pattern[..pattern.len() - 1].contains('*')
        && !pattern.contains('?')
    {
        let prefix = &pattern[..pattern.len() - 1];
        return text.len() >= prefix.len()
            && text[..prefix.len()].eq_ignore_ascii_case(prefix);
    }

    // Complex patterns: use DP with case-insensitive comparison
    wildcard_match_dp(text, pattern)
}

/// Byte-level wildcard DP for ASCII strings. No heap allocation.
fn wildcard_match_dp_ascii(text: &[u8], pattern: &[u8], max_size: usize) -> bool {
    let m = text.len();
    let n = pattern.len();

    if n == 0 {
        return m == 0;
    }
    if m > max_size || n > max_size {
        return false;
    }

    let mut prev = vec![false; n + 1];
    let mut curr = vec![false; n + 1];

    prev[0] = true;

    for j in 1..=n {
        if pattern[j - 1] == b'*' {
            prev[j] = prev[j - 1];
        } else {
            break;
        }
    }

    for i in 1..=m {
        curr[0] = false;
        for j in 1..=n {
            let pc = pattern[j - 1];
            if pc == b'*' {
                curr[j] = curr[j - 1] || prev[j];
            } else if pc == b'?' || text[i - 1].eq_ignore_ascii_case(&pc) {
                curr[j] = prev[j - 1];
            } else {
                curr[j] = false;
            }
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Dynamic programming implementation of wildcard matching
/// Uses rolling array optimization (2 rows instead of full matrix).
fn wildcard_match_dp(text: &str, pattern: &str) -> bool {
    /// Maximum input size for wildcard matching to prevent DoS via excessive memory allocation.
    const WILDCARD_MAX_INPUT_SIZE: usize = 4096;

    // ASCII fast path: use byte-level DP (no heap allocation)
    if text.is_ascii() && pattern.is_ascii() {
        return wildcard_match_dp_ascii(
            text.as_bytes(),
            pattern.as_bytes(),
            WILDCARD_MAX_INPUT_SIZE,
        );
    }

    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let m = text_chars.len();
    let n = pattern_chars.len();

    if n == 0 {
        return m == 0;
    }

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
                pattern_char => {
                    let text_char = text_chars[i - 1];
                    let matches = if pattern_char.is_ascii() && text_char.is_ascii() {
                        text_char == pattern_char
                            || text_char.eq_ignore_ascii_case(&pattern_char)
                    } else {
                        text_char.to_lowercase().eq(pattern_char.to_lowercase())
                    };
                    curr[j] = prev[j - 1] && matches;
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
    fn test_wildcard_dp_basic_patterns() {
        assert!(wildcard_match("hello", "hello"));
        assert!(!wildcard_match("hello", "world"));

        assert!(wildcard_match("test.exe", "*.exe"));
        assert!(wildcard_match("file.txt", "*.txt"));
        assert!(wildcard_match("", "*"));
        assert!(wildcard_match("anything", "*"));
        assert!(wildcard_match("prefix-suffix", "*suffix"));
        assert!(wildcard_match("prefix-suffix", "prefix*"));

        assert!(wildcard_match("cat", "?at"));
        assert!(wildcard_match("bat", "?at"));
        assert!(!wildcard_match("at", "?at"));
        assert!(wildcard_match("abc", "???"));
        assert!(!wildcard_match("ab", "???"));

        assert!(wildcard_match("test123.txt", "test*.txt"));
        assert!(wildcard_match("file_1.txt", "file_?.txt"));
    }

    #[test]
    fn test_wildcard_dp_edge_cases() {
        assert!(wildcard_match("", ""));
        assert!(!wildcard_match("a", ""));
        assert!(wildcard_match("", "*"));
        assert!(!wildcard_match("", "?"));

        assert!(wildcard_match("test", "**test"));
        assert!(wildcard_match("test", "***"));
        assert!(wildcard_match("", "**"));

        assert!(wildcard_match("test", "****test"));

        assert!(wildcard_match("TEST.EXE", "*.exe"));
        assert!(wildcard_match("File.TXT", "*.txt"));
    }

    #[test]
    fn test_wildcard_dp_complex_patterns() {
        assert!(wildcard_match("a.b.c.d", "*.d"));
        assert!(wildcard_match("a.b.c.d", "a.*.c.*"));

        assert!(wildcard_match("test_2024.log", "test_????.log"));
        assert!(wildcard_match("image001.png", "image???.png"));

        assert!(wildcard_match("/path/to/file.txt", "/path/*/file.txt"));
        assert!(wildcard_match(
            "c:\\users\\test\\*\\*.txt",
            "c:\\users\\test\\*\\*.txt"
        ));
    }

    #[test]
    fn test_wildcard_dp_performance_safety() {
        let long_text = "a".repeat(1000);
        let long_pattern = "*".repeat(100);

        let result = wildcard_match(&long_text, &long_pattern);
        assert!(result);

        assert!(!wildcard_match(&long_text, ""));

        assert!(wildcard_match(&long_text, "*"));
    }

    #[test]
    fn test_wildcard_match_function() {
        assert!(wildcard_match("test.exe", "*.exe"));
        assert!(wildcard_match("file.txt", "*.txt"));
        assert!(wildcard_match("document.pdf", "*.pdf"));
        assert!(!wildcard_match("test.exe", "*.txt"));
    }
}
