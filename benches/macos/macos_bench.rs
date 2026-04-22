//! macOS platform performance benchmarks
//!
//! Benchmarks for macOS-specific operations using Criterion.
//! Run with: cargo bench --bench macos_bench
//!
//! These benchmarks measure:
//! - AppleScript-based window operation latency
//! - Core Graphics native API performance
//! - osascript process startup overhead

#[cfg(target_os = "macos")]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::process::Command;
use std::time::Instant;

/// Benchmark: Get window info via AppleScript (current implementation)
fn bench_applescript_get_window_info(c: &mut Criterion) {
    let script = r#"
        tell application "System Events"
            set frontApp to first application process whose frontmost is true
            set appName to name of frontApp
            try
                set winTitle to name of first window of frontApp
            on error
                set winTitle to ""
            end try
            try
                set winPos to position of first window of frontApp
            on error
                set winPos to {0, 0}
            end try
            try
                set winSize to size of first window of frontApp
            on error
                set winSize to {800, 600}
            end try
            return {appName, winTitle, item 1 of winPos, item 2 of winPos, item 1 of winSize, item 2 of winSize}
        end tell
    "#;

    c.bench_function("applescript_get_window_info", |b| {
        b.iter(|| {
            let output = Command::new("osascript")
                .arg("-e")
                .arg(black_box(script))
                .output()
                .expect("Failed to execute AppleScript");
            assert!(output.status.success());
            black_box(output)
        });
    });
}

/// Benchmark: Set window position via AppleScript (current implementation)
fn bench_applescript_set_window_pos(c: &mut Criterion) {
    c.bench_with_input(
        BenchmarkId::new("applescript_set_window_pos", "default"),
        &(100_i32, 100_i32, 800_i32, 600_i32),
        |b, &(x, y, w, h)| {
            let screen_height = 1080_i32;
            let apple_y = screen_height - y - h;

            let script = format!(
                r#"tell application "System Events"
                    set position of first window of (first application process whose frontmost is true) to {{{}, {}}}
                    set size of first window of (first application process whose frontmost is true) to {{{}, {}}}
                end tell"#,
                x, apple_y, w, h
            );

            b.iter(|| {
                let output = Command::new("osascript")
                    .arg("-e")
                    .arg(black_box(&script))
                    .output()
                    .expect("Failed to execute AppleScript");
                assert!(output.status.success());
            });
        },
    );
}

/// Benchmark: osascript process startup overhead (minimal script)
fn bench_osascript_startup_overhead(c: &mut Criterion) {
    c.bench_function("osascript_startup_overhead", |b| {
        b.iter(|| {
            let output = Command::new("osascript")
                .arg("-e")
                .arg(black_box("return \"hello\""))
                .output()
                .expect("Failed to execute osascript");
            assert!(output.status.success());
        });
    });
}

/// Benchmark: Core Graphics display info (native API)
fn bench_core_graphics_display_info(c: &mut Criterion) {
    use core_graphics::display::{CGDisplay, CGDisplayBounds};

    c.bench_function("coregraphics_cgdisplaybounds", |b| {
        b.iter(|| unsafe {
            let display_id = CGDisplay::main().id;
            let bounds = CGDisplayBounds(display_id);
            assert!(bounds.size.width > 0.0);
            assert!(bounds.size.height > 0.0);
            black_box(bounds)
        });
    });
}

/// Benchmark comparison: AppleScript vs Core Graphics for monitor info
fn bench_monitor_info_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("monitor_info_comparison");

    // Core Graphics native approach
    group.bench_function("cg_display_bounds", |b| {
        use core_graphics::display::{CGDisplay, CGDisplayBounds};
        b.iter(|| unsafe {
            let display_id = CGDisplay::main().id;
            black_box(CGDisplayBounds(display_id))
        });
    });

    // AppleScript fallback (if implemented)
    // Note: This would require an AppleScript that returns monitor info
    // Currently not used in production, so skipped

    group.finish();
}

#[cfg(target_os = "macos")]
criterion_group!(
    macos_benches,
    bench_applescript_get_window_info,
    bench_applescript_set_window_pos,
    bench_osascript_startup_overhead,
    bench_core_graphics_display_info,
    bench_monitor_info_comparison,
);

#[cfg(target_os = "macos")]
criterion_main!(macos_benches);
