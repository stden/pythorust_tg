//! N8N monitoring and backup utilities.
//!
//! Provides:
//! - Service health monitoring with auto-restart
//! - Configuration backup and restore

pub mod backup;
pub mod monitor;

pub use backup::N8NBackup;
pub use monitor::N8NMonitor;
