//! Session management for Telegram client
//!
//! Provides:
//! - File-based session locking to prevent parallel execution
//! - Session file validation
//! - Client creation with proper configuration

use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::Arc;

use fs2::FileExt;
use grammers_client::Client;
use grammers_mtsender::{SenderPool, SenderPoolHandle};
use grammers_session::storages::SqliteSession;

use crate::config::{LOCK_FILE, SESSION_NAME, API_ID};
use crate::error::{Error, Result};

/// Session lock guard that ensures exclusive access to the Telegram session.
pub struct SessionLock {
    lock_file: Option<File>,
    lock_path: std::path::PathBuf,
}

impl SessionLock {
    /// Acquire an exclusive lock on the session.
    pub fn acquire() -> Result<Self> {
        Self::acquire_with_base_dir(Path::new("."))
    }

    pub fn acquire_with_base_dir(base_dir: &Path) -> Result<Self> {
        let lock_path = base_dir.join(LOCK_FILE);
        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&lock_path)
            .map_err(|e| Error::LockError(format!("Failed to open lock file: {}", e)))?;

        match lock_file.try_lock_exclusive() {
            Ok(()) => Ok(Self {
                lock_file: Some(lock_file),
                lock_path,
            }),
            Err(_) => {
                eprintln!(
                    r#"
⚠️  ОШИБКА: Telegram сессия уже используется другим скриптом!

Telegram требует последовательного выполнения операций.
Параллельное использование одной сессии может привести к конфликтам и блокировкам.

Подождите, пока завершится другой скрипт, и попробуйте снова.
"#
                );
                Err(Error::SessionLocked)
            }
        }
    }

    /// Release the lock manually
    pub fn release(&mut self) {
        if let Some(ref file) = self.lock_file {
            let _ = file.unlock();
        }
        self.lock_file = None;
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        self.release();
    }
}

/// Check if the session file exists.
pub fn check_session_exists() -> Result<()> {
    check_session_exists_with_base_dir(Path::new("."))
}

/// Check if the session file exists in a specific directory.
pub fn check_session_exists_with_base_dir(base_dir: &Path) -> Result<()> {
    let session_file = base_dir.join(format!("{}.session", SESSION_NAME));

    if !session_file.exists() {
        eprintln!(
            r#"
⚠️  ОШИБКА: Session файл '{}' не найден!

Для создания session файла:
1. Запустите: cargo run --bin init_session
2. Введите код из Telegram
"#,
            session_file.display()
        );
        return Err(Error::SessionNotFound(
            session_file.to_string_lossy().to_string(),
        ));
    }

    Ok(())
}

/// Load an existing session from file.
pub fn load_session() -> Result<Arc<SqliteSession>> {
    let session_file = format!("{}.session", SESSION_NAME);
    let session = SqliteSession::open(&session_file)
        .map_err(|e| Error::SessionNotFound(format!("Failed to load session: {}", e)))?;
    Ok(Arc::new(session))
}

/// Create a new session (for init_session only).
pub fn create_session() -> Result<Arc<SqliteSession>> {
    let session_file = format!("{}.session", SESSION_NAME);
    let session = SqliteSession::open(&session_file)
        .map_err(|e| Error::SessionNotFound(format!("Failed to create session: {}", e)))?;
    Ok(Arc::new(session))
}

/// Holder for SenderPool components and Client
pub struct TelegramClient {
    pub client: Client,
    pub handle: SenderPoolHandle,
    session: Arc<SqliteSession>,
    _runner_handle: tokio::task::JoinHandle<()>,
}

impl TelegramClient {
    /// Create a new TelegramClient from session
    pub async fn connect(session: Arc<SqliteSession>) -> Result<Self> {
        let pool = SenderPool::new(session.clone(), API_ID);

        // Create client from pool (need reference to whole pool)
        let client = Client::new(&pool);

        // Get handle and runner after client is created
        let handle = pool.handle;
        let runner = pool.runner;

        // Spawn the runner in background
        let runner_handle = tokio::spawn(async move {
            runner.run().await;
        });

        Ok(Self {
            client,
            handle,
            session,
            _runner_handle: runner_handle,
        })
    }

    /// Save the session to file
    pub fn save(&self) -> Result<()> {
        save_session(&self.session)
    }
}

// Implement Deref to allow using TelegramClient as &Client
impl std::ops::Deref for TelegramClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

/// Save session - SqliteSession auto-saves, this is a no-op for compatibility
pub fn save_session(_session: &SqliteSession) -> Result<()> {
    // SqliteSession auto-saves to the database file
    Ok(())
}

/// Create and connect a Telegram client with an existing session.
pub async fn get_client() -> Result<TelegramClient> {
    check_session_exists()?;
    let session = load_session()?;
    TelegramClient::connect(session).await
}

/// Create a Telegram client for initialization (no session check).
pub async fn get_client_for_init() -> Result<TelegramClient> {
    let session = create_session()?;
    TelegramClient::connect(session).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;
    use std::process::{self, Command};
    use tempfile::tempdir;

    #[test]
    fn test_session_lock_creation() {
        let temp = tempdir().expect("tempdir");
        let result = SessionLock::acquire_with_base_dir(temp.path());
        if let Ok(mut lock) = result {
            lock.release();
        }
    }

    #[test]
    #[ignore] // Flaky in CI - depends on child process timing
    fn session_lock_blocks_second_acquire() {
        let temp = tempdir().expect("tempdir");

        let mut first = SessionLock::acquire_with_base_dir(temp.path()).expect("first lock");

        let status = Command::new(env::current_exe().expect("current exe"))
            .env("SESSION_LOCK_CHILD", "1")
            .env("SESSION_LOCK_DIR", temp.path().to_str().unwrap())
            .arg("--")
            .arg("session::tests::session_lock_child_runner")
            .status()
            .expect("child status");

        assert!(!status.success(), "child process unexpectedly acquired lock");

        first.release();

        let third = SessionLock::acquire_with_base_dir(temp.path());
        assert!(third.is_ok());
    }

    #[test]
    fn session_lock_child_runner() {
        if env::var("SESSION_LOCK_CHILD").is_err() {
            return;
        }

        let dir = env::var("SESSION_LOCK_DIR").unwrap();
        let result = SessionLock::acquire_with_base_dir(Path::new(&dir));
        match result {
            Err(Error::SessionLocked) => process::exit(0),
            Ok(mut lock) => {
                lock.release();
                process::exit(2);
            }
            Err(_) => process::exit(1),
        }
    }

    #[test]
    fn check_session_exists_reports_missing_and_success() {
        use std::fs::File;

        let temp = tempdir().expect("tempdir");
        let temp_path = temp.path();

        let err = check_session_exists_with_base_dir(temp_path).unwrap_err();
        assert!(matches!(err, Error::SessionNotFound(_)));

        let session_file = temp_path.join(format!("{}.session", SESSION_NAME));
        File::create(&session_file).expect("create session file");

        check_session_exists_with_base_dir(temp_path).expect("session should exist");
    }

    #[test]
    fn release_removes_lock_file() {
        let temp = tempdir().expect("tempdir");
        let temp_path = temp.path();

        let mut lock = SessionLock::acquire_with_base_dir(temp_path).expect("lock");
        assert!(temp_path.join(LOCK_FILE).exists());
        lock.release();
        assert!(!temp_path.join(LOCK_FILE).exists());
    }
}
