// Windows Launcher E2E Tests
// These tests launch real programs, ignored by default, run manually

#[cfg(target_os = "windows")]
mod launcher_tests {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;
    use wakem::platform::windows::Launcher;
    use wakem::types::LaunchAction;

    fn is_process_running(process_name: &str) -> bool {
        let output = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().any(|line| {
                    line.to_lowercase().contains(&process_name.to_lowercase())
                })
            }
            Err(_) => false,
        }
    }

    fn kill_process(process_name: &str) {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", process_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();
        thread::sleep(Duration::from_millis(200));
    }

    fn wait_for_process(process_name: &str, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(timeout_ms) {
            if is_process_running(process_name) {
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }
        false
    }

    #[test]
    #[ignore = "Launches real programs - run manually with: cargo test --test e2e_windows_launcher -- --ignored"]
    fn test_launch_simple_program() {
        kill_process("CalculatorApp.exe");
        kill_process("calc.exe");
        thread::sleep(Duration::from_millis(300));

        let launcher = Launcher::new();
        let action = LaunchAction {
            program: "calc.exe".to_string(),
            args: Vec::new(),
            working_dir: None,
            env_vars: Vec::new(),
        };

        let result = launcher.launch(&action);
        assert!(
            result.is_ok(),
            "Should launch calc.exe successfully: {:?}",
            result.err()
        );

        let calc_running = wait_for_process("CalculatorApp.exe", 5000)
            || wait_for_process("calc.exe", 5000);
        assert!(calc_running, "Calculator should be running");

        kill_process("CalculatorApp.exe");
        kill_process("calc.exe");
    }

    #[test]
    #[ignore = "Launches real programs - run manually with: cargo test --test e2e_windows_launcher -- --ignored"]
    fn test_launch_program_with_args() {
        let temp_file = std::env::temp_dir().join("wakem_e2e_test.txt");
        std::fs::write(&temp_file, "Test content").expect("Failed to create temp file");

        kill_process("notepad.exe");
        thread::sleep(Duration::from_millis(300));

        let launcher = Launcher::new();
        let action = LaunchAction {
            program: "notepad.exe".to_string(),
            args: vec![temp_file.to_string_lossy().to_string()],
            working_dir: None,
            env_vars: Vec::new(),
        };

        let result = launcher.launch(&action);
        assert!(result.is_ok(), "Should launch notepad.exe successfully");

        let notepad_running = wait_for_process("notepad.exe", 5000);
        assert!(notepad_running, "Notepad should be running");

        kill_process("notepad.exe");
        thread::sleep(Duration::from_millis(300));
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    #[ignore = "Launches real programs - run manually with: cargo test --test e2e_windows_launcher -- --ignored"]
    fn test_launcher_parse_command_and_launch() {
        // Clean up
        kill_process("CalculatorApp.exe");
        kill_process("calc.exe");
        thread::sleep(Duration::from_millis(300));

        let launcher = Launcher::new();

        // Parse command string
        let action = Launcher::parse_command("calc.exe");
        assert_eq!(action.program, "calc.exe");
        assert!(action.args.is_empty());

        // Launch
        let result = launcher.launch(&action);
        assert!(result.is_ok(), "Should launch successfully");

        // Wait and verify
        let calc_running = wait_for_process("CalculatorApp.exe", 5000)
            || wait_for_process("calc.exe", 5000);
        assert!(calc_running, "Calculator should be running");

        // Clean up
        kill_process("CalculatorApp.exe");
        kill_process("calc.exe");
    }

    #[test]
    #[ignore = "Launches real programs - run manually with: cargo test --test e2e_windows_launcher -- --ignored"]
    fn test_launch_program_with_multiple_args() {
        // Test launching with multiple arguments
        // Using ping command as it's available on all Windows systems

        let launcher = Launcher::new();
        let action = LaunchAction {
            program: "ping.exe".to_string(),
            args: vec!["127.0.0.1".to_string(), "-n".to_string(), "1".to_string()],
            working_dir: None,
            env_vars: Vec::new(),
        };

        // Launch ping
        let result = launcher.launch(&action);
        assert!(
            result.is_ok(),
            "Should launch ping.exe with args successfully: {:?}",
            result.err()
        );

        // Give it time to complete
        thread::sleep(Duration::from_millis(2000));
    }

    #[test]
    #[ignore = "Launches real programs - run manually with: cargo test --test e2e_windows_launcher -- --ignored"]
    fn test_launch_nonexistent_program() {
        let launcher = Launcher::new();
        let action = LaunchAction {
            program: "this_program_does_not_exist_12345.exe".to_string(),
            args: Vec::new(),
            working_dir: None,
            env_vars: Vec::new(),
        };

        let result = launcher.launch(&action);
        assert!(
            result.is_err(),
            "Should fail to launch non-existent program"
        );
    }

    #[test]
    #[ignore = "Launches real programs - run manually with: cargo test --test e2e_windows_launcher -- --ignored"]
    fn test_launch_system_program_cmd() {
        // Test launching cmd.exe with /c to run a simple command
        let temp_output = std::env::temp_dir().join("wakem_cmd_test.txt");

        // Clean up any existing file
        let _ = std::fs::remove_file(&temp_output);

        let launcher = Launcher::new();
        let action = LaunchAction {
            program: "cmd.exe".to_string(),
            args: vec![
                "/c".to_string(),
                format!(
                    "echo wakem_launcher_test > {}",
                    temp_output.to_string_lossy()
                ),
            ],
            working_dir: None,
            env_vars: Vec::new(),
        };

        // Launch cmd
        let result = launcher.launch(&action);
        assert!(
            result.is_ok(),
            "Should launch cmd.exe successfully: {:?}",
            result.err()
        );

        // Wait for command to complete
        thread::sleep(Duration::from_millis(1000));

        // Verify output file was created
        assert!(
            temp_output.exists(),
            "Output file should be created by cmd.exe"
        );

        // Clean up
        let _ = std::fs::remove_file(&temp_output);
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_only_placeholder() {
    // Windows-only tests
}
