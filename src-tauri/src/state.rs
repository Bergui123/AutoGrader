//! Shared application state managed by Tauri.
//!
//! Two storage tiers, by design:
//!   * `db`    — durable, local-only SQLite. Holds ALL PII (student names, IDs,
//!               grades, teacher notes) and licensing bookkeeping.
//!   * `creds` — ephemeral AI credentials. RAM ONLY. Never written to disk or
//!               SQLite, and zeroized on drop / re-activation.

use std::sync::{Mutex, RwLock};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use zeroize::Zeroize;

use crate::config::AppConfig;

/// Ephemeral credentials handed out by the license gateway on activation.
/// The token is wiped from memory when this value is dropped or replaced.
#[derive(Clone)]
pub struct EphemeralCreds {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

impl Drop for EphemeralCreds {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}

impl std::fmt::Debug for EphemeralCreds {
    // Never leak the token into logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EphemeralCreds")
            .field("access_token", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub creds: RwLock<Option<EphemeralCreds>>,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(db: Connection, config: AppConfig) -> Self {
        AppState {
            db: Mutex::new(db),
            creds: RwLock::new(None),
            config,
        }
    }

    /// Store fresh ephemeral creds in RAM, replacing (and zeroizing) any prior.
    pub fn set_creds(&self, creds: EphemeralCreds) {
        let mut guard = self.creds.write().expect("creds lock poisoned");
        *guard = Some(creds); // old value dropped here -> zeroized
    }

    pub fn clear_creds(&self) {
        let mut guard = self.creds.write().expect("creds lock poisoned");
        *guard = None;
    }

    pub fn has_valid_creds(&self) -> bool {
        let guard = self.creds.read().expect("creds lock poisoned");
        match guard.as_ref() {
            Some(c) => c.expires_at > Utc::now(),
            None => false,
        }
    }
}
