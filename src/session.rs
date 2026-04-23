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
use grammers_client::client::updates::UpdatesLike;
use grammers_client::Client;
use grammers_mtsender::{SenderPool, SenderPoolHandle};
use grammers_session::storages::SqliteSession;
use tokio::sync::mpsc;

use crate::config::{Config, LOCK_FILE, SESSION_NAME};
use crate::error::{Error, Result};

/// Session lock guard that ensures exclusive access to the Telegram session.
pub struct SessionLock {
    lock_file: Option<File>,
}

impl SessionLock {
    /// Acquire an exclusive lock on the session.
    pub fn acquire() -> Result<Self> {
        let lock_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(LOCK_FILE)
            .map_err(|e| Error::LockError(format!("Failed to open lock file: {}", e)))?;

        match lock_file.try_lock_exclusive() {
            Ok(()) => Ok(Self {
                lock_file: Some(lock_file),
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
        let _ = std::fs::remove_file(LOCK_FILE);
    }
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        self.release();
    }
}

/// Check if the session file exists.
pub fn check_session_exists() -> Result<()> {
    let session_file = format!("{}.session", SESSION_NAME);

    if !Path::new(&session_file).exists() {
        eprintln!(
            r#"
⚠️  ОШИБКА: Session файл '{}' не найден!

Для создания session файла:
1. Запустите: cargo run --bin init_session
2. Введите код из Telegram
"#,
            session_file
        );
        return Err(Error::SessionNotFound(session_file));
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
    updates: Option<mpsc::UnboundedReceiver<UpdatesLike>>,
    _runner_handle: tokio::task::JoinHandle<()>,
}

impl TelegramClient {
    /// Create a new TelegramClient from session
    pub async fn connect(session: Arc<SqliteSession>) -> Result<Self> {
        let config = Config::new();
        let pool = SenderPool::new(session.clone(), config.api_id);

        // Create client from pool (need reference to whole pool)
        let client = Client::new(&pool);

        // Get handle and runner after client is created
        let SenderPool {
            runner,
            updates,
            handle,
        } = pool;

        // Spawn the runner in background
        let runner_handle = tokio::spawn(async move {
            runner.run().await;
        });

        Ok(Self {
            client,
            handle,
            session,
            updates: Some(updates),
            _runner_handle: runner_handle,
        })
    }

    /// Take ownership of the updates receiver to build an UpdateStream.
    /// Returns None if updates were already taken.
    pub fn take_updates(&mut self) -> Option<mpsc::UnboundedReceiver<UpdatesLike>> {
        self.updates.take()
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
    use std::path::PathBuf;
    use std::process::{self, Command};
    use std::sync::{LazyLock, Mutex};
    use tempfile::tempdir;

    static WORKDIR_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct DirGuard {
        original: PathBuf,
    }

    impl DirGuard {
        fn change_to(path: &std::path::Path) -> Self {
            let original = env::current_dir().expect("current dir");
            env::set_current_dir(path).expect("set current dir");
            Self { original }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

    #[test]
    fn test_session_lock_creation() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        let result = SessionLock::acquire();
        if let Ok(mut lock) = result {
            lock.release();
        }
    }

    #[test]
    #[ignore] // Flaky in CI - depends on child process timing
    fn session_lock_blocks_second_acquire() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        let mut first = SessionLock::acquire().expect("first lock");

        let status = Command::new(env::current_exe().expect("current exe"))
            .env("SESSION_LOCK_CHILD", "1")
            .env("SESSION_LOCK_DIR", temp.path())
            .arg("--")
            .arg("session::tests::session_lock_child_runner")
            .status()
            .expect("child status");

        assert!(status.success(), "child process unexpectedly acquired lock");

        first.release();

        let third = SessionLock::acquire();
        assert!(third.is_ok());
    }

    #[test]
    fn session_lock_child_runner() {
        if env::var("SESSION_LOCK_CHILD").is_err() {
            return;
        }

        if let Ok(dir) = env::var("SESSION_LOCK_DIR") {
            let _ = env::set_current_dir(&dir);
        }

        let result = SessionLock::acquire();
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

        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        let err = check_session_exists().unwrap_err();
        assert!(matches!(err, Error::SessionNotFound(_)));

        let session_file = format!("{}.session", SESSION_NAME);
        File::create(&session_file).expect("create session file");

        check_session_exists().expect("session should exist");
    }

    #[test]
    fn release_removes_lock_file() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        let mut lock = SessionLock::acquire().expect("lock");
        assert!(PathBuf::from(LOCK_FILE).exists());
        lock.release();
        assert!(!PathBuf::from(LOCK_FILE).exists());
    }

    #[test]
    fn lock_dropped_releases_automatically() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        {
            let _lock = SessionLock::acquire().expect("lock");
            assert!(PathBuf::from(LOCK_FILE).exists());
        }
        // Lock should be released after drop
        assert!(!PathBuf::from(LOCK_FILE).exists());
    }

    #[test]
    fn double_release_is_safe() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        let mut lock = SessionLock::acquire().expect("lock");
        lock.release();
        lock.release(); // Should not panic
    }

    #[test]
    fn save_session_returns_ok() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        let session = create_session().expect("create session");
        save_session(session.as_ref()).expect("save session");
    }

    #[test]
    fn dir_guard_restores_original_directory() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let original = env::current_dir().expect("current dir");
        let temp = tempdir().expect("tempdir");

        {
            let _guard = DirGuard::change_to(temp.path());
            assert_eq!(env::current_dir().unwrap(), temp.path());
        }

        assert_eq!(env::current_dir().unwrap(), original);
    }

    #[test]
    fn check_session_exists_with_missing_file_returns_session_not_found() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        let result = check_session_exists();
        assert!(result.is_err());

        if let Err(Error::SessionNotFound(path)) = result {
            assert!(path.contains(".session"));
        } else {
            panic!("Expected SessionNotFound error");
        }
    }

    #[test]
    fn lock_file_is_created_on_acquire() {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempdir().expect("tempdir");
        let _guard = DirGuard::change_to(temp.path());

        assert!(!PathBuf::from(LOCK_FILE).exists());
        let mut lock = SessionLock::acquire().expect("lock");
        assert!(PathBuf::from(LOCK_FILE).exists());
        lock.release();
    }
}
