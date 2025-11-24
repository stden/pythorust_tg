//! Export utilities for saving chat messages to files

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use chrono::{DateTime, Utc};
use grammers_client::types::Message;
use grammers_client::types::peer::Peer;

use crate::config::KNOWN_SENDERS;
use crate::error::Result;

/// Export context for writing messages to a file
pub struct ExportWriter {
    writer: BufWriter<File>,
    sender_cache: HashMap<i64, String>,
}

impl ExportWriter {
    /// Create a new export writer
    pub fn new(chat_name: &str) -> Result<Self> {
        let filename = format!("{}.md", chat_name);
        let file = File::create(&filename)?;
        let writer = BufWriter::new(file);

        // Initialize sender cache with known senders
        let mut sender_cache = HashMap::new();
        for (id, name) in KNOWN_SENDERS.iter() {
            sender_cache.insert(*id, name.to_string());
        }

        Ok(Self {
            writer,
            sender_cache,
        })
    }

    /// Write the header for the export file
    pub fn write_header(&mut self, prompt: &str) -> Result<()> {
        writeln!(self.writer, "{}", prompt)?;
        Ok(())
    }

    /// Get or resolve sender name
    pub fn get_sender_name(&mut self, sender_id: i64, message: &Message) -> String {
        if let Some(name) = self.sender_cache.get(&sender_id) {
            return name.clone();
        }

        // Try to get sender from message
        let name = if let Some(sender) = message.sender() {
            match sender {
                Peer::User(user) => {
                    let name = user.full_name();
                    if name.is_empty() {
                        user.username()
                            .map(|u| format!("@{}", u))
                            .unwrap_or_else(|| "Неизвестный".to_string())
                    } else {
                        name
                    }
                }
                Peer::Group(g) => g.title().unwrap_or("Group").to_string(),
                Peer::Channel(c) => c.title().to_string(),
            }
        } else {
            "Неизвестный отправитель".to_string()
        };

        self.sender_cache.insert(sender_id, name.clone());
        name
    }

    /// Write a single message to the file
    pub fn write_message(
        &mut self,
        sender_name: &str,
        text: &str,
        emojis: &str,
        timestamp: Option<DateTime<Utc>>,
        media_path: Option<&str>,
    ) -> Result<()> {
        let line = if let Some(ts) = timestamp {
            let ts_str = ts.format("%d.%m.%Y %H:%M:%S").to_string();
            if let Some(path) = media_path {
                format!("{} {}: {} {} {}", ts_str, sender_name, text, emojis, path)
            } else {
                format!("{}: {} {}", sender_name, text, emojis)
            }
        } else if let Some(path) = media_path {
            format!("{}: {} {} {}", sender_name, text, emojis, path)
        } else {
            format!("{}: {} {}", sender_name, text, emojis)
        };

        writeln!(self.writer, "{}", line.trim())?;
        Ok(())
    }

    /// Flush and close the writer
    pub fn finish(mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

/// Create media directory for a chat
pub fn create_media_dir(chat_name: &str) -> Result<()> {
    fs::create_dir_all(chat_name)?;
    Ok(())
}

/// Check if media directory exists
pub fn media_dir_exists(chat_name: &str) -> bool {
    Path::new(chat_name).is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::sync::{LazyLock, Mutex};
    use tempfile::TempDir;

    static WORKDIR_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct DirGuard {
        original: std::path::PathBuf,
    }

    impl DirGuard {
        fn enter(temp: &TempDir) -> crate::error::Result<Self> {
            let original = std::env::current_dir()?;
            std::env::set_current_dir(temp.path())?;
            Ok(Self { original })
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    #[test]
    fn writes_header_and_messages() -> crate::error::Result<()> {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir()?;
        let _guard = DirGuard::enter(&temp)?;

        let mut writer = ExportWriter::new("chat_test")?;
        writer.write_header("Prompt line")?;

        let timestamp = Utc
            .with_ymd_and_hms(2024, 11, 22, 10, 30, 5)
            .single()
            .expect("timestamp");
        writer.write_message("Alice", "Hello", "🔥", Some(timestamp), Some("media/photo.jpg"))?;
        writer.write_message("Bob", "No media", "", None, None)?;
        writer.finish()?;

        let contents = std::fs::read_to_string("chat_test.md")?;
        assert!(contents.starts_with("Prompt line\n"));
        assert!(contents.contains("22.11.2024 10:30:05 Alice: Hello 🔥 media/photo.jpg"));
        assert!(contents.contains("Bob: No media"));

        Ok(())
    }

    #[test]
    fn media_dir_helpers_create_directories() -> crate::error::Result<()> {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir()?;
        let _guard = DirGuard::enter(&temp)?;

        assert!(!media_dir_exists("media_chat"));
        create_media_dir("media_chat")?;
        assert!(media_dir_exists("media_chat"));

        Ok(())
    }

    #[test]
    fn write_message_with_media_without_timestamp() -> crate::error::Result<()> {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir()?;
        let _guard = DirGuard::enter(&temp)?;

        let mut writer = ExportWriter::new("media_only")?;
        writer.write_header("Header")?;
        writer.write_message("User", "Attachment", "😀", None, Some("media/clip.mp4"))?;
        writer.finish()?;

        let output = std::fs::read_to_string("media_only.md")?;
        let lines: Vec<_> = output.lines().collect();
        assert_eq!(lines[0], "Header");
        assert_eq!(lines[1], "User: Attachment 😀 media/clip.mp4");

        Ok(())
    }

    #[test]
    fn write_message_with_timestamp_without_media() -> crate::error::Result<()> {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir()?;
        let _guard = DirGuard::enter(&temp)?;

        let mut writer = ExportWriter::new("timestamp_only")?;
        writer.write_header("Header")?;

        let ts = Utc
            .with_ymd_and_hms(2024, 12, 1, 8, 15, 0)
            .single()
            .expect("timestamp");
        writer.write_message("User", "Ping", "", Some(ts), None)?;
        writer.finish()?;

        let output = std::fs::read_to_string("timestamp_only.md")?;
        let lines: Vec<_> = output.lines().collect();
        assert_eq!(lines[0], "Header");
        assert_eq!(lines[1], "User: Ping");

        Ok(())
    }

    #[test]
    fn create_media_dir_builds_nested_paths() -> crate::error::Result<()> {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir()?;
        let _guard = DirGuard::enter(&temp)?;

        assert!(!media_dir_exists("nested/dir"));
        create_media_dir("nested/dir")?;
        assert!(media_dir_exists("nested/dir"));
        assert!(media_dir_exists("nested"));

        Ok(())
    }

    #[test]
    fn write_message_trims_excess_whitespace() -> crate::error::Result<()> {
        let _lock = WORKDIR_LOCK.lock().unwrap();
        let temp = tempfile::tempdir()?;
        let _guard = DirGuard::enter(&temp)?;

        let mut writer = ExportWriter::new("trim_spaces")?;
        writer.write_header("Header")?;
        writer.write_message("User", "Hello  ", "  ", None, None)?;
        writer.finish()?;

        let output = std::fs::read_to_string("trim_spaces.md")?;
        let lines: Vec<_> = output.lines().collect();
        assert_eq!(lines[0], "Header");
        assert_eq!(lines[1], "User: Hello");

        Ok(())
    }
}
