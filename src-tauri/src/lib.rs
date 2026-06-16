//! Library crate: app construction so the logic stays unit-testable and the
//! thin `main.rs` just calls `run()`.

mod commands;
mod config;
mod db;
mod error;
mod gdpr;
mod licensing;
mod state;

use std::time::Duration;

use tauri::Manager;

use crate::config::AppConfig;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::from_env();

    let conn = db::open().expect("failed to open local database");
    let app_state = AppState::new(conn, config.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(move |app| {
            spawn_heartbeat_pulse(app.handle().clone(), config.heartbeat_hours);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_license_status,
            commands::activate,
            commands::run_heartbeat,
            commands::get_data_paths,
            commands::open_local_student_data,
            commands::create_student,
            commands::list_students,
            commands::create_assignment,
            commands::list_assignments,
            commands::create_submission,
            commands::save_extraction,
            commands::confirm_verified_text,
            commands::save_grade_result,
            commands::scrub_image,
            commands::scrub_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AI Grader");
}

/// Silent 24h (configurable) background heartbeat. Fires once shortly after
/// boot, then on the configured cadence. Failures are swallowed — the grace
/// logic in `licensing` handles the consequences.
fn spawn_heartbeat_pulse(app: tauri::AppHandle, hours: i64) {
    let interval = Duration::from_secs((hours.max(1) as u64) * 3600);
    tauri::async_runtime::spawn(async move {
        // Small initial delay so we don't race window creation.
        tokio::time::sleep(Duration::from_secs(15)).await;
        loop {
            {
                let state = app.state::<AppState>();
                let _ = licensing::heartbeat(&state).await;
            }
            tokio::time::sleep(interval).await;
        }
    });
}
