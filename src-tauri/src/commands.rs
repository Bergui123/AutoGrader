//! Tauri command surface — the only bridge between React and the Rust core.
//! Every command returns `Result<_, AppError>`, which serializes to a plain
//! error string in the frontend.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::ai::GeminiClient;
use crate::db::{self, models::*, repo};
use crate::error::{AppError, AppResult};
use crate::licensing::{self, LicenseStatus};
use crate::state::AppState;
use crate::{export, extract, gdpr};

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

/// "No Hostage" escape hatch — open the local student data tree in the OS
/// file explorer. Works even while the app is Locked.
#[tauri::command]
pub fn open_local_student_data() -> AppResult<()> {
    let dir = db::students_dir()?;
    db::ensure_path_exists(&dir)?;
    reveal_in_file_explorer(&dir)
}

#[tauri::command]
pub fn open_student_folder(state: State<'_, AppState>, student_id: i64) -> AppResult<()> {
    let folder = {
        let conn = state.db.lock().expect("db lock poisoned");
        let s = repo::get_student(&conn, student_id)?;
        PathBuf::from(s.local_folder_path)
    };
    db::ensure_path_exists(&folder)?;
    reveal_in_file_explorer(&folder)
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

// ── Intelligent routing (extension + MIME sniffing, spec §5) ──────────────────

#[derive(Serialize)]
pub struct RouteInfo {
    pub route: String, // "image" | "digital" | "unsupported"
    pub mime_type: String,
}

#[tauri::command]
pub fn detect_route(path: String) -> RouteInfo {
    let p = PathBuf::from(&path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut head = [0u8; 16];
    let sniff = {
        use std::io::Read;
        std::fs::File::open(&p)
            .and_then(|mut f| f.read(&mut head))
            .map(|n| &head[..n])
            .ok()
    };
    if let Some(b) = sniff {
        if b.starts_with(b"%PDF") {
            return RouteInfo { route: "image".into(), mime_type: "application/pdf".into() };
        }
        if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return RouteInfo { route: "image".into(), mime_type: "image/jpeg".into() };
        }
        if b.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return RouteInfo { route: "image".into(), mime_type: "image/png".into() };
        }
        if b.len() >= 12 && &b[4..8] == b"ftyp" && (&b[8..12] == b"heic" || &b[8..12] == b"heif") {
            return RouteInfo { route: "image".into(), mime_type: "image/heic".into() };
        }
    }
    match ext.as_str() {
        "jpg" | "jpeg" => RouteInfo { route: "image".into(), mime_type: "image/jpeg".into() },
        "png" => RouteInfo { route: "image".into(), mime_type: "image/png".into() },
        "heic" | "heif" => RouteInfo { route: "image".into(), mime_type: "image/heic".into() },
        "pdf" => RouteInfo { route: "image".into(), mime_type: "application/pdf".into() },
        "docx" => RouteInfo { route: "digital".into(), mime_type: "docx".into() },
        "xlsx" => RouteInfo { route: "digital".into(), mime_type: "xlsx".into() },
        "pptx" => RouteInfo { route: "digital".into(), mime_type: "pptx".into() },
        "txt" => RouteInfo { route: "digital".into(), mime_type: "text/plain".into() },
        _ => RouteInfo { route: "unsupported".into(), mime_type: "application/octet-stream".into() },
    }
}

// ── Classes ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_class(state: State<'_, AppState>, name: String) -> AppResult<Class> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::create_class(&conn, name.trim())
}

#[tauri::command]
pub fn list_classes(state: State<'_, AppState>) -> AppResult<Vec<Class>> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::list_classes(&conn)
}

#[tauri::command]
pub fn delete_class(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::delete_class(&conn, id)
}

// ── Students ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_student(
    state: State<'_, AppState>,
    class_id: Option<i64>,
    first_name: String,
    last_name: String,
) -> AppResult<Student> {
    // Each student gets a local folder under Documents/AIGrader/Students.
    let folder = db::students_dir()?
        .join(export::sanitize(&format!("{first_name} {last_name}")));
    db::ensure_path_exists(&folder)?;

    let conn = state.db.lock().expect("db lock poisoned");
    repo::create_student(
        &conn,
        class_id,
        first_name.trim(),
        last_name.trim(),
        &folder.to_string_lossy(),
    )
}

#[tauri::command]
pub fn list_students(
    state: State<'_, AppState>,
    class_id: Option<i64>,
) -> AppResult<Vec<Student>> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::list_students(&conn, class_id)
}

#[tauri::command]
pub fn get_student(state: State<'_, AppState>, id: i64) -> AppResult<Student> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::get_student(&conn, id)
}

#[tauri::command]
pub fn update_student_notes(
    state: State<'_, AppState>,
    id: i64,
    notes: String,
) -> AppResult<()> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::update_student_notes(&conn, id, &notes)
}

#[tauri::command]
pub fn delete_student(state: State<'_, AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::delete_student(&conn, id)
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
pub fn list_assignments(
    state: State<'_, AppState>,
    class_id: Option<i64>,
) -> AppResult<Vec<Assignment>> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::list_assignments(&conn, class_id)
}

// ── Submissions ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_submission(
    state: State<'_, AppState>,
    assignment_id: i64,
    student_id: Option<i64>,
    source_route: String,
    file_type: String,
    pages: Vec<String>,
) -> AppResult<Submission> {
    let mut conn = state.db.lock().expect("db lock poisoned");
    repo::create_submission(
        &mut conn,
        assignment_id,
        student_id,
        &source_route,
        &file_type,
        &pages,
    )
}

#[tauri::command]
pub fn list_submissions(
    state: State<'_, AppState>,
    assignment_id: Option<i64>,
    student_id: Option<i64>,
    status: Option<String>,
) -> AppResult<Vec<Submission>> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::list_submissions(&conn, assignment_id, student_id, status.as_deref())
}

#[tauri::command]
pub fn get_submission(state: State<'_, AppState>, id: i64) -> AppResult<Submission> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::get_submission(&conn, id)
}

#[tauri::command]
pub fn confirm_verified_text(
    state: State<'_, AppState>,
    submission_id: i64,
    verified_markdown: String,
) -> AppResult<()> {
    let conn = state.db.lock().expect("db lock poisoned");
    repo::set_verified(&conn, submission_id, &verified_markdown)
}

// ── Grading gate ────────────────────────────────────────────────────────────────

fn ensure_grading_allowed(state: &AppState) -> AppResult<()> {
    let status = licensing::current_status(state)?;
    if licensing::grading_allowed(&status) {
        Ok(())
    } else {
        Err(AppError::Locked)
    }
}

// ── Extraction pipeline (auto-routed, multi-page) ─────────────────────────────

#[derive(Serialize)]
pub struct ExtractionOutput {
    pub markdown: String,
    pub route: String,
}

/// Phase 2: extract a submission's content. Images run the Multi-Pass Consensus
/// vision pipeline over ALL pages; digital files are parsed natively. Result is
/// GDPR-scrubbed before any upload and saved as the submission's extraction.
#[tauri::command]
pub async fn extract_submission(
    state: State<'_, AppState>,
    submission_id: i64,
) -> AppResult<ExtractionOutput> {
    ensure_grading_allowed(&state)?;

    let (submission, pages) = {
        let conn = state.db.lock().expect("db lock poisoned");
        let sub = repo::get_submission(&conn, submission_id)?;
        let pages = repo::submission_pages(&conn, submission_id)?;
        (sub, pages)
    };

    let markdown = match submission.source_route.as_str() {
        "digital" => {
            // Digital files are parsed individually and concatenated.
            let mut out = String::new();
            for p in &pages {
                let raw = extract::extract_digital(&PathBuf::from(p))?;
                out.push_str(&gdpr::scrub_text(&raw));
                out.push_str("\n\n");
            }
            out.trim().to_string()
        }
        "image" => {
            // Scrub every page, then run consensus OCR over the whole paper.
            let mut payloads = Vec::with_capacity(pages.len());
            for p in &pages {
                payloads.push(gdpr::scrub_for_vision(&PathBuf::from(p))?);
            }
            let client = GeminiClient::new(&state);
            client.extract_handwritten(&payloads).await?
        }
        other => return Err(AppError::Other(format!("unknown route: {other}"))),
    };

    {
        let conn = state.db.lock().expect("db lock poisoned");
        repo::set_extracted(&conn, submission_id, &markdown)?;
    }
    Ok(ExtractionOutput { markdown, route: submission.source_route })
}

// ── Grading (returns result for review; does NOT finalize) ────────────────────

#[tauri::command]
pub async fn grade_submission(
    state: State<'_, AppState>,
    submission_id: i64,
) -> AppResult<GradeResult> {
    ensure_grading_allowed(&state)?;

    let (assignment, text) = {
        let conn = state.db.lock().expect("db lock poisoned");
        let sub = repo::get_submission(&conn, submission_id)?;
        let assignment = repo::get_assignment(&conn, sub.assignment_id)?;
        let text = repo::submission_text(&conn, submission_id)?;
        (assignment, text)
    };

    let client = GeminiClient::new(&state);
    client.grade(&assignment, &text).await
}

/// Phase 5: persist the teacher-reviewed (possibly edited) grade, auto-sync the
/// final score, and inject the Markdown correction into the student's folder.
#[derive(Serialize)]
pub struct FinalizeOutput {
    pub final_score: f64,
    pub output_path: Option<String>,
}

#[tauri::command]
pub fn finalize_grade(
    state: State<'_, AppState>,
    submission_id: i64,
    result: GradeResult,
) -> AppResult<FinalizeOutput> {
    let students_root = db::students_dir()?;
    let conn = state.db.lock().expect("db lock poisoned");

    let submission = repo::get_submission(&conn, submission_id)?;
    let assignment = repo::get_assignment(&conn, submission.assignment_id)?;

    // Write the correction file into the student's folder (if linked).
    let output_path = match submission.student_id {
        Some(sid) => {
            let student = repo::get_student(&conn, sid)?;
            let path = export::write_correction(&students_root, &student, &assignment, &result)?;
            Some(path.to_string_lossy().into_owned())
        }
        None => None,
    };

    let raw_json = serde_json::to_string(&result)?;
    repo::finalize_grade(
        &conn,
        submission_id,
        &raw_json,
        result.summary.final_score,
        output_path.as_deref().unwrap_or(""),
    )?;

    Ok(FinalizeOutput {
        final_score: result.summary.final_score,
        output_path,
    })
}

// ── GDPR scrub (exposed for preview / pipeline) ────────────────────────────────

#[derive(Serialize)]
pub struct ScrubResult {
    pub base64: String,
    pub mime_type: String,
    pub byte_len: usize,
}

#[tauri::command]
pub fn scrub_image(path: String) -> AppResult<ScrubResult> {
    let payload = gdpr::scrub_for_vision(&PathBuf::from(path))?;
    Ok(ScrubResult {
        base64: payload.base64(),
        byte_len: payload.bytes.len(),
        mime_type: payload.mime_type,
    })
}

#[tauri::command]
pub fn scrub_text(input: String) -> String {
    gdpr::scrub_text(&input)
}
