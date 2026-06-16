//! Hand-rolled forward-only migrations keyed on SQLite's `user_version`.
//! Each entry runs once, in order; `user_version` records the high-water mark.

use rusqlite::Connection;

use crate::error::AppResult;

/// Ordered DDL steps. Append new migrations; never edit or reorder existing
/// ones once shipped.
const MIGRATIONS: &[&str] = &[
    // ── 1: licensing + config key/value store ──────────────────────────────
    r#"
    CREATE TABLE IF NOT EXISTS app_config (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
    // ── 2: core grading domain (all rows are local-only PII) ────────────────
    r#"
    CREATE TABLE IF NOT EXISTS students (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        display_name TEXT NOT NULL,
        external_ref TEXT,                 -- teacher's own student/ID number
        created_at   TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS assignments (
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        title               TEXT NOT NULL,
        subject             TEXT NOT NULL,   -- Math, History, French, ...
        education_level     TEXT NOT NULL,   -- Elementary, High School, University
        custom_instructions TEXT NOT NULL DEFAULT '',
        rubric              TEXT NOT NULL DEFAULT '',
        max_score           REAL NOT NULL DEFAULT 100.0,
        created_at          TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS submissions (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        assignment_id INTEGER NOT NULL REFERENCES assignments(id) ON DELETE CASCADE,
        student_id    INTEGER REFERENCES students(id) ON DELETE SET NULL,
        source_route  TEXT NOT NULL,        -- 'image' | 'digital'
        original_path TEXT NOT NULL,        -- local file path, never uploaded
        mime_type     TEXT,
        created_at    TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS extractions (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        submission_id     INTEGER NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,
        markdown          TEXT NOT NULL DEFAULT '',   -- AI / parser output
        verified_markdown TEXT,                        -- teacher-corrected text
        status            TEXT NOT NULL DEFAULT 'pending', -- pending|extracted|verified
        created_at        TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS grades (
        id                    INTEGER PRIMARY KEY AUTOINCREMENT,
        submission_id         INTEGER NOT NULL REFERENCES submissions(id) ON DELETE CASCADE,
        final_score           REAL NOT NULL,
        total_points_deducted REAL NOT NULL,
        general_feedback      TEXT NOT NULL DEFAULT '',
        raw_json              TEXT NOT NULL DEFAULT '{}',  -- full structured AI response
        created_at            TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS inline_corrections (
        id                 INTEGER PRIMARY KEY AUTOINCREMENT,
        grade_id           INTEGER NOT NULL REFERENCES grades(id) ON DELETE CASCADE,
        location_reference TEXT NOT NULL,   -- quote, "Cell C5", "Slide 3", ...
        correction_comment TEXT NOT NULL,
        points_deducted    REAL NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_submissions_assignment ON submissions(assignment_id);
    CREATE INDEX IF NOT EXISTS idx_extractions_submission ON extractions(submission_id);
    CREATE INDEX IF NOT EXISTS idx_grades_submission      ON grades(submission_id);
    CREATE INDEX IF NOT EXISTS idx_corrections_grade      ON inline_corrections(grade_id);
    "#,
];

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    for (idx, ddl) in MIGRATIONS.iter().enumerate() {
        let version = (idx + 1) as i64;
        if version > current {
            conn.execute_batch(ddl)?;
            // PRAGMA does not accept bound params; version is a trusted int.
            conn.execute_batch(&format!("PRAGMA user_version = {version};"))?;
        }
    }
    Ok(())
}
