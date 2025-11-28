//! Error types for the Telegram reader

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Session file not found: {0}")]
    SessionNotFound(String),

    #[error("Session is locked by another process")]
    SessionLocked,

    #[error("Failed to acquire session lock: {0}")]
    LockError(String),

    #[error("Telegram API error: {0}")]
    TelegramError(String),

    #[error("Chat not found in configuration: {0}")]
    ChatNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("OpenAI API error: {0}")]
    OpenAiError(String),

    #[error("Linear API error: {0}")]
    LinearError(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authorization required")]
    AuthorizationRequired,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<grammers_client::InvocationError> for Error {
    fn from(err: grammers_client::InvocationError) -> Self {
        Error::TelegramError(err.to_string())
    }
}
