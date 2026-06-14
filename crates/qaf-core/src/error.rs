use thiserror::Error;

/// Top-level error type for the Qaf workspace.
#[derive(Debug, Error)]
pub enum QafError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("{0}")]
    Other(String),
}
