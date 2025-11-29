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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_session_not_found() {
        let err = Error::SessionNotFound("test.session".to_string());
        assert!(err.to_string().contains("Session file not found"));
        assert!(err.to_string().contains("test.session"));
    }

    #[test]
    fn test_error_display_session_locked() {
        let err = Error::SessionLocked;
        assert!(err.to_string().contains("locked by another process"));
    }

    #[test]
    fn test_error_display_chat_not_found() {
        let err = Error::ChatNotFound("test_chat".to_string());
        assert!(err.to_string().contains("Chat not found"));
        assert!(err.to_string().contains("test_chat"));
    }

    #[test]
    fn test_error_display_openai_error() {
        let err = Error::OpenAiError("rate limit exceeded".to_string());
        assert!(err.to_string().contains("OpenAI"));
        assert!(err.to_string().contains("rate limit"));
    }

    #[test]
    fn test_error_display_linear_error() {
        let err = Error::LinearError("API key invalid".to_string());
        assert!(err.to_string().contains("Linear"));
        assert!(err.to_string().contains("API key"));
    }

    #[test]
    fn test_error_display_invalid_argument() {
        let err = Error::InvalidArgument("missing required field".to_string());
        assert!(err.to_string().contains("Invalid argument"));
    }

    #[test]
    fn test_error_display_authorization_required() {
        let err = Error::AuthorizationRequired;
        assert!(err.to_string().contains("Authorization required"));
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::IoError(_)));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(Error::Unknown("test".to_string()));
        assert!(result.is_err());
    }
}
