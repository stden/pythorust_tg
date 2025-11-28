//! Chat operations and entity resolution

use grammers_client::types::peer::Peer;
use grammers_client::Client;

use crate::config::ChatEntity;
use crate::error::{Error, Result};

/// Resolve a ChatEntity to an actual Peer
pub async fn resolve_chat(client: &Client, entity: &ChatEntity) -> Result<Peer> {
    match entity {
        ChatEntity::Channel(target_id) => {
            // For channels, we need to get the entity by ID
            // This requires the channel to be in the user's dialogs
            let mut dialogs = client.iter_dialogs();

            while let Some(dialog) = dialogs
                .next()
                .await
                .map_err(|e| Error::TelegramError(e.to_string()))?
            {
                if let Peer::Channel(channel) = &dialog.peer {
                    // Compare using raw ID from the underlying TL type
                    let channel_id = channel.raw.id;
                    if channel_id == *target_id {
                        return Ok(Peer::Channel(channel.clone()));
                    }
                }
            }

            Err(Error::ChatNotFound(format!(
                "Channel {} not found in dialogs",
                target_id
            )))
        }
        ChatEntity::Chat(target_id) => {
            let mut dialogs = client.iter_dialogs();

            while let Some(dialog) = dialogs
                .next()
                .await
                .map_err(|e| Error::TelegramError(e.to_string()))?
            {
                if let Peer::Group(group) = &dialog.peer {
                    // Groups can be either Chat or Channel (megagroup)
                    let group_id = match &group.raw {
                        grammers_tl_types::enums::Chat::Empty(c) => c.id,
                        grammers_tl_types::enums::Chat::Chat(c) => c.id,
                        grammers_tl_types::enums::Chat::Forbidden(c) => c.id,
                        grammers_tl_types::enums::Chat::Channel(c) => c.id,
                        grammers_tl_types::enums::Chat::ChannelForbidden(c) => c.id,
                    };
                    if group_id == *target_id {
                        return Ok(Peer::Group(group.clone()));
                    }
                }
            }

            Err(Error::ChatNotFound(format!(
                "Chat {} not found in dialogs",
                target_id
            )))
        }
        ChatEntity::Username(username) => client
            .resolve_username(username)
            .await
            .map_err(|e| Error::TelegramError(e.to_string()))?
            .ok_or_else(|| Error::ChatNotFound(format!("Username @{} not found", username))),
        ChatEntity::UserId(target_id) => {
            let mut dialogs = client.iter_dialogs();

            while let Some(dialog) = dialogs
                .next()
                .await
                .map_err(|e| Error::TelegramError(e.to_string()))?
            {
                if let Peer::User(user) = &dialog.peer {
                    let user_id = user.raw.id();
                    if user_id == *target_id {
                        return Ok(Peer::User(user.clone()));
                    }
                }
            }

            Err(Error::ChatNotFound(format!(
                "User {} not found in dialogs",
                target_id
            )))
        }
    }
}

/// Get the display name for a peer
pub fn peer_name(peer: &Peer) -> String {
    peer.name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Find chat by name (searches in config, then tries as username)
pub async fn find_chat(client: &Client, name: &str) -> Result<Peer> {
    use crate::config::Config;

    // Try to find in config first
    let config = Config::new();
    if let Some(entity) = config.chats.get(name) {
        return resolve_chat(client, entity).await;
    }

    // Try as username (with or without @)
    let username = name.trim_start_matches('@');
    client
        .resolve_username(username)
        .await
        .map_err(|e| Error::TelegramError(e.to_string()))?
        .ok_or_else(|| Error::ChatNotFound(format!("Chat '{}' not found", name)))
}
