//! Command implementations
//!
//! All Telegram CLI commands are implemented here.
//! Each module corresponds to a subcommand in the CLI.

pub mod active_chats;
pub mod autoanswer;
pub mod chat_analyzer;
pub mod crm;
pub mod delete_zoom;
pub mod digest;
pub mod download_chat;
pub mod download_user_chat;
pub mod export;
pub mod hunt;
pub mod index;
pub mod init_session;
pub mod lightrag;
pub mod like;
pub mod linear;
pub mod linear_bot;
pub mod list_chats;
pub mod moderate;
pub mod monitor;
pub mod n8n;
pub mod read;
pub mod search;
pub mod send_message;
pub mod send_viral;
pub mod tg;

// Re-export commonly used types
pub use active_chats::run as active_chats_run;
pub use autoanswer::run as autoanswer_run;
pub use chat_analyzer::{run as chat_analyzer_run, AnalyzerConfig as ChatAnalyzerConfig};
pub use crm::{parse_chat as crm_parse, CrmConfig};
pub use digest::{run as digest_run, DigestConfig};
pub use hunt::{hunt_users, HuntCriteria};
pub use like::run as like_run;
pub use linear::{run as linear_run, LinearArgs};
pub use list_chats::run as list_chats_run;
pub use moderate::{run as moderate_run, ModerateConfig};
pub use read::run as read_run;
pub use tg::run as tg_run;
