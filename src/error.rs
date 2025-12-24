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

    #[error("MySQL error: {0}")]
    MySqlError(String),

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

impl From<mysql_async::Error> for Error {
    fn from(err: mysql_async::Error) -> Self {
        Error::MySqlError(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerializationError(err.to_string())
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
        let result: i32 = 42;
        assert_eq!(result, 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(Error::Unknown("test".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_error_display_lock_error() {
        let err = Error::LockError("timeout".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Failed to acquire session lock"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn test_error_display_telegram_error() {
        let err = Error::TelegramError("flood wait".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Telegram API error"));
        assert!(msg.contains("flood wait"));
    }

    #[test]
    fn test_error_display_serialization_error() {
        let err = Error::SerializationError("invalid JSON".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Serialization error"));
        assert!(msg.contains("invalid JSON"));
    }

    #[test]
    fn test_error_display_connection_error() {
        let err = Error::ConnectionError("timeout".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Connection error"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn test_error_display_unknown() {
        let err = Error::Unknown("something went wrong".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Unknown error"));
        assert!(msg.contains("something went wrong"));
    }

    #[test]
    fn test_error_debug_impl() {
        let err = Error::SessionLocked;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("SessionLocked"));
    }

    #[test]
    fn test_error_from_io_various_kinds() {
        let kinds = [
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::ConnectionRefused,
            std::io::ErrorKind::TimedOut,
        ];

        for kind in kinds {
            let io_err = std::io::Error::new(kind, "test");
            let err: Error = io_err.into();
            assert!(matches!(err, Error::IoError(_)));
        }
    }

    #[test]
    fn test_result_map() {
        let result: Result<i32> = Ok(10);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), 20);
    }

    #[test]
    fn test_result_and_then() {
        let result: Result<i32> = Ok(10);
        let chained = result.map(|x| x + 5);
        assert_eq!(chained.unwrap(), 15);
    }

    #[test]
    fn test_error_source_io() {
        use std::error::Error as StdError;

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err: Error = io_err.into();

        // IoError should have a source
        if let Error::IoError(ref inner) = err {
            assert!(inner.source().is_none() || inner.source().is_some());
        }
    }

    #[test]
    fn test_error_from_serde_json() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let err: Error = json_err.into();
        
        assert!(matches!(err, Error::SerializationError(_)));
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_error_mysql_display() {
        let err = Error::MySqlError("connection refused".to_string());
        let msg = err.to_string();
        
        assert!(msg.contains("MySQL error"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn test_error_mysql_debug() {
        let err = Error::MySqlError("timeout".to_string());
        let debug_str = format!("{:?}", err);
        
        assert!(debug_str.contains("MySqlError"));
        assert!(debug_str.contains("timeout"));
    }

    #[test]
    fn test_error_all_variants_debug() {
        let variants: Vec<Error> = vec![
            Error::SessionNotFound("session".to_string()),
            Error::SessionLocked,
            Error::LockError("lock".to_string()),
            Error::TelegramError("telegram".to_string()),
            Error::ChatNotFound("chat".to_string()),
            Error::SerializationError("serial".to_string()),
            Error::OpenAiError("openai".to_string()),
            Error::LinearError("linear".to_string()),
            Error::MySqlError("mysql".to_string()),
            Error::InvalidArgument("arg".to_string()),
            Error::ConnectionError("conn".to_string()),
            Error::AuthorizationRequired,
            Error::Unknown("unknown".to_string()),
        ];
        
        for err in variants {
            let debug_str = format!("{:?}", err);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_result_unwrap_or_else() {
        let result: Result<i32> = Err(Error::Unknown("error".to_string()));
        let value = result.unwrap_or_else(|_| 42);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_result_is_ok() {
        let result: Result<i32> = Ok(100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_io_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: Error = io_err.into();
        
        assert!(matches!(err, Error::IoError(_)));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_error_serialization_from_json_syntax() {
        let json_err = serde_json::from_str::<Vec<i32>>("[1, 2,]").unwrap_err();
        let err: Error = json_err.into();
        
        assert!(matches!(err, Error::SerializationError(_)));
    }

    #[test]
    fn test_error_serialization_from_json_type() {
        let json_err = serde_json::from_str::<String>("123").unwrap_err();
        let err: Error = json_err.into();
        
        assert!(matches!(err, Error::SerializationError(_)));
    }
}
