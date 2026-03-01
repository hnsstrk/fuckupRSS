use std::sync::PoisonError;
use thiserror::Error;

/// Unified error type for the fuckupRSS application.
/// Wraps all common error types and provides automatic conversion to String
/// for Tauri command compatibility.
///
/// Implements `serde::Serialize` (required by Tauri 2.x for command error types)
/// by serializing as the Display string representation of the error.
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

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Serialize FuckupError as its Display string for Tauri 2.x command compatibility.
/// Tauri 2.x requires error types in command return values to implement Serialize.
impl serde::Serialize for FuckupError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
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

/// Convenience macro to convert any error to String via `.map_err(|e| e.to_string())?`.
///
/// Usage:
/// ```rust
/// let result = some_operation().map_err(|e| e.to_string())?;
/// // becomes:
/// let result = to_cmd_err!(some_operation());
/// ```
#[macro_export]
macro_rules! to_cmd_err {
    ($expr:expr) => {
        $expr.map_err(|e| e.to_string())?
    };
}
