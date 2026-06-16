//! Repository layer (Repository Pattern). Every function takes `&Connection`
//! so calls compose under the single `AppState.db` mutex and stay testable
//! against an in-memory DB.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::db::models::*;
use crate::error::{AppError, AppResult};

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn not_found(what: &str, id: i64) -> AppError {
    AppError::Other(format!("{what} {id} not found"))
}

// ── Key/value config store (backs licensing flags) ──────────────────────────

pub fn config_get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM app_config WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
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

// ── Classes ──────────────────────────────────────────────────────────────────

pub fn create_class(conn: &Connection, name: &str) -> AppResult<Class> {
    let now = now();
    conn.execute(
        "INSERT INTO classes(name, created_at) VALUES(?1, ?2)",
        params![name, now],
    )?;
    Ok(Class {
        id: conn.last_insert_rowid(),
        name: name.to_string(),
        created_at: now,
    })
}

pub fn list_classes(conn: &Connection) -> AppResult<Vec<Class>> {
    let mut stmt =
        conn.prepare("SELECT id, name, created_at FROM classes ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| {
        Ok(Class {
            id: r.get(0)?,
            name: r.get(1)?,
            created_at: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn delete_class(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM classes WHERE id = ?1", params![id])?;
    Ok(())
}

// ── Students ────────────────────────────────────────────────────────────────

fn map_student(r: &rusqlite::Row) -> rusqlite::Result<Student> {
    Ok(Student {
        id: r.get(0)?,
        class_id: r.get(1)?,
        first_name: r.get(2)?,
        last_name: r.get(3)?,
        local_folder_path: r.get(4)?,
        teacher_notes: r.get(5)?,
        created_at: r.get(6)?,
    })
}

const STUDENT_COLS: &str =
    "id, class_id, first_name, last_name, local_folder_path, teacher_notes, created_at";

pub fn create_student(
    conn: &Connection,
    class_id: Option<i64>,
    first_name: &str,
    last_name: &str,
    local_folder_path: &str,
) -> AppResult<Student> {
    let now = now();
    conn.execute(
        "INSERT INTO students(class_id, first_name, last_name, local_folder_path, teacher_notes, created_at)
         VALUES(?1, ?2, ?3, ?4, '', ?5)",
        params![class_id, first_name, last_name, local_folder_path, now],
    )?;
    get_student(conn, conn.last_insert_rowid())
}

pub fn get_student(conn: &Connection, id: i64) -> AppResult<Student> {
    conn.query_row(
        &format!("SELECT {STUDENT_COLS} FROM students WHERE id = ?1"),
        params![id],
        map_student,
    )
    .optional()?
    .ok_or_else(|| not_found("student", id))
}

pub fn list_students(conn: &Connection, class_id: Option<i64>) -> AppResult<Vec<Student>> {
    let (sql, args): (String, Vec<i64>) = match class_id {
        Some(cid) => (
            format!("SELECT {STUDENT_COLS} FROM students WHERE class_id = ?1 ORDER BY last_name, first_name COLLATE NOCASE"),
            vec![cid],
        ),
        None => (
            format!("SELECT {STUDENT_COLS} FROM students ORDER BY last_name, first_name COLLATE NOCASE"),
            vec![],
        ),
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args), map_student)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn update_student_notes(conn: &Connection, id: i64, notes: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE students SET teacher_notes = ?1 WHERE id = ?2",
        params![notes, id],
    )?;
    Ok(())
}

pub fn delete_student(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM students WHERE id = ?1", params![id])?;
    Ok(())
}

// ── Assignments ──────────────────────────────────────────────────────────────

fn map_assignment(r: &rusqlite::Row) -> rusqlite::Result<Assignment> {
    Ok(Assignment {
        id: r.get(0)?,
        class_id: r.get(1)?,
        title: r.get(2)?,
        subject: r.get(3)?,
        education_level: r.get(4)?,
        custom_persona: r.get(5)?,
        rubric_template: r.get(6)?,
        grading_prompt: r.get(7)?,
        max_score: r.get(8)?,
        created_at: r.get(9)?,
    })
}

const ASSIGN_COLS: &str = "id, class_id, title, subject, education_level, custom_persona, rubric_template, grading_prompt, max_score, created_at";

pub fn create_assignment(conn: &Connection, a: &NewAssignment) -> AppResult<Assignment> {
    let now = now();
    conn.execute(
        "INSERT INTO assignments
           (class_id, title, subject, education_level, custom_persona, rubric_template, grading_prompt, max_score, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            a.class_id, a.title, a.subject, a.education_level,
            a.custom_persona, a.rubric_template, a.grading_prompt, a.max_score, now
        ],
    )?;
    get_assignment(conn, conn.last_insert_rowid())
}

pub fn get_assignment(conn: &Connection, id: i64) -> AppResult<Assignment> {
    conn.query_row(
        &format!("SELECT {ASSIGN_COLS} FROM assignments WHERE id = ?1"),
        params![id],
        map_assignment,
    )
    .optional()?
    .ok_or_else(|| not_found("assignment", id))
}

pub fn list_assignments(conn: &Connection, class_id: Option<i64>) -> AppResult<Vec<Assignment>> {
    let (sql, args): (String, Vec<i64>) = match class_id {
        Some(cid) => (
            format!("SELECT {ASSIGN_COLS} FROM assignments WHERE class_id = ?1 ORDER BY created_at DESC"),
            vec![cid],
        ),
        None => (
            format!("SELECT {ASSIGN_COLS} FROM assignments ORDER BY created_at DESC"),
            vec![],
        ),
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(args), map_assignment)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

// ── Submissions ──────────────────────────────────────────────────────────────

fn map_submission(r: &rusqlite::Row) -> rusqlite::Result<Submission> {
    Ok(Submission {
        id: r.get(0)?,
        assignment_id: r.get(1)?,
        student_id: r.get(2)?,
        source_route: r.get(3)?,
        file_type: r.get(4)?,
        status: r.get(5)?,
        extracted_markdown: r.get(6)?,
        verified_markdown: r.get(7)?,
        evaluation_json: r.get(8)?,
        final_score: r.get(9)?,
        local_output_path: r.get(10)?,
        created_at: r.get(11)?,
    })
}

const SUB_COLS: &str = "id, assignment_id, student_id, source_route, file_type, status, extracted_markdown, verified_markdown, evaluation_json, final_score, local_output_path, created_at";

/// Create a submission with one or more page files (Composite pattern).
pub fn create_submission(
    conn: &mut Connection,
    assignment_id: i64,
    student_id: Option<i64>,
    source_route: &str,
    file_type: &str,
    pages: &[String],
) -> AppResult<Submission> {
    let now = now();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO submissions
           (assignment_id, student_id, source_route, file_type, status, created_at)
         VALUES(?1, ?2, ?3, ?4, 'ungraded', ?5)",
        params![assignment_id, student_id, source_route, file_type, now],
    )?;
    let id = tx.last_insert_rowid();
    for (i, p) in pages.iter().enumerate() {
        tx.execute(
            "INSERT INTO submission_files(submission_id, path, ord) VALUES(?1, ?2, ?3)",
            params![id, p, i as i64],
        )?;
    }
    tx.commit()?;
    get_submission(conn, id)
}

pub fn get_submission(conn: &Connection, id: i64) -> AppResult<Submission> {
    conn.query_row(
        &format!("SELECT {SUB_COLS} FROM submissions WHERE id = ?1"),
        params![id],
        map_submission,
    )
    .optional()?
    .ok_or_else(|| not_found("submission", id))
}

pub fn submission_pages(conn: &Connection, submission_id: i64) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT path FROM submission_files WHERE submission_id = ?1 ORDER BY ord",
    )?;
    let rows = stmt.query_map(params![submission_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn list_submissions(
    conn: &Connection,
    assignment_id: Option<i64>,
    student_id: Option<i64>,
    status: Option<&str>,
) -> AppResult<Vec<Submission>> {
    // Nullable filters: a NULL parameter disables that clause.
    let sql = format!(
        "SELECT {SUB_COLS} FROM submissions
         WHERE (?1 IS NULL OR assignment_id = ?1)
           AND (?2 IS NULL OR student_id = ?2)
           AND (?3 IS NULL OR status = ?3)
         ORDER BY created_at DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![assignment_id, student_id, status], map_submission)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn set_extracted(conn: &Connection, submission_id: i64, markdown: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE submissions SET extracted_markdown = ?1 WHERE id = ?2",
        params![markdown, submission_id],
    )?;
    Ok(())
}

pub fn set_verified(conn: &Connection, submission_id: i64, verified: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE submissions SET verified_markdown = ?1, status = 'verified' WHERE id = ?2",
        params![verified, submission_id],
    )?;
    Ok(())
}

/// Best available text for grading: verified if present, else raw extracted.
pub fn submission_text(conn: &Connection, submission_id: i64) -> AppResult<String> {
    let sub = get_submission(conn, submission_id)?;
    Ok(sub
        .verified_markdown
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(sub.extracted_markdown))
}

/// Finalize: persist the (possibly teacher-edited) evaluation + score + output.
pub fn finalize_grade(
    conn: &Connection,
    submission_id: i64,
    evaluation_json: &str,
    final_score: f64,
    output_path: &str,
) -> AppResult<()> {
    conn.execute(
        "UPDATE submissions
         SET evaluation_json = ?1, final_score = ?2, local_output_path = ?3, status = 'graded'
         WHERE id = ?4",
        params![evaluation_json, final_score, output_path, submission_id],
    )?;
    Ok(())
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
    fn class_student_assignment_flow() {
        let mut conn = mem();
        let c = create_class(&conn, "Math 101").unwrap();
        let s = create_student(&conn, Some(c.id), "Xavier", "Bergeron", "/x").unwrap();
        assert_eq!(s.full_name(), "Xavier Bergeron");

        update_student_notes(&conn, s.id, "Strong in algebra").unwrap();
        assert_eq!(get_student(&conn, s.id).unwrap().teacher_notes, "Strong in algebra");

        let a = create_assignment(
            &conn,
            &NewAssignment {
                class_id: Some(c.id),
                title: "Quiz".into(),
                subject: "Math".into(),
                education_level: "High School".into(),
                custom_persona: String::new(),
                rubric_template: "Q1 5pts".into(),
                grading_prompt: String::new(),
                max_score: 10.0,
            },
        )
        .unwrap();

        let sub = create_submission(
            &mut conn,
            a.id,
            Some(s.id),
            "image",
            "png",
            &["/p1.png".into(), "/p2.png".into()],
        )
        .unwrap();
        assert_eq!(submission_pages(&conn, sub.id).unwrap().len(), 2);

        finalize_grade(&conn, sub.id, "{}", 8.0, "/out.md").unwrap();
        let graded = list_submissions(&conn, None, Some(s.id), Some("graded")).unwrap();
        assert_eq!(graded.len(), 1);
        assert_eq!(graded[0].final_score, Some(8.0));
    }
}
