use std::fmt;

/// Application-level error type
#[derive(Debug)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn invalid_tx(msg: impl Into<String>) -> Self {
        Self::new("INVALID_TX", msg)
    }

    pub fn invalid_undo(msg: impl Into<String>) -> Self {
        Self::new("INVALID_UNDO", msg)
    }

    pub fn invalid_block(msg: impl Into<String>) -> Self {
        Self::new("INVALID_BLOCK", msg)
    }

    pub fn io_error(msg: impl Into<String>) -> Self {
        Self::new("IO_ERROR", msg)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::io_error(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::new("JSON_ERROR", e.to_string())
    }
}
