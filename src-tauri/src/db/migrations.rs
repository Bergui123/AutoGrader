//! Forward-only migrations keyed on SQLite's `user_version`. The schema below
//! mirrors the master spec §8 (App_Config, Class, Student, Assignment,
//! Submission) plus a `submission_files` table for multi-page submissions.

use rusqlite::Connection;

use crate::error::AppResult;

const MIGRATIONS: &[&str] = &[
    // ── 1: full domain schema ───────────────────────────────────────────────
    r#"
    CREATE TABLE IF NOT EXISTS app_config (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    -- Classes group students (e.g. "Math 101").
    CREATE TABLE IF NOT EXISTS classes (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        name       TEXT NOT NULL,
        created_at TEXT NOT NULL
    );

    -- All student PII lives here, locally, only (GDPR §4).
    CREATE TABLE IF NOT EXISTS students (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        class_id          INTEGER REFERENCES classes(id) ON DELETE SET NULL,
        first_name        TEXT NOT NULL,
        last_name         TEXT NOT NULL DEFAULT '',
        local_folder_path TEXT NOT NULL DEFAULT '',
        teacher_notes     TEXT NOT NULL DEFAULT '',
        created_at        TEXT NOT NULL
    );

    -- Assignments carry the Dynamic Persona inputs + polymorphic grading config.
    CREATE TABLE IF NOT EXISTS assignments (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        class_id        INTEGER REFERENCES classes(id) ON DELETE SET NULL,
        title           TEXT NOT NULL,
        subject         TEXT NOT NULL,
        education_level TEXT NOT NULL,
        custom_persona  TEXT NOT NULL DEFAULT '',   -- extra persona instructions
        rubric_template TEXT NOT NULL DEFAULT '',   -- itemized points (Strategy A)
        grading_prompt  TEXT NOT NULL DEFAULT '',   -- freeform behavior (Strategy B)
        max_score       REAL NOT NULL DEFAULT 100.0,
        created_at      TEXT NOT NULL
    );

    -- One submission = one student's paper for one assignment (may be N pages).
    CREATE TABLE IF NOT EXISTS submissions (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        assignment_id     INTEGER NOT NULL REFERENCES assignments(id) ON DELETE CASCADE,
        student_id        INTEGER REFERENCES students(id) ON DELETE SET NULL,
        source_route      TEXT NOT NULL,              -- 'image' | 'digital'
        file_type         TEXT NOT NULL DEFAULT '',   -- jpg/png/pdf/docx/xlsx/pptx/txt
        status            TEXT NOT NULL DEFAULT 'ungraded', -- ungraded|verified|graded
        extracted_markdown TEXT NOT NULL DEFAULT '',
        verified_markdown  TEXT,
        evaluation_json    TEXT,                       -- structured GradeResult
        final_score        REAL,
        local_output_path  TEXT,                       -- Correction_*.md path
        created_at         TEXT NOT NULL
    );

    -- Pages belonging to a submission, in order (Composite/Builder pattern).
    CREATE TABLE IF NOT EXISTS submission_files (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        submission_id INTEGER NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,
        path          TEXT NOT NULL,
        ord           INTEGER NOT NULL DEFAULT 0
    );

    CREATE INDEX IF NOT EXISTS idx_students_class      ON students(class_id);
    CREATE INDEX IF NOT EXISTS idx_assignments_class   ON assignments(class_id);
    CREATE INDEX IF NOT EXISTS idx_submissions_student ON submissions(student_id);
    CREATE INDEX IF NOT EXISTS idx_submissions_assign  ON submissions(assignment_id);
    CREATE INDEX IF NOT EXISTS idx_subfiles_submission ON submission_files(submission_id);
    "#,
];

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (idx, ddl) in MIGRATIONS.iter().enumerate() {
        let version = (idx + 1) as i64;
        if version > current {
            conn.execute_batch(ddl)?;
            conn.execute_batch(&format!("PRAGMA user_version = {version};"))?;
        }
    }
    Ok(())
}
