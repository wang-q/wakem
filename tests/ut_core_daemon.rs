// Daemon Core Logic Tests

use wakem::config::Config;
use wakem::daemon::ServerState;
use wakem::shutdown::ShutdownSignal;
use wakem::types::{InputEvent, KeyEvent, KeyState, MouseEventType};

// ==================== ServerState initialization and configuration loading ====================

/// Test ServerState default initialization
#[tokio::test]
async fn test_server_state_new() {
    let state = ServerState::new(ShutdownSignal::new());

    // Verify default state
    let (active, config_loaded) = state.get_status().await;
    assert!(active, "Default should be enabled");
    assert!(!config_loaded, "Default config not loaded");

    // Verify state can be set
    state.set_active(false).await;
    let (active, _) = state.get_status().await;
    assert!(!active);
}

/// Test Default trait implementation
#[test]
fn test_server_state_default() {
    let state = ServerState::default();
    // Verify Default trait works correctly
    let _ = state;
}

/// Test basic config loading
#[tokio::test]
async fn test_load_config_basic() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();

    let result = state.load_config(config).await;
    assert!(
        result.is_ok(),
        "Basic configuration should load successfully"
    );

    // Verify configuration is marked as loaded
    let (_, config_loaded) = state.get_status().await;
    assert!(config_loaded, "Configuration should be marked as loaded");
}

/// Test config loading with key mappings
#[tokio::test]
async fn test_load_config_with_key_mappings() {
    let state = ServerState::new(ShutdownSignal::new());

    let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(
        result.is_ok(),
        "Configuration with key mappings should load successfully"
    );
}

/// Test config loading with layers (Hold mode)
#[tokio::test]
async fn test_load_config_with_layers_hold_mode() {
    let state = ServerState::new(ShutdownSignal::new());

    let config_str = r#"
[keyboard.layers.navigate]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(
        result.is_ok(),
        "Layer configuration in Hold mode should load successfully"
    );
}

/// Test config loading with layers (Toggle mode)
#[tokio::test]
async fn test_load_config_with_layers_toggle_mode() {
    let state = ServerState::new(ShutdownSignal::new());

    let config_str = r#"
[keyboard.layers.symbols]
activation_key = "Space"
mode = "Toggle"

[keyboard.layers.symbols.mappings]
A = "1"
B = "2"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(
        result.is_ok(),
        "Layer configuration in Toggle mode should load successfully"
    );
}

/// Test config loading with window presets
#[tokio::test]
async fn test_load_config_with_window_presets() {
    let state = ServerState::new(ShutdownSignal::new());

    let config_str = r#"
[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(
        result.is_ok(),
        "Configuration with window shortcuts should load successfully"
    );
}

/// Test full config loading
#[tokio::test]
async fn test_load_config_full() {
    let state = ServerState::new(ShutdownSignal::new());

    let config_str = r#"
log_level = "debug"
tray_icon = true
auto_reload = true

[keyboard.remap]
CapsLock = "Backspace"

[keyboard.layers.navigate]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"

[launch]
F1 = "notepad.exe"

[network]
enabled = true
instance_id = 1
auth_key = "test_key"

[macros]
test_macro = []

[macro_bindings]
F5 = "test_macro"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(
        result.is_ok(),
        "Full configuration should load successfully"
    );
}

// ==================== Input event handling ====================

/// Test keyboard event handling (basic)
#[tokio::test]
async fn test_process_input_event_key() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Create a simple keyboard press event
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed); // 'A' key
    let event = InputEvent::Key(key_event);

    // Process event (should not panic)
    state.process_input_event(event).await;
}

/// Test mouse wheel event handling
#[tokio::test]
async fn test_process_input_event_mouse_wheel() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Create mouse wheel event
    let mouse_event = wakem::types::MouseEvent::new(MouseEventType::Wheel(120), 0, 0);
    let event = InputEvent::Mouse(mouse_event);

    // Process event
    state.process_input_event(event).await;
}

/// Test event handling in disabled state
#[tokio::test]
async fn test_process_input_event_disabled() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Disable mapping
    state.set_active(false).await;

    // Create event
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    let event = InputEvent::Key(key_event);

    // Process event (should be ignored but not panic)
    state.process_input_event(event).await;

    // Verify still in disabled state
    let (active, _) = state.get_status().await;
    assert!(!active);
}

/// Test injected event ignored
#[tokio::test]
async fn test_process_injected_event_ignored() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Create an injected event (is_injected = true)
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected();
    let event = InputEvent::Key(key_event);

    // Process injected event (should be ignored but not panic)
    state.process_input_event(event).await;
}

/// Test mouse move event handling
#[tokio::test]
async fn test_process_input_event_mouse_move() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Create mouse move event
    let mouse_event = wakem::types::MouseEvent::new(MouseEventType::Move, 100, 200);
    let event = InputEvent::Mouse(mouse_event);

    // Process event
    state.process_input_event(event).await;
}

/// Test mouse button event handling
#[tokio::test]
async fn test_process_input_event_mouse_button() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Create mouse button press event
    let mouse_event = wakem::types::MouseEvent::new(
        MouseEventType::ButtonDown(wakem::types::MouseButton::Left),
        0,
        0,
    );
    let event = InputEvent::Mouse(mouse_event);

    // Process event
    state.process_input_event(event).await;
}

// ==================== Macro management functions ====================

/// Test start and stop macro recording
#[tokio::test]
async fn test_start_stop_macro_recording() {
    let state = ServerState::new(ShutdownSignal::new());

    // Start recording
    let result = state.start_macro_recording("test_macro").await;
    assert!(result.is_ok(), "Start recording should succeed");

    // Verify recording is active
    assert!(
        state.is_recording_macro().await,
        "Should be in recording state"
    );

    // Stop recording
    let result = state.stop_macro_recording().await;
    assert!(result.is_ok(), "Stop recording should succeed");

    // Verify no longer recording
    assert!(
        !state.is_recording_macro().await,
        "Should not be in recording state"
    );
}

/// Test play macro
#[tokio::test]
async fn test_play_macro() {
    let state = ServerState::new(ShutdownSignal::new());

    // First add a simple macro to the config (using empty steps)
    let config_str = r#"
[macros]
simple_macro = []
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    // Play macro
    let result = state.play_macro("simple_macro").await;
    // Note: This may fail because macro playback depends on output device
    // We only verify it does not panic
    let _ = result;
}

/// Test play non-existent macro (error handling)
#[tokio::test]
async fn test_play_nonexistent_macro() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Try to play non-existent macro
    let result = state.play_macro("nonexistent_macro").await;
    assert!(
        result.is_err(),
        "Playing non-existent macro should return error"
    );
}

/// Test get macros list
#[tokio::test]
async fn test_get_macros_list() {
    let state = ServerState::new(ShutdownSignal::new());

    // In empty config, macro list should be empty
    let macros = state.get_macros().await;
    assert!(
        macros.is_empty(),
        "Macro list should be empty in empty config"
    );

    // Add some macros
    let config_str = r#"
[macros]
macro1 = []
macro2 = []
macro3 = []
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    let macros = state.get_macros().await;
    assert_eq!(macros.len(), 3, "Should have 3 macros");
    assert!(macros.contains(&"macro1".to_string()));
    assert!(macros.contains(&"macro2".to_string()));
    assert!(macros.contains(&"macro3".to_string()));
}

/// Test Delete macro
#[tokio::test]
async fn test_delete_macro() {
    let state = ServerState::new(ShutdownSignal::new());

    // First add a macro with a test-specific instance_id to avoid overwriting user config
    let config_str = r#"
[macros]
temp_macro = []

[network]
instance_id = 999
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    // Verify macro exists
    let macros = state.get_macros().await;
    assert!(macros.contains(&"temp_macro".to_string()));

    // Delete macro - should succeed without affecting user config
    let result = state.delete_macro("temp_macro").await;
    assert!(result.is_ok(), "Delete macro should succeed");

    // Verify macro is deleted
    let macros = state.get_macros().await;
    assert!(!macros.contains(&"temp_macro".to_string()));
}

/// Test Delete non-existent macro (error handling)
#[tokio::test]
async fn test_delete_nonexistent_macro() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Delete non-existent macro
    let result = state.delete_macro("nonexistent").await;
    assert!(
        result.is_err(),
        "Delete non-existent macroshould return error"
    );
}

/// Test Bind macro to trigger key
#[tokio::test]
async fn test_bind_macro() {
    let state = ServerState::new(ShutdownSignal::new());

    // First add a macro with a test-specific instance_id to avoid overwriting user config
    let config_str = r#"
[macros]
my_macro = []

[network]
instance_id = 999
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    // Bind macro - should succeed without affecting user config
    let result = state.bind_macro("my_macro", "F5").await;
    assert!(result.is_ok(), "Bind macro should succeed");
}

/// Test Bind non-existent macro (error handling)
#[tokio::test]
async fn test_bind_nonexistent_macro() {
    let state = ServerState::new(ShutdownSignal::new());
    let config = Config::default();
    let _ = state.load_config(config).await;

    // Bind non-existent macro
    let result = state.bind_macro("nonexistent", "F5").await;
    assert!(
        result.is_err(),
        "Bind non-existent macroshould return error"
    );
}

// ==================== State management ====================

/// Test enable/disable state toggle
#[tokio::test]
async fn test_set_active_state_toggle() {
    let state = ServerState::new(ShutdownSignal::new());

    // Default is enabled
    let (active, _) = state.get_status().await;
    assert!(active);

    // Switch to disabled
    state.set_active(false).await;
    let (active, _) = state.get_status().await;
    assert!(!active);

    // Switch back to enabled
    state.set_active(true).await;
    let (active, _) = state.get_status().await;
    assert!(active);

    // Set same value multiple times
    state.set_active(true).await;
    state.set_active(true).await;
    let (active, _) = state.get_status().await;
    assert!(active);
}

/// Test status query consistency
#[tokio::test]
async fn test_get_status_consistency() {
    let state = ServerState::new(ShutdownSignal::new());

    // Multiple queries should return same result
    let status1 = state.get_status().await;
    let status2 = state.get_status().await;
    let status3 = state.get_status().await;

    assert_eq!(status1, status2);
    assert_eq!(status2, status3);
}

/// Test platform service initialization (Windows specific)
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_init_notification_service() {
    let state = ServerState::new(ShutdownSignal::new());

    let ctx = wakem::platform::traits::NotificationInitContext {
        native_handle: Some(12345),
    };
    state.init_notification_service(&ctx).await;

    // Verify notification function available (should not panic)
    let result = state.show_notification("Test", "Test message").await;
    // May fail (Windows API), but should not panic
    let _ = result;
}

/// Test platform service initialization (macOS version)
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_init_notification_service() {
    let state = ServerState::new(ShutdownSignal::new());

    let ctx = wakem::platform::traits::NotificationInitContext {
        native_handle: None,
    };
    state.init_notification_service(&ctx).await;

    // Verify notification function available (should not panic)
    let result = state.show_notification("Test", "Test message").await;
    let _ = result;
}
