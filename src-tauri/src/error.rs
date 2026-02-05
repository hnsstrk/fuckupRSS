use std::sync::PoisonError;
use thiserror::Error;

/// Unified error type for the fuckupRSS application.
/// Wraps all common error types and provides automatic conversion to String
/// for Tauri command compatibility.
#[derive(Error, Debug)]
pub enum FuckupError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Database lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest_new::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Generic(String),
}

impl<T> From<PoisonError<T>> for FuckupError {
    fn from(err: PoisonError<T>) -> Self {
        FuckupError::LockPoisoned(err.to_string())
    }
}

// Allow converting from FuckupError to String for Tauri command compatibility
impl From<FuckupError> for String {
    fn from(err: FuckupError) -> Self {
        err.to_string()
    }
}

/// Type alias for Results returned by Tauri commands
pub type CmdResult<T> = Result<T, FuckupError>;
