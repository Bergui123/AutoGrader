//! Local-first SQLite layer. The database file lives under the OS Documents
//! tree alongside the student data directories, so a teacher can always reach
//! their data with a normal file explorer — even when the app is locked.

pub mod migrations;
pub mod models;
pub mod repo;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// `Documents/AIGrader` — the root for both the SQLite DB and student files.
pub fn data_root() -> AppResult<PathBuf> {
    let docs = dirs::document_dir()
        .ok_or_else(|| AppError::Config("could not resolve Documents directory".into()))?;
    Ok(docs.join("AIGrader"))
}

pub fn students_dir() -> AppResult<PathBuf> {
    Ok(data_root()?.join("Students"))
}

pub fn db_path() -> AppResult<PathBuf> {
    Ok(data_root()?.join("aigrader.db"))
}

/// Open (creating if needed) the local database and apply migrations.
pub fn open() -> AppResult<Connection> {
    let root = data_root()?;
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(students_dir()?)?;

    let conn = Connection::open(db_path()?)?;
    configure(&conn)?;
    migrations::run(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )?;
    Ok(())
}

/// Test/util helper: open an in-memory DB with the schema applied.
#[allow(dead_code)]
pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    migrations::run(&conn)?;
    Ok(conn)
}

/// Used by the "No Hostage" lock state to point the file explorer somewhere
/// that definitely exists.
pub fn ensure_path_exists(p: &Path) -> AppResult<()> {
    if !p.exists() {
        std::fs::create_dir_all(p)?;
    }
    Ok(())
}
