//! Library crate: app construction so the logic stays unit-testable and the
//! thin `main.rs` just calls `run()`.

mod ai;
mod commands;
mod config;
mod db;
mod error;
mod export;
mod extract;
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

    // DEV ONLY: inject a Gemini API key from the environment so the grading
    // flow can be exercised locally without the production Cloud Function.
    if let Some(key) = &config.dev_api_key {
        use chrono::{Duration, Utc};
        app_state.set_creds(crate::state::EphemeralCreds {
            access_token: key.clone(),
            expires_at: Utc::now() + Duration::days(3650),
        });
        if let Ok(conn) = app_state.db.lock() {
            let _ = db::repo::config_set(&conn, "activated", "true");
        }
    }

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
            commands::open_student_folder,
            commands::detect_route,
            commands::create_class,
            commands::list_classes,
            commands::delete_class,
            commands::create_student,
            commands::list_students,
            commands::get_student,
            commands::update_student_notes,
            commands::delete_student,
            commands::create_assignment,
            commands::list_assignments,
            commands::create_submission,
            commands::list_submissions,
            commands::get_submission,
            commands::confirm_verified_text,
            commands::extract_submission,
            commands::grade_submission,
            commands::finalize_grade,
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
