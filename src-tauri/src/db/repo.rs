//! Repository functions. All take a `&Connection` so they compose under the
//! single `AppState.db` mutex and stay unit-testable against an in-memory DB.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::db::models::*;
use crate::error::AppResult;

// ── Key/value config store (also backs licensing flags) ─────────────────────

pub fn config_get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let v = conn
        .query_row(
            "SELECT value FROM app_config WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(v)
}

pub fn config_set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_config(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn config_delete(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute("DELETE FROM app_config WHERE key = ?1", params![key])?;
    Ok(())
}

// ── Students ────────────────────────────────────────────────────────────────

pub fn create_student(
    conn: &Connection,
    display_name: &str,
    external_ref: Option<&str>,
) -> AppResult<Student> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO students(display_name, external_ref, created_at)
         VALUES(?1, ?2, ?3)",
        params![display_name, external_ref, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Student {
        id,
        display_name: display_name.to_string(),
        external_ref: external_ref.map(str::to_string),
        created_at: now,
    })
}

pub fn list_students(conn: &Connection) -> AppResult<Vec<Student>> {
    let mut stmt = conn.prepare(
        "SELECT id, display_name, external_ref, created_at
         FROM students ORDER BY display_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Student {
            id: r.get(0)?,
            display_name: r.get(1)?,
            external_ref: r.get(2)?,
            created_at: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// ── Assignments ──────────────────────────────────────────────────────────────

pub fn create_assignment(conn: &Connection, a: &NewAssignment) -> AppResult<Assignment> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO assignments
           (title, subject, education_level, custom_instructions, rubric, max_score, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            a.title,
            a.subject,
            a.education_level,
            a.custom_instructions,
            a.rubric,
            a.max_score,
            now
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Assignment {
        id,
        title: a.title.clone(),
        subject: a.subject.clone(),
        education_level: a.education_level.clone(),
        custom_instructions: a.custom_instructions.clone(),
        rubric: a.rubric.clone(),
        max_score: a.max_score,
        created_at: now,
    })
}

pub fn list_assignments(conn: &Connection) -> AppResult<Vec<Assignment>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, subject, education_level, custom_instructions,
                rubric, max_score, created_at
         FROM assignments ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Assignment {
            id: r.get(0)?,
            title: r.get(1)?,
            subject: r.get(2)?,
            education_level: r.get(3)?,
            custom_instructions: r.get(4)?,
            rubric: r.get(5)?,
            max_score: r.get(6)?,
            created_at: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// ── Submissions ──────────────────────────────────────────────────────────────

pub fn create_submission(
    conn: &Connection,
    assignment_id: i64,
    student_id: Option<i64>,
    source_route: &str,
    original_path: &str,
    mime_type: Option<&str>,
) -> AppResult<Submission> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO submissions
           (assignment_id, student_id, source_route, original_path, mime_type, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![assignment_id, student_id, source_route, original_path, mime_type, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Submission {
        id,
        assignment_id,
        student_id,
        source_route: source_route.to_string(),
        original_path: original_path.to_string(),
        mime_type: mime_type.map(str::to_string),
        created_at: now,
    })
}

// ── Extractions ──────────────────────────────────────────────────────────────

pub fn upsert_extraction(
    conn: &Connection,
    submission_id: i64,
    markdown: &str,
    status: &str,
) -> AppResult<i64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO extractions(submission_id, markdown, status, created_at)
         VALUES(?1, ?2, ?3, ?4)",
        params![submission_id, markdown, status, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn set_verified_markdown(
    conn: &Connection,
    extraction_id: i64,
    verified: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE extractions SET verified_markdown = ?1, status = 'verified' WHERE id = ?2",
        params![verified, extraction_id],
    )?;
    Ok(())
}

// ── Grades ───────────────────────────────────────────────────────────────────

/// Persist a structured grading result and its inline corrections atomically.
pub fn save_grade(
    conn: &mut Connection,
    submission_id: i64,
    result: &GradeResult,
    raw_json: &str,
) -> AppResult<i64> {
    let now = Utc::now().to_rfc3339();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO grades
           (submission_id, final_score, total_points_deducted, general_feedback, raw_json, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            submission_id,
            result.summary.final_score,
            result.summary.total_points_deducted,
            result.summary.general_feedback,
            raw_json,
            now
        ],
    )?;
    let grade_id = tx.last_insert_rowid();
    for c in &result.inline_corrections {
        tx.execute(
            "INSERT INTO inline_corrections
               (grade_id, location_reference, correction_comment, points_deducted)
             VALUES(?1, ?2, ?3, ?4)",
            params![grade_id, c.location_reference, c.correction_comment, c.points_deducted],
        )?;
    }
    tx.commit()?;
    Ok(grade_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn config_roundtrip() {
        let conn = mem();
        assert!(config_get(&conn, "k").unwrap().is_none());
        config_set(&conn, "k", "v1").unwrap();
        config_set(&conn, "k", "v2").unwrap();
        assert_eq!(config_get(&conn, "k").unwrap().as_deref(), Some("v2"));
    }

    #[test]
    fn grade_with_corrections_persists() {
        let mut conn = mem();
        let a = create_assignment(
            &conn,
            &NewAssignment {
                title: "T".into(),
                subject: "Math".into(),
                education_level: "High School".into(),
                custom_instructions: String::new(),
                rubric: String::new(),
                max_score: 100.0,
            },
        )
        .unwrap();
        let s = create_submission(&conn, a.id, None, "image", "/tmp/x.png", Some("image/png"))
            .unwrap();
        let result = GradeResult {
            inline_corrections: vec![InlineCorrection {
                id: 0,
                location_reference: "Cell C5".into(),
                correction_comment: "wrong formula".into(),
                points_deducted: -2.0,
            }],
            summary: GradeSummary {
                general_feedback: "ok".into(),
                total_points_deducted: -2.0,
                final_score: 98.0,
            },
        };
        let gid = save_grade(&mut conn, s.id, &result, "{}").unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM inline_corrections WHERE grade_id = ?1",
                params![gid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
