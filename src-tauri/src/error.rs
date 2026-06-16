//! Unified error type. Implements `Serialize` so it can cross the Tauri
//! command boundary into the React layer as a plain string message.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("image processing error: {0}")]
    Image(#[from] image::ImageError),

    #[error("license error: {0}")]
    License(String),

    #[error("not activated")]
    NotActivated,

    #[error("application is locked: grace period expired")]
    Locked,

    #[error("configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
