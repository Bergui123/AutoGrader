//! Runtime configuration. Values come from the environment (a `.env` next to
//! the binary in dev, or real env vars in production). Secrets such as the
//! Vertex AI access token are NEVER read here — those arrive at activation
//! time and live only in RAM (see `state::EphemeralCreds`).

use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Google Cloud Function that validates activation codes and emits
    /// ephemeral AI credentials (the Gemini API key) + heartbeat verdicts.
    pub license_endpoint: String,
    /// Gemini model id, e.g. `gemini-2.5-pro`.
    pub model: String,
    pub heartbeat_hours: i64,
    pub grace_days: i64,
    /// DEV ONLY: if set, the app injects this Gemini API key into RAM creds at
    /// boot and marks itself activated, so the full grading flow can be tested
    /// locally without the production Cloud Function. Leave unset in prod.
    pub dev_api_key: Option<String>,
}

fn var_or(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn num_or(key: &str, default: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

impl AppConfig {
    pub fn from_env() -> Self {
        // Best-effort load of a sibling `.env` so dev works without exporting.
        load_dotenv_best_effort();

        AppConfig {
            license_endpoint: var_or(
                "AIGRADER_LICENSE_ENDPOINT",
                // Placeholder; replace via .env. Activation will fail loudly
                // until a real endpoint is configured.
                "https://localhost/__unconfigured_license_endpoint__",
            ),
            model: var_or("AIGRADER_MODEL", "gemini-2.5-pro"),
            heartbeat_hours: num_or("AIGRADER_HEARTBEAT_HOURS", 24),
            grace_days: num_or("AIGRADER_GRACE_DAYS", 7),
            dev_api_key: env::var("AIGRADER_DEV_API_KEY")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
        }
    }

    pub fn is_license_endpoint_configured(&self) -> bool {
        !self.license_endpoint.contains("__unconfigured")
    }
}

/// Minimal `.env` loader (no external crate). Parses `KEY=VALUE` lines,
/// ignoring comments and blank lines, and only sets vars not already present.
fn load_dotenv_best_effort() {
    for candidate in [".env", "../.env"] {
        if let Ok(contents) = std::fs::read_to_string(candidate) {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let k = k.trim();
                    let v = v.trim().trim_matches('"').trim_matches('\'');
                    if env::var(k).is_err() {
                        env::set_var(k, v);
                    }
                }
            }
            return;
        }
    }
}
