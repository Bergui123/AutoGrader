//! Tauri command surface — the only bridge between React and the Rust core.
//! Every command returns `Result<_, AppError>`, which serializes to a plain
//! error string in the frontend.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::db::{self, models::*, repo};
use crate::error::{AppError, AppResult};
use crate::gdpr;
use crate::licensing::{self, LicenseStatus};
use crate::state::AppState;

// ── Licensing / boot ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_license_status(state: State<'_, AppState>) -> AppResult<LicenseStatus> {
    licensing::current_status(&state)
}

#[tauri::command]
pub async fn activate(state: State<'_, AppState>, code: String) -> AppResult<LicenseStatus> {
    licensing::activate(&state, code.trim()).await
}

#[tauri::command]
pub async fn run_heartbeat(state: State<'_, AppState>) -> AppResult<LicenseStatus> {
    licensing::heartbeat(&state).await
}

#[derive(Serialize)]
pub struct DataPaths {
    pub data_root: String,
    pub students_dir: String,
    pub db_path: String,
}

#[tauri::command]
pub fn get_data_paths() -> AppResult<DataPaths> {
    Ok(DataPaths {
        data_root: db::data_root()?.to_string_lossy().into_owned(),
        students_dir: db::students_dir()?.to_string_lossy().into_owned(),
        db_path: db::db_path()?.to_string_lossy().into_owned(),
    })
}

/// The "No Hostage" escape hatch — open the local student data tree in the
/// native OS file explorer. Works even while the app is Locked.
#[tauri::command]
pub fn open_local_student_data() -> AppResult<()> {
    let dir = db::students_dir()?;
    db::ensure_path_exists(&dir)?;
    reveal_in_file_explorer(&dir)
}

fn reveal_in_file_explorer(path: &PathBuf) -> AppResult<()> {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program)
        .arg(path)
        .spawn()
        .map_err(AppError::Io)?;
    Ok(())
}

// ── Grading-gate guard ────────────────────────────────────────────────────────

fn ensure_grading_allowed(state: &AppState) -> AppResult<()> {
    let status = licensing::current_status(state)?;
    if licensing::grading_allowed(&status) {
        Ok(())
    } else {
        Err(AppError::Locked)
    }
}

// ── Students ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_student(
    state: State<'_, AppState>,
    display_name: String,
    external_ref: Option<String>,
) -> AppResult<Student> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::create_student(&conn, &display_name, external_ref.as_deref())
}

#[tauri::command]
pub fn list_students(state: State<'_, AppState>) -> AppResult<Vec<Student>> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::list_students(&conn)
}

// ── Assignments ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_assignment(
    state: State<'_, AppState>,
    assignment: NewAssignment,
) -> AppResult<Assignment> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::create_assignment(&conn, &assignment)
}

#[tauri::command]
pub fn list_assignments(state: State<'_, AppState>) -> AppResult<Vec<Assignment>> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::list_assignments(&conn)
}

// ── Submissions / extractions ─────────────────────────────────────────────────

/// Register a dropped file. `source_route` is decided by the frontend dropzone
/// auto-router ('image' or 'digital'); the file itself stays local.
#[tauri::command]
pub fn create_submission(
    state: State<'_, AppState>,
    assignment_id: i64,
    student_id: Option<i64>,
    source_route: String,
    original_path: String,
    mime_type: Option<String>,
) -> AppResult<Submission> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::create_submission(
        &conn,
        assignment_id,
        student_id,
        &source_route,
        &original_path,
        mime_type.as_deref(),
    )
}

#[tauri::command]
pub fn save_extraction(
    state: State<'_, AppState>,
    submission_id: i64,
    markdown: String,
    status: String,
) -> AppResult<i64> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::upsert_extraction(&conn, submission_id, &markdown, &status)
}

/// Pre-Flight Verification Intercept: persist the teacher's corrected text.
#[tauri::command]
pub fn confirm_verified_text(
    state: State<'_, AppState>,
    extraction_id: i64,
    verified_markdown: String,
) -> AppResult<()> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::set_verified_markdown(&conn, extraction_id, &verified_markdown)
}

// ── Grading ────────────────────────────────────────────────────────────────────

/// Persist a structured grading result. Gated: refuses when the app is Locked.
#[tauri::command]
pub fn save_grade_result(
    state: State<'_, AppState>,
    submission_id: i64,
    result: GradeResult,
) -> AppResult<i64> {
    ensure_grading_allowed(&state)?;
    let raw_json = serde_json::to_string(&result)?;
    let mut conn = state.db.lock().expect("db lock poisoned");
    repo::save_grade(&mut conn, submission_id, &result, &raw_json)
}

// ── GDPR scrub (exposed for the pre-upload pipeline / preview) ──────────────────

#[derive(Serialize)]
pub struct ScrubResult {
    /// base64 of the sanitized bytes, ready to hand to the AI client.
    pub base64: String,
    pub mime_type: String,
    pub byte_len: usize,
}

#[tauri::command]
pub fn scrub_image(path: String) -> AppResult<ScrubResult> {
    let payload = gdpr::scrub_image_file(&PathBuf::from(path))?;
    Ok(ScrubResult {
        base64: base64_encode(&payload.bytes),
        mime_type: payload.mime_type,
        byte_len: payload.bytes.len(),
    })
}

#[tauri::command]
pub fn scrub_text(input: String) -> String {
    gdpr::scrub_text(&input)
}

/// Tiny dependency-free base64 (standard alphabet, padded).
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}
