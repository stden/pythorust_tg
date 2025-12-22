//! Send message to Telegram user or chat
//!
//! Отправка сообщений в Telegram

use grammers_client::types::peer::Peer;

use crate::config::{ChatEntity, Config};
use crate::error::{Error, Result};
use crate::session::{get_client, SessionLock};

#[derive(Debug, PartialEq, Eq)]
enum Target<'a> {
    Username(&'a str),
    UserId(i64),
    ChatName(&'a str),
}

fn parse_target(target: &str) -> Target<'_> {
    if target.starts_with('@') {
        Target::Username(target)
    } else if let Ok(user_id) = target.parse::<i64>() {
        Target::UserId(user_id)
    } else {
        Target::ChatName(target)
    }
}

/// Get ID from Peer
fn get_peer_id(peer: &Peer) -> i64 {
    match peer {
        Peer::User(u) => u.raw.id(),
        Peer::Group(g) => match &g.raw {
            grammers_tl_types::enums::Chat::Chat(c) => c.id,
            grammers_tl_types::enums::Chat::Forbidden(f) => f.id,
            _ => 0,
        },
        Peer::Channel(c) => c.raw.id,
    }
}

/// Send a message to a user by ID
pub async fn send_to_user(user_id: i64, message: &str) -> Result<()> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    // Find user in dialogs
    let mut dialogs = client.iter_dialogs();
    while let Some(dialog) = dialogs.next().await.transpose() {
        if let Ok(dialog) = dialog {
            if let Peer::User(_) = &dialog.peer {
                if get_peer_id(&dialog.peer) == user_id {
                    client
                        .send_message(&dialog.peer, message)
                        .await
                        .map_err(|e| Error::TelegramError(e.to_string()))?;
                    println!("✓ Сообщение отправлено пользователю {}", user_id);
                    return Ok(());
                }
            }
        }
    }

    Err(Error::InvalidArgument(format!(
        "Пользователь {} не найден",
        user_id
    )))
}

/// Send a message to a chat by name (from config)
pub async fn send_to_chat(chat_name: &str, message: &str) -> Result<()> {
    let config = Config::new();
    let chat_entity = config
        .get_chat(chat_name)
        .ok_or_else(|| Error::InvalidArgument(format!("Чат '{}' не найден в конфиге", chat_name)))?
        .clone();

    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    match chat_entity {
        ChatEntity::Channel(id) | ChatEntity::Chat(id) => {
            // Find channel/chat in dialogs
            let mut dialogs = client.iter_dialogs();
            while let Some(dialog) = dialogs.next().await.transpose() {
                if let Ok(dialog) = dialog {
                    let is_channel_or_group =
                        matches!(&dialog.peer, Peer::Channel(_) | Peer::Group(_));
                    if is_channel_or_group && get_peer_id(&dialog.peer) == id {
                        client
                            .send_message(&dialog.peer, message)
                            .await
                            .map_err(|e| Error::TelegramError(e.to_string()))?;
                        println!("✓ Сообщение отправлено в {}", chat_name);
                        return Ok(());
                    }
                }
            }
            Err(Error::InvalidArgument(format!("Чат {} не найден", id)))
        }
        ChatEntity::Username(username) => {
            let entity = client
                .resolve_username(&username)
                .await
                .map_err(|e| Error::TelegramError(e.to_string()))?
                .ok_or_else(|| {
                    Error::InvalidArgument(format!("Username @{} не найден", username))
                })?;

            client
                .send_message(&entity, message)
                .await
                .map_err(|e| Error::TelegramError(e.to_string()))?;
            println!("✓ Сообщение отправлено @{}", username);
            Ok(())
        }
        ChatEntity::UserId(id) => {
            // Need to drop lock before recursive call
            drop(_lock);
            send_to_user(id, message).await
        }
    }
}

/// Send a message to username directly
pub async fn send_to_username(username: &str, message: &str) -> Result<()> {
    let _lock = SessionLock::acquire()?;
    let client = get_client().await?;

    let username = username.strip_prefix('@').unwrap_or(username);

    let entity = client
        .resolve_username(username)
        .await
        .map_err(|e| Error::TelegramError(e.to_string()))?
        .ok_or_else(|| Error::InvalidArgument(format!("Username @{} не найден", username)))?;

    client
        .send_message(&entity, message)
        .await
        .map_err(|e| Error::TelegramError(e.to_string()))?;

    println!("✓ Сообщение отправлено @{}", username);
    Ok(())
}

/// CLI entry point
pub async fn run(target: &str, message: &str) -> Result<()> {
    match parse_target(target) {
        Target::Username(username) => send_to_username(username, message).await,
        Target::UserId(user_id) => send_to_user(user_id, message).await,
        Target::ChatName(chat_name) => send_to_chat(chat_name, message).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_target_detects_usernames() {
        assert_eq!(parse_target("@alice"), Target::Username("@alice"));
        assert_eq!(parse_target("@"), Target::Username("@"));
    }

    #[test]
    fn parse_target_detects_numeric_user_ids() {
        assert_eq!(parse_target("123"), Target::UserId(123));
        assert_eq!(parse_target("-7"), Target::UserId(-7));
        assert_eq!(parse_target("001"), Target::UserId(1));
    }

    #[test]
    fn parse_target_falls_back_to_chat_name() {
        assert_eq!(parse_target("chat_alpha"), Target::ChatName("chat_alpha"));
    }
}
