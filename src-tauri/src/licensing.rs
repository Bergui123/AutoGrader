//! Remote licensing gatekeeper + 7-day "No Hostage" grace logic (spec §3).
//!
//! Contract with the Cloud Function (replace endpoint via .env):
//!   POST {endpoint}        body: {"activation_code": "..."}            (activate)
//!   POST {endpoint}/pulse  body: {"activation_code": "..."}            (heartbeat)
//!   200 response (both): {
//!       "status": "active" | "expired" | "invalid",
//!       "access_token": "ya29....",   // ephemeral Vertex AI token (active only)
//!       "expires_in": 3600            // seconds
//!   }
//!
//! Local bookkeeping lives in `app_config`. Ephemeral creds live ONLY in RAM
//! (`AppState.creds`) and are never persisted.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::db::repo;
use crate::error::{AppError, AppResult};
use crate::state::{AppState, EphemeralCreds};

// app_config keys
const K_ACTIVATED: &str = "activated";
const K_ACTIVATION_CODE: &str = "activation_code";
const K_LAST_HEARTBEAT: &str = "last_heartbeat_at";
const K_GRACE_STARTED: &str = "grace_period_started_at";

/// License state surfaced to the UI to drive the gate / banner / lock screen.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LicenseStatus {
    /// No valid activation flag — show the Activation Gate.
    Unactivated,
    /// Fully licensed and (when creds are present) ready to grade.
    Active { ai_ready: bool },
    /// Heartbeat failing/expired but within the 7-day window.
    Grace { days_left: i64, ai_ready: bool },
    /// Grace window elapsed — lock grading, keep local data reachable.
    Locked,
}

#[derive(Debug, Deserialize)]
struct GatewayResponse {
    status: String,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Debug, Serialize)]
struct GatewayRequest<'a> {
    activation_code: &'a str,
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Derive the current status purely from persisted flags + the grace clock.
/// `ai_ready` reflects whether usable ephemeral creds exist in RAM right now.
pub fn compute_status(
    conn: &rusqlite::Connection,
    config: &AppConfig,
    ai_ready: bool,
) -> AppResult<LicenseStatus> {
    let activated = repo::config_get(conn, K_ACTIVATED)?.as_deref() == Some("true");
    if !activated {
        return Ok(LicenseStatus::Unactivated);
    }

    if let Some(started_raw) = repo::config_get(conn, K_GRACE_STARTED)? {
        if let Some(started) = parse_ts(&started_raw) {
            let elapsed_days = (Utc::now() - started).num_days();
            let days_left = config.grace_days - elapsed_days;
            if days_left <= 0 {
                return Ok(LicenseStatus::Locked);
            }
            return Ok(LicenseStatus::Grace { days_left, ai_ready });
        }
    }

    Ok(LicenseStatus::Active { ai_ready })
}

/// Apply a successful gateway verdict: store ephemeral creds in RAM, mark
/// activated, clear any grace clock.
fn apply_active(state: &AppState, resp: &GatewayResponse) -> AppResult<()> {
    if let Some(token) = &resp.access_token {
        let ttl = resp.expires_in.unwrap_or(3600).max(60);
        state.set_creds(EphemeralCreds {
            access_token: token.clone(),
            expires_at: Utc::now() + Duration::seconds(ttl),
        });
    }
    let conn = state.db.lock().expect("db lock poisoned");
    repo::config_set(&conn, K_ACTIVATED, "true")?;
    repo::config_set(&conn, K_LAST_HEARTBEAT, &Utc::now().to_rfc3339())?;
    repo::config_delete(&conn, K_GRACE_STARTED)?;
    Ok(())
}

/// Start the grace clock if it isn't already running. Idempotent.
fn begin_grace(state: &AppState) -> AppResult<()> {
    let conn = state.db.lock().expect("db lock poisoned");
    if repo::config_get(&conn, K_GRACE_STARTED)?.is_none() {
        repo::config_set(&conn, K_GRACE_STARTED, &Utc::now().to_rfc3339())?;
    }
    // Expired verdict invalidates any cached AI creds immediately.
    drop(conn);
    state.clear_creds();
    Ok(())
}

async fn post(endpoint: &str, code: &str) -> AppResult<GatewayResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let resp = client
        .post(endpoint)
        .json(&GatewayRequest { activation_code: code })
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json::<GatewayResponse>().await?)
}

/// Validate a user-entered activation code against the Cloud Function.
pub async fn activate(state: &AppState, code: &str) -> AppResult<LicenseStatus> {
    if !state.config.is_license_endpoint_configured() {
        return Err(AppError::Config(
            "AIGRADER_LICENSE_ENDPOINT is not configured (see .env.example)".into(),
        ));
    }
    let resp = post(&state.config.license_endpoint, code).await?;
    match resp.status.as_str() {
        "active" => {
            apply_active(state, &resp)?;
            // Persist the code so the 24h heartbeat can re-validate silently.
            let conn = state.db.lock().expect("db lock poisoned");
            repo::config_set(&conn, K_ACTIVATION_CODE, code)?;
            drop(conn);
            current_status(state)
        }
        "expired" => Err(AppError::License("activation code has expired".into())),
        _ => Err(AppError::License("invalid activation code".into())),
    }
}

/// Silent background pulse. On "active" refreshes creds + clears grace; on
/// "expired"/"invalid" starts the grace clock; on network failure leaves the
/// grace clock to the staleness check below.
pub async fn heartbeat(state: &AppState) -> AppResult<LicenseStatus> {
    let code = {
        let conn = state.db.lock().expect("db lock poisoned");
        repo::config_get(&conn, K_ACTIVATION_CODE)?
    };
    let Some(code) = code else {
        return current_status(state); // not activated; nothing to pulse
    };

    let endpoint = format!("{}/pulse", state.config.license_endpoint);
    match post(&endpoint, &code).await {
        Ok(resp) if resp.status == "active" => {
            apply_active(state, &resp)?;
        }
        Ok(_) => {
            // Explicit expired/invalid verdict.
            begin_grace(state)?;
        }
        Err(_) => {
            // No internet: only enter grace once we're clearly stale
            // (last good heartbeat older than the heartbeat interval).
            let stale = {
                let conn = state.db.lock().expect("db lock poisoned");
                match repo::config_get(&conn, K_LAST_HEARTBEAT)?.and_then(|s| parse_ts(&s)) {
                    Some(last) => {
                        (Utc::now() - last).num_hours() > state.config.heartbeat_hours
                    }
                    None => true,
                }
            };
            if stale {
                begin_grace(state)?;
            }
        }
    }
    current_status(state)
}

/// Snapshot the status without any network call (used at boot and after ops).
pub fn current_status(state: &AppState) -> AppResult<LicenseStatus> {
    let ai_ready = state.has_valid_creds();
    let conn = state.db.lock().expect("db lock poisoned");
    compute_status(&conn, &state.config, ai_ready)
}

/// Whether grading is permitted right now. False when Locked or Unactivated.
pub fn grading_allowed(status: &LicenseStatus) -> bool {
    matches!(
        status,
        LicenseStatus::Active { .. } | LicenseStatus::Grace { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AppConfig {
        AppConfig {
            license_endpoint: "https://example.test/lic".into(),
            model: "gemini-2.5-pro".into(),
            heartbeat_hours: 24,
            grace_days: 7,
            dev_api_key: None,
        }
    }

    #[test]
    fn unactivated_by_default() {
        let conn = crate::db::open_in_memory().unwrap();
        let s = compute_status(&conn, &cfg(), false).unwrap();
        assert!(matches!(s, LicenseStatus::Unactivated));
    }

    #[test]
    fn grace_then_lock() {
        let conn = crate::db::open_in_memory().unwrap();
        repo::config_set(&conn, K_ACTIVATED, "true").unwrap();

        // grace started 2 days ago -> 5 days left
        let two_days_ago = (Utc::now() - Duration::days(2)).to_rfc3339();
        repo::config_set(&conn, K_GRACE_STARTED, &two_days_ago).unwrap();
        match compute_status(&conn, &cfg(), false).unwrap() {
            LicenseStatus::Grace { days_left, .. } => assert_eq!(days_left, 5),
            other => panic!("expected grace, got {other:?}"),
        }

        // grace started 8 days ago -> locked
        let eight_days_ago = (Utc::now() - Duration::days(8)).to_rfc3339();
        repo::config_set(&conn, K_GRACE_STARTED, &eight_days_ago).unwrap();
        assert!(matches!(
            compute_status(&conn, &cfg(), false).unwrap(),
            LicenseStatus::Locked
        ));
    }
}
